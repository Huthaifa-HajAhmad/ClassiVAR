//! The rule set: loaded from a TOML data file and checked once, at load time.
//! After that the rest of the program can trust it.
//!
//! A `RuleSet` is checked when it is made through `RuleSet::parse`,
//! `RuleSet::builtin`, or serde (which runs the same checks), so a bad rule file
//! does not reach the scoring code. The fields stay public, so code that changes
//! them afterwards (some tests do) skips the checks.

mod basics;
mod check;
mod check_locus;
mod check_predictors;
mod check_tables;
mod criterion;
mod facts;
mod path;
mod population;
mod read;
mod review;
mod tables;
#[cfg(test)]
mod tests;

use std::sync::OnceLock;

use serde::Deserialize;

use crate::model::{Class, Strength};

pub use basics::*;
pub use criterion::{Criterion, GeneLevel, StrengthGuide};
pub use facts::{Fact, Hint};
pub use path::{HELPER_CRITERIA, ReviewStep};
pub use population::{
    DiseaseArchitecture, PopulationFacts, PopulationFinding, PopulationRules, max_credible_frequency,
};
pub use review::{ReviewItem, Signoff, parse_signoffs};
pub use tables::*;

/// The built-in rule set, embedded in the program.
const BUILTIN: &str = include_str!("../../../../rules/acmg-amp-points.toml");

/// Point values and class limits above this are almost certainly typing mistakes.
/// The limit also keeps every sum far away from the edge of `i32`.
const MAX_POINTS: i32 = 1000;

/// The rule file as written. It becomes a `RuleSet` only after `RuleSet::check` passes.
#[derive(Deserialize)]
struct RuleSetFile {
    path: Option<Vec<ReviewStep>>,
    #[serde(default)]
    facts: Vec<Fact>,
    meta: Meta,
    sources: Vec<Source>,
    points: Points,
    class_min_points: ClassMinPoints,
    exclusive_pairs: Vec<ExclusivePair>,
    #[serde(default)]
    combination_caps: Vec<CombinationCap>,
    #[serde(default)]
    predictors: Vec<Predictor>,
    functional: Option<FunctionalRules>,
    locus_evidence: Option<LocusRules>,
    non_segregation: Option<NonSegregationRules>,
    de_novo: Option<DeNovoRules>,
    pm3: Option<Pm3Rules>,
    population: Option<PopulationRules>,
    // A file may have no tier table at all (the engine treats that as "no tiers").
    #[serde(default)]
    vus_tiers: Vec<VusTier>,
    #[serde(default)]
    ba1_exceptions: Vec<Ba1Exception>,
    criteria: Vec<Criterion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "RuleSetFile")]
pub struct RuleSet {
    /// Optional ordered review; absent rules keep the free workbench available.
    pub path: Option<Vec<ReviewStep>>,
    /// Optional fact questions whose answers add hints to the review steps.
    pub facts: Vec<Fact>,
    pub meta: Meta,
    pub sources: Vec<Source>,
    pub points: Points,
    pub class_min_points: ClassMinPoints,
    pub vus_tiers: Vec<VusTier>,
    pub exclusive_pairs: Vec<ExclusivePair>,
    /// Pairs whose combined points are limited (empty in older rule files).
    pub combination_caps: Vec<CombinationCap>,
    /// Calibrated prediction tools for PP3 and BP4 (empty in older rule files).
    pub predictors: Vec<Predictor>,
    /// Functional-assay evidence limits for PS3 and BS3 (absent in older rule files).
    pub functional: Option<FunctionalRules>,
    /// PP1 and PP4 points and the strengths the paper allows (absent in older rule files).
    pub locus_evidence: Option<LocusRules>,
    /// The situations in which BS4 applies (absent in older rule files).
    pub non_segregation: Option<NonSegregationRules>,
    /// De novo points for PS2 (absent in older rule files).
    pub de_novo: Option<DeNovoRules>,
    pub pm3: Option<Pm3Rules>,
    /// BA1, BS1 and PM2 limits (absent in older rule files).
    pub population: Option<PopulationRules>,
    pub ba1_exceptions: Vec<Ba1Exception>,
    pub criteria: Vec<Criterion>,
}

impl TryFrom<RuleSetFile> for RuleSet {
    type Error = RuleError;

