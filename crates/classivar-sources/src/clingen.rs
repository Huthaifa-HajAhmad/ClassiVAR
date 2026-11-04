//! ClinGen gene-disease validity: how strong the evidence is that a gene
//! causes a disease, as rated by a ClinGen expert panel.
//!
//! ClinGen publishes one file with all ratings (about 1 MB). We download it,
//! keep it in memory for a few hours, and pick out the rows for one gene.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{Fetched, SourceError};

const SOURCE: &str = "ClinGen gene-disease validity";
const URL: &str = "https://search.clinicalgenome.org/kb/gene-validity/download";
/// How long the downloaded file is reused.
const KEEP_FOR: Duration = Duration::from_secs(6 * 60 * 60);

/// One rating of one gene and disease pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curation {
    pub disease: String,
    pub disease_id: String,
    /// For example `AD` or `AR`.
    pub inheritance: String,
    /// Definitive, Strong, Moderate, Limited, Disputed, Refuted, or No Known Disease Relationship.
    pub classification: String,
    pub report_url: String,
    /// `YYYY-MM-DD`.
    pub classified_on: String,
    pub expert_panel: String,
}

impl Curation {
    /// Higher is stronger evidence. Used for sorting.
    fn rank(&self) -> u8 {
        match self.classification.as_str() {
            "Definitive" => 6,
            "Strong" => 5,
            "Moderate" => 4,
            "Limited" => 3,
            "Disputed" => 2,
            "Refuted" => 1,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClinGenGene {
    pub symbol: String,
    /// Strongest first. Empty if ClinGen has not rated this gene.
    pub curations: Vec<Curation>,
}

static CACHE: Mutex<Option<(Instant, Arc<String>)>> = Mutex::new(None);

/// The downloaded file, from memory if it is recent.
fn download() -> Result<Arc<String>, SourceError> {
    let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((when, text)) = cache.as_ref()
        && when.elapsed() < KEEP_FOR
    {
        return Ok(Arc::clone(text));
    }
    let mut response = crate::http_agent().get(URL).call().map_err(|e| SourceError::Network(e.to_string()))?;
    let text = response.body_mut().read_to_string().map_err(|e| SourceError::Network(e.to_string()))?;
    let text = Arc::new(text);
    *cache = Some((Instant::now(), Arc::clone(&text)));
    Ok(text)
}

/// Look a gene up in ClinGen's validity file. Blocks; call from a background thread.
pub fn lookup_gene(symbol: &str) -> Result<Fetched<ClinGenGene>, SourceError> {
    let symbol = crate::check_symbol(symbol)?;
    let text = download()?;
    let curations = parse_file(&text, &symbol)?;
    Ok(Fetched::now(SOURCE, ClinGenGene { symbol, curations }))
}

/// The file starts with a few title lines, then a header row, then data rows.
/// If the header row is missing, the format has changed: say so, do not return "nothing found".
fn parse_file(text: &str, symbol: &str) -> Result<Vec<Curation>, SourceError> {
    let mut reader = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_reader(text.as_bytes());
    let mut seen_header = false;
    let mut found = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| SourceError::Unexpected(e.to_string()))?;
        let first = record.get(0).unwrap_or("");
        if !seen_header {
            seen_header = first == "GENE SYMBOL";
            continue;
        }
        if first.starts_with("++") {
            continue;
        }
        if !first.eq_ignore_ascii_case(symbol) {
            continue;
        }
        if record.len() < 10 {
            return Err(SourceError::Unexpected(format!("a row for {first} has only {} columns", record.len())));
        }
        found.push(Curation {
            disease: record[2].to_string(),
            disease_id: record[3].to_string(),
            inheritance: record[4].to_string(),
            classification: record[6].to_string(),
            report_url: record[7].to_string(),
            classified_on: record[8].chars().take(10).collect(),
            expert_panel: record[9].to_string(),
        });
    }
    if !seen_header {
        return Err(SourceError::Unexpected("the header row 'GENE SYMBOL' was not found".to_string()));
    }
    found.sort_by(|a, b| b.rank().cmp(&a.rank()).then_with(|| a.disease.cmp(&b.disease)));
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn picks_out_one_gene_and_sorts_strongest_first() {
        let rows = parse_file(SAMPLE, "BRCA1").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].disease, "BRCA1-related cancer predisposition");
        assert_eq!(rows[0].classification, "Definitive");
        assert_eq!(rows[0].inheritance, "AD");
        assert_eq!(rows[0].classified_on, "2024-08-29");
        assert_eq!(rows[2].classification, "Limited");
    }

    #[test]
    fn a_gene_that_is_not_in_the_file_gives_an_empty_list() {
        assert!(parse_file(SAMPLE, "TP53").unwrap().is_empty());
    }

    #[test]
    fn a_file_without_the_header_row_is_an_error_not_an_empty_answer() {
        assert!(matches!(parse_file("<html>maintenance</html>", "BRCA1"), Err(SourceError::Unexpected(_))));
    }

    /// Needs the internet. Run on purpose with: cargo test -p classivar-sources -- --ignored
    #[test]
    #[ignore]
    fn live_brca1_has_a_definitive_rating() {
        let gene = lookup_gene("BRCA1").unwrap();
        assert!(gene.value.curations.iter().any(|c| c.classification == "Definitive"));
    }
}
