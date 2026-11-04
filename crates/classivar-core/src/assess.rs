//! Scoring: turns the criteria a curator applied into points, a class, and an
//! explanation. The curator always decides which criteria apply. This code
//! only adds up and explains; it never picks criteria itself.

use std::collections::HashSet;

use crate::model::{Class, Direction, Strength};
use crate::rules::RuleSet;

/// One criterion the curator applied, at the strength they chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub code: String,
    pub strength: Strength,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub code: String,
    pub direction: Direction,
    pub strength: Strength,
    pub default_strength: Strength,
    /// Signed: positive for pathogenic evidence, negative for benign.
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// Pathogenic and benign evidence are both present.
    ConflictingEvidence,
    /// Two criteria that should not be used together.
    ExclusivePair { a: String, b: String, reason: String },
    /// A criterion the experts no longer recommend.
    Discouraged { code: String, note: Option<String> },
    /// The curator changed a criterion's strength from its default.
    StrengthChanged { code: String, from: Strength, to: Strength },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextClass {
    pub class: Class,
    pub points_needed: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub items: Vec<Item>,
    pub pathogenic_points: i32,
    /// Negative or zero.
    pub benign_points: i32,
    pub total_points: i32,
    pub points_class: Class,
    /// The 2015 combining table. None if the evidence uses a strength the 2015
    /// table does not cover (for example moderate benign evidence).
    pub classic_class: Option<Class>,
    /// Only set when official VUS tier ranges exist in the rule file.
    pub vus_tier: Option<String>,
    pub warnings: Vec<Warning>,
    /// How many more points of pathogenic evidence reach the next class up.
    pub to_upgrade: Option<NextClass>,
    /// How many more points of benign evidence reach the next class down.
    pub to_downgrade: Option<NextClass>,
}

impl Assessment {
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
}

impl RuleSet {
    pub fn assess(&self, applied: &[Applied]) -> Result<Assessment, AssessError> {
        let mut items = Vec::with_capacity(applied.len());
        let mut seen = HashSet::new();
        let mut warnings = Vec::new();

        for a in applied {
            let c = self.criterion(&a.code).ok_or_else(|| AssessError::UnknownCriterion(a.code.clone()))?;
            if !seen.insert(a.code.as_str()) {
                return Err(AssessError::DuplicateCriterion(a.code.clone()));
            }
            if !c.allowed_strengths().contains(&a.strength) {
                return Err(AssessError::StrengthNotAllowed { code: a.code.clone(), strength: a.strength });
            }
            let magnitude = self.magnitude(a.strength);
            let points = match c.direction {
                Direction::Pathogenic => magnitude,
                Direction::Benign => -magnitude,
            };
            if a.strength != c.strength {
                warnings.push(Warning::StrengthChanged { code: a.code.clone(), from: c.strength, to: a.strength });
            }
            if c.discouraged {
                warnings.push(Warning::Discouraged { code: a.code.clone(), note: c.note.clone() });
            }
            items.push(Item {
                code: a.code.clone(),
                direction: c.direction,
                strength: a.strength,
                default_strength: c.strength,
                points,
            });
        }

        let pathogenic_points: i32 = items.iter().filter(|i| i.points > 0).map(|i| i.points).sum();
        let benign_points: i32 = items.iter().filter(|i| i.points < 0).map(|i| i.points).sum();
        if pathogenic_points > 0 && benign_points < 0 {
            warnings.push(Warning::ConflictingEvidence);
        }
        for pair in &self.exclusive_pairs {
            if seen.contains(pair.a.as_str()) && seen.contains(pair.b.as_str()) {
                warnings.push(Warning::ExclusivePair {
                    a: pair.a.clone(),
                    b: pair.b.clone(),
                    reason: pair.reason.clone(),
                });
            }
        }

        let total_points = pathogenic_points + benign_points;
        let points_class = self.class_for_points(total_points);

        let to_upgrade = points_class.up().and_then(|next| {
            self.class_min(next).map(|min| NextClass { class: next, points_needed: min - total_points })
        });
        let to_downgrade = points_class.down().and_then(|next| {
            self.class_min(points_class).map(|min| NextClass { class: next, points_needed: total_points - (min - 1) })
        });

        let vus_tier = if points_class == Class::Vus { self.vus_tier_for(total_points) } else { None };

        Ok(Assessment {
            classic_class: classic_class(&items),
            items,
            pathogenic_points,
            benign_points,
            total_points,
            points_class,
            vus_tier,
            warnings,
            to_upgrade,
            to_downgrade,
        })
    }

