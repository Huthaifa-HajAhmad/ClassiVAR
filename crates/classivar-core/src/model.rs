//! Basic vocabulary shared by the rules and the scoring: strengths, directions,
//! categories and classes.

use serde::{Deserialize, Serialize};

/// How much weight one piece of evidence carries.
///
/// There is deliberately no ordering on this type: `StandAlone` is worth the same
/// points as `VeryStrong`, so a plain "bigger than" would be misleading. Compare
/// points through the rule set instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strength {
    Supporting,
    Moderate,
    Strong,
    VeryStrong,
    /// Only used by BA1. Enough on its own for Benign.
    StandAlone,
}

impl Strength {
    pub fn label(self) -> &'static str {
        match self {
            Self::Supporting => "Supporting",
            Self::Moderate => "Moderate",
            Self::Strong => "Strong",
            Self::VeryStrong => "Very strong",
            Self::StandAlone => "Stand-alone",
        }
    }
}

/// Which way a piece of evidence points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Pathogenic,
    Benign,
}

/// The kind of evidence. The app groups criteria into one lane per category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Population,
    Computational,
    VariantType,
    Phenotype,
    Functional,
    FamilyCases,
    Database,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::VariantType,
        Category::Population,
        Category::Computational,
        Category::Functional,
        Category::FamilyCases,
        Category::Phenotype,
        Category::Database,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Population => "Population",
            Self::Computational => "Computational",
            Self::VariantType => "Variant type",
            Self::Phenotype => "Phenotype",
            Self::Functional => "Functional",
            Self::FamilyCases => "Family and cases",
            Self::Database => "Other reports",
        }
    }
}

/// The five official classes. Ordered from most benign to most pathogenic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Benign,
    LikelyBenign,
    Vus,
    LikelyPathogenic,
    Pathogenic,
}

impl Class {
    pub fn label(self) -> &'static str {
        match self {
            Self::Benign => "Benign",
            Self::LikelyBenign => "Likely benign",
            Self::Vus => "Uncertain significance (VUS)",
            Self::LikelyPathogenic => "Likely pathogenic",
            Self::Pathogenic => "Pathogenic",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Benign => "B",
            Self::LikelyBenign => "LB",
            Self::Vus => "VUS",
            Self::LikelyPathogenic => "LP",
            Self::Pathogenic => "P",
        }
    }

    /// The next class toward pathogenic, if there is one.
    pub fn up(self) -> Option<Class> {
        match self {
            Self::Benign => Some(Self::LikelyBenign),
            Self::LikelyBenign => Some(Self::Vus),
            Self::Vus => Some(Self::LikelyPathogenic),
            Self::LikelyPathogenic => Some(Self::Pathogenic),
            Self::Pathogenic => None,
        }
    }

    /// The next class toward benign, if there is one.
    pub fn down(self) -> Option<Class> {
        match self {
            Self::Pathogenic => Some(Self::LikelyPathogenic),
            Self::LikelyPathogenic => Some(Self::Vus),
            Self::Vus => Some(Self::LikelyBenign),
            Self::LikelyBenign => Some(Self::Benign),
            Self::Benign => None,
        }
    }
}
