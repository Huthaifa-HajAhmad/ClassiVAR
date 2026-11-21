//! The disease a variant is classified for, and how it is passed on.
//!
//! ACMG/AMP classifies a variant for a condition, and ClinVar keeps a lab's class
//! per condition. So a case names one condition, each earlier record may name its
//! own, and two records for different conditions are never compared as an
//! upgrade or a downgrade.

use serde::{Deserialize, Serialize};

/// The longest condition name, in characters (ClinVar names can be long).
pub const MAX_CONDITION_CHARS: usize = 200;

/// How a condition is inherited. The criteria that depend on it (PM3, BS2, BP2,
/// the recessive rule of PS2, and population limits) read it from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Inheritance {
    AutosomalDominant,
    AutosomalRecessive,
    XLinked,
    Semidominant,
    Mitochondrial,
    /// Not known, or not stated.
    Unknown,
}

impl Inheritance {
    pub const ALL: [Inheritance; 6] = [
        Self::AutosomalDominant,
        Self::AutosomalRecessive,
        Self::XLinked,
        Self::Semidominant,
        Self::Mitochondrial,
        Self::Unknown,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AutosomalDominant => "Autosomal dominant",
            Self::AutosomalRecessive => "Autosomal recessive",
            Self::XLinked => "X-linked",
            Self::Semidominant => "Semidominant",
            Self::Mitochondrial => "Mitochondrial",
            Self::Unknown => "Not known",
        }
    }

    /// The short form ClinGen uses in its gene-disease validity file.
    pub fn short(self) -> &'static str {
        match self {
            Self::AutosomalDominant => "AD",
            Self::AutosomalRecessive => "AR",
            Self::XLinked => "XL",
            Self::Semidominant => "SD",
            Self::Mitochondrial => "MT",
            Self::Unknown => "?",
        }
    }

    /// Read ClinGen's `MOI` column (`AD`, `AR`, `XL`, `SD`, `MT`). Anything else,
    /// including `UD` (undetermined), is `Unknown`: never a guess.
    pub fn from_clingen(moi: &str) -> Inheritance {
        match moi.trim().to_ascii_uppercase().as_str() {
            "AD" => Self::AutosomalDominant,
            "AR" => Self::AutosomalRecessive,
            "XL" | "XLR" | "XLD" => Self::XLinked,
            "SD" => Self::Semidominant,
            "MT" => Self::Mitochondrial,
            _ => Self::Unknown,
        }
    }
}

/// A condition: its name, its MONDO id when known, and its inheritance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    pub name: String,
    /// For example `MONDO:0011450`. Two conditions with different ids differ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mondo: Option<String>,
    pub inheritance: Inheritance,
}

/// Is an earlier record about the case's condition?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionMatch {
    Same,
    /// The two name different MONDO ids. The record is shown but never compared.
    Different,
    /// One side names no condition, or the names differ without ids to settle
    /// it (synonyms are common). The record is compared, and the screen asks
    /// the curator to check.
    Unknown,
}

impl Condition {
    /// Compare the case's condition with a record's. Only MONDO ids can prove a
    /// difference; equal names (ignoring case and spaces) prove sameness.
    pub fn compare(case: Option<&Condition>, record: Option<&Condition>) -> ConditionMatch {
        let (Some(case), Some(record)) = (case, record) else { return ConditionMatch::Unknown };
        if let (Some(a), Some(b)) = (&case.mondo, &record.mondo) {
            return if a.trim().eq_ignore_ascii_case(b.trim()) {
                ConditionMatch::Same
            } else {
                ConditionMatch::Different
            };
        }
        let plain = |name: &str| name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        if plain(&case.name) == plain(&record.name) { ConditionMatch::Same } else { ConditionMatch::Unknown }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition(name: &str, mondo: Option<&str>) -> Condition {
        Condition { name: name.into(), mondo: mondo.map(str::to_string), inheritance: Inheritance::Unknown }
    }

    #[test]
    fn only_different_mondo_ids_make_two_conditions_different() {
        let cf = condition("Cystic fibrosis", Some("MONDO:0009061"));
        let cbavd = condition("Congenital bilateral absence of vas deferens", Some("MONDO:0010178"));
        assert_eq!(Condition::compare(Some(&cf), Some(&cbavd)), ConditionMatch::Different);
        assert_eq!(Condition::compare(Some(&cf), Some(&condition("CF", Some("mondo:0009061")))), ConditionMatch::Same);
    }

    #[test]
    fn names_can_prove_sameness_but_not_difference() {
        let a = condition("Cystic  fibrosis", None);
        assert_eq!(Condition::compare(Some(&a), Some(&condition("cystic fibrosis", None))), ConditionMatch::Same);
        // A synonym is not proof of a different disease.
        assert_eq!(Condition::compare(Some(&a), Some(&condition("Mucoviscidosis", None))), ConditionMatch::Unknown);
        assert_eq!(Condition::compare(None, Some(&a)), ConditionMatch::Unknown);
        assert_eq!(Condition::compare(Some(&a), None), ConditionMatch::Unknown);
    }

    #[test]
    fn clingen_inheritance_codes_are_read_and_anything_else_is_unknown() {
        assert_eq!(Inheritance::from_clingen("AD"), Inheritance::AutosomalDominant);
        assert_eq!(Inheritance::from_clingen(" ar "), Inheritance::AutosomalRecessive);
        assert_eq!(Inheritance::from_clingen("XL"), Inheritance::XLinked);
        assert_eq!(Inheritance::from_clingen("UD"), Inheritance::Unknown);
        assert_eq!(Inheritance::from_clingen(""), Inheritance::Unknown);
        for inheritance in Inheritance::ALL {
            assert!(!inheritance.label().is_empty());
        }
    }
}