    /// A tier is only named when every tier has an official range.
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

/// The ACMG/AMP 2015 combining table (Richards et al., Table 5).
fn classic_class(items: &[Item]) -> Option<Class> {
    let count = |direction: Direction, strength: Strength| {
        items.iter().filter(|i| i.direction == direction && i.strength == strength).count()
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
    // The 2015 table has no moderate or very strong benign rows.
    let benign_uncovered =
        count(Direction::Benign, Strength::Moderate) + count(Direction::Benign, Strength::VeryStrong);
    if benign_uncovered > 0 {
        return None;
    }

    let pathogenic = (pvs >= 1 && (ps >= 1 || pm >= 2 || (pm >= 1 && pp >= 1) || pp >= 2))
        || ps >= 2
        || (ps >= 1 && (pm >= 3 || (pm >= 2 && pp >= 2) || (pm >= 1 && pp >= 4)));
    let likely_pathogenic = (pvs >= 1 && pm >= 1)
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

    // If both sides are met, the 2015 guidance says VUS.
    Some(match (pathogenic_class, benign_class) {
        (Some(_), Some(_)) | (None, None) => Class::Vus,
        (Some(class), None) | (None, Some(class)) => class,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Strength::*;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin()
    }

    fn apply(list: &[(&str, Strength)]) -> Assessment {
        let applied: Vec<Applied> =
            list.iter().map(|(code, strength)| Applied { code: code.to_string(), strength: *strength }).collect();
        rules().assess(&applied).expect("valid evidence")
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
    fn no_evidence_is_a_vus_at_zero() {
        let a = apply(&[]);
        assert_eq!(a.total_points, 0);
        assert_eq!(a.points_class, Class::Vus);
        assert_eq!(a.classic_class, Some(Class::Vus));
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
        assert_eq!(a.points_class, Class::Benign);
        assert_eq!(a.frameworks_agree(), Some(true));
    }

    #[test]
    fn moderate_benign_evidence_is_not_covered_by_the_classic_table() {
        let a = apply(&[("BP4", Moderate)]);
        assert_eq!(a.classic_class, None);
        assert_eq!(a.frameworks_agree(), None);
    }

    #[test]
    fn exclusive_pairs_and_discouraged_criteria_produce_warnings() {
        let a = apply(&[("PP3", Supporting), ("BP4", Supporting), ("PP5", Supporting)]);
        assert!(
            a.warnings.iter().any(|w| matches!(w, Warning::ExclusivePair { a, b, .. } if a == "PP3" && b == "BP4"))
        );
        assert!(a.warnings.iter().any(|w| matches!(w, Warning::Discouraged { code, .. } if code == "PP5")));
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
        let one = |code: &str, strength| vec![Applied { code: code.into(), strength }];
        assert_eq!(r.assess(&one("XX9", Strong)), Err(AssessError::UnknownCriterion("XX9".into())));
        assert_eq!(
            r.assess(&one("PS1", StandAlone)),
            Err(AssessError::StrengthNotAllowed { code: "PS1".into(), strength: StandAlone })
        );
        let twice =
            vec![Applied { code: "PS1".into(), strength: Strong }, Applied { code: "PS1".into(), strength: Strong }];
        assert_eq!(r.assess(&twice), Err(AssessError::DuplicateCriterion("PS1".into())));
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
