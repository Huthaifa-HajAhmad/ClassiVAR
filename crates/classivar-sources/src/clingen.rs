//! ClinGen gene-disease validity: how strong the evidence is that a gene
//! causes a disease, as rated by a ClinGen expert panel.
//!
//! ClinGen publishes one file with all ratings (about 1 MB). We download it,
//! check it, keep the checked rows in memory for a few hours, and pick out the
//! rows for one gene.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{Fetched, SourceError};

const SOURCE: &str = "ClinGen gene-disease validity";
const URL: &str = "https://search.clinicalgenome.org/kb/gene-validity/download";
/// How long the downloaded file is reused.
const KEEP_FOR: Duration = Duration::from_secs(6 * 60 * 60);
/// The file is about 1 MB. Anything far larger is not the file we expect.
const MAX_BYTES: u64 = 20 * 1024 * 1024;

/// Every rating ClinGen uses, strongest first. A rating not in this list is an
/// error: a new rating might be the strongest one, so it must not be sorted last by guess.
const CLASSIFICATIONS: [&str; 7] =
    ["Definitive", "Strong", "Moderate", "Limited", "Disputed", "Refuted", "No Known Disease Relationship"];

/// One rating of one gene and disease pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curation {
    pub disease: String,
    pub disease_id: String,
    /// For example `AD` or `AR`.
    pub inheritance: String,
    /// One of: Definitive, Strong, Moderate, Limited, Disputed, Refuted, No Known Disease Relationship.
    pub classification: String,
    pub report_url: String,
    /// `YYYY-MM-DD`.
    pub classified_on: String,
    pub expert_panel: String,
}

impl Curation {
    /// Higher is stronger evidence. Used for sorting. The checked parser only
    /// writes classifications from the list; if one outside it appears anyway,
    /// it sorts last (as the weakest) instead of failing the lookup.
    fn rank(&self) -> usize {
        CLASSIFICATIONS.iter().position(|c| c == &self.classification).map_or(0, |at| CLASSIFICATIONS.len() - at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClinGenGene {
    pub symbol: String,
    /// Strongest first. Empty if ClinGen has not rated this gene.
    pub curations: Vec<Curation>,
}

/// The checked file: every row, with the upper-case gene symbol it belongs to.
type Rows = Arc<Vec<(String, Curation)>>;

static CACHE: Mutex<Option<(Instant, Rows)>> = Mutex::new(None);

/// The checked rows, from memory if they are recent. The lock is held only to
/// look at or fill the cache, never during the download, so a slow download
/// does not block other lookups.
fn rows() -> Result<Rows, SourceError> {
    {
        let cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((when, rows)) = cache.as_ref()
            && when.elapsed() < KEEP_FOR
        {
            return Ok(Arc::clone(rows));
        }
    }
    let text = crate::get_ok(URL, &[], "text/csv", MAX_BYTES)?;
    // Only a file that passed the checks is kept, so a maintenance page or an
    // empty file is not remembered for hours.
    let rows: Rows = Arc::new(parse_file(&text)?);
    *CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((Instant::now(), Arc::clone(&rows)));
    Ok(rows)
}

/// The curations for one gene, strongest first and then by disease name. Used by
/// the lookup and by the tests, so a mistake in the filter cannot pass unnoticed.
fn curations_for(rows: &[(String, Curation)], symbol: &str) -> Vec<Curation> {
    let mut curations: Vec<Curation> = rows.iter().filter(|(gene, _)| gene == symbol).map(|(_, c)| c.clone()).collect();
    curations.sort_by(|a, b| b.rank().cmp(&a.rank()).then_with(|| a.disease.cmp(&b.disease)));
    curations
}

/// Look a gene up in ClinGen's validity file. Blocks; call from a background thread.
pub fn lookup_gene(symbol: &str) -> Result<Fetched<ClinGenGene>, SourceError> {
    let symbol = crate::check_symbol(symbol)?;
    let rows = rows()?;
    let curations = curations_for(&rows, &symbol);
    Ok(Fetched::now(SOURCE, ClinGenGene { symbol, curations }))
}

/// The columns we read, by the name in the header row. The order of columns in
/// the file does not matter, but a missing column is an error.
const COLUMNS: [&str; 8] = [
    "GENE SYMBOL",
    "DISEASE LABEL",
    "DISEASE ID (MONDO)",
    "MOI",
    "CLASSIFICATION",
    "ONLINE REPORT",
    "CLASSIFICATION DATE",
    "GCEP",
];

/// The date at the start of a ClinGen date such as `2024-08-29T17:00:00.000Z`.
fn date_part(text: &str, gene: &str) -> Result<String, SourceError> {
    let day = text.get(..10).unwrap_or("");
    if classivar_core::is_calendar_date(day) {
        Ok(day.to_string())
    } else {
        Err(SourceError::Unexpected(format!("'{text}' is not a date (row for {gene})")))
    }
}

/// Where each column we read sits in the header row. A missing column is an error.
fn header_columns(header: &csv::StringRecord) -> Result<HashMap<&'static str, usize>, SourceError> {
    let mut map = HashMap::new();
    for name in COLUMNS {
        let at = header
            .iter()
            .position(|cell| cell.trim() == name)
            .ok_or_else(|| SourceError::Unexpected(format!("the header row has no '{name}' column")))?;
        map.insert(name, at);
    }
    Ok(map)
}
/// The file starts with a few title lines, then a header row, then data rows.
/// Every problem is an error, never "nothing found": no header row, a missing
/// column, a short row, an unknown rating, or a file with no data rows at all.
fn parse_file(text: &str) -> Result<Vec<(String, Curation)>, SourceError> {
    let unexpected = |message: String| SourceError::Unexpected(message);
    let mut reader = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_reader(text.as_bytes());
    let mut columns: Option<HashMap<&'static str, usize>> = None;
    let mut found = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| unexpected(e.to_string()))?;
        let first = record.get(0).unwrap_or("");

        let Some(columns) = &columns else {
            if first == "GENE SYMBOL" {
                columns = Some(header_columns(&record)?);
            }
            continue;
        };
        if first.starts_with("++") {
            continue;
        }
        // Footer and spacer rows (",,,,,") have no gene symbol; they are not data.
        if first.trim().is_empty() {
            continue;
        }

        let cell = |name: &str| -> Result<&str, SourceError> {
            record.get(columns[name]).ok_or_else(|| unexpected(format!("a row for {first} is missing '{name}'")))
        };
        let classification = cell("CLASSIFICATION")?;
        if !CLASSIFICATIONS.contains(&classification) {
            return Err(unexpected(format!("unknown rating '{classification}' for {first}")));
        }
        found.push((
            cell("GENE SYMBOL")?.to_ascii_uppercase(),
            Curation {
                disease: cell("DISEASE LABEL")?.to_string(),
                disease_id: cell("DISEASE ID (MONDO)")?.to_string(),
                inheritance: cell("MOI")?.to_string(),
                classification: classification.to_string(),
                report_url: cell("ONLINE REPORT")?.to_string(),
                classified_on: date_part(cell("CLASSIFICATION DATE")?, first)?,
                expert_panel: cell("GCEP")?.to_string(),
            },
        ));
    }
    match (columns, found.is_empty()) {
        (None, _) => Err(unexpected("the header row 'GENE SYMBOL' was not found".to_string())),
        (Some(_), true) => Err(unexpected("the file has a header row but no ratings".to_string())),
        (Some(_), false) => Ok(found),
    }
}

#[cfg(test)]
mod tests;
