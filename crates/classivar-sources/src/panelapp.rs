//! PanelApp (Genomics England): which expert gene panels include a gene, and
//! how confident the panel reviewers are that the gene causes the disease.
//!
//! Important: being on a PanelApp panel is expert evidence about the gene and
//! disease link, but it is not the same as a ClinGen gene-disease validity
//! classification. The program shows it as one input; the curator decides.

use serde::Deserialize;

use crate::{Fetched, SourceError};

/// PanelApp runs as two services with the same format. Australia's is
/// updated faster, according to the owner of this project; the England one is the larger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instance {
    England,
    Australia,
}

impl Instance {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::England => "PanelApp England (Genomics England)",
            Self::Australia => "PanelApp Australia",
        }
    }

    fn url(self) -> &'static str {
        match self {
            Self::England => "https://panelapp.genomicsengland.co.uk/api/v1/genes/",
            Self::Australia => "https://panelapp-aus.org/api/v1/genes/",
        }
    }
}
/// More pages than this means something is wrong, so stop and say so.
const MAX_PAGES: usize = 10;

/// How confident the panel reviewers are about a gene. PanelApp's traffic-light colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Level 0 in PanelApp.
    NotRated,
    /// Red, level 1: low evidence.
    Red,
    /// Amber, level 2: some evidence.
    Amber,
    /// Green, level 3: high evidence.
    Green,
}

impl Confidence {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotRated => "not rated",
            Self::Red => "red (low)",
            Self::Amber => "amber (some)",
            Self::Green => "green (high)",
        }
    }

    fn from_level(level: &str) -> Result<Self, SourceError> {
        match level {
            "0" => Ok(Self::NotRated),
            "1" => Ok(Self::Red),
            "2" => Ok(Self::Amber),
            "3" => Ok(Self::Green),
            other => Err(SourceError::Unexpected(format!("unknown confidence level '{other}'"))),
        }
    }
}

/// One gene on one panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelEntry {
    pub panel_id: u32,
    pub panel_name: String,
    pub panel_version: String,
    pub confidence: Confidence,
    pub mode_of_inheritance: String,
    pub phenotypes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelAppGene {
    pub symbol: String,
    /// Best confidence first, then by panel name. Empty if the gene is on no panel.
    pub entries: Vec<PanelEntry>,
}

impl PanelAppGene {
    pub fn count_at(&self, confidence: Confidence) -> usize {
        self.entries.iter().filter(|e| e.confidence == confidence).count()
    }
}

#[derive(Deserialize)]
struct Page {
    next: Option<String>,
    results: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    entity_name: String,
    confidence_level: String,
    #[serde(default)]
    mode_of_inheritance: String,
    #[serde(default)]
    phenotypes: Vec<String>,
    panel: RawPanel,
}

#[derive(Deserialize)]
struct RawPanel {
    id: u32,
    name: String,
    version: String,
}

/// Look a gene up on PanelApp. Blocks until the answer arrives; call from a background thread.
pub fn lookup_gene(instance: Instance, symbol: &str) -> Result<Fetched<PanelAppGene>, SourceError> {
    let symbol = crate::check_symbol(symbol)?;
    let agent = crate::http_agent();

    let mut entries = Vec::new();
    let mut request = agent.get(instance.url()).query("entity_name", &symbol).query("format", "json");
    for _ in 0..MAX_PAGES {
        let mut response = request.call().map_err(|e| SourceError::Network(e.to_string()))?;
        let body = response.body_mut().read_to_string().map_err(|e| SourceError::Network(e.to_string()))?;
        let page = parse_page(&body, &symbol)?;
        entries.extend(page.entries);
        match page.next {
            Some(next) => request = agent.get(&next),
            None => return Ok(Fetched::now(instance.source_name(), finish(symbol, entries))),
        }
    }
    Err(SourceError::Unexpected(format!("more than {MAX_PAGES} pages of results")))
}

