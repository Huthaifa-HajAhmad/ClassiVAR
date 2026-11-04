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

/// Look a gene up in G2P. Blocks until the answer arrives; call from a background thread.
pub fn lookup_gene(symbol: &str) -> Result<Fetched<G2pGene>, SourceError> {
    let symbol = crate::check_symbol(symbol)?;
    let url = format!("{URL}/{symbol}/summary/");
    let records = match crate::http_agent().get(&url).header("Accept", "application/json").call() {
        Ok(mut response) => {
            let body = response.body_mut().read_to_string().map_err(|e| SourceError::Network(e.to_string()))?;
            parse_summary(&body)?
        }
        // G2P answers 404 for a gene it does not hold. The live test proves a known gene still works,
        // so a changed address would be caught there.
        Err(ureq::Error::StatusCode(404)) => Vec::new(),
        Err(error) => return Err(SourceError::Network(error.to_string())),
    };
    Ok(Fetched::now(SOURCE, G2pGene { symbol, records }))
}

fn parse_summary(body: &str) -> Result<Vec<G2pRecord>, SourceError> {
    let summary: Summary = serde_json::from_str(body).map_err(|e| SourceError::Unexpected(e.to_string()))?;
    Ok(summary.records_summary)
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
