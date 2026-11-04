//! Gene2Phenotype (G2P, EMBL-EBI): expert-curated gene and disease records,
//! each with a confidence level, mode of inheritance and disease mechanism.
//!
//! This is the same G2P data that DECIPHER shows on its gene pages.

use serde::Deserialize;

use crate::{Fetched, SourceError};

const SOURCE: &str = "Gene2Phenotype (EMBL-EBI)";
const URL: &str = "https://www.ebi.ac.uk/gene2phenotype/api/gene";

/// One gene and disease record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct G2pRecord {
    pub disease: String,
    /// For example `monoallelic_autosomal`.
    #[serde(default)]
    pub genotype: String,
    /// As G2P writes it: definitive, strong, moderate, limited, and so on.
    pub confidence: String,
    #[serde(default)]
    pub molecular_mechanism: String,
    #[serde(default)]
    pub panels: Vec<String>,
    #[serde(default)]
    pub stable_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct G2pGene {
    pub symbol: String,
    /// Empty if G2P has no record for this gene.
    pub records: Vec<G2pRecord>,
}

#[derive(Deserialize)]
struct Summary {
    records_summary: Vec<G2pRecord>,
}

/// The most G2P may send for one gene. A gene summary is a few kilobytes.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// Look a gene up in G2P. Blocks until the answer arrives; call from a background thread.
pub fn lookup_gene(symbol: &str) -> Result<Fetched<G2pGene>, SourceError> {
    let symbol = crate::check_symbol(symbol)?;
    let url = format!("{URL}/{symbol}/summary/");
    let reply = crate::get(&url, &[], "application/json", MAX_BYTES)?;
    let records = match reply.status {
        200..=299 => parse_summary(&reply.body)?,
        404 => parse_not_found(&reply.body)?,
        other => return Err(SourceError::Status(other)),
    };
    Ok(Fetched::now(SOURCE, G2pGene { symbol, records }))
}

fn parse_summary(body: &str) -> Result<Vec<G2pRecord>, SourceError> {
    let summary: Summary = serde_json::from_str(body).map_err(|e| SourceError::Unexpected(e.to_string()))?;
    Ok(summary.records_summary)
}

/// A 404 means "no record" only when G2P says so in its documented way: a JSON
/// body `{"error": "No matching Gene found ..."}`. Any other 404 (for example an
/// HTML page after the address moved) is an error, not an empty answer.
fn parse_not_found(body: &str) -> Result<Vec<G2pRecord>, SourceError> {
    #[derive(Deserialize)]
    struct NotFound {
        error: String,
    }
    match serde_json::from_str::<NotFound>(body) {
        Ok(answer) if answer.error.to_ascii_lowercase().contains("no matching gene") => Ok(Vec::new()),
        _ => Err(SourceError::Unexpected("a 404 answer that is not G2P's 'no matching gene' message".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real answer for BRCA1 on 2026-11-03, with the fields we do not use left in.
    const SAMPLE: &str = r#"{"gene_symbol":"BRCA1","records_summary":[
        {"disease":"BRCA1-related Fanconi anemia","genotype":"biallelic_autosomal","confidence":"strong","panels":["DD","Skeletal"],"variant_consequence":["absent gene product"],"variant_type":[],"molecular_mechanism":"loss of function","last_updated":"2023-05-24","stable_id":"G2P00119"},
        {"disease":"BRCA1-related cancer","genotype":"monoallelic_autosomal","confidence":"definitive","panels":["Cancer"],"variant_consequence":["absent gene product"],"variant_type":[],"molecular_mechanism":"loss of function","last_updated":"2022-12-07","stable_id":"G2P01796"}]}"#;

    #[test]
    fn parses_the_real_shape() {
        let records = parse_summary(SAMPLE).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].disease, "BRCA1-related cancer");
        assert_eq!(records[1].confidence, "definitive");
        assert_eq!(records[1].genotype, "monoallelic_autosomal");
        assert_eq!(records[1].panels, vec!["Cancer"]);
    }

    #[test]
    fn a_404_is_an_empty_answer_only_for_the_documented_message() {
        let unknown = r#"{"error":"No matching Gene found for: NOTAGENE1"}"#;
        assert_eq!(parse_not_found(unknown).unwrap(), Vec::new());
        // A moved address answers 404 with a web page, and other JSON errors are not "no record".
        assert!(matches!(parse_not_found("<html>Not Found</html>"), Err(SourceError::Unexpected(_))));
        assert!(matches!(parse_not_found(r#"{"error":"Endpoint removed"}"#), Err(SourceError::Unexpected(_))));
        assert!(matches!(parse_not_found(""), Err(SourceError::Unexpected(_))));
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(matches!(parse_summary("<html>maintenance</html>"), Err(SourceError::Unexpected(_))));
        assert!(matches!(parse_summary(r#"{"unexpected":true}"#), Err(SourceError::Unexpected(_))));
    }

    /// Needs the internet. Run on purpose with: cargo test -p classivar-sources -- --ignored
    #[test]
    #[ignore]
    fn live_brca1_has_a_definitive_record() {
        let gene = lookup_gene("BRCA1").unwrap();
        assert!(gene.value.records.iter().any(|r| r.confidence == "definitive"));
    }

    #[test]
    #[ignore]
    fn live_unknown_gene_is_empty() {
        assert!(lookup_gene("NOTAGENE1").unwrap().value.records.is_empty());
    }
}
