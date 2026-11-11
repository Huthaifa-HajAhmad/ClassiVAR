//! Load-time checks of the PP1 and PP4 table (ClinGen 2024).

use std::collections::HashSet;

use crate::model::{Direction, Strength};

use super::check::needs_source;
use super::{Criterion, LocusRules, MAX_POINTS, RuleError, RuleSet};

fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

impl RuleSet {
    /// The PP1/PP4 tables need a known source, the criteria PP1 and PP4, rising
    /// rows, and strength choices that the criteria allow and that never promise
    /// more points than the evidence has earned.
    pub(super) fn check_locus(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.locus_evidence else { return Ok(()) };
        needs_source(known, "co-segregation and phenotype limits", &rules.source)?;
        let bad = |problem: &str| Err(RuleError::BadLocusEvidence(problem.to_string()));
        let (Some(pp1), Some(pp4)) = (self.criterion("PP1"), self.criterion("PP4")) else {
            return bad("it needs the criteria PP1 and PP4");
        };
        if [pp1, pp4].iter().any(|c| c.direction != Direction::Pathogenic || c.discouraged) {
            return bad("PP1 and PP4 must be pathogenic criteria that are not discouraged");
        }
        // Table 4 is written in the standard points, so a different scale would make its choices wrong.
        if (self.points.supporting, self.points.moderate, self.points.strong) != (1, 2, 4) {
            return bad("it needs the standard point scale: Supporting 1, Moderate 2, Strong 4");
        }
        if !positive(rules.cap_points) || rules.cap_points > f64::from(MAX_POINTS) {
            return bad("cap_points must be a positive number");
        }
        // The class is withheld by the combination cap, so it must match the limit the helper uses.
        let pair_cap = self
            .combination_caps
            .iter()
            .find(|cap| (cap.a == "PP1" && cap.b == "PP4") || (cap.a == "PP4" && cap.b == "PP1"));
        if pair_cap.is_none_or(|cap| f64::from(cap.max_points) != rules.cap_points) {
            return bad("it needs a PP1/PP4 combination cap equal to cap_points");
        }
        if !positive(rules.yield_text_minimum) || rules.yield_text_minimum > 100.0 {
            return bad("yield_text_minimum must be a percentage above 0");
        }
        if rules.homogeneity_text.trim().is_empty() || rules.notes.iter().any(|note| note.trim().is_empty()) {
            return bad("the homogeneity text and every note must be filled in");
        }
        check_locus_rows(rules)?;
        self.check_locus_bands(rules, pp1, pp4)
    }

    /// Every band's strength choices must use strengths the criteria allow and
    /// must not be worth more than the band's points or the pair limit.
    fn check_locus_bands(&self, rules: &LocusRules, pp1: &Criterion, pp4: &Criterion) -> Result<(), RuleError> {
        let bad = |problem: &str| Err(RuleError::BadLocusEvidence(problem.to_string()));
        for band in &rules.bands {
            if band.options.is_empty() {
                return bad("every band needs at least one strength choice");
            }
            for choice in &band.options {
                if choice.pp1.is_none() && choice.pp4.is_none() {
                    return bad("a strength choice names neither PP1 nor PP4");
                }
                let allowed = |criterion: &Criterion, strength: Option<Strength>| {
                    strength.is_none_or(|s| criterion.allowed_strengths().contains(&s))
                };
                if !allowed(pp1, choice.pp1) || !allowed(pp4, choice.pp4) {
                    return bad("a strength choice uses a strength the criterion does not allow");
                }
                let worth: i32 = [choice.pp1, choice.pp4].into_iter().flatten().map(|s| self.magnitude(s)).sum();
                if f64::from(worth) > band.min_points || f64::from(worth) > rules.cap_points {
                    return bad("a strength choice is worth more points than its band has earned");
                }
            }
        }
        Ok(())
    }
}

/// The yield rows, the kinds of co-segregation and the point bands must be filled
/// in and rise.
fn check_locus_rows(rules: &LocusRules) -> Result<(), RuleError> {
    let bad = |problem: &str| Err(RuleError::BadLocusEvidence(problem.to_string()));
    if rules.pp4_yield.is_empty()
        || rules.pp4_yield.iter().any(|step| !positive(step.points) || !positive(step.at) || step.at > 100.0)
    {
        return bad("every yield row needs a yield between 0 and 100 and positive points, and there must be rows");
    }
    if rules.pp4_yield.windows(2).any(|w| w[0].at >= w[1].at || w[0].points >= w[1].points) {
        return bad("yield rows must rise in both yield and points");
    }
    if rules.segregation.is_empty()
        || rules
            .segregation
            .iter()
            .any(|kind| kind.id.trim().is_empty() || kind.label.trim().is_empty() || !positive(kind.points_each))
    {
        return bad("every co-segregation kind needs an id, a label and positive points, and there must be kinds");
    }
    let mut ids = HashSet::new();
    if !rules.segregation.iter().all(|kind| ids.insert(kind.id.as_str())) {
        return bad("co-segregation ids must be unique");
    }
    if rules.bands.is_empty()
        || rules.bands.iter().any(|band| !positive(band.min_points) || band.min_points > rules.cap_points)
        || rules.bands.windows(2).any(|w| w[0].min_points >= w[1].min_points)
    {
        return bad("bands must start above zero, rise, and not start above cap_points");
    }
    Ok(())
}
