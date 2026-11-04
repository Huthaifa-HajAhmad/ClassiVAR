//! Re-evaluating a reported variant: what did earlier records use, what do the
//! rules and evidence say today, and what changed?
//!
//! The answer is a candidate, never a decision. The curator reviews the reasons.

use crate::assess::{Applied, AssessError, Assessment};
use crate::model::{Class, Strength};
use crate::rules::RuleSet;

/// What one earlier record (a lab, in ClinVar for example) said and used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    pub label: String,
    pub year: u16,
    pub reported_class: Class,
    /// A VUS sub-tier name the record used, if any. Free text, as recorded.
    pub reported_tier: Option<String>,
    pub criteria: Vec<Applied>,
}

/// A record, scored again with today's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rescored {
    pub label: String,
    pub year: u16,
    pub reported_class: Class,
    pub reported_tier: Option<String>,
    pub assessment: Assessment,
}

impl Rescored {
    /// Does the record's own evidence, scored with today's points, give the class it reported?
    pub fn reported_matches_rescore(&self) -> bool {
        self.reported_class == self.assessment.points_class
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Today's class equals the latest record's class.
    Holds,
    UpgradeCandidate,
    DowngradeCandidate,
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
    pub today: Assessment,
    pub rows: Vec<Row>,
    /// Index into `records` of the latest record (latest year; on a tie, entered last).
    pub latest_index: Option<usize>,
    /// None when there are no records, or when today's evidence is empty
    /// (an empty list would look like "downgrade" and mislead).
    pub verdict: Option<Verdict>,
    pub reasons: Vec<Reason>,
}

impl RuleSet {
    pub fn reevaluate(&self, submissions: &[Submission], today: &[Applied]) -> Result<Reevaluation, AssessError> {
        let records = submissions
            .iter()
            .map(|s| {
                Ok(Rescored {
                    label: s.label.clone(),
                    year: s.year,
                    reported_class: s.reported_class,
                    reported_tier: s.reported_tier.clone(),
                    assessment: self.assess(&s.criteria)?,
                })
            })
            .collect::<Result<Vec<_>, AssessError>>()?;
        let today_assessment = self.assess(today)?;

        let strength_in = |list: &[Applied], code: &str| list.iter().find(|a| a.code == code).map(|a| a.strength);

        // Rows follow the order of the rule file, so the table is stable.
        let rows: Vec<Row> = self
            .criteria
            .iter()
            .filter_map(|c| {
                let mut cells: Vec<Option<Strength>> =
                    submissions.iter().map(|s| strength_in(&s.criteria, &c.code)).collect();
                cells.push(strength_in(today, &c.code));
                cells.iter().any(Option::is_some).then(|| Row { code: c.code.clone(), cells })
            })
            .collect();

        // The latest record is the one with the latest year; on a tie, the one entered last.
        let latest_index = submissions.iter().enumerate().max_by_key(|(index, s)| (s.year, *index)).map(|(i, _)| i);

        let mut reasons = Vec::new();
        let mut verdict = None;
        if let Some(latest) = latest_index.map(|i| &submissions[i]) {
            for c in &self.criteria {
                match (strength_in(&latest.criteria, &c.code), strength_in(today, &c.code)) {
                    (None, Some(strength)) => reasons.push(Reason::Added { code: c.code.clone(), strength }),
                    (Some(_), None) => reasons.push(Reason::Removed { code: c.code.clone() }),
                    (Some(from), Some(to)) if from != to => {
                        let rule_changed = c.original_strength == Some(from) && from != c.strength && to == c.strength;
                        reasons.push(if rule_changed {
                            Reason::RuleChanged { code: c.code.clone(), from, to, source: c.source.clone() }
                        } else {
                            Reason::StrengthChanged { code: c.code.clone(), from, to }
                        });
                    }
                    _ => {}
                }
            }
            if !today.is_empty() {
                verdict = Some(match today_assessment.points_class.cmp(&latest.reported_class) {
                    std::cmp::Ordering::Equal => Verdict::Holds,
                    std::cmp::Ordering::Greater => Verdict::UpgradeCandidate,
                    std::cmp::Ordering::Less => Verdict::DowngradeCandidate,
                });
            }
        }

        Ok(Reevaluation { records, today: today_assessment, rows, latest_index, verdict, reasons })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Strength::*;

    fn applied(list: &[(&str, Strength)]) -> Vec<Applied> {
        list.iter().map(|(code, strength)| Applied { code: code.to_string(), strength: *strength }).collect()
    }

    fn record(label: &str, year: u16, class: Class, list: &[(&str, Strength)]) -> Submission {
        Submission { label: label.into(), year, reported_class: class, reported_tier: None, criteria: applied(list) }
    }

    /// The fictional variant from the design sketch.
    fn sketch_records() -> Vec<Submission> {
        vec![
            record("Lab A", 2019, Class::Vus, &[("PM2", Moderate), ("PP3", Supporting)]),
            record("Lab B", 2022, Class::Vus, &[("PM2", Moderate), ("PP3", Supporting), ("PP4", Supporting)]),
        ]
    }

    #[test]
    fn the_sketch_example_holds_and_explains_the_pm2_rule_change() {
        let today = applied(&[("PM2", Supporting), ("PP3", Supporting), ("PP4", Supporting), ("PM5", Moderate)]);
        let r = RuleSet::builtin().reevaluate(&sketch_records(), &today).unwrap();

        assert_eq!(r.records[0].assessment.total_points, 3);
        assert_eq!(r.records[1].assessment.total_points, 4);
        assert_eq!(r.today.total_points, 5);
        assert_eq!(r.verdict, Some(Verdict::Holds));
        assert_eq!(
            r.reasons,
            vec![
                Reason::RuleChanged {
                    code: "PM2".into(),
                    from: Moderate,
                    to: Supporting,
                    source: "clingen-svi-pm2".into()
                },
                Reason::Added { code: "PM5".into(), strength: Moderate },
            ]
        );
        // Table rows follow the rule file order: PM2, PM5, PP3, PP4.
        let codes: Vec<&str> = r.rows.iter().map(|row| row.code.as_str()).collect();
        assert_eq!(codes, ["PM2", "PM5", "PP3", "PP4"]);
        assert_eq!(r.rows[0].cells, vec![Some(Moderate), Some(Moderate), Some(Supporting)]);
        assert_eq!(r.rows[1].cells, vec![None, None, Some(Moderate)]);
    }

    #[test]
    fn new_strong_evidence_makes_an_upgrade_candidate() {
        let today = applied(&[
            ("PM2", Supporting),
            ("PP3", Supporting),
            ("PP4", Supporting),
            ("PM5", Moderate),
            ("PS3", Strong),
        ]);
        let r = RuleSet::builtin().reevaluate(&sketch_records(), &today).unwrap();
        assert_eq!(r.today.total_points, 9);
        assert_eq!(r.verdict, Some(Verdict::UpgradeCandidate));
        assert!(r.reasons.contains(&Reason::Added { code: "PS3".into(), strength: Strength::Strong }));
    }

    #[test]
    fn dropping_evidence_from_a_likely_pathogenic_record_is_a_downgrade_candidate() {
        let records = vec![record("Lab A", 2020, Class::LikelyPathogenic, &[("PS3", Strong), ("PM2", Moderate)])];
        let today = applied(&[("PM2", Supporting)]);
        let r = RuleSet::builtin().reevaluate(&records, &today).unwrap();
        assert_eq!(r.verdict, Some(Verdict::DowngradeCandidate));
        assert!(r.reasons.contains(&Reason::Removed { code: "PS3".into() }));
    }

    #[test]
    fn a_strength_changed_by_the_curator_is_not_called_a_rule_change() {
        let records = vec![record("Lab A", 2021, Class::Vus, &[("PP3", Supporting)])];
        let today = applied(&[("PP3", Moderate)]);
        let r = RuleSet::builtin().reevaluate(&records, &today).unwrap();
        assert_eq!(r.reasons, vec![Reason::StrengthChanged { code: "PP3".into(), from: Supporting, to: Moderate }]);
    }

    #[test]
    fn no_verdict_without_records_or_without_todays_evidence() {
        let rules = RuleSet::builtin();
        assert_eq!(rules.reevaluate(&[], &applied(&[("PM2", Supporting)])).unwrap().verdict, None);
        assert_eq!(rules.reevaluate(&sketch_records(), &[]).unwrap().verdict, None);
    }

    #[test]
    fn the_latest_record_is_the_one_with_the_latest_year() {
        // Entered out of order: the 2022 record is first.
        let records = vec![
            record("Lab B", 2022, Class::LikelyPathogenic, &[("PS3", Strong), ("PM2", Moderate)]),
            record("Lab A", 2019, Class::Vus, &[("PM2", Moderate)]),
        ];
        let today = applied(&[("PS3", Strong), ("PM2", Moderate)]);
        let r = RuleSet::builtin().reevaluate(&records, &today).unwrap();
        assert_eq!(r.verdict, Some(Verdict::Holds));
        assert!(r.reasons.is_empty());
    }

    #[test]
    fn a_record_whose_class_does_not_match_its_own_evidence_is_flagged() {
        let records = vec![record("Lab A", 2020, Class::Pathogenic, &[("PM2", Supporting)])];
        let r = RuleSet::builtin().reevaluate(&records, &applied(&[("PM2", Supporting)])).unwrap();
        assert!(!r.records[0].reported_matches_rescore());
    }

    #[test]
    fn bad_evidence_in_a_record_is_rejected() {
        let records = vec![record("Lab A", 2020, Class::Vus, &[("ZZ9", Supporting)])];
        assert_eq!(RuleSet::builtin().reevaluate(&records, &[]), Err(AssessError::UnknownCriterion("ZZ9".into())));
    }
}
