//! Population frequency: when BA1, BS1 or PM2 applies.
//!
//! - BA1 (Ghosh 2018): allele frequency above 0.05 in a continental population
//!   dataset of at least 2,000 observed alleles, in a gene without its own BA1 rule.
//! - BS1 (Whiffin 2017): the filtering allele frequency (the lower bound of the
//!   95% confidence interval, gnomAD's FAF95) is at or above the maximum credible
//!   frequency for the disease, worked out from its prevalence, penetrance and
//!   genetic make-up. An expert panel's gene-specific limit replaces it.
//! - PM2 (ClinGen 2020, Supporting): absent from controls, or "at extremely low
//!   frequency if recessive". No general number exists for "extremely low", so
//!   for recessive disease only an expert panel's limit is used.

use serde::Deserialize;

use crate::condition::Inheritance;
use crate::model::Strength;

use super::{EvidenceReading, RuleSet};

/// The population table of the rule file.
#[derive(Debug, Clone, Deserialize)]
pub struct PopulationRules {
    pub ba1_source: String,
    /// BA1 needs a frequency strictly above this (0.05).
    pub ba1_frequency: f64,
    /// ... in a dataset with at least this many observed alleles at the site (2,000).
    pub ba1_min_alleles: u64,
    pub bs1_source: String,
    pub pm2_source: String,
    /// Cautions shown with the helper, in the sources' words.
    #[serde(default)]
    pub notes: Vec<String>,
}

/// What is known about the disease, for the maximum credible allele frequency.
/// Every value is a fraction from 0 (excluded) to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiseaseArchitecture {
    /// How common the disease is, for example 1/500 = 0.002.
    pub prevalence: f64,
    pub penetrance: f64,
    /// The largest share of cases one variant could explain.
    pub allelic_contribution: f64,
    /// The largest share of cases this gene could explain (recessive formula only).
    pub genetic_contribution: f64,
}

/// What the curator read from the population data.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PopulationFacts {
    /// The highest allele frequency in a continental population.
    pub frequency: Option<f64>,
    /// The observed alleles at the site in that population.
    pub alleles: Option<u64>,
    /// The filtering allele frequency (gnomAD FAF95, highest group).
    pub filtering_frequency: Option<f64>,
    /// The variant is absent from the dataset (allele count 0), where it is well covered.
    pub absent: bool,
    pub inheritance: Option<Inheritance>,
    pub architecture: Option<DiseaseArchitecture>,
    /// An expert panel's gene-specific limits, used instead of the general ones.
    pub gene_ba1: Option<f64>,
    pub gene_bs1: Option<f64>,
    pub gene_pm2: Option<f64>,
}

/// One criterion's answer: the reading to apply, or why it does not apply.
#[derive(Debug, Clone, PartialEq)]
pub struct PopulationFinding {
    pub code: &'static str,
    pub reading: Option<EvidenceReading>,
    pub why: String,
}

fn fraction(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value <= 1.0
}

/// The maximum credible population allele frequency (Whiffin 2017). Dominant:
/// prevalence × ½ × allelic contribution ÷ penetrance (the paper's worked
/// example halves the prevalence, as each carrier has one copy among two).
/// Recessive: √prevalence × allelic contribution × √genetic contribution ÷
/// √penetrance. None for X-linked, mitochondrial or unknown inheritance (the
/// paper gives no formula) or for a value outside 0-1.
pub fn max_credible_frequency(inheritance: Inheritance, a: &DiseaseArchitecture) -> Option<f64> {
    let values = [a.prevalence, a.penetrance, a.allelic_contribution, a.genetic_contribution];
    if !values.into_iter().all(fraction) {
        return None;
    }
    match inheritance {
        Inheritance::AutosomalDominant | Inheritance::Semidominant => {
            Some(a.prevalence * 0.5 * a.allelic_contribution / a.penetrance)
        }
        Inheritance::AutosomalRecessive => {
            Some(a.prevalence.sqrt() * a.allelic_contribution * a.genetic_contribution.sqrt() / a.penetrance.sqrt())
        }
        Inheritance::XLinked | Inheritance::Mitochondrial | Inheritance::Unknown => None,
    }
}

impl RuleSet {
    /// What the population facts say about BA1, BS1 and PM2, in that order.
    /// `on_ba1_exception_list` is true when the variant is one ClinGen exempts from BA1.
    pub fn read_population(
        &self,
        facts: &PopulationFacts,
        on_ba1_exception_list: bool,
    ) -> Option<Vec<PopulationFinding>> {
        let table = self.population.as_ref()?;
        let ba1 = self.population_ba1(table, facts, on_ba1_exception_list);
        let bs1 = self.population_bs1(facts, ba1.reading.is_some());
        let common = ba1.reading.is_some() || bs1.reading.is_some();
        let pm2 = self.population_pm2(facts, common);
        Some(vec![ba1, bs1, pm2])
    }