    fn try_from(file: RuleSetFile) -> Result<Self, RuleError> {
        let rules = RuleSet {
            path: file.path,
            facts: file.facts,
            meta: file.meta,
            sources: file.sources,
            points: file.points,
            class_min_points: file.class_min_points,
            vus_tiers: file.vus_tiers,
            exclusive_pairs: file.exclusive_pairs,
            combination_caps: file.combination_caps,
            predictors: file.predictors,
            functional: file.functional,
            locus_evidence: file.locus_evidence,
            non_segregation: file.non_segregation,
            de_novo: file.de_novo,
            pm3: file.pm3,
            population: file.population,
            ba1_exceptions: file.ba1_exceptions,
            criteria: file.criteria,
        };
        rules.check()?;
        Ok(rules)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("guided review path: {0}")]
    BadPath(String),
    #[error("could not read the rule file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("criterion {0} appears twice")]
    DuplicateCode(String),
    #[error("{item} cites source '{source_id}', which is not listed under [[sources]]")]
    UnknownSource { item: String, source_id: String },
    #[error("exclusive pair {a}/{b} names a criterion that does not exist")]
    UnknownPairCode { a: String, b: String },
    #[error("combination cap {a}/{b}: {problem}")]
    BadCombinationCap { a: String, b: String, problem: String },
    #[error("criterion {0}: a strength-guide situation is empty or appears twice")]
    BadSituation(String),
    #[error("criterion {0}: its strength guide offers no answer that applies")]
    GuideWithoutAnswer(String),
    #[error("criterion {0}: guide_source is set but the criterion has no guide")]
    GuideSourceWithoutGuide(String),
    #[error("source {0} appears twice under [[sources]]")]
    DuplicateSource(String),
    #[error(
        "criterion {0} has a strength guide, but the helper that sets {0} cannot answer a guide; remove the guide from {0}"
    )]
    GuideOnHelperCriterion(String),
    #[error("criterion {0}: original_strength must be a recorded strength different from today's default")]
    BadOriginalStrength(String),
    #[error("exclusive pair {0}: the two criteria must differ and the pair must appear once")]
    BadExclusivePair(String),
    #[error(
        "class thresholds must fall in strict order (pathogenic > likely_pathogenic > vus > likely_benign) and vus must be 0"
    )]
    BadThresholds,
    #[error(
        "point values must satisfy 0 < supporting < moderate < strong < very_strong, and stand_alone must be enough on its own for Benign (more than the likely_benign limit); all at most {MAX_POINTS}"
    )]
    BadPoints,
    #[error("criterion {code} has strength stand_alone but is not a benign criterion")]
    StandAloneNotBenign { code: String },
    #[error("criterion {code}: its default strength is not among the strengths a curator may choose")]
    DefaultNotAllowed { code: String },
    #[error("criterion {code}: its strength guide has no source, or offers a strength the criterion does not allow")]
    BadGuide { code: String },
    #[error(
        "criterion {0}: its gene-level question needs a guide, a known source, and distinct answers of at most 3 steps"
    )]
    BadGeneLevel(String),
    #[error("criterion {code}: max_strength is not a strength a record can use")]
    MaxStrengthNotRecorded { code: String },
    #[error("criterion {code}: levels must be strengths a record can use, weakest first, each once")]
    BadLevels { code: String },
    #[error("the rule file has no criteria, or has criteria for only one direction")]
    MissingCriteria,
    #[error("the rule set is marked reviewed but has no reviewer name or no review date in the form YYYY-MM-DD")]
    ReviewIncomplete,
    #[error("functional evidence limits: {0}")]
    BadFunctional(String),
    #[error("co-segregation and phenotype limits: {0}")]
    BadLocusEvidence(String),
    #[error("non-segregation situations: {0}")]
    BadNonSegregation(String),
    #[error("de novo points: {0}")]
    BadDeNovo(String),
    #[error("population frequency: {0}")]
    BadPopulation(String),
    #[error("PM3 points: {0}")]
    BadPm3(String),
    #[error("BA1 reference: {0}")]
    BadBa1(String),
    #[error("predictor {id}: {problem}")]
    BadPredictor { id: String, problem: String },
    #[error("VUS tiers: {0}")]
    BadTiers(String),
}

impl RuleSet {
    /// Read and check a rule file.
    pub fn parse(text: &str) -> Result<RuleSet, RuleError> {
        let file: RuleSetFile = toml::from_str(text)?;
        RuleSet::try_from(file)
    }

    /// The rule set embedded in the program. A test proves it is valid; the
    /// program also checks it once at start-up and shows the error if it is not.
    pub fn builtin() -> Result<&'static RuleSet, &'static RuleError> {
        static RULES: OnceLock<Result<RuleSet, RuleError>> = OnceLock::new();
        RULES.get_or_init(|| RuleSet::parse(BUILTIN)).as_ref()
    }

    /// The id and version of this rule set.
    pub fn reference(&self) -> RuleSetRef {
        RuleSetRef { id: self.meta.id.clone(), version: self.meta.version.clone() }
    }

    pub fn criterion(&self, code: &str) -> Option<&Criterion> {
        self.criteria.iter().find(|c| c.code == code)
    }

    pub fn source(&self, id: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn predictor(&self, id: &str) -> Option<&Predictor> {
        self.predictors.iter().find(|p| p.id == id)
    }

    /// Points (always positive) for a strength.
    pub fn magnitude(&self, strength: Strength) -> i32 {
        match strength {
            Strength::Supporting => self.points.supporting,
            Strength::Moderate => self.points.moderate,
            Strength::Strong => self.points.strong,
            Strength::VeryStrong => self.points.very_strong,
            Strength::StandAlone => self.points.stand_alone,
        }
    }

    /// Lowest total for a class. Benign has no lower limit.
    pub fn class_min(&self, class: Class) -> Option<i32> {
        match class {
            Class::Pathogenic => Some(self.class_min_points.pathogenic),
            Class::LikelyPathogenic => Some(self.class_min_points.likely_pathogenic),
            Class::Vus => Some(self.class_min_points.vus),
            Class::LikelyBenign => Some(self.class_min_points.likely_benign),
            Class::Benign => None,
        }
    }

    pub fn class_for_points(&self, total: i32) -> Class {
        let t = &self.class_min_points;
        if total >= t.pathogenic {
            Class::Pathogenic
        } else if total >= t.likely_pathogenic {
            Class::LikelyPathogenic
        } else if total >= t.vus {
            Class::Vus
        } else if total >= t.likely_benign {
            Class::LikelyBenign
        } else {
            Class::Benign
        }
    }
}
