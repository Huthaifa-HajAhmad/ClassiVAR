//! Scoring: turns the criteria a curator applied into points, a class, and an
//! explanation. The curator always decides which criteria apply. This code
//! only adds up and explains; it never picks criteria itself.

mod classic;
mod item;
mod pairs;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{Class, Direction, Strength};
use crate::rules::{RuleSet, RuleSetRef};
use classic::{classic_class, table_sides};
use pairs::double_counting_warnings;

/// One criterion the curator applied, at the strength they chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub code: String,
    pub strength: Strength,
    /// The strength-guide answer that set this strength, when a guide was used.
    /// Kept in the case file so the reason for the strength is not lost. Old
    /// files have no field and load with `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub situation: Option<String>,
    /// Why the curator chose this strength, in their own words or from a helper
    /// (for example "REVEL 0.95, Pejaver 2022"). Old files have no field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The longest reason a case file may carry, in characters.
pub const MAX_REASON_CHARS: usize = 500;

impl Applied {
    /// A criterion applied without a guide answer.
    pub fn new(code: impl Into<String>, strength: Strength) -> Self {
        Self { code: code.into(), strength, situation: None, reason: None }
    }

    /// A criterion whose strength came from a named guide situation.
    pub fn guided(code: impl Into<String>, strength: Strength, situation: impl Into<String>) -> Self {
        Self { code: code.into(), strength, situation: Some(situation.into()), reason: None }
    }

    /// The same criterion with the reason for its strength.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub code: String,
    pub direction: Direction,
    pub strength: Strength,
    pub default_strength: Strength,
    /// The guide answer that set `strength`, when one was used.
    pub situation: Option<String>,
    /// Signed: positive for pathogenic evidence, negative for benign.
    /// Zero when the criterion is listed but not counted (see `counted`).
    pub points: i32,
    /// False for a discouraged criterion in a current assessment: it is shown,
    /// but adds nothing. See `Counting`.
    pub counted: bool,
}

/// How to treat criteria the experts no longer recommend (PP5, BP6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counting {
    /// A new assessment today: discouraged criteria are listed but add no points.
    Current,
    /// Re-scoring what an earlier record did: discouraged criteria count, so
    /// the record's own class can be reproduced.
    AsRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// Pathogenic and benign evidence are both present.
    ConflictingEvidence,
    /// A criterion the experts no longer recommend. `counted` says whether it was scored.
    Discouraged { code: String, note: Option<String>, counted: bool },
    /// The curator changed a criterion's strength from its default.
    /// `reason` is what the curator wrote, if anything.
    StrengthChanged { code: String, from: Strength, to: Strength, reason: Option<String> },
    /// Both criteria count, and one of them already covers the same observation.
    DoubleCounted { a: String, b: String, reason: String },
}

/// Two criteria that cannot both be true. While one exists, no class is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub a: String,
    pub b: String,
    pub reason: String,
}

/// Two criteria that may be used together, but not at these strengths: their
/// combined points exceed a published limit. No class is given until one is
/// lowered or removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinationOveruse {
    pub a: String,
    pub b: String,
    /// The two criteria's points together (always positive).
    pub points: i32,
    pub max_points: i32,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextClass {
    pub class: Class,
    pub points_needed: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// The rules that produced this result.
    pub rule_set: RuleSetRef,
    pub items: Vec<Item>,
    /// False until at least one criterion is counted. Without evidence there is
    /// no classification, only a total of zero.
    pub has_evidence: bool,
    pub pathogenic_points: i32,
    /// Negative or zero.
    pub benign_points: i32,
    pub total_points: i32,
    /// What the points alone give. Use `class()` to know whether to show it.
    pub points_class: Class,
    /// The 2015 combining table (with the ClinGen 2020 addition, see `classic_class`).
    /// None if there is no evidence, the evidence contradicts itself, or it uses
    /// a strength the table does not cover (for example moderate benign evidence).
    pub classic_class: Option<Class>,
    /// Only set when official VUS tier ranges exist in the rule file.
    pub vus_tier: Option<String>,
    /// Criteria that contradict each other. The class is withheld until they are resolved.
    pub conflicts: Vec<Conflict>,
    /// Pairs whose combined points exceed a published limit. The class is withheld
    /// until one is lowered or removed.
    pub combination_overuse: Vec<CombinationOveruse>,
    /// The evidence meets both a pathogenic and a benign rule of the 2015 table. The
    /// 2015 guidance calls that uncertain, while the points would just add it up and
    /// could give a confident class. So the class is withheld: this needs expert judgment.
    pub both_sides_met: bool,
    pub warnings: Vec<Warning>,
    /// How many more points of pathogenic evidence reach the next class up.
    pub to_upgrade: Option<NextClass>,
    /// How many more points of benign evidence reach the next class down.
    pub to_downgrade: Option<NextClass>,
}

impl Assessment {
    /// The class to show, or None when there is nothing to classify (no evidence),
    /// the evidence contradicts itself, it meets rules on both sides, or a pair
    /// exceeds its combined limit.
    pub fn class(&self) -> Option<Class> {
        (self.has_evidence
            && self.conflicts.is_empty()
            && !(self.both_sides_met && self.points_class != Class::Vus)
            && self.combination_overuse.is_empty())
        .then_some(self.points_class)
    }

