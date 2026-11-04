//! Scoring: turns the criteria a curator applied into points, a class, and an
//! explanation. The curator always decides which criteria apply. This code
//! only adds up and explains; it never picks criteria itself.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{Class, Direction, Strength};
use crate::rules::{RuleSet, RuleSetRef};

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
}

impl Applied {
    /// A criterion applied without a guide answer.
    pub fn new(code: impl Into<String>, strength: Strength) -> Self {
        Self { code: code.into(), strength, situation: None }
    }

    /// A criterion whose strength came from a named guide situation.
    pub fn guided(code: impl Into<String>, strength: Strength, situation: impl Into<String>) -> Self {
        Self { code: code.into(), strength, situation: Some(situation.into()) }
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
    StrengthChanged { code: String, from: Strength, to: Strength },
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
}

impl RuleSet {
    /// Score a new assessment: discouraged criteria are listed but not counted.
    pub fn assess(&self, applied: &[Applied]) -> Result<Assessment, AssessError> {
        self.assess_as(applied, Counting::Current)
    }

    pub fn assess_as(&self, applied: &[Applied], counting: Counting) -> Result<Assessment, AssessError> {
        let mut items = Vec::with_capacity(applied.len());
        let mut seen = HashSet::new();
        let mut warnings = Vec::new();

        for a in applied {
            let c = self.criterion(&a.code).ok_or_else(|| AssessError::UnknownCriterion(a.code.clone()))?;
            if !seen.insert(a.code.as_str()) {
                return Err(AssessError::DuplicateCriterion(a.code.clone()));
            }
            // Today's strength limits apply to new assessments only; an old record may predate them.
            let allowed = match counting {
                Counting::Current => c.allowed_strengths(),
                Counting::AsRecorded => c.recorded_strengths(),
            };
            if !allowed.contains(&a.strength) {
                return Err(AssessError::StrengthNotAllowed { code: a.code.clone(), strength: a.strength });
            }
            // A guide answer is required for a new assessment: the engine must not
            // fall back to a strength the tree did not give. Earlier records may
            // predate the guide, so their saved strength is read as it was made.
            if counting == Counting::Current && !c.guide.is_empty() {
                let Some(situation) = &a.situation else {
                    return Err(AssessError::GuideAnswerRequired { code: a.code.clone() });
                };
                let matches =
                    c.guide.iter().find(|g| g.id == *situation).is_some_and(|g| g.strength == Some(a.strength));
                if !matches {
                    return Err(AssessError::GuideMismatch { code: a.code.clone(), situation: situation.clone() });
                }
            }
            let counted = !(c.discouraged && counting == Counting::Current);
            let magnitude = if counted { self.magnitude(a.strength) } else { 0 };
            let points = match c.direction {
                Direction::Pathogenic => magnitude,
                Direction::Benign => -magnitude,
            };
            if a.strength != c.strength && counting == Counting::Current {
                warnings.push(Warning::StrengthChanged { code: a.code.clone(), from: c.strength, to: a.strength });
            }
            if c.discouraged {
                warnings.push(Warning::Discouraged { code: a.code.clone(), note: c.note.clone(), counted });
            }
            items.push(Item {
                code: a.code.clone(),
                direction: c.direction,
                strength: a.strength,
                default_strength: c.strength,
                situation: a.situation.clone(),
                points,
                counted,
            });
        }

        let pathogenic_points: i32 = items.iter().filter(|i| i.points > 0).map(|i| i.points).sum();
        let benign_points: i32 = items.iter().filter(|i| i.points < 0).map(|i| i.points).sum();
        // PVS1 and PP3 must not both count for a splice variant: Abou Tayoun 2018
        // says the splicing prediction is already part of PVS1 there.
        let counted = |code: &str| items.iter().any(|i| i.counted && i.code == code);
        if counted("PVS1") && counted("PP3") {
            warnings.push(Warning::DoubleCounted {
                a: "PVS1".to_string(),
                b: "PP3".to_string(),
                reason: "If PVS1 describes a splice variant, its effect is already counted; do not also count PP3 for the same prediction.".to_string(),
            });
        }
        if pathogenic_points > 0 && benign_points < 0 {
            warnings.push(Warning::ConflictingEvidence);
        }
        let conflicts: Vec<Conflict> = self
            .exclusive_pairs
            .iter()
            .filter(|pair| seen.contains(pair.a.as_str()) && seen.contains(pair.b.as_str()))
            .map(|pair| Conflict { a: pair.a.clone(), b: pair.b.clone(), reason: pair.reason.clone() })
            .collect();
        let combination_overuse: Vec<CombinationOveruse> = self
            .combination_caps
            .iter()
            .filter_map(|cap| {
                let a = items.iter().find(|i| i.counted && i.code == cap.a)?;
                let b = items.iter().find(|i| i.counted && i.code == cap.b)?;
                let points = a.points.abs() + b.points.abs();
                (points > cap.max_points).then(|| CombinationOveruse {
                    a: cap.a.clone(),
                    b: cap.b.clone(),
                    points,
                    max_points: cap.max_points,
                    reason: cap.reason.clone(),
                })
            })
            .collect();

        let total_points = pathogenic_points + benign_points;
        // Stand-alone benign evidence (BA1) is enough on its own: it is Benign
        // whatever else applies. Treating it as "8 points" alone would let other
        // pathogenic evidence outvote it, which the 2015 framework does not allow.
        let stand_alone_benign =
            items.iter().any(|i| i.counted && i.direction == Direction::Benign && i.strength == Strength::StandAlone);
        let points_class = if stand_alone_benign { Class::Benign } else { self.class_for_points(total_points) };
        let has_evidence = items.iter().any(|i| i.counted);
        let (pathogenic_side, benign_side) = table_sides(&items);
        let both_sides_met = pathogenic_side.is_some() && benign_side.is_some();
        // Both sides met is the ordinary VUS case when the points agree. Withholding
        // is only needed when the points would give a confident class instead.
        let withheld_both_sides = both_sides_met && points_class != Class::Vus;
        // A withheld class has no "next class": the hint would contradict the banner.
        let class_known =
            has_evidence && conflicts.is_empty() && !withheld_both_sides && combination_overuse.is_empty();

        let to_upgrade = (class_known && !stand_alone_benign)
            .then(|| {
                points_class.up().and_then(|next| {
                    self.class_min(next).map(|min| NextClass { class: next, points_needed: min - total_points })
                })
            })
            .flatten();
        let to_downgrade = class_known
            .then(|| {
                points_class.down().and_then(|next| {
                    self.class_min(points_class)
                        .map(|min| NextClass { class: next, points_needed: total_points - (min - 1) })
                })
            })
            .flatten();

        let vus_tier = if class_known && points_class == Class::Vus { self.vus_tier_for(total_points) } else { None };

        Ok(Assessment {
            rule_set: self.reference(),
            // Contradictory evidence gets no class from either method.
            classic_class: if has_evidence && conflicts.is_empty() { classic_class(&items) } else { None },
            items,
            has_evidence,
            pathogenic_points,
            benign_points,
            total_points,
            points_class,
            vus_tier,
            conflicts,
            combination_overuse,
            both_sides_met,
            warnings,
            to_upgrade,
            to_downgrade,
        })
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

/// The ACMG/AMP 2015 combining table (Richards et al., Table 5), plus one
/// addition: ClinGen's PM2 recommendation of 4 September 2020 lowered PM2 to
/// Supporting and, so that PVS1 + PM2 stays Likely pathogenic, added the rule
/// "one Very Strong + one Supporting = Likely pathogenic". That rule is the
/// `pvs >= 1 && pp >= 1` line below.
fn classic_class(items: &[Item]) -> Option<Class> {
    // The 2015 table has no moderate or very strong benign rows.
    let uncovered = items.iter().any(|i| {
        i.counted && i.direction == Direction::Benign && matches!(i.strength, Strength::Moderate | Strength::VeryStrong)
    });
    if uncovered {
        return None;
    }
    // If both sides are met, the 2015 guidance says VUS.
    Some(match table_sides(items) {
        (Some(_), Some(_)) | (None, None) => Class::Vus,
        (Some(class), None) | (None, Some(class)) => class,
    })
}

/// What the rows of the 2015 table give on each side: (pathogenic side, benign side).
fn table_sides(items: &[Item]) -> (Option<Class>, Option<Class>) {
    let counted = || items.iter().filter(|i| i.counted);
    let count = |direction: Direction, strength: Strength| {
        counted().filter(|i| i.direction == direction && i.strength == strength).count()
    };
    let (pvs, ps, pm, pp) = (
        count(Direction::Pathogenic, Strength::VeryStrong),
        count(Direction::Pathogenic, Strength::Strong),
        count(Direction::Pathogenic, Strength::Moderate),
        count(Direction::Pathogenic, Strength::Supporting),
    );
    let (ba, bs, bp) = (
        count(Direction::Benign, Strength::StandAlone),
        count(Direction::Benign, Strength::Strong),
        count(Direction::Benign, Strength::Supporting),
    );
    let pathogenic = (pvs >= 1 && (ps >= 1 || pm >= 2 || (pm >= 1 && pp >= 1) || pp >= 2))
        || ps >= 2
        || (ps >= 1 && (pm >= 3 || (pm >= 2 && pp >= 2) || (pm >= 1 && pp >= 4)));
    let likely_pathogenic = (pvs >= 1 && pm >= 1)
        || (pvs >= 1 && pp >= 1) // ClinGen 2020 addition, see above
        || (ps >= 1 && pm >= 1)
        || (ps >= 1 && pp >= 2)
        || pm >= 3
        || (pm >= 2 && pp >= 2)
        || (pm >= 1 && pp >= 4);
    let benign = ba >= 1 || bs >= 2;
    let likely_benign = (bs >= 1 && bp >= 1) || bp >= 2;

    let pathogenic_class = if pathogenic {
        Some(Class::Pathogenic)
    } else if likely_pathogenic {
        Some(Class::LikelyPathogenic)
    } else {
        None
    };
    let benign_class = if benign {
        Some(Class::Benign)
    } else if likely_benign {
        Some(Class::LikelyBenign)
    } else {
        None
    };

    (pathogenic_class, benign_class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Strength::*;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn applied(list: &[(&str, Strength)]) -> Vec<Applied> {
        list.iter()
            .map(|(code, strength)| match rules().criterion(code).and_then(|c| c.situation_for(*strength)) {
                Some(situation) => Applied::guided(*code, *strength, situation),
                None => Applied::new(*code, *strength),
            })
            .collect()
    }

    fn apply(list: &[(&str, Strength)]) -> Assessment {
        rules().assess(&applied(list)).expect("valid evidence")
    }

    #[test]
    fn builtin_rules_are_valid_and_complete() {
        let r = rules();
        assert_eq!(r.criteria.len(), 28);
        assert_eq!(r.criteria.iter().filter(|c| c.direction == Direction::Pathogenic).count(), 16);
        assert_eq!(r.criteria.iter().filter(|c| c.direction == Direction::Benign).count(), 12);
        assert!(r.criteria.iter().all(|c| r.source(&c.source).is_some()));
    }

    #[test]
    fn the_sketch_example_is_a_vus_one_point_short_of_likely_pathogenic() {
        // PM2 supporting (1) + PP3 (1) + PP4 (1) + PM5 moderate (2) = 5
        let a = apply(&[("PM2", Supporting), ("PP3", Supporting), ("PP4", Supporting), ("PM5", Moderate)]);
        assert_eq!(a.total_points, 5);
        assert_eq!(a.points_class, Class::Vus);
        assert_eq!(a.class(), Some(Class::Vus));
        assert_eq!(a.to_upgrade, Some(NextClass { class: Class::LikelyPathogenic, points_needed: 1 }));
        assert_eq!(a.to_downgrade, Some(NextClass { class: Class::LikelyBenign, points_needed: 6 }));
        assert_eq!(a.vus_tier, None, "no official tier ranges exist yet");
    }

    #[test]
    fn class_edges() {
        let r = rules();
        assert_eq!(r.class_for_points(10), Class::Pathogenic);
        assert_eq!(r.class_for_points(9), Class::LikelyPathogenic);
        assert_eq!(r.class_for_points(6), Class::LikelyPathogenic);
        assert_eq!(r.class_for_points(5), Class::Vus);
        assert_eq!(r.class_for_points(0), Class::Vus);
        assert_eq!(r.class_for_points(-1), Class::LikelyBenign);
        assert_eq!(r.class_for_points(-6), Class::LikelyBenign);
        assert_eq!(r.class_for_points(-7), Class::Benign);
    }

    #[test]
    fn no_evidence_is_not_a_classification() {
        let a = apply(&[]);
        assert!(!a.has_evidence);
        assert_eq!(a.total_points, 0);
        assert_eq!(a.class(), None, "zero points with no criteria must not look like a VUS");
        assert_eq!(a.classic_class, None);
        assert_eq!(a.frameworks_agree(), None);
    }

    #[test]
    fn classic_table_pathogenic_and_likely_pathogenic() {
        // PVS1 + PS3 -> Pathogenic
        assert_eq!(apply(&[("PVS1", VeryStrong), ("PS3", Strong)]).classic_class, Some(Class::Pathogenic));
        // Two strong -> Pathogenic
        assert_eq!(apply(&[("PS1", Strong), ("PS3", Strong)]).classic_class, Some(Class::Pathogenic));
        // PVS1 + PM2 at moderate -> Likely pathogenic
        assert_eq!(apply(&[("PVS1", VeryStrong), ("PM2", Moderate)]).classic_class, Some(Class::LikelyPathogenic));
        // Three moderate -> Likely pathogenic
        assert_eq!(
            apply(&[("PM1", Moderate), ("PM3", Moderate), ("PM4", Moderate)]).classic_class,
            Some(Class::LikelyPathogenic)
        );
        // One strong + one supporting is not enough
        assert_eq!(apply(&[("PS3", Strong), ("PP3", Supporting)]).classic_class, Some(Class::Vus));
    }

    #[test]
    fn pvs1_with_pm2_at_supporting_stays_likely_pathogenic_in_both_methods() {
        // The ClinGen 2020 addition: one Very Strong + one Supporting = Likely pathogenic.
        let a = apply(&[("PVS1", VeryStrong), ("PM2", Supporting)]);
        assert_eq!(a.total_points, 9);
        assert_eq!(a.points_class, Class::LikelyPathogenic);
        assert_eq!(a.classic_class, Some(Class::LikelyPathogenic));
        assert_eq!(a.frameworks_agree(), Some(true));
    }

    #[test]
    fn classic_table_benign_side() {
        assert_eq!(apply(&[("BA1", StandAlone)]).classic_class, Some(Class::Benign));
        assert_eq!(apply(&[("BS1", Strong), ("BS2", Strong)]).classic_class, Some(Class::Benign));
        assert_eq!(apply(&[("BS1", Strong), ("BP4", Supporting)]).classic_class, Some(Class::LikelyBenign));
        assert_eq!(apply(&[("BP4", Supporting), ("BP7", Supporting)]).classic_class, Some(Class::LikelyBenign));
    }

    #[test]
    fn both_sides_met_gives_vus_in_the_classic_table() {
        let a = apply(&[("PS1", Strong), ("PS3", Strong), ("BS1", Strong), ("BS2", Strong)]);
        assert_eq!(a.classic_class, Some(Class::Vus));
        assert!(a.warnings.contains(&Warning::ConflictingEvidence));
    }

    #[test]
    fn both_sides_met_withholds_the_class_instead_of_adding_up() {
        // PS1 + PS3 meets Pathogenic (2 strong); BS1 + BS2 + BS4 meets Benign. The points
        // alone would say -4: a confident Likely benign for evidence the table calls uncertain.
        let a = apply(&[("PS1", Strong), ("PS3", Strong), ("BS1", Strong), ("BS2", Strong), ("BS4", Strong)]);
        assert_eq!(a.total_points, -4);
        assert_eq!(a.points_class, Class::LikelyBenign);
        assert!(a.both_sides_met);
        assert_eq!(a.class(), None);
        assert_eq!(a.to_upgrade, None);
        assert_eq!(a.to_downgrade, None);

        // Evidence on both sides that meets a rule on one side only still adds up.
        let a = apply(&[("PS3", Strong), ("BS1", Strong), ("BS2", Strong)]);
        assert!(!a.both_sides_met);
        assert_eq!(a.class(), Some(Class::LikelyBenign));
        assert_eq!(a.classic_class, Some(Class::Benign));
    }

    #[test]
    fn both_sides_met_is_reported_as_the_vus_both_methods_agree_on() {
        // PS3 + PS4 meets Pathogenic; BS1 + BS2 meets Benign. The points are 0 and
        // the 2015 table says VUS, so refusing to name a class would be wrong.
        let a = apply(&[("PS3", Strong), ("PS4", Strong), ("BS1", Strong), ("BS2", Strong)]);
        assert_eq!(a.total_points, 0);
        assert_eq!(a.points_class, Class::Vus);
        assert!(a.both_sides_met);
        assert_eq!(a.class(), Some(Class::Vus));
        assert_eq!(a.classic_class, Some(Class::Vus));
        assert_eq!(a.frameworks_agree(), Some(true));
    }

    #[test]
    fn ba1_has_no_other_strength_and_stays_benign() {
        let ba1 = rules().criterion("BA1").unwrap();
        assert_eq!(ba1.allowed_strengths(), vec![StandAlone]);
        assert_eq!(ba1.recorded_strengths(), vec![StandAlone]);
        // A record cannot be given a weaker BA1 either.
        assert!(rules().assess_as(&[Applied::new("BA1", Supporting)], Counting::AsRecorded).is_err());
    }

    #[test]
    fn a_guided_criterion_needs_a_guide_answer() {
        // The engine, not only the screen, refuses a new PVS1 at a strength the
        // tree did not give.
        let bare = [Applied::new("PVS1", VeryStrong)];
        assert_eq!(rules().assess(&bare), Err(AssessError::GuideAnswerRequired { code: "PVS1".into() }));
        let pvs1 = rules().criterion("PVS1").unwrap();
        let start_loss = pvs1
            .guide
            .iter()
            .find(|g| g.situation.starts_with("Start-loss") && g.strength == Some(Supporting))
            .unwrap();
        assert!(rules().assess(&[Applied::guided("PVS1", Supporting, start_loss.id.clone())]).is_ok());
    }

    #[test]
    fn pvs1_and_pp3_together_are_flagged_as_double_counting() {
        let a = apply(&[("PVS1", VeryStrong), ("PP3", Strong)]);
        assert!(
            a.warnings.iter().any(|w| matches!(w, Warning::DoubleCounted { a, b, .. } if a == "PVS1" && b == "PP3"))
        );
    }

    #[test]
    fn pp1_and_pp4_together_are_capped_at_five_points() {
        // ClinGen 2024: Strong + Strong (8) is not an allowed pair for these two.
        let a = apply(&[("PP1", Strong), ("PP4", Strong)]);
        assert_eq!(a.total_points, 8);
        assert_eq!(a.combination_overuse.len(), 1);
        assert_eq!(a.class(), None);
        assert_eq!(a.combination_overuse[0].max_points, 5);

        // Strong + Supporting is the strongest allowed pair.
        let a = apply(&[("PP1", Strong), ("PP4", Supporting)]);
        assert!(a.combination_overuse.is_empty());
        assert_eq!(a.class(), Some(Class::Vus));
    }

    #[test]
    fn a_start_loss_at_the_guide_strength_does_not_become_likely_pathogenic() {
        // PVS1_Supporting for a start-loss + PM2_Supporting: 2 points, a VUS. At the old
        // Very Strong default the same variant scored 9 points, Likely pathogenic.
        let a = apply(&[("PVS1", Supporting), ("PM2", Supporting)]);
        assert_eq!(a.total_points, 2);
        assert_eq!(a.class(), Some(Class::Vus));
    }

    #[test]
    fn pvs1_and_pm4_are_not_used_together() {
        let a = apply(&[("PVS1", Strong), ("PM4", Moderate)]);
        assert_eq!(a.class(), None);
        assert!(a.conflicts.iter().any(|c| c.a == "PVS1" && c.b == "PM4"));
    }

    #[test]
    fn weak_evidence_cannot_be_raised_to_a_confident_class() {
        let r = rules();
        for code in ["PP2", "PP4", "BP1", "BP4", "BP7"] {
            let one = [Applied::new(code, VeryStrong)];
            assert_eq!(
                r.assess(&one),
                Err(AssessError::StrengthNotAllowed { code: code.into(), strength: VeryStrong }),
                "{code}"
            );
        }
    }

    #[test]
    fn points_and_classic_table_can_disagree() {
        // PVS1 alone is 8 points (Likely pathogenic) but the 2015 table needs more.
        let a = apply(&[("PVS1", VeryStrong)]);
        assert_eq!(a.points_class, Class::LikelyPathogenic);
        assert_eq!(a.classic_class, Some(Class::Vus));
        assert_eq!(a.frameworks_agree(), Some(false));
    }

    #[test]
    fn ba1_alone_is_benign_by_points_too() {
        let a = apply(&[("BA1", StandAlone)]);
        assert_eq!(a.total_points, -8);
        assert_eq!(a.class(), Some(Class::Benign));
        assert_eq!(a.frameworks_agree(), Some(true));
    }

    #[test]
    fn stand_alone_benign_evidence_is_not_outvoted() {
        // BA1 is stand-alone: adding pathogenic evidence must not lift it to
        // Likely benign or VUS. -8 + 4 = -4 would otherwise be Likely benign.
        let a = apply(&[("BA1", StandAlone), ("PS3", Strong)]);
        assert_eq!(a.total_points, -4);
        assert_eq!(a.class(), Some(Class::Benign));
        assert_eq!(a.classic_class, Some(Class::Benign));
        assert_eq!(a.to_upgrade, None, "there is no higher class to reach from stand-alone benign");

        // BA1 against evidence that itself reaches Pathogenic (two strong) is a real
        // conflict, as with a known common pathogenic variant: it needs expert judgment.
        let a = apply(&[("BA1", StandAlone), ("PS3", Strong), ("PS4", Strong)]);
        assert_eq!(a.total_points, 0);
        assert!(a.both_sides_met);
        assert_eq!(a.class(), None);
    }

    #[test]
    fn a_withheld_class_has_no_next_class_hint() {
        let a = apply(&[("PS1", Strong), ("PM5", Moderate)]);
        assert_eq!(a.class(), None);
        assert_eq!(a.to_upgrade, None);
        assert_eq!(a.to_downgrade, None);
        assert_eq!(a.vus_tier, None);
    }

    #[test]
    fn moderate_benign_evidence_is_not_covered_by_the_classic_table() {
        let a = apply(&[("BP4", Moderate)]);
        assert_eq!(a.classic_class, None);
        assert_eq!(a.frameworks_agree(), None);
    }

    #[test]
    fn contradicting_criteria_withhold_the_class() {
        // PS1 + PM5 would add to 6 points (Likely pathogenic) although PS1 alone is only 4.
        let a = apply(&[("PS1", Strong), ("PM5", Moderate)]);
        assert_eq!(a.conflicts.len(), 1);
        assert_eq!((a.conflicts[0].a.as_str(), a.conflicts[0].b.as_str()), ("PS1", "PM5"));
        assert_eq!(a.class(), None);

        // BA1 + PM2 must not be turned into "Likely benign" by adding the points.
        let a = apply(&[("BA1", StandAlone), ("PM2", Moderate)]);
        assert_eq!(a.class(), None);

        // PP3 + BP4: computational evidence cannot point both ways.
        assert_eq!(apply(&[("PP3", Supporting), ("BP4", Supporting)]).class(), None);

        // Removing one of the pair gives the class back.
        assert_eq!(apply(&[("PS1", Strong)]).class(), Some(Class::Vus));
    }

    #[test]
    fn contradictory_evidence_gets_no_class_from_the_2015_table_either() {
        let a = apply(&[("PS1", Strong), ("PM5", Moderate)]);
        assert_eq!(a.classic_class, None);
        assert_eq!(a.frameworks_agree(), None);
    }

    #[test]
    fn an_old_record_may_use_a_strength_that_is_capped_today() {
        let pp3 = applied(&[("PP3", VeryStrong)]);
        assert!(rules().assess(&pp3).is_err());
        let old = rules().assess_as(&pp3, Counting::AsRecorded).unwrap();
        assert_eq!(old.total_points, 8);
    }

    #[test]
    fn discouraged_criteria_are_listed_but_not_counted_today() {
        let a = apply(&[("PP5", Supporting)]);
        assert!(!a.has_evidence);
        assert_eq!(a.total_points, 0);
        assert_eq!(a.class(), None);
        assert_eq!(a.items[0].points, 0);
        assert!(
            a.warnings.iter().any(|w| matches!(w, Warning::Discouraged { code, counted: false, .. } if code == "PP5"))
        );

        // PS3 Strong + PP5 Strong would be 8 points; PP5 adds nothing.
        let a = apply(&[("PS3", Strong), ("PP5", Strong)]);
        assert_eq!(a.total_points, 4);
        assert_eq!(a.classic_class, Some(Class::Vus));
    }

    #[test]
    fn discouraged_criteria_count_when_rescoring_an_old_record() {
        let a = rules().assess_as(&applied(&[("PS3", Strong), ("PP5", Strong)]), Counting::AsRecorded).unwrap();
        assert_eq!(a.total_points, 8);
        assert!(a.warnings.iter().any(|w| matches!(w, Warning::Discouraged { counted: true, .. })));
    }

    #[test]
    fn changing_a_strength_is_reported() {
        let a = apply(&[("PM2", Moderate)]);
        assert_eq!(a.total_points, 2);
        assert!(a.warnings.contains(&Warning::StrengthChanged { code: "PM2".into(), from: Supporting, to: Moderate }));
    }

    #[test]
    fn bad_input_is_rejected() {
        let r = rules();
        let one = |code: &str, strength| vec![Applied::new(code, strength)];
        assert_eq!(r.assess(&one("XX9", Strong)), Err(AssessError::UnknownCriterion("XX9".into())));
        assert_eq!(
            r.assess(&one("PS1", StandAlone)),
            Err(AssessError::StrengthNotAllowed { code: "PS1".into(), strength: StandAlone })
        );
        let twice = vec![Applied::new("PS1", Strong), Applied::new("PS1", Strong)];
        assert_eq!(r.assess(&twice), Err(AssessError::DuplicateCriterion("PS1".into())));
    }

    #[test]
    fn a_computational_criterion_cannot_be_raised_to_very_strong() {
        let r = rules();
        let pp3 = vec![Applied::new("PP3", VeryStrong)];
        assert_eq!(r.assess(&pp3), Err(AssessError::StrengthNotAllowed { code: "PP3".into(), strength: VeryStrong }));
        // Strong is still allowed (calibrated predictors can reach it).
        assert!(r.assess(&[Applied::new("PP3", Strong)]).is_ok());
    }

    #[test]
    fn the_assessment_names_the_rules_that_produced_it() {
        let a = apply(&[("PS3", Strong)]);
        assert_eq!(a.rule_set, rules().reference());
    }

    #[test]
    fn vus_tiers_are_named_only_when_every_tier_has_a_range() {
        let mut r = rules().clone();
        assert_eq!(r.vus_tier_for(3), None);
        for (tier, min) in r.vus_tiers.iter_mut().zip([4, 2, 0]) {
            tier.min_points = Some(min); // made-up numbers, for the test only
        }
        assert_eq!(r.vus_tier_for(5).as_deref(), Some("VUS-high"));
        assert_eq!(r.vus_tier_for(2).as_deref(), Some("VUS-mid"));
        assert_eq!(r.vus_tier_for(0).as_deref(), Some("VUS-low"));
    }
}
