//! Load-time checks of the helper tables: assay limits (PS3 and BS3), the BA1
//! reference list, PM3 points, de novo points, BS4 situations, and the VUS tiers.

use std::collections::HashSet;

use crate::model::Direction;

use super::check::{check_strength_ladder, limits_move_away_from_the_middle, needs_source};
use super::{BA1_SOURCE, RuleError, RuleSet};

impl RuleSet {
    /// The functional table needs a known source, the criteria PS3 and BS3, and
    /// limits that run away from the indeterminate middle without overlapping.
    pub(super) fn check_functional(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.functional else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadFunctional(problem.to_string()));
        needs_source(known, "functional evidence", &rules.source)?;
        // `read_functional_controls` and `read_oddspath` read the damage as PS3
        // and the absence of damage as BS3, so the codes must sit that way.
        let (Some(ps3), Some(bs3)) = (self.criterion("PS3"), self.criterion("BS3")) else {
            return bad("it needs the criteria PS3 and BS3");
        };
        if ps3.direction != Direction::Pathogenic
            || ps3.discouraged
            || bs3.direction != Direction::Benign
            || bs3.discouraged
        {
            return bad("PS3 must be a counted pathogenic criterion and BS3 a counted benign one");
        }
        // Moderate needs a mix, so at least one pathogenic and one benign control.
        if rules.controls_moderate_min < 2 {
            return bad("controls_moderate_min must be at least 2 (one pathogenic and one benign control)");
        }
        for (limits, rising) in [(&rules.pathogenic, true), (&rules.benign, false)] {
            if let Err(problem) = check_strength_ladder(limits) {
                return bad(problem);
            }
            // An OddsPath value cannot be negative.
            if limits.iter().any(|cut| !cut.at.is_finite() || cut.at < 0.0) {
                return bad("an OddsPath limit is not a number of zero or more");
            }
            if !limits_move_away_from_the_middle(limits, rising) {
                return bad("stronger limits must lie further from the middle of the scale");
            }
        }
        if let (Some(p), Some(b)) = (rules.pathogenic.first(), rules.benign.first())
            && b.at > p.at
        {
            return bad("the benign and pathogenic limits overlap");
        }
        Ok(())
    }

    /// The BA1 entries are a dated manual reference, not evidence logic. The list
    /// only makes sense with the source that published it, and each entry must
    /// still be findable in ClinVar.
    pub(super) fn check_ba1(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        if self.ba1_exceptions.is_empty() {
            return Ok(());
        }
        if !known.contains(BA1_SOURCE) {
            return Err(RuleError::BadBa1("BA1 entries need the clingen-ba1-2018 source".to_string()));
        }
        let mut ids = HashSet::new();
        for row in &self.ba1_exceptions {
            if row.gene.trim().is_empty()
                || row.variant.trim().is_empty()
                || row.clinvar_id.is_empty()
                || !row.clinvar_id.bytes().all(|c| c.is_ascii_digit())
                || !row.allele_id.starts_with("CA")
                || !ids.insert(&row.allele_id)
            {
                return Err(RuleError::BadBa1(
                    "BA1 entries need a gene, variant, numeric ClinVar id and unique allele id".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// The PM3 table needs a known source, the criterion PM3, rows with unique
    /// ids and nonnegative finite points, and thresholds that rise with the
    /// strength, offering only strengths the criterion may use.
    pub(super) fn check_pm3(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let bad = |s: &str| Err(RuleError::BadPm3(s.to_string()));
        let Some(table) = &self.pm3 else { return Ok(()) };
        needs_source(known, "PM3 points", &table.source)?;
        let Some(criterion) = self.criterion("PM3").filter(|c| c.direction == Direction::Pathogenic && !c.discouraged)
        else {
            return bad("PM3 must be a counted pathogenic criterion");
        };
        let mut ids = HashSet::new();
        let rows_ok = !table.rows.is_empty()
            && table.rows.iter().all(|r| {
                !r.id.trim().is_empty()
                    && !r.label.trim().is_empty()
                    && ids.insert(&r.id)
                    && r.points.is_finite()
                    && r.points >= 0.0
                    && r.max_points.is_none_or(|m| m.is_finite() && m > 0.0)
            });
        if !rows_ok {
            return bad("PM3 rows need unique ids, labels, nonnegative finite points and positive finite limits");
        }
        let cuts = &table.thresholds;
        if cuts.is_empty()
            || check_strength_ladder(cuts).is_err()
            || cuts.iter().any(|c| !c.at.is_finite() || c.at <= 0.0)
            || cuts.windows(2).any(|w| w[0].at >= w[1].at)
        {
            return bad("PM3 thresholds must rise in points and strength from Supporting to Very Strong");
        }
        if cuts.iter().any(|cut| !criterion.allowed_strengths().contains(&cut.strength)) {
            return bad("a PM3 threshold offers a strength the PM3 criterion does not let a curator use");
        }
        Ok(())
    }

    /// The de novo table needs a known source, the criterion PS2, rows that never
    /// give less for confirmed parents, and thresholds that rise with the strength.
    pub(super) fn check_de_novo(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.de_novo else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadDeNovo(problem.to_string()));
        needs_source(known, "de novo points", &rules.source)?;
        let ps2 = match self.criterion("PS2") {
            Some(ps2) if ps2.direction == Direction::Pathogenic && !ps2.discouraged => ps2,
            _ => return bad("it needs the criterion PS2, as a pathogenic criterion that is not discouraged"),
        };
        let points_ok = |points: f64| points.is_finite() && points >= 0.0;
        let rows_ok = rules.phenotypes.iter().all(|row| {
            !row.id.trim().is_empty()
                && !row.label.trim().is_empty()
                && points_ok(row.confirmed)
                && points_ok(row.unconfirmed)
                && row.unconfirmed <= row.confirmed
                && row.max_points.is_none_or(|max| max.is_finite() && max > 0.0)
        });
        if rules.phenotypes.is_empty() || !rows_ok {
            return bad(
                "each row needs an id, a label, points of zero or more that are not higher without confirmed parents, and a limit above zero if it has one",
            );
        }
        let mut ids = HashSet::new();
        if !rules.phenotypes.iter().all(|row| ids.insert(row.id.as_str())) {
            return bad("row ids must be unique");
        }
        let cuts = &rules.thresholds;
        if cuts.is_empty() {
            return bad("there must be thresholds");
        }
        if let Err(problem) = check_strength_ladder(cuts) {
            return bad(problem);
        }
        if cuts.iter().any(|cut| !cut.at.is_finite() || cut.at <= 0.0) || cuts.windows(2).any(|w| w[0].at >= w[1].at) {
            return bad("threshold points must be above zero and rise with the strength");
        }
        let allowed = ps2.allowed_strengths();
        if cuts.iter().any(|cut| !allowed.contains(&cut.strength)) {
            return bad("a threshold offers a strength the PS2 criterion does not let a curator use");
        }
        if rules.recessive_text.trim().is_empty() {
            return bad("the recessive text is empty");
        }
        Ok(())
    }

    /// The BS4 table needs a known source, the criterion BS4 at a strength it allows,
    /// and situations with unique ids, of which at least one applies.
    pub(super) fn check_non_segregation(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.non_segregation else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadNonSegregation(problem.to_string()));
        needs_source(known, "non-segregation situations", &rules.source)?;
        let Some(bs4) = self.criterion("BS4") else { return bad("it needs the criterion BS4") };
        if bs4.direction != Direction::Benign || bs4.discouraged {
            return bad("BS4 must be a benign criterion that is not discouraged");
        }
        if !bs4.allowed_strengths().contains(&rules.strength) {
            return bad("the strength is one that BS4 does not allow");
        }
        // The paper recommends -4 points; the helper says so on screen.
        if self.magnitude(rules.strength) != 4 {
            return bad("the strength must be worth 4 points, the paper's -4");
        }
        let shaped = rules
            .situations
            .iter()
            .all(|s| !s.id.trim().is_empty() && !s.label.trim().is_empty() && !s.text.trim().is_empty());
        if rules.situations.is_empty() || !shaped {
            return bad("there must be situations, each with an id, a label and a text");
        }
        let mut ids = HashSet::new();
        if !rules.situations.iter().all(|s| ids.insert(s.id.as_str())) {
            return bad("situation ids must be unique");
        }
        if !rules.situations.iter().any(|s| s.applies) {
            return bad("at least one situation must apply, or BS4 could never be offered");
        }
        Ok(())
    }

    /// Tiers are either all unranged (no official cut-offs yet) or all ranged,
    /// with different minimums, the lowest starting where the VUS range starts,
    /// and every minimum inside the VUS range.
    pub(super) fn check_tiers(&self) -> Result<(), RuleError> {
        let bad = |message: &str| Err(RuleError::BadTiers(message.to_string()));
        // Both unranged and ranged tiers need distinct names: the record screens
        // use the name as the id of the tier buttons and to compare answers.
        let mut names = HashSet::new();
        for tier in &self.vus_tiers {
            if tier.name.trim().is_empty() || !names.insert(tier.name.as_str()) {
                return bad("tier names must be filled in and unique");
            }
        }
        let ranged = self.vus_tiers.iter().filter(|t| t.min_points.is_some()).count();
        if ranged == 0 {
            return Ok(());
        }
        if ranged != self.vus_tiers.len() {
            return bad("either every tier has min_points or none does");
        }
        let (low, high) = (self.class_min_points.vus, self.class_min_points.likely_pathogenic);
        let mut minimums: Vec<i32> = self.vus_tiers.iter().filter_map(|t| t.min_points).collect();
        minimums.sort_unstable();
        if minimums.windows(2).any(|w| w[0] == w[1]) {
            return bad("two tiers start at the same point value");
        }
        if minimums.first() != Some(&low) {
            return bad("the lowest tier must start where the VUS range starts");
        }
        if minimums.last().is_some_and(|m| *m >= high) {
            return bad("a tier starts outside the VUS range");
        }
        Ok(())
    }
}
