//! Looking up facts from outside databases.
//!
//! Every lookup returns its source and the date it was fetched. A lookup that
//! fails is an error, never an empty answer: "could not check" must never look
//! like "nothing found". "Not found" is only reported when the service itself
//! says so.
//!
//! Lookups block while they wait for the network. Call them from a background
//! thread, not from the screen.

pub mod clingen;
mod date;
pub mod g2p;
pub mod panelapp;

use std::time::{Duration, SystemTime};

/// A value together with where it came from and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched<T> {
    pub source: &'static str,
    /// UTC date, as `YYYY-MM-DD`.
    pub fetched_on: String,
    pub value: T,
}

impl<T> Fetched<T> {
    fn now(source: &'static str, value: T) -> Self {
        Self { source, fetched_on: date::utc_date(SystemTime::now()), value }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("'{0}' is not a gene symbol (use letters, digits and hyphens, up to 20 characters)")]
    InvalidSymbol(String),
    #[error("could not reach the service: {0}")]
    Network(String),
    #[error("the service sent an answer this program does not understand: {0}")]
    Unexpected(String),
}

/// Gene symbols are letters, digits and hyphens. Checking once here keeps odd text out of web addresses.
fn check_symbol(symbol: &str) -> Result<String, SourceError> {
    let symbol = symbol.trim().to_ascii_uppercase();
    let valid =
        !symbol.is_empty() && symbol.len() <= 20 && symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if valid { Ok(symbol) } else { Err(SourceError::InvalidSymbol(symbol)) }
}

/// One web client setup for every lookup, so they all give up after the same time.
fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_are_checked_and_normalised() {
        assert_eq!(check_symbol(" brca1 ").unwrap(), "BRCA1");
        assert_eq!(check_symbol("HLA-A").unwrap(), "HLA-A");
        assert!(check_symbol("").is_err());
        assert!(check_symbol("BRCA1&format=xml").is_err());
        assert!(check_symbol("A VERY LONG TEXT THAT IS NOT A GENE").is_err());
    }
}
