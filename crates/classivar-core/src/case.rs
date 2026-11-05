//! A saved case: everything the curator entered, in a file they can keep, open
//! again, and attach to a review.
//!
//! The file is JSON. It holds the variant name, the gene, the gene decision, the
//! evidence for today and for each earlier record, and which rules and which
//! program version were used. It holds no results: the class is worked out again
//! when the case is opened, so an old file can never show a stale class.
//!
//! Do not put patient names or identifiers in the variant name. The file is a
//! plain text file on the curator's computer.

use serde::{Deserialize, Serialize};

use crate::assess::{Applied, AssessError, Counting};
use crate::model::Class;
use crate::rules::{RuleSet, RuleSetRef};

/// The version of the file layout. Raise it when a change would make an old
/// reader misunderstand a new file.
pub const CASE_FORMAT: u32 = 1;

/// The curator's own answer to: does this gene cause this disease?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneDecision {
    NotDecided,
    Established,
    NotEstablished,
}

/// What an earlier record (for example a lab in ClinVar) said and used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedRecord {
    pub label: String,
    pub year: u16,
    /// None until the curator says what the record reported.
    pub class: Option<Class>,
    /// The VUS sub-tier the record named (for example "VUS-low"), if any. Older
    /// files do not have this field, so it may be missing.
    #[serde(default)]
    pub reported_tier: Option<String>,
    pub criteria: Vec<Applied>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseFile {
    pub format: u32,
    /// The version of ClassiVAR that wrote the file.
    pub app_version: String,
    /// UTC date of saving, `YYYY-MM-DD`.
    pub saved_on: String,
    /// The rules in use when the file was saved.
    pub rule_set: RuleSetRef,
    pub variant: String,
    pub gene: String,
    pub gene_decision: GeneDecision,
    pub today: Vec<Applied>,
    pub records: Vec<SavedRecord>,
}

#[derive(Debug, thiserror::Error)]
pub enum CaseError {
    #[error("this is not a ClassiVAR case file: {0}")]
    Parse(#[from] serde_json::Error),
    #[error(
        "this file uses layout version {0}, which this program does not know (it knows version {CASE_FORMAT}); it was probably saved by a newer ClassiVAR"
    )]
    UnknownFormat(u32),
    #[error("{place} cannot be scored with the current rules: {error}")]
    BadEvidence { place: String, error: AssessError },
}

