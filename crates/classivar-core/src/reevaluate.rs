//! Re-evaluating a reported variant: what did earlier records use, what do the
//! rules and evidence say today, and what changed?
//!
//! The answer is a candidate, never a decision. The curator reviews the reasons.

use crate::assess::{Applied, AssessError, Assessment, Counting};
use crate::case::GeneDecision;
use crate::condition::ConditionMatch;
use crate::model::{Class, Strength};
use crate::rules::{RuleSet, RuleSetRef};

/// What one earlier record (a lab, in ClinVar for example) said and used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    pub label: String,
    pub year: u16,
    pub reported_class: Class,
    /// A VUS sub-tier name the record used, if any. Free text, as recorded.
    pub reported_tier: Option<String>,
    /// The rule set the record was made with, if it is known.
    pub rule_set: Option<RuleSetRef>,
    /// Whether the record is about the case's condition (see `Condition::compare`).
    /// A record for a different condition is shown but never compared.
    pub condition: ConditionMatch,
    pub criteria: Vec<Applied>,
}

/// A record, scored again with today's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rescored {
    pub label: String,
    pub year: u16,
    pub reported_class: Class,
    pub reported_tier: Option<String>,
    pub condition: ConditionMatch,
    /// Each record is scored on its own, so one bad record does not hide the others.
    pub assessment: Result<Assessment, AssessError>,
}

/// Does a record's reported class fit its own evidence?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consistency {
    /// It matches the class from the points, or from the 2015 combining table.
    Matches,
    /// It matches neither.
    Differs,
    /// There is nothing to compare: the record has no evidence, its evidence
    /// contradicts itself, or it could not be scored.
    NotChecked,
}

