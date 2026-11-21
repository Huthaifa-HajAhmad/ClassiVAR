//! Scoring expert-panel classifications from the ClinGen Evidence Repository
//! (ERepo) with this engine, to measure how often it reaches the panel's class
//! from the panel's own criteria.
//!
//! Each row gives the criteria a ClinGen expert panel marked as met, with their
//! strengths ("PM2_Supporting", "PM3_Very Strong", or a bare code at its 2015
//! strength), and the panel's class. Rows are scored as recorded (no strength
//! caps, no guide answers, PP5/BP6 counted), as an earlier record is. A
//! difference is expected where a panel used its own gene-specific rules or the
//! 2015 table instead of points; the report counts both.

use crate::assess::{Applied, Counting};
use crate::model::{Class, Strength};
use crate::rules::RuleSet;

/// One expert-panel classification, as kept in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelClassification {
    pub allele: String,
    pub panel: String,
    pub class: Class,
    /// The met criteria as the repository writes them, comma-separated.
    pub met: String,
}

/// What the engine made of one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The points give the panel's class.
    PointsAgree,
    /// The points differ, but the 2015 table gives the panel's class.
    TableAgrees,
    /// The engine withholds a class (contradiction, both sides, combined limit).
    Withheld,
    /// Neither the points nor the table give the panel's class.
    Differs,
    /// The panel met no criterion. ClassiVAR gives no class without evidence;
    /// the panel calls this VUS. Not a disagreement about the evidence.
    NoEvidence,
    /// A code or strength this rule file cannot read.
    Unreadable,
}

/// Read a class as the repository writes it.
pub fn class_of(text: &str) -> Option<Class> {
    match text.trim().to_ascii_lowercase().as_str() {
        "pathogenic" => Some(Class::Pathogenic),
        "likely pathogenic" => Some(Class::LikelyPathogenic),
        "uncertain significance" => Some(Class::Vus),
        "likely benign" => Some(Class::LikelyBenign),
        "benign" => Some(Class::Benign),
        _ => None,
    }
}

fn strength_of(suffix: &str) -> Option<Strength> {
    match suffix.trim().to_ascii_lowercase().replace([' ', '-'], "").as_str() {
        "supporting" => Some(Strength::Supporting),
        "moderate" => Some(Strength::Moderate),
        "strong" => Some(Strength::Strong),
        "verystrong" => Some(Strength::VeryStrong),
        "standalone" => Some(Strength::StandAlone),
        _ => None,
    }
}

/// The repository's met codes as applied criteria. A bare code takes the
/// strength the 2015 paper gave it (an expert panel writes PM2_Supporting when it
/// means Supporting). Errors name the code that could not be read.
pub fn parse_met(rules: &RuleSet, met: &str) -> Result<Vec<Applied>, String> {
    let mut applied: Vec<Applied> = Vec::new();
    for token in met.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let (code, suffix) = token.split_once('_').unwrap_or((token, ""));
        let criterion = rules.criterion(code).ok_or_else(|| format!("unknown code {token}"))?;
        let strength = if suffix.is_empty() {
            criterion.original_strength.unwrap_or(criterion.strength)
        } else {
            strength_of(suffix).ok_or_else(|| format!("unknown strength {token}"))?
        };
        if applied.iter().any(|a| a.code == code) {
            return Err(format!("{code} twice"));
        }
        applied.push(Applied::new(code, strength));
    }
    Ok(applied)
}

/// Score one row and say how it compares with the panel's class.
pub fn outcome(rules: &RuleSet, row: &PanelClassification) -> Outcome {
    let Ok(applied) = parse_met(rules, &row.met) else { return Outcome::Unreadable };
    if applied.is_empty() {
        return Outcome::NoEvidence;
    }
    let Ok(assessment) = rules.assess_as(&applied, Counting::AsRecorded) else { return Outcome::Unreadable };
    match assessment.class() {
        Some(class) if class == row.class => Outcome::PointsAgree,
        _ if assessment.classic_class == Some(row.class) => Outcome::TableAgrees,
        None => Outcome::Withheld,
        _ => Outcome::Differs,
    }
}

/// Read the snapshot: a header line, then `allele <TAB> panel <TAB> class <TAB> met`.
pub fn parse_snapshot(text: &str) -> Result<Vec<PanelClassification>, String> {
    text.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [allele, panel, class, met] = fields.as_slice() else {
                return Err(format!("line {}: expected 4 fields", i + 2));
            };
            let class = class_of(class).ok_or_else(|| format!("line {}: unknown class {class}", i + 2))?;
            Ok(PanelClassification {
                allele: allele.to_string(),
                panel: panel.to_string(),
                class,
                met: met.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn row(class: Class, met: &str) -> PanelClassification {
        PanelClassification { allele: "CA1".into(), panel: "test".into(), class, met: met.into() }
    }

    #[test]
    fn codes_are_read_with_their_suffix_or_their_2015_strength() {
        let applied = parse_met(rules(), "PVS1, PM2, PM3_Very Strong, BA1_Stand Alone, PP4_Moderate").unwrap();
        let strengths: Vec<_> = applied.iter().map(|a| (a.code.as_str(), a.strength)).collect();
        assert_eq!(
            strengths,
            [
                ("PVS1", Strength::VeryStrong),
                ("PM2", Strength::Moderate),
                ("PM3", Strength::VeryStrong),
                ("BA1", Strength::StandAlone),
                ("PP4", Strength::Moderate)
            ]
        );
        assert!(parse_met(rules(), "PX9").unwrap_err().contains("PX9"));
        assert!(parse_met(rules(), "PM2_Huge").is_err());
        assert!(parse_met(rules(), "PM2, PM2_Supporting").is_err());
        assert_eq!(parse_met(rules(), "").unwrap(), vec![]);
    }

    #[test]
    fn each_row_is_sorted_into_one_outcome() {
        assert_eq!(outcome(rules(), &row(Class::LikelyPathogenic, "PVS1, PM2_Supporting")), Outcome::PointsAgree);
        // PVS1 alone: points say LP (8), the 2015 table says VUS.
        assert_eq!(outcome(rules(), &row(Class::Vus, "PVS1")), Outcome::TableAgrees);
        assert_eq!(outcome(rules(), &row(Class::Vus, "PP3, BP4")), Outcome::Withheld);
        assert_eq!(outcome(rules(), &row(Class::Benign, "PVS1, PS3")), Outcome::Differs);
        assert_eq!(outcome(rules(), &row(Class::Benign, "PX9")), Outcome::Unreadable);
        // A panel's VUS with nothing met: ClassiVAR gives no class without evidence.
        assert_eq!(outcome(rules(), &row(Class::Vus, "")), Outcome::NoEvidence);
    }

    #[test]
    fn the_snapshot_reads_four_columns_and_names_a_bad_line() {
        let text = "allele\tpanel\tclass\tmet\nCA1\tPAH\tPathogenic\tPVS1, PM2\n";
        assert_eq!(parse_snapshot(text).unwrap()[0].class, Class::Pathogenic);
        assert!(parse_snapshot("h\nCA1\tPAH\tMaybe\tPVS1\n").unwrap_err().contains("line 2"));
        assert!(parse_snapshot("h\nCA1\tPAH\n").unwrap_err().contains("4 fields"));
    }
}