    /// Do the points and the 2015 table give the same class? None if the 2015 table does not apply.
    pub fn frameworks_agree(&self) -> Option<bool> {
        self.classic_class.map(|c| c == self.points_class)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssessError {
    #[error("unknown criterion {0}")]
    UnknownCriterion(String),
    #[error("criterion {0} was applied twice")]
    DuplicateCriterion(String),
    #[error("strength {strength:?} is not allowed for {code}")]
    StrengthNotAllowed { code: String, strength: Strength },
    #[error("criterion {code}: the guide answer '{situation}' does not set this strength")]
    GuideMismatch { code: String, situation: String },
    #[error("criterion {code} has a strength guide; a new assessment must answer it")]
    GuideAnswerRequired { code: String },
    #[error("criterion {code}: the reason is empty or longer than {MAX_REASON_CHARS} characters")]
    BadReason { code: String },
}

/// Is a counted stand-alone benign criterion (BA1) applied? It is enough on its own
/// for Benign, whatever else applies. Treating it as "8 points" alone would let
/// other pathogenic evidence outvote it, which the 2015 framework does not allow.
fn has_stand_alone_benign(items: &[Item]) -> bool {
    items.iter().any(|i| i.counted && i.direction == Direction::Benign && i.strength == Strength::StandAlone)
}

/// What scoring a list of applied criteria gives: the scored items, the notes the
/// scoring raised, and the codes that were seen.
type ScoredItems<'a> = (Vec<Item>, Vec<Warning>, HashSet<&'a str>);

impl RuleSet {
    /// Score a new assessment: discouraged criteria are listed but not counted.
    pub fn assess(&self, applied: &[Applied]) -> Result<Assessment, AssessError> {
        self.assess_as(applied, Counting::Current)
    }

    pub fn assess_as(&self, applied: &[Applied], counting: Counting) -> Result<Assessment, AssessError> {
        let (items, mut warnings, seen) = self.score_items(applied, counting)?;
        let pathogenic_points: i32 = items.iter().filter(|i| i.points > 0).map(|i| i.points).sum();
        let benign_points: i32 = items.iter().filter(|i| i.points < 0).map(|i| i.points).sum();
        if pathogenic_points > 0 && benign_points < 0 {
            warnings.push(Warning::ConflictingEvidence);
        }
        let conflicts = self.conflicts_among(&seen);
        let total_points = pathogenic_points + benign_points;
        let points_class =
            if has_stand_alone_benign(&items) { Class::Benign } else { self.class_for_points(total_points) };
        let has_evidence = items.iter().any(|i| i.counted);
        let (pathogenic_side, benign_side) = table_sides(&items);

        let mut assessment = Assessment {
            rule_set: self.reference(),
            // Contradictory evidence gets no class from either method.
            classic_class: if has_evidence && conflicts.is_empty() { classic_class(&items) } else { None },
            has_evidence,
            pathogenic_points,
            benign_points,
            total_points,
            points_class,
            vus_tier: None,
            conflicts,
            combination_overuse: self.combination_overuse(&items),
            both_sides_met: pathogenic_side.is_some() && benign_side.is_some(),
            warnings,
            to_upgrade: None,
            to_downgrade: None,
            items,
        };
        // A withheld class has no "next class": the hint would contradict the banner.
        if assessment.class().is_some() {
            self.add_class_hints(&mut assessment);
        }
        Ok(assessment)
    }

    /// Score each applied criterion, and collect the notes the scoring raised.
    /// The set of codes seen is returned so the exclusive pairs can be found.
    fn score_items<'a>(&self, applied: &'a [Applied], counting: Counting) -> Result<ScoredItems<'a>, AssessError> {
        let mut items = Vec::with_capacity(applied.len());
        let mut warnings = Vec::new();
        let mut seen = HashSet::new();
        for a in applied {
            let c = self.criterion(&a.code).ok_or_else(|| AssessError::UnknownCriterion(a.code.clone()))?;
            if !seen.insert(a.code.as_str()) {
                return Err(AssessError::DuplicateCriterion(a.code.clone()));
            }
            let (item, notes) = self.score_item(a, c, counting)?;
            items.push(item);
            warnings.extend(notes);
        }
        warnings.extend(double_counting_warnings(&items));
        Ok((items, warnings, seen))
    }

    /// Fill in what would move a shown class: the points to the next class up and
    /// down, and the VUS tier when the rules give official ranges.
    fn add_class_hints(&self, assessment: &mut Assessment) {
        let (class, total) = (assessment.points_class, assessment.total_points);
        // Stand-alone benign evidence fixes the class: there is no way up from it by points.
        if !has_stand_alone_benign(&assessment.items) {
            assessment.to_upgrade = class
                .up()
                .and_then(|next| self.class_min(next).map(|min| NextClass { class: next, points_needed: min - total }));
        }
        assessment.to_downgrade = class.down().and_then(|next| {
            self.class_min(class).map(|min| NextClass { class: next, points_needed: total - (min - 1) })
        });
        if class == Class::Vus {
            assessment.vus_tier = self.vus_tier_for(total);
        }
    }

    /// A tier is only named when every tier has an official range (the rule
    /// file check makes sure the ranges are consistent).
    fn vus_tier_for(&self, total: i32) -> Option<String> {
        if self.vus_tiers.is_empty() || self.vus_tiers.iter().any(|t| t.min_points.is_none()) {
            return None;
        }
        self.vus_tiers
            .iter()
            .filter_map(|t| t.min_points.filter(|min| *min <= total).map(|min| (min, &t.name)))
            .max_by_key(|(min, _)| *min)
            .map(|(_, name)| name.clone())
    }
}