impl Rescored {
    pub fn consistency(&self) -> Consistency {
        let Ok(assessment) = &self.assessment else { return Consistency::NotChecked };
        if assessment.class().is_none() {
            return Consistency::NotChecked;
        }
        if self.reported_class == assessment.points_class || Some(self.reported_class) == assessment.classic_class {
            Consistency::Matches
        } else {
            Consistency::Differs
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Today's class equals the latest record's class.
    Holds,
    UpgradeCandidate,
    DowngradeCandidate,
    /// Today's class differs from the latest record's, but no evidence changed.
    /// The difference comes from how the same evidence is combined or counted
    /// (points or the 2015 table, or a criterion that no longer counts).
    FrameworkDiffers,
    /// The latest record's class does not fit its own evidence, so comparing
    /// with it would mean little.
    RecordInconsistent,
    /// Every record names a different condition than the case, so no record
    /// can be compared with today's class.
    DifferentCondition,
}

/// One change between the latest record and today's evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// New evidence today that the latest record did not use.
    Added { code: String, strength: Strength },
    /// The latest record used it; today's evidence does not.
    Removed { code: String },
    /// Same criterion, different strength chosen.
    StrengthChanged { code: String, from: Strength, to: Strength },
    /// The record used the strength from the 2015 paper, and a later
    /// recommendation changed it. Today's strength follows the current rule.
    RuleChanged { code: String, from: Strength, to: Strength, source: String },
    /// The record was made with another rule set (or another version of it).
    /// Only a caller that supplies `Submission.rule_set` can produce this: the
    /// desktop app does not save a per-record rule set, because for a record from
    /// an outside laboratory the rules it used are usually unknown.
    RuleSetChanged { from: RuleSetRef, to: RuleSetRef },
    /// The record named a VUS sub-tier; today's evidence falls in another one.
    TierChanged { from: String, to: String },
    /// The curator says the gene-disease link is not established, so today's
    /// pathogenic class is shown as VUS (Richards 2015, genes of uncertain significance).
    GeneLinkNotEstablished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub code: String,
    /// One cell per record, in order, then one for today. None = not used.
    pub cells: Vec<Option<Strength>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reevaluation {
    pub records: Vec<Rescored>,
    pub today: Result<Assessment, AssessError>,
    pub rows: Vec<Row>,
    /// Index into `records` of the latest record for the case's condition
    /// (latest year; on a tie, entered last). Records for a different condition are skipped.
    pub latest_index: Option<usize>,
    /// None when there are no records, or when today's evidence cannot be
    /// classified (empty, contradictory, or not scorable).
    pub verdict: Option<Verdict>,
    pub reasons: Vec<Reason>,
}

fn strength_in(list: &[Applied], code: &str) -> Option<Strength> {
    list.iter().find(|a| a.code == code).map(|a| a.strength)
}

impl RuleSet {
    /// Compare the records with today's evidence, taking the gene-disease link as established.
    pub fn reevaluate(&self, submissions: &[Submission], today: &[Applied]) -> Reevaluation {
        self.reevaluate_in(submissions, today, GeneDecision::Established)
    }

    /// Like `reevaluate`, with the curator's answer on the gene-disease link: when
    /// it is not established, today's pathogenic class is shown as VUS.
    pub fn reevaluate_in(&self, submissions: &[Submission], today: &[Applied], gene: GeneDecision) -> Reevaluation {
        let records: Vec<Rescored> = submissions
            .iter()
            .map(|s| Rescored {
                label: s.label.clone(),
                year: s.year,
                reported_class: s.reported_class,
                reported_tier: s.reported_tier.clone(),
                condition: s.condition,
                // An old record is scored the way it was made, so its own class can be reproduced.
                assessment: self.assess_as(&s.criteria, Counting::AsRecorded),
            })
            .collect();
        let today_assessment = self.assess(today).map(|a| match gene {
            GeneDecision::NotEstablished => a.in_gene_of_uncertain_significance(),
            _ => a,
        });
        let rows = self.history_rows(submissions, today);

        // The latest record for the same condition: the latest year; on a tie, the one entered last.
        let latest_index = submissions
            .iter()
            .enumerate()
            .filter(|(_, s)| s.condition != ConditionMatch::Different)
            .max_by_key(|(index, s)| (s.year, *index))
            .map(|(i, _)| i);
        let (reasons, verdict) = match latest_index {
            Some(index) => self.compare_with_latest(&submissions[index], &records[index], today, &today_assessment),
            None if !submissions.is_empty() => (Vec::new(), Some(Verdict::DifferentCondition)),
            None => (Vec::new(), None),
        };

        Reevaluation { records, today: today_assessment, rows, latest_index, verdict, reasons }
    }

    /// One row per criterion that any record or today used, in the rule file's
    /// order so the table is stable.
    fn history_rows(&self, submissions: &[Submission], today: &[Applied]) -> Vec<Row> {
        self.criteria
            .iter()
            .filter_map(|c| {
                let mut cells: Vec<Option<Strength>> =
                    submissions.iter().map(|s| strength_in(&s.criteria, &c.code)).collect();
                cells.push(strength_in(today, &c.code));
                cells.iter().any(Option::is_some).then(|| Row { code: c.code.clone(), cells })
            })
            .collect()
    }

    /// What changed between the latest record and today's evidence, and what that
    /// means for the class (`None` when today's evidence has no class to compare).
    fn compare_with_latest(
        &self,
        latest: &Submission,
        rescored: &Rescored,
        today: &[Applied],
        today_assessment: &Result<Assessment, AssessError>,
    ) -> (Vec<Reason>, Option<Verdict>) {
        let mut reasons = self.evidence_reasons(latest, today);
        // Only changes in the evidence itself, or in the gene-disease link, count when deciding
        // between "different method" and a real upgrade or downgrade. The rule-set and tier notes do not.
        let lowered_for_gene = today_assessment.as_ref().is_ok_and(Assessment::lowered_for_gene);
        if lowered_for_gene {
            reasons.push(Reason::GeneLinkNotEstablished);
        }
        let evidence_changed = !reasons.is_empty();
        if let Some(old) = latest.rule_set.as_ref().filter(|old| **old != self.reference()) {
            reasons.push(Reason::RuleSetChanged { from: old.clone(), to: self.reference() });
        }
        if let (Some(from), Ok(today)) = (&latest.reported_tier, today_assessment)
            && let Some(to) = &today.vus_tier
            && from != to
        {
            reasons.push(Reason::TierChanged { from: from.clone(), to: to.clone() });
        }
        let verdict = today_assessment.as_ref().ok().and_then(Assessment::class).map(|today_class| {
            if rescored.consistency() == Consistency::Differs {
                return Verdict::RecordInconsistent;
            }
            match today_class.cmp(&latest.reported_class) {
                std::cmp::Ordering::Equal => Verdict::Holds,
                // No evidence changed, so a different class comes from the method, not from the evidence.
                _ if !evidence_changed => Verdict::FrameworkDiffers,
                std::cmp::Ordering::Greater => Verdict::UpgradeCandidate,
                std::cmp::Ordering::Less => Verdict::DowngradeCandidate,
            }
        });
        (reasons, verdict)
    }

    /// The criteria that were added, removed or given another strength since the latest record.
    fn evidence_reasons(&self, latest: &Submission, today: &[Applied]) -> Vec<Reason> {
        let mut reasons = Vec::new();
        for c in &self.criteria {
            match (strength_in(&latest.criteria, &c.code), strength_in(today, &c.code)) {
                (None, Some(strength)) => reasons.push(Reason::Added { code: c.code.clone(), strength }),
                (Some(_), None) => reasons.push(Reason::Removed { code: c.code.clone() }),
                (Some(from), Some(to)) if from != to => {
                    let rule_changed = c.original_strength == Some(from) && from != c.strength && to == c.strength;
                    reasons.push(if rule_changed {
                        Reason::RuleChanged {
                            code: c.code.clone(),
                            from,
                            to,
                            source: c.strength_source_id().to_string(),
                        }
                    } else {
                        Reason::StrengthChanged { code: c.code.clone(), from, to }
                    });
                }
                _ => {}
            }
        }
        reasons
    }
}

#[cfg(test)]
mod tests;
