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

/// The most earlier records one case can hold. The history table gets unreadable
/// long before this; the cap also keeps a damaged file from filling the screen.
pub const MAX_RECORDS: usize = 30;

// The longest each field can be, in characters. The app types against the same caps
// (the public ones), so a file with longer fields cannot come from this app; it is
// refused with the field's name instead of being rendered.
/// The longest variant name, in characters.
pub const MAX_VARIANT_CHARS: usize = 300;
const MAX_GENE_CHARS: usize = 40;
const MAX_STAMP_CHARS: usize = 40;
/// The longest lab or source name of an earlier record, in characters.
pub const MAX_LAB_CHARS: usize = 60;
const MAX_TIER_CHARS: usize = 40;
// The range the year buttons use; a file from elsewhere gets the same range.
/// The earliest and latest year of an earlier record.
pub const MIN_YEAR: u16 = 2000;
pub const MAX_YEAR: u16 = 2100;

/// The first thing a file must say: its layout version.
#[derive(Deserialize)]
struct CaseHeader {
    format: u32,
}

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
    #[error("{place} is longer than {max} characters, so this cannot be a ClassiVAR case file")]
    TooLong { place: String, max: usize },
    #[error("this file holds {0} earlier records and this program opens at most {MAX_RECORDS}")]
    TooManyRecords(usize),
    #[error("{place} cannot be scored with the current rules: {error}")]
    BadEvidence { place: String, error: AssessError },
}

impl CaseFile {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a case is plain data and always converts to JSON")
    }

    /// Read a case and check it once, here, so the rest of the program can trust
    /// it: the layout version is known, every field is a size the app writes,
    /// the record count fits the screen, and every criterion exists in `rules`,
    /// is used once, and has a strength the rules accept.
    pub fn from_json(text: &str, rules: &RuleSet) -> Result<CaseFile, CaseError> {
        // The layout version is read first: a future file whose field names
        // have changed must be refused as "newer program", not as broken JSON.
        let header: CaseHeader = serde_json::from_str(text)?;
        if header.format != CASE_FORMAT {
            return Err(CaseError::UnknownFormat(header.format));
        }
        let mut case: CaseFile = serde_json::from_str(text)?;
        let capped = |place: String, value: &str, max: usize| -> Result<(), CaseError> {
            (value.chars().count() <= max).then_some(()).ok_or(CaseError::TooLong { place, max })
        };
        capped("The variant name".to_string(), &case.variant, MAX_VARIANT_CHARS)?;
        capped("The gene name".to_string(), &case.gene, MAX_GENE_CHARS)?;
        capped("The saved-on date".to_string(), &case.saved_on, MAX_STAMP_CHARS)?;
        capped("The writing program version".to_string(), &case.app_version, MAX_STAMP_CHARS)?;
        for record in &case.records {
            capped(format!("The record '{}' lab name", record.label), &record.label, MAX_LAB_CHARS)?;
            if let Some(tier) = &record.reported_tier {
                capped(format!("The record '{}' VUS tier name", record.label), tier, MAX_TIER_CHARS)?;
            }
        }
        if case.records.len() > MAX_RECORDS {
            return Err(CaseError::TooManyRecords(case.records.len()));
        }
        // Typing clamps the year into this range; so does opening, so the year
        // buttons on an opened record cannot push past their own range.
        for record in &mut case.records {
            record.year = record.year.clamp(MIN_YEAR, MAX_YEAR);
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
mod tests;
