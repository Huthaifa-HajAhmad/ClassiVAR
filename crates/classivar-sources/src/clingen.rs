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
    /// Higher is stronger evidence. Used for sorting.
    fn rank(&self) -> usize {
        let at = CLASSIFICATIONS.iter().position(|c| *c == self.classification);
        CLASSIFICATIONS.len() - at.expect("the rating was checked against the list when the file was read")
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
    let is_date = day.len() == 10
        && day.char_indices().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
    if is_date {
        Ok(day.to_string())
    } else {
        Err(SourceError::Unexpected(format!("'{text}' is not a date (row for {gene})")))
    }
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
                let mut map = HashMap::new();
                for name in COLUMNS {
                    let at = record
                        .iter()
                        .position(|cell| cell.trim() == name)
                        .ok_or_else(|| unexpected(format!("the header row has no '{name}' column")))?;
                    map.insert(name, at);
                }
                columns = Some(map);
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
    if columns.is_none() {
        return Err(unexpected("the header row 'GENE SYMBOL' was not found".to_string()));
    }
    if found.is_empty() {
        return Err(unexpected("the file has a header row but no ratings".to_string()));
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "\"GENE SYMBOL\",\"GENE ID (HGNC)\",\"DISEASE LABEL\",\"DISEASE ID (MONDO)\",\"MOI\",\"SOP\",\"CLASSIFICATION\",\"ONLINE REPORT\",\"CLASSIFICATION DATE\",\"GCEP\"\n";

    /// The real file layout (title lines, header, separator rows), shortened. Rows follow the real BRCA1 answer of 2026-11-03.
    const SAMPLE: &str = "\"CLINGEN GENE DISEASE VALIDITY CURATIONS\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"\n\
\"FILE CREATED: 2026-11-03\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"\n\
\"+++++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"++++++++++++++++++\",\"+++++++++\",\"+++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"+++++++++++++++++++\",\"+++++++++++++++++++\"\n\
\"GENE SYMBOL\",\"GENE ID (HGNC)\",\"DISEASE LABEL\",\"DISEASE ID (MONDO)\",\"MOI\",\"SOP\",\"CLASSIFICATION\",\"ONLINE REPORT\",\"CLASSIFICATION DATE\",\"GCEP\"\n\
\"+++++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"++++++++++++++++++\",\"+++++++++\",\"+++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"+++++++++++++++++++\",\"+++++++++++++++++++\"\n\
\"BRCA1\",\"HGNC:1100\",\"Fanconi anemia, complementation group S\",\"MONDO:0054748\",\"AR\",\"SOP7\",\"Definitive\",\"https://example.org/r1\",\"2020-05-14T00:31:37.538Z\",\"Hereditary Cancer Gene Curation Expert Panel\"\n\
\"BRCA2\",\"HGNC:1101\",\"BRCA2-related cancer predisposition\",\"MONDO:0700269\",\"AD\",\"SOP10\",\"Definitive\",\"https://example.org/r2\",\"2024-08-29T17:00:00.000Z\",\"Hereditary Cancer Gene Curation Expert Panel\"\n\
\"BRCA1\",\"HGNC:1100\",\"An invented disease for this test\",\"MONDO:0000001\",\"AD\",\"SOP10\",\"Limited\",\"https://example.org/r3\",\"2023-01-02T00:00:00.000Z\",\"Test panel\"\n\
\"BRCA1\",\"HGNC:1100\",\"BRCA1-related cancer predisposition\",\"MONDO:0700268\",\"AD\",\"SOP10\",\"Definitive\",\"https://example.org/r4\",\"2024-08-29T17:00:00.000Z\",\"Hereditary Cancer Gene Curation Expert Panel\"\n";

    fn gene(rows: &[(String, Curation)], symbol: &str) -> Vec<Curation> {
        super::curations_for(rows, symbol)
    }

    #[test]
    fn picks_out_one_gene_and_sorts_strongest_first() {
        let rows = parse_file(SAMPLE).unwrap();
        assert_eq!(rows.len(), 4);
        let brca1 = gene(&rows, "BRCA1");
        assert_eq!(brca1.len(), 3);
        assert_eq!(brca1[0].disease, "BRCA1-related cancer predisposition");
        assert_eq!(brca1[0].classification, "Definitive");
        assert_eq!(brca1[0].inheritance, "AD");
        assert_eq!(brca1[0].classified_on, "2024-08-29");
        assert_eq!(brca1[2].classification, "Limited");
    }

    #[test]
    fn a_gene_that_is_not_in_the_file_gives_an_empty_list() {
        assert!(gene(&parse_file(SAMPLE).unwrap(), "TP53").is_empty());
    }

    #[test]
    fn columns_are_read_by_name_so_a_reordered_file_still_works() {
        // The same data with the CLASSIFICATION and MOI columns swapped.
        let swapped = SAMPLE
            .replace("\"MOI\",\"SOP\",\"CLASSIFICATION\"", "\"CLASSIFICATION\",\"SOP\",\"MOI\"")
            .replace("\"AD\",\"SOP10\",\"Limited\"", "\"Limited\",\"SOP10\",\"AD\"")
            .replace("\"AD\",\"SOP10\",\"Definitive\"", "\"Definitive\",\"SOP10\",\"AD\"")
            .replace("\"AR\",\"SOP7\",\"Definitive\"", "\"Definitive\",\"SOP7\",\"AR\"");
        let rows = parse_file(&swapped).unwrap();
        let brca1 = gene(&rows, "BRCA1");
        assert_eq!(brca1[0].classification, "Definitive");
        assert_eq!(brca1[0].inheritance, "AD");
    }

    #[test]
    fn a_header_with_a_missing_column_is_an_error() {
        let broken = SAMPLE.replace("\"CLASSIFICATION DATE\"", "\"DATE\"");
        assert!(matches!(parse_file(&broken), Err(SourceError::Unexpected(m)) if m.contains("CLASSIFICATION DATE")));
    }

    #[test]
    fn a_file_without_the_header_row_is_an_error_not_an_empty_answer() {
        assert!(matches!(parse_file("<html>maintenance</html>"), Err(SourceError::Unexpected(_))));
    }

    #[test]
    fn a_header_with_no_data_rows_is_an_error() {
        assert!(matches!(parse_file(HEADER), Err(SourceError::Unexpected(_))));
    }

    #[test]
    fn a_footer_row_is_skipped_not_an_error() {
        // A spacer/footer line of empty cells used to abort the whole file, which
        // made every gene lookup fail until the file changed.
        let with_footer = format!("{SAMPLE},,,,,,,,,\n");
        let rows = parse_file(&with_footer).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(gene(&rows, "BRCA1").len(), 3);
    }

    #[test]
    fn an_unknown_rating_is_an_error_not_the_weakest_rating() {
        let new_rating = SAMPLE.replace("\"Limited\"", "\"Super Definitive\"");
        assert!(matches!(parse_file(&new_rating), Err(SourceError::Unexpected(m)) if m.contains("Super Definitive")));
    }

    #[test]
    fn a_date_that_is_not_a_date_is_an_error() {
        let bad = SAMPLE.replace("2020-05-14T00:31:37.538Z", "unknown");
        assert!(matches!(parse_file(&bad), Err(SourceError::Unexpected(m)) if m.contains("not a date")));
    }

    #[test]
    fn a_short_row_is_an_error() {
        let short = format!("{HEADER}\"BRCA1\",\"HGNC:1100\"\n");
        assert!(matches!(parse_file(&short), Err(SourceError::Unexpected(_))));
    }

    /// Needs the internet. Run on purpose with: cargo test -p classivar-sources -- --ignored
    #[test]
    #[ignore]
    fn live_brca1_has_a_definitive_rating() {
        let gene = lookup_gene("BRCA1").unwrap();
        assert!(gene.value.curations.iter().any(|c| c.classification == "Definitive"));
    }
}
