//! Looking up facts from outside databases.
//!
//! Every lookup returns its source and the date it was fetched. A lookup that
//! fails is an error, never an empty answer: "could not check" must never look
//! like "nothing found". "Not found" is only reported when the service itself
//! says so, in the form it documents.
//!
//! Lookups block while they wait for the network. Call them from a background
//! thread, not from the screen.

pub mod clingen;
mod date;
pub mod g2p;
pub mod panelapp;

use std::sync::OnceLock;
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

/// Today in UTC, as `YYYY-MM-DD`.
pub fn utc_today() -> String {
    date::utc_date(SystemTime::now())
}

/// The current year in UTC, for example 2026.
pub fn current_year() -> u16 {
    date::utc_date(SystemTime::now()).get(..4).and_then(|year| year.parse().ok()).unwrap_or(2000)
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("'{0}' is not a gene symbol (use letters, digits and hyphens, up to 20 characters)")]
    InvalidSymbol(String),
    #[error("could not reach the service: {0}")]
    Network(String),
    #[error("the service did not answer in time")]
    Timeout,
    #[error("the service is limiting requests (HTTP 429); try again later")]
    RateLimited,
    #[error("the service reported an error on its side (HTTP {0}); try again later")]
    ServerError(u16),
    #[error("the service refused the request (HTTP {0})")]
    Status(u16),
    #[error("the answer is larger than {0} bytes, so it was not read")]
    TooLarge(u64),
    #[error("the service sent an answer this program does not understand: {0}")]
    Unexpected(String),
}

/// Is this text shaped like a gene symbol? Letters, digits and hyphens, up to 20
/// characters, ignoring spaces around it. The screen asks this before it starts a
/// lookup, so it can say what is wrong without waiting for the network.
pub fn is_gene_symbol(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && text.len() <= 20 && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// A gene symbol in upper case. Checking once here keeps odd text out of web addresses.
fn check_symbol(symbol: &str) -> Result<String, SourceError> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if is_gene_symbol(&symbol) { Ok(symbol) } else { Err(SourceError::InvalidSymbol(symbol)) }
}

/// An answer from a service. Rate limits and server errors are retried and then
/// become errors. Every other status (200, 404, 403 ...) reaches the caller with
/// its body, and the caller decides what it means.
pub(crate) struct Reply {
    pub status: u16,
    pub body: String,
}

/// Three tries in all, with a wait of 1 second and then 2 seconds between them.
/// Timeouts, rate limits (429) and server errors (5xx) are tried again. Other
/// failures (no connection, bad certificate) fail at once, because trying again
/// would not change the answer in a way the user could trust.
const TRIES: u32 = 3;

fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(1 << attempt.min(3))
}

/// Errors that a later try may fix: too many requests, or a fault on the service's side.
fn temporary_failure(status: u16) -> Option<SourceError> {
    match status {
        429 => Some(SourceError::RateLimited),
        500..=599 => Some(SourceError::ServerError(status)),
        _ => None,
    }
}

fn describe(error: ureq::Error) -> SourceError {
    match error {
        ureq::Error::Timeout(_) => SourceError::Timeout,
        ureq::Error::BodyExceedsLimit(limit) => SourceError::TooLarge(limit),
        other => SourceError::Network(other.to_string()),
    }
}

/// Where a service can read about this program. It is a project page, not a
/// person's address: the services ask automated clients to say who they are.
const PROJECT_URL: &str = "https://github.com/Huthaifa-HajAhmad/ClassiVAR";

fn user_agent() -> String {
    format!("ClassiVAR/{} (variant curation learning tool; {PROJECT_URL})", env!("CARGO_PKG_VERSION"))
}

/// One web client setup for every lookup. It gives up after 30 seconds, names
/// this program in the request, and hands back every HTTP status so the caller
/// can read the body of a 404 (a status is not treated as a failure here).
fn http_agent() -> &'static ureq::Agent {
    // One agent for the whole program: its connections are reused, so the pages of
    // one PanelApp search and later lookups do not each open a new connection.
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .user_agent(user_agent())
            .build()
            .into()
    })
}

