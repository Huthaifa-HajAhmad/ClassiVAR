//! The small, plain parts of a rule file: who made it, its sources, the points
//! and the class limits.

use serde::{Deserialize, Serialize};

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
    /// Required when `status` is `reviewed`: who checked the file.
    pub reviewed_by: Option<String>,
    /// Required when `status` is `reviewed`: the date of the review, `YYYY-MM-DD`.
    pub reviewed_on: Option<String>,
}

/// Names one rule set. Saved results and records carry this, so a result can be
/// matched to the rules that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSetRef {
    pub id: String,
    pub version: String,
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

/// A limit on two criteria used together. ClinGen's 2024 phenotype and
/// co-segregation guidance caps the pair at 5 points, so two Strong answers are
/// not allowed even though each is allowed alone.
#[derive(Debug, Clone, Deserialize)]
pub struct CombinationCap {
    pub a: String,
    pub b: String,
    /// The most the two may add up to, in this program's points.
    pub max_points: i32,
    pub reason: String,
    pub source: String,
}
