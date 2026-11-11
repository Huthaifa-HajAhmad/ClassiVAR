//! Reading a score, an assay result or a patient count against the published
//! tables. A reading is evidence the curator may apply; it is never applied here.

use crate::model::Strength;

use super::{DeNovoReading, EvidenceReading, LocusReading, Pm3Reading, RuleSet};

/// Points come in steps of 0.1 or more; rounding removes sums such as 2.9999999999999996.
fn round_points(points: f64) -> f64 {
    (points * 100.0).round() / 100.0
}

impl RuleSet {
    /// What a tool's score means for the evidence, or `None` when the score lies
    /// between the limits, is outside the tool's stated range, is not a finite
    /// number, or the tool id is unknown.
    pub fn read_predictor(&self, id: &str, score: f64) -> Option<EvidenceReading> {
        let tool = self.predictor(id)?;
        if !score.is_finite() {
            return None;
        }
        if let (Some(min), Some(max)) = (tool.min_score, tool.max_score)
            && !(min..=max).contains(&score)
        {
            return None;
        }
        // Limits run weakest first, so the last one a score reaches is the strongest.
        if let Some(cut) = tool.pathogenic.iter().rfind(|cut| score >= cut.at) {
            self.reading("PP3", cut.strength)
        } else {
            let cut = tool.benign.iter().rfind(|cut| score <= cut.at)?;
            self.reading("BP4", cut.strength)
        }
    }

    /// The strength of functional evidence from the number of validation controls
    /// alone (no statistical analysis). `damaging` says whether the assay showed
    /// damage (PS3) or no damage (BS3). `None` when the rules have no functional table
    /// or the assay has too few controls to give any evidence.
    pub fn read_functional_controls(&self, damaging: bool, controls: u32) -> Option<EvidenceReading> {
        let rules = self.functional.as_ref()?;
        if controls < rules.controls_supporting_min {
            return None;
        }
        let strength = if controls >= rules.controls_moderate_min { Strength::Moderate } else { Strength::Supporting };
        self.reading(if damaging { "PS3" } else { "BS3" }, strength)
    }

    /// The strength of functional evidence from an OddsPath value. `None` when the
    /// value lies in the indeterminate range, is negative or is not a finite number.
    pub fn read_oddspath(&self, value: f64) -> Option<EvidenceReading> {
        let rules = self.functional.as_ref()?;
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        if let Some(cut) = rules.pathogenic.iter().rfind(|cut| value > cut.at) {
            self.reading("PS3", cut.strength)
        } else {
            let cut = rules.benign.iter().rfind(|cut| value < cut.at)?;
            self.reading("BS3", cut.strength)
        }
    }

    /// The PP4 points a diagnostic yield (percent) earns. When `variants_on_allele`
    /// variants share one allele, the paper divides the yield (a probability) among
    /// them before it reads the table; points must not be divided. `None` when the
    /// rules have no table, the yield is not between 0 and 100, there are no
    /// variants, or the (divided) yield is below the table's first row.
    pub fn pp4_points(&self, diagnostic_yield: f64, variants_on_allele: u32) -> Option<f64> {
        let rules = self.locus_evidence.as_ref()?;
        if !(0.0..=100.0).contains(&diagnostic_yield) || variants_on_allele == 0 {
            return None;
        }
        let shared = diagnostic_yield / f64::from(variants_on_allele);
        rules.pp4_yield.iter().rfind(|step| shared >= step.at).map(|step| step.points)
    }

    /// The PP1 points for `count` informative co-segregations of one kind. `None`
    /// when the rules have no table or the kind id is unknown.
    pub fn pp1_points(&self, kind_id: &str, count: u32) -> Option<f64> {
        let kind = self.locus_evidence.as_ref()?.segregation.iter().find(|kind| kind.id == kind_id)?;
        Some(round_points(kind.points_each * f64::from(count)))
    }