/// Fetch a web address, retrying rate limits and server errors a few times.
/// The body is read only up to `max_bytes`.
pub(crate) fn get(url: &str, query: &[(&str, &str)], accept: &str, max_bytes: u64) -> Result<Reply, SourceError> {
    let agent = http_agent();
    let mut last_failure = SourceError::Network("no attempt was made".to_string());
    for attempt in 0..TRIES {
        if attempt > 0 {
            std::thread::sleep(backoff(attempt - 1));
        }
        let mut request = agent.get(url).header("Accept", accept);
        for (key, value) in query {
            request = request.query(key, value);
        }
        let mut response = match request.call() {
            Ok(response) => response,
            Err(ureq::Error::Timeout(_)) => {
                last_failure = SourceError::Timeout;
                continue;
            }
            Err(error) => return Err(describe(error)),
        };
        let status = response.status().as_u16();
        if let Some(failure) = temporary_failure(status) {
            last_failure = failure;
            continue;
        }
        let body = response.body_mut().with_config().limit(max_bytes).read_to_string().map_err(describe)?;
        return Ok(Reply { status, body });
    }
    Err(last_failure)
}

/// Like `get`, but any answer other than 2xx is an error. Use it when "not found" is not a normal answer.
pub(crate) fn get_ok(url: &str, query: &[(&str, &str)], accept: &str, max_bytes: u64) -> Result<String, SourceError> {
    let reply = get(url, query, accept, max_bytes)?;
    if (200..300).contains(&reply.status) { Ok(reply.body) } else { Err(SourceError::Status(reply.status)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_agent_names_the_program_its_version_and_the_project_page() {
        let agent = user_agent();
        assert!(agent.starts_with(&format!("ClassiVAR/{} ", env!("CARGO_PKG_VERSION"))), "{agent}");
        assert!(
            agent.contains("https://github.com/Huthaifa-HajAhmad/ClassiVAR"),
            "a service needs a way to reach the project: {agent}"
        );
        assert!(!agent.contains('@'), "no email address in a request header: {agent}");
    }

    #[test]
    fn symbols_are_checked_and_normalised() {
        assert_eq!(check_symbol(" brca1 ").unwrap(), "BRCA1");
        assert_eq!(check_symbol("HLA-A").unwrap(), "HLA-A");
        assert!(check_symbol("").is_err());
        assert!(check_symbol("BRCA1&format=xml").is_err());
        assert!(check_symbol("A VERY LONG TEXT THAT IS NOT A GENE").is_err());
    }

    #[test]
    fn only_symbol_shaped_text_is_a_gene_symbol() {
        assert!(is_gene_symbol("BRCA1"));
        assert!(is_gene_symbol("NKX2-5"));
        assert!(is_gene_symbol(" c9orf72 "));
        assert!(!is_gene_symbol(""));
        assert!(!is_gene_symbol("   "));
        assert!(!is_gene_symbol("BRCA1; DROP"));
        assert!(!is_gene_symbol("a b"));
        assert!(!is_gene_symbol(&"A".repeat(21)));
    }

    #[test]
    fn only_rate_limits_and_server_faults_are_retried() {
        assert!(matches!(temporary_failure(429), Some(SourceError::RateLimited)));
        assert!(matches!(temporary_failure(503), Some(SourceError::ServerError(503))));
        assert!(temporary_failure(404).is_none());
        assert!(temporary_failure(200).is_none());
        assert!(temporary_failure(403).is_none());
    }

    #[test]
    fn waits_grow_but_stay_short() {
        assert_eq!(backoff(0), Duration::from_secs(1));
        assert_eq!(backoff(1), Duration::from_secs(2));
        assert!(backoff(10) <= Duration::from_secs(8));
    }

    #[test]
    fn the_current_year_is_plausible() {
        assert!(current_year() >= 2026);
    }
}