impl CaseFile {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a case is plain data and always converts to JSON")
    }

    /// Read a case and check it once, here, so the rest of the program can trust it:
    /// the layout version is known, and every criterion exists in `rules`, is used
    /// once, and has a strength the rules accept.
    pub fn from_json(text: &str, rules: &RuleSet) -> Result<CaseFile, CaseError> {
        let case: CaseFile = serde_json::from_str(text)?;
        if case.format != CASE_FORMAT {
            return Err(CaseError::UnknownFormat(case.format));
        }
        // Validate today's block the way old records are read, so a strength that
        // was legal when the case was saved still opens. The screen lowers it to
        // today's cap afterwards and says so.
        rules
            .assess_as(&case.today, Counting::AsRecorded)
            .map_err(|error| CaseError::BadEvidence { place: "Today's evidence".to_string(), error })?;
        for record in &case.records {
            // Earlier records are checked the way they are scored: as they were made.
            rules
                .assess_as(&record.criteria, Counting::AsRecorded)
                .map_err(|error| CaseError::BadEvidence { place: format!("The record '{}'", record.label), error })?;
        }
        Ok(case)
    }

    /// Does this case come from other rules than `rules`? The class may then differ from the one the curator saw.
    pub fn saved_with_other_rules(&self, rules: &RuleSet) -> bool {
        self.rule_set != rules.reference()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Strength::*;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn applied(code: &str, strength: crate::model::Strength) -> Applied {
        Applied::new(code, strength)
    }

    fn sample() -> CaseFile {
        CaseFile {
            format: CASE_FORMAT,
            app_version: "0.1.0".into(),
            saved_on: "2026-11-04".into(),
            rule_set: rules().reference(),
            variant: "NM_007294.4:c.5266dup".into(),
            gene: "BRCA1".into(),
            gene_decision: GeneDecision::Established,
            today: vec![applied("PM2", Supporting), applied("PP3", Supporting)],
            records: vec![
                SavedRecord {
                    label: "Lab A".into(),
                    year: 2019,
                    class: Some(Class::Vus),
                    reported_tier: Some("VUS-low".into()),
                    criteria: vec![applied("PM2", Moderate)],
                },
                SavedRecord { label: "Lab B".into(), year: 2022, class: None, reported_tier: None, criteria: vec![] },
            ],
        }
    }

    #[test]
    fn a_case_survives_saving_and_opening() {
        let case = sample();
        let back = CaseFile::from_json(&case.to_json(), rules()).unwrap();
        assert_eq!(back, case);
        assert!(!back.saved_with_other_rules(rules()));
    }

    #[test]
    fn reopening_preserves_assessments_for_today_and_earlier_records() {
        let case = sample();
        let back = CaseFile::from_json(&case.to_json(), rules()).unwrap();
        assert_eq!(rules().assess(&case.today).unwrap(), rules().assess(&back.today).unwrap());
        for (before, after) in case.records.iter().zip(&back.records) {
            assert_eq!(
                rules().assess_as(&before.criteria, Counting::AsRecorded).unwrap(),
                rules().assess_as(&after.criteria, Counting::AsRecorded).unwrap()
            );
        }
    }

    #[test]
    fn reasons_use_a_character_limit_in_today_and_earlier_records() {
        for earlier_record in [false, true] {
            for length in [crate::MAX_REASON_CHARS, crate::MAX_REASON_CHARS + 1] {
                let mut case = sample();
                let item = Applied::new("PM2", Moderate).with_reason("é".repeat(length));
                if earlier_record {
                    case.records[0].criteria = vec![item];
                } else {
                    case.today = vec![item];
                }
                let result = CaseFile::from_json(&case.to_json(), rules());
                if length == crate::MAX_REASON_CHARS {
                    assert_eq!(result.unwrap(), case);
                } else {
                    let expected_place = if earlier_record { "The record 'Lab A'" } else { "Today's evidence" };
                    assert!(matches!(result,
                        Err(CaseError::BadEvidence { place, error: AssessError::BadReason { code } })
                        if place == expected_place && code == "PM2"
                    ));
                }
            }
        }
    }

    #[test]
    fn files_saved_before_reasons_existed_still_open() {
        let case = sample();
        let mut json = serde_json::to_value(&case).unwrap();
        for item in json["today"].as_array_mut().unwrap() {
            item.as_object_mut().unwrap().remove("reason");
        }
        for record in json["records"].as_array_mut().unwrap() {
            for item in record["criteria"].as_array_mut().unwrap() {
                item.as_object_mut().unwrap().remove("reason");
            }
        }
        assert_eq!(CaseFile::from_json(&json.to_string(), rules()).unwrap(), case);
    }

    #[test]
    fn the_reason_for_a_strength_is_saved_and_an_empty_one_is_refused() {
        let mut case = sample();
        case.today[0] = Applied::new("PM2", Moderate).with_reason("Absent from 40,000 controls in this population");
        let json = case.to_json();
        assert!(json.contains("\"reason\": \"Absent from 40,000 controls in this population\""));
        assert_eq!(CaseFile::from_json(&json, rules()).unwrap(), case);

        case.today[0] = Applied::new("PM2", Moderate).with_reason("   ");
        assert!(matches!(CaseFile::from_json(&case.to_json(), rules()), Err(CaseError::BadEvidence { .. })));
    }

    #[test]
    fn a_file_written_before_tiers_existed_can_still_be_opened() {
        let json =
            sample().to_json().replace("\"reported_tier\": \"VUS-low\",", "").replace("\"reported_tier\": null,", "");
        assert!(!json.contains("reported_tier"));
        let back = CaseFile::from_json(&json, rules()).unwrap();
        assert_eq!(back.records[0].reported_tier, None);
    }

    #[test]
    fn the_file_is_readable_text_with_plain_names() {
        let json = sample().to_json();
        assert!(json.contains("\"gene_decision\": \"established\""));
        assert!(json.contains("\"class\": \"vus\""));
        assert!(json.contains("\"strength\": \"supporting\""));
        assert!(!json.contains("total_points"), "results are not saved, they are worked out again");
    }

    #[test]
    fn a_file_from_a_newer_layout_is_refused() {
        let json = sample().to_json().replace("\"format\": 1", "\"format\": 2");
        assert!(matches!(CaseFile::from_json(&json, rules()), Err(CaseError::UnknownFormat(2))));
    }

    #[test]
    fn text_that_is_not_a_case_is_refused() {
        assert!(matches!(CaseFile::from_json("<html></html>", rules()), Err(CaseError::Parse(_))));
        assert!(matches!(CaseFile::from_json("{}", rules()), Err(CaseError::Parse(_))));
    }

    #[test]
    fn evidence_the_rules_do_not_accept_is_refused_with_its_place() {
        let mut case = sample();
        case.today.push(applied("ZZ9", Strong));
        let error = CaseFile::from_json(&case.to_json(), rules()).unwrap_err();
        assert!(matches!(&error, CaseError::BadEvidence { place, .. } if place.contains("Today")));

        let mut case = sample();
        case.records[0].criteria.push(applied("PM2", Moderate));
        let error = CaseFile::from_json(&case.to_json(), rules()).unwrap_err();
        assert!(
            matches!(&error, CaseError::BadEvidence { place, error: AssessError::DuplicateCriterion(_) } if place.contains("Lab A"))
        );
    }

    #[test]
    fn an_old_case_with_a_since_capped_strength_still_opens() {
        let mut case = sample();
        case.records[0].criteria = vec![applied("PP3", VeryStrong)];
        assert!(CaseFile::from_json(&case.to_json(), rules()).is_ok());
        // A case saved before the cap may also have it in the curator's own block.
        // It opens; the screen lowers it to today's limit and says so.
        case.today = vec![applied("PP3", VeryStrong)];
        assert!(CaseFile::from_json(&case.to_json(), rules()).is_ok());
    }

    #[test]
    fn a_guided_criterion_round_trips_with_its_situation() {
        let mut case = sample();
        let situation = rules()
            .criterion("PVS1")
            .unwrap()
            .situation_for(Supporting)
            .expect("PVS1 has a Supporting guide")
            .to_string();
        case.today = vec![Applied::guided("PVS1", Supporting, situation.clone())];
        let opened = CaseFile::from_json(&case.to_json(), rules()).unwrap();
        assert_eq!(opened.today[0].situation.as_deref(), Some(situation.as_str()));
    }

    #[test]
    fn an_old_case_with_pvs1_but_no_situation_still_opens() {
        // Cases saved before the guide existed are read as they were made; the
        // screen fills in the matching situation before it scores them.
        let mut case = sample();
        case.today = vec![Applied::new("PVS1", VeryStrong)];
        assert!(CaseFile::from_json(&case.to_json(), rules()).is_ok());
    }

    #[test]
    fn a_case_saved_with_other_rules_says_so() {
        let mut case = sample();
        case.rule_set.version = "0.0.1".into();
        assert!(case.saved_with_other_rules(rules()));
    }
}
