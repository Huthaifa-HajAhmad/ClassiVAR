//! Criteria that are used together: the pairs that contradict each other, the
//! pairs with a combined limit, and the pairs that may count one observation twice.

use std::collections::HashSet;

use crate::rules::RuleSet;

use super::{CombinationOveruse, Conflict, Item, Warning};

/// Pairs of criteria that may count one observation twice, and what to tell the curator.
const DOUBLE_COUNTED: [(&str, &str, &str); 3] = [
    // Abou Tayoun 2018: for a splice variant, the splicing prediction is already part of PVS1.
    (
        "PVS1",
        "PP3",
        "If PVS1 describes a splice variant, its effect is already counted; do not also count PP3 for the same prediction.",
    ),
    // ClinGen SVI v1.1, Table 2: PS2 and PM6 are alternative codes for one de novo total.
    (
        "PS2",
        "PM6",
        "ClinGen Table 2 lets either code carry the total, not both. Remove one, or count only one observation.",
    ),
    // ClinGen 2024: a family seen before counts as phenotype specificity (PP4) or prevalence (PS4), not both.
    (
        "PP4",
        "PS4",
        "One family observation is PP4 or PS4, not both (ClinGen 2024). Remove one, or count the family once.",
    ),
];

/// A note for each pair of `DOUBLE_COUNTED` that is counted together.
pub(super) fn double_counting_warnings(items: &[Item]) -> Vec<Warning> {
    let counted = |code: &str| items.iter().any(|i| i.counted && i.code == code);
    DOUBLE_COUNTED
        .iter()
        .filter(|(a, b, _)| counted(a) && counted(b))
        .map(|(a, b, reason)| Warning::DoubleCounted { a: a.to_string(), b: b.to_string(), reason: reason.to_string() })
        .collect()
}

impl RuleSet {
    /// The exclusive pairs of which the curator applied both criteria.
    pub(super) fn conflicts_among(&self, applied: &HashSet<&str>) -> Vec<Conflict> {
        self.exclusive_pairs
            .iter()
            .filter(|pair| applied.contains(pair.a.as_str()) && applied.contains(pair.b.as_str()))
            .map(|pair| Conflict { a: pair.a.clone(), b: pair.b.clone(), reason: pair.reason.clone() })
            .collect()
    }

    /// The capped pairs whose counted points together go over the limit.
    pub(super) fn combination_overuse(&self, items: &[Item]) -> Vec<CombinationOveruse> {
        self.combination_caps
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
            .collect()
    }
}
