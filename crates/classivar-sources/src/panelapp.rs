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

    /// The start of every address this service may send us in a "next page" link.
    /// It ends with a slash, so `panelapp-aus.org.example.com` does not match.
    fn origin(self) -> &'static str {
        match self {
            Self::England => "https://panelapp.genomicsengland.co.uk/",
            Self::Australia => "https://panelapp-aus.org/",
        }
    }
}

/// More pages than this means something is wrong, so stop and say so.
const MAX_PAGES: usize = 10;
/// The most one page may send. A page is a few hundred kilobytes at most.
const MAX_BYTES: u64 = 5 * 1024 * 1024;

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
    /// How many entries the service says match, over all pages.
    count: u64,
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
    let entries =
        collect_pages(instance, &symbol, |url, query| crate::get_ok(url, query, "application/json", MAX_BYTES))?;
    Ok(Fetched::now(instance.source_name(), finish(symbol, entries)))
}

/// Follow the pages of one search and check that the answer is complete. `fetch`
/// gets a web address and a query and returns the page text; the tests give it
/// made-up pages instead of the network.
fn collect_pages(
    instance: Instance,
    symbol: &str,
    mut fetch: impl FnMut(&str, &[(&str, &str)]) -> Result<String, SourceError>,
) -> Result<Vec<PanelEntry>, SourceError> {
    let mut entries = Vec::new();
    let mut received = 0;
    let mut url = instance.url().to_string();
    let mut query: Vec<(&str, &str)> = vec![("entity_name", symbol), ("format", "json")];
    for _ in 0..MAX_PAGES {
        let page = parse_page(&fetch(&url, &query)?, symbol)?;
        entries.extend(page.entries);
        received += page.received;
        match page.next {
            Some(next) => {
                check_next_link(instance, &next)?;
                url = next;
                // The next link already carries the search, so nothing is added to it.
                query.clear();
            }
            None => {
                if received != page.count {
                    return Err(SourceError::Unexpected(format!(
                        "the service reports {} entries but sent {received}; the answer may be incomplete",
                        page.count
                    )));
                }
                return Ok(entries);
            }
        }
    }
    Err(SourceError::Unexpected(format!("more than {MAX_PAGES} pages of results")))
}

/// A "next page" link must stay on the same service over a secure connection,
/// so a changed or hijacked answer cannot send us somewhere else.
fn check_next_link(instance: Instance, next: &str) -> Result<(), SourceError> {
    if next.starts_with(instance.origin()) {
        Ok(())
    } else {
        Err(SourceError::Unexpected(format!("a next-page link that leaves {}", instance.origin())))
    }
}

struct ParsedPage {
    next: Option<String>,
    /// Entries for this gene only.
    entries: Vec<PanelEntry>,
    /// Entries on the page before filtering by gene. Compared with the service's own count.
    received: u64,
    count: u64,
}

/// Read one page. Only entries for exactly this gene are kept, in case the service matches loosely.
fn parse_page(body: &str, symbol: &str) -> Result<ParsedPage, SourceError> {
    let page: Page = serde_json::from_str(body).map_err(|e| SourceError::Unexpected(e.to_string()))?;
    let received = page.results.len() as u64;
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
    Ok(ParsedPage { next: page.next, entries, received, count: page.count })
}

fn finish(symbol: String, mut entries: Vec<PanelEntry>) -> PanelAppGene {
    entries.sort_by(|a, b| b.confidence.cmp(&a.confidence).then_with(|| a.panel_name.cmp(&b.panel_name)));
    PanelAppGene { symbol, entries }
}

#[cfg(test)]
mod tests;