    fn population_ba1(&self, table: &PopulationRules, facts: &PopulationFacts, exempt: bool) -> PopulationFinding {
        let finding = |reading, why: String| PopulationFinding { code: "BA1", reading, why };
        let limit = facts.gene_ba1.unwrap_or(table.ba1_frequency);
        let Some(frequency) = facts.frequency.filter(|f| (0.0..=1.0).contains(f)) else {
            return finding(None, "Type the highest continental allele frequency to judge BA1.".into());
        };
        if frequency <= limit {
            return finding(None, format!("{frequency} is not above {limit}."));
        }
        if exempt {
            return finding(None, "This variant is on ClinGen's BA1 exception list: do not apply BA1.".into());
        }
        match facts.alleles {
            Some(n) if n >= table.ba1_min_alleles => finding(
                self.reading("BA1", Strength::StandAlone),
                format!("{frequency} is above {limit} in {n} alleles."),
            ),
            Some(n) => finding(None, format!("Only {n} alleles: BA1 needs at least {}.", table.ba1_min_alleles)),
            None => finding(None, format!("Type the allele number: BA1 needs at least {}.", table.ba1_min_alleles)),
        }
    }

    fn population_bs1(&self, facts: &PopulationFacts, ba1_applies: bool) -> PopulationFinding {
        let finding = |reading, why: String| PopulationFinding { code: "BS1", reading, why };
        if ba1_applies {
            return finding(None, "BA1 applies, which is enough on its own.".into());
        }
        let credible = facts.inheritance.zip(facts.architecture).and_then(|(i, a)| max_credible_frequency(i, &a));
        let Some(limit) = facts.gene_bs1.or(credible) else {
            return finding(None, "Give the disease's prevalence, penetrance and contributions (dominant or recessive), or the gene's own BS1 limit.".into());
        };
        let Some(filtering) = facts.filtering_frequency.filter(|f| (0.0..=1.0).contains(f)) else {
            return finding(None, "Type the filtering allele frequency (gnomAD FAF95): BS1 is judged on it, not on the observed frequency.".into());
        };
        if filtering >= limit {
            finding(
                self.reading("BS1", Strength::Strong),
                format!("Filtering frequency {filtering:.3e} is at or above {limit:.3e}."),
            )
        } else {
            finding(None, format!("Filtering frequency {filtering:.3e} is below {limit:.3e}."))
        }
    }

    fn population_pm2(&self, facts: &PopulationFacts, common: bool) -> PopulationFinding {
        let finding = |reading, why: String| PopulationFinding { code: "PM2", reading, why };
        if common {
            return finding(None, "The variant is too common for PM2.".into());
        }
        if facts.absent {
            return finding(self.reading("PM2", Strength::Supporting), "Absent from the population data.".into());
        }
        let recessive = facts.inheritance == Some(Inheritance::AutosomalRecessive);
        match (recessive, facts.gene_pm2, facts.frequency) {
            (true, Some(limit), Some(frequency)) if frequency <= limit => finding(
                self.reading("PM2", Strength::Supporting),
                format!("{frequency} is at or below the gene's limit of {limit} for a recessive disease."),
            ),
            (true, None, _) => finding(
                None,
                "For a recessive disease, \"extremely low\" has no general number: use the gene's expert panel limit."
                    .into(),
            ),
            _ => finding(None, "Present in the population data, so not absent.".into()),
        }
    }
}

impl RuleSet {
    /// The population table needs its sources, a frequency between 0 and 1, an
    /// allele count, and the criteria it reads as written: BA1 stand-alone
    /// benign, BS1 benign and PM2 pathogenic.
    pub(super) fn check_population(&self, known: &std::collections::HashSet<&str>) -> Result<(), super::RuleError> {
        let Some(table) = &self.population else { return Ok(()) };
        let bad = |problem: &str| Err(super::RuleError::BadPopulation(problem.to_string()));
        for source in [&table.ba1_source, &table.bs1_source, &table.pm2_source] {
            super::check::needs_source(known, "population frequency", source)?;
        }
        if !(table.ba1_frequency > 0.0 && table.ba1_frequency < 1.0) || table.ba1_min_alleles == 0 {
            return bad("ba1_frequency must lie between 0 and 1 and ba1_min_alleles must be at least 1");
        }
        let is = |code: &str, direction: crate::model::Direction| {
            self.criterion(code).is_some_and(|c| c.direction == direction && !c.discouraged)
        };
        let ba1_alone = self.criterion("BA1").is_some_and(|c| c.strength == Strength::StandAlone);
        if !(ba1_alone && is("BS1", crate::model::Direction::Benign) && is("PM2", crate::model::Direction::Pathogenic))
        {
            return bad("it needs BA1 (stand-alone benign), BS1 (benign) and PM2 (pathogenic)");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
