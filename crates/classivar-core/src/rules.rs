//! The rule set: loaded from a TOML data file and checked once, at load time.
//! After that the rest of the program can trust it.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::model::{Category, Class, Direction, Strength};

/// The built-in rule set, embedded in the program.
const BUILTIN: &str = include_str!("../../../rules/acmg-amp-points.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Unverified,
    Reviewed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: ReviewStatus,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub id: String,
    pub citation: String,
    pub doi: Option<String>,
    pub url: Option<String>,
}

/// Points per strength (always positive; the sign comes from the direction).
#[derive(Debug, Clone, Deserialize)]
pub struct Points {
    pub supporting: i32,
    pub moderate: i32,
    pub strong: i32,
    pub very_strong: i32,
    pub stand_alone: i32,
}

/// The lowest total that still gives each class. Below `likely_benign` is Benign.
#[derive(Debug, Clone, Deserialize)]
pub struct ClassMinPoints {
    pub pathogenic: i32,
    pub likely_pathogenic: i32,
    pub vus: i32,
    pub likely_benign: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VusTier {
    pub name: String,
    pub description: String,
    pub source: String,
    /// Lowest total for this tier. Empty until official ranges are published.
    pub min_points: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExclusivePair {
    pub a: String,
    pub b: String,
    pub reason: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Criterion {
    pub code: String,
    pub direction: Direction,
    /// Default strength. The user can change it per case.
    pub strength: Strength,
    pub category: Category,
    pub title: String,
    pub summary: String,
    pub source: String,
    /// Set only when a later recommendation changed the strength. This is the
    /// strength in the 2015 paper, so the app can explain old classifications.
    pub original_strength: Option<Strength>,
    #[serde(default)]
    pub discouraged: bool,
    pub note: Option<String>,
}

impl Criterion {
    /// The strengths a curator may choose for this criterion. "Stand-alone" is
    /// only offered to a criterion whose default is stand-alone (BA1).
    pub fn allowed_strengths(&self) -> &'static [Strength] {
        if self.strength == Strength::StandAlone {
            &[Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong, Strength::StandAlone]
        } else {
            &[Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong]
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleSet {
    pub meta: Meta,
    pub sources: Vec<Source>,
    pub points: Points,
    pub class_min_points: ClassMinPoints,
    pub vus_tiers: Vec<VusTier>,
    pub exclusive_pairs: Vec<ExclusivePair>,
    pub criteria: Vec<Criterion>,
}

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("could not read the rule file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("criterion {0} appears twice")]
    DuplicateCode(String),
    #[error("{item} cites source '{source_id}', which is not listed under [[sources]]")]
    UnknownSource { item: String, source_id: String },
    #[error("exclusive pair {a}/{b} names a criterion that does not exist")]
    UnknownPairCode { a: String, b: String },
    #[error("class thresholds must fall in strict order: pathogenic > likely_pathogenic > vus > likely_benign")]
    BadThresholds,
    #[error("every point value must be positive")]
    BadPoints,
    #[error("criterion {code} has strength stand_alone but is not a benign criterion")]
    StandAloneNotBenign { code: String },
}

impl RuleSet {
    /// Read and check a rule file.
    pub fn parse(text: &str) -> Result<RuleSet, RuleError> {
        let rules: RuleSet = toml::from_str(text)?;
        rules.check()?;
        Ok(rules)
    }

    /// The rule set embedded in the program. A test proves it is valid.
    pub fn builtin() -> &'static RuleSet {
        static RULES: OnceLock<RuleSet> = OnceLock::new();
        RULES.get_or_init(|| RuleSet::parse(BUILTIN).expect("the built-in rule file is valid"))
    }

    pub fn criterion(&self, code: &str) -> Option<&Criterion> {
        self.criteria.iter().find(|c| c.code == code)
    }

    pub fn source(&self, id: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
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

    fn check(&self) -> Result<(), RuleError> {
        let t = &self.class_min_points;
        if !(t.pathogenic > t.likely_pathogenic && t.likely_pathogenic > t.vus && t.vus > t.likely_benign) {
            return Err(RuleError::BadThresholds);
        }
        let p = &self.points;
        if [p.supporting, p.moderate, p.strong, p.very_strong, p.stand_alone].iter().any(|v| *v <= 0) {
            return Err(RuleError::BadPoints);
        }

        let known_sources: HashSet<&str> = self.sources.iter().map(|s| s.id.as_str()).collect();
        let needs_source = |item: String, source_id: &str| {
            if known_sources.contains(source_id) {
                Ok(())
            } else {
                Err(RuleError::UnknownSource { item, source_id: source_id.to_string() })
            }
        };

        let mut seen = HashSet::new();
        for c in &self.criteria {
            if !seen.insert(c.code.as_str()) {
                return Err(RuleError::DuplicateCode(c.code.clone()));
            }
            needs_source(c.code.clone(), &c.source)?;
            if c.strength == Strength::StandAlone && c.direction != Direction::Benign {
                return Err(RuleError::StandAloneNotBenign { code: c.code.clone() });
            }
        }
        for tier in &self.vus_tiers {
            needs_source(tier.name.clone(), &tier.source)?;
        }
        for pair in &self.exclusive_pairs {
            needs_source(format!("{}/{}", pair.a, pair.b), &pair.source)?;
            if !seen.contains(pair.a.as_str()) || !seen.contains(pair.b.as_str()) {
                return Err(RuleError::UnknownPairCode { a: pair.a.clone(), b: pair.b.clone() });
            }
        }
        Ok(())
    }
}