struct ParsedPage {
    next: Option<String>,
    entries: Vec<PanelEntry>,
}

/// Read one page. Only entries for exactly this gene are kept, in case the service matches loosely.
fn parse_page(body: &str, symbol: &str) -> Result<ParsedPage, SourceError> {
    let page: Page = serde_json::from_str(body).map_err(|e| SourceError::Unexpected(e.to_string()))?;
    let entries = page
        .results
        .into_iter()
        .filter(|raw| raw.entity_name.eq_ignore_ascii_case(symbol))
        .map(|raw| {
            Ok(PanelEntry {
                panel_id: raw.panel.id,
                panel_name: raw.panel.name,
                panel_version: raw.panel.version,
                confidence: Confidence::from_level(&raw.confidence_level)?,
                mode_of_inheritance: raw.mode_of_inheritance,
                phenotypes: raw.phenotypes,
            })
        })
        .collect::<Result<Vec<_>, SourceError>>()?;
    Ok(ParsedPage { next: page.next, entries })
}

fn finish(symbol: String, mut entries: Vec<PanelEntry>) -> PanelAppGene {
    entries.sort_by(|a, b| b.confidence.cmp(&a.confidence).then_with(|| a.panel_name.cmp(&b.panel_name)));
    PanelAppGene { symbol, entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped like a real PanelApp response for BRCA1, with fields trimmed.
    /// The first two entries follow the real answer of 2026-11-03; the third is invented to cover level 1.
    const SAMPLE: &str = include_str!("../tests/fixtures/panelapp_brca1_sample.json");

    #[test]
    fn parses_a_response_and_ignores_fields_it_does_not_need() {
        let page = parse_page(SAMPLE, "BRCA1").unwrap();
        assert_eq!(page.next, None);
        assert_eq!(page.entries.len(), 3);
        assert_eq!(page.entries[0].panel_name, "Familial breast cancer");
        assert_eq!(page.entries[0].panel_version, "1.28");
        assert_eq!(page.entries[0].confidence, Confidence::Green);
        assert_eq!(page.entries[0].phenotypes, vec!["{Breast-ovarian cancer, familial, 1}, OMIM:604370"]);
    }

    #[test]
    fn entries_are_sorted_best_confidence_first() {
        let gene = finish("BRCA1".into(), parse_page(SAMPLE, "BRCA1").unwrap().entries);
        let order: Vec<_> = gene.entries.iter().map(|e| e.confidence).collect();
        assert_eq!(order, vec![Confidence::Green, Confidence::Amber, Confidence::Red]);
        assert_eq!(gene.count_at(Confidence::Green), 1);
    }

    #[test]
    fn an_unknown_confidence_level_is_an_error_not_a_guess() {
        let body = SAMPLE.replace("\"confidence_level\": \"3\"", "\"confidence_level\": \"7\"");
        assert!(matches!(parse_page(&body, "BRCA1"), Err(SourceError::Unexpected(_))));
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(matches!(parse_page("<html>maintenance</html>", "BRCA1"), Err(SourceError::Unexpected(_))));
    }

    #[test]
    fn entries_for_other_genes_are_dropped() {
        assert!(parse_page(SAMPLE, "BRCA2").unwrap().entries.is_empty());
    }

    /// Needs the internet. Run on purpose with: cargo test -p classivar-sources -- --ignored
    #[test]
    #[ignore]
    fn live_brca1_is_on_a_green_panel_in_both_services() {
        for instance in [Instance::England, Instance::Australia] {
            let gene = lookup_gene(instance, "BRCA1").unwrap();
            assert_eq!(gene.source, instance.source_name());
            assert!(gene.value.count_at(Confidence::Green) >= 1, "{instance:?}");
        }
    }

    #[test]
    #[ignore]
    fn live_unknown_gene_is_empty_not_an_error() {
        let gene = lookup_gene(Instance::England, "NOTAGENE1").unwrap();
        assert!(gene.value.entries.is_empty());
    }
}