    /// Adds the PP1 and PP4 points, applies the limit for the two together and
    /// lists the strength choices the paper allows. `None` when the rules have no
    /// table or a point value is negative or not a number.
    ///
    /// With `pp1_allowed` false (only one gene can cause the phenotype) only PP4
    /// choices are listed: the highest band that has a PP4-only choice, at or
    /// below the total. Table 4 says "maximum allowable", so a lower band's choice
    /// is also allowed.
    pub fn read_locus(&self, pp1_points: f64, pp4_points: f64, pp1_allowed: bool) -> Option<LocusReading> {
        let rules = self.locus_evidence.as_ref()?;
        if [pp1_points, pp4_points].iter().any(|points| !points.is_finite() || *points < 0.0) {
            return None;
        }
        let sum = round_points(pp1_points + pp4_points);
        let total = sum.min(rules.cap_points);
        let choices = rules
            .bands
            .iter()
            .rev()
            .filter(|band| total >= band.min_points)
            .map(|band| {
                band.options.iter().filter(|choice| pp1_allowed || choice.pp1.is_none()).copied().collect::<Vec<_>>()
            })
            .find(|options| !options.is_empty())
            .unwrap_or_default();
        Some(LocusReading { sum, total, capped: sum > rules.cap_points, choices })
    }

    /// The BS4 evidence for a situation. `None` when the rules have no such table,
    /// the id is unknown, or the source says BS4 does not apply in that situation.
    pub fn read_non_segregation(&self, situation_id: &str) -> Option<EvidenceReading> {
        let rules = self.non_segregation.as_ref()?;
        let situation = rules.situations.iter().find(|s| s.id == situation_id)?;
        if !situation.applies {
            return None;
        }
        self.reading("BS4", rules.strength)
    }

    /// Adds the de novo points of the patients and reads the total as a PS2
    /// strength and as a PM6 strength. `counts` has one `(confirmed, unconfirmed)`
    /// pair of patient numbers for each row of the table, in the table's order.
    /// With `recessive_without_second_variant` true the strength is lowered one
    /// level. `None` when the rules have no table or `counts` does not match its rows.
    pub fn read_de_novo(&self, counts: &[(u32, u32)], recessive_without_second_variant: bool) -> Option<DeNovoReading> {
        let rules = self.de_novo.as_ref()?;
        if counts.len() != rules.phenotypes.len() {
            return None;
        }
        let sum: f64 = rules
            .phenotypes
            .iter()
            .zip(counts)
            .map(|(row, (confirmed, unconfirmed))| {
                let points = row.confirmed * f64::from(*confirmed) + row.unconfirmed * f64::from(*unconfirmed);
                row.max_points.map_or(points, |max| points.min(max))
            })
            .sum();
        let points = round_points(sum);
        // Thresholds run weakest first; the last one the total reaches is the strength.
        let reached = rules.thresholds.iter().rposition(|cut| points >= cut.at);
        let level = if recessive_without_second_variant { reached.and_then(|i| i.checked_sub(1)) } else { reached };
        let strength = level.map(|i| rules.thresholds[i].strength);
        Some(DeNovoReading {
            points,
            evidence: strength.and_then(|s| self.reading("PS2", s)),
            pm6: strength.and_then(|s| self.reading("PM6", s)),
            lowered_for_recessive: recessive_without_second_variant && reached.is_some(),
        })
    }

    /// Adds PM3 patient counts after the curator has checked the source's
    /// eligibility conditions. Counts follow the rule table's row order.
    pub fn read_pm3(&self, counts: &[u32]) -> Option<Pm3Reading> {
        let rules = self.pm3.as_ref()?;
        if counts.len() != rules.rows.len() {
            return None;
        }
        let points = round_points(
            rules
                .rows
                .iter()
                .zip(counts)
                .map(|(row, count)| {
                    let sum = row.points * f64::from(*count);
                    row.max_points.map_or(sum, |max| sum.min(max))
                })
                .sum(),
        );
        let evidence = rules
            .thresholds
            .iter()
            .rev()
            .find(|cut| points >= cut.at)
            .and_then(|cut| self.reading("PM3", cut.strength));
        Some(Pm3Reading { points, evidence })
    }

    /// A published strength for `code`, lowered to the highest this program offers
    /// for that criterion when the source goes higher.
    fn reading(&self, code: &'static str, published: Strength) -> Option<EvidenceReading> {
        let allowed = self.criterion(code)?.allowed_strengths();
        let strength = if allowed.contains(&published) { published } else { *allowed.last()? };
        Some(EvidenceReading { code, published, strength })
    }
}
