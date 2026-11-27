//! Review notes are separate from counted evidence. Missing decisions mean
//! not yet reviewed; applying a criterion is recorded only in today's evidence.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Applied, CaseError};
use crate::rules::RuleSet;

/// Most decisions, and most fact answers, a case file may hold.
pub const MAX_REVIEW_ENTRIES: usize = 300;
/// Longest criterion code or fact id.
pub const MAX_REVIEW_NAME_CHARS: usize = 80;
/// Longest answer to a fact question.
pub const MAX_REVIEW_ANSWER_CHARS: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    DoesNotApply,
    Deferred,
}

/// Saved review answers, including entries from rules no longer in use.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseReview {
    #[serde(default)]
    pub decisions: BTreeMap<String, ReviewDecision>,
    #[serde(default)]
    pub facts: BTreeMap<String, String>,
}

impl CaseReview {
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.facts.is_empty()
    }

    /// Names of entries these rules cannot read: decisions about criteria they lack,
    /// and facts they do not ask or answers they do not list. The entries stay saved
    /// for the rules that made them, are shown as set aside, and never count as progress.
    pub fn set_aside(&self, rules: &RuleSet) -> Vec<&str> {
        let decisions = self.decisions.keys().filter(|code| rules.criterion(code).is_none());
        let facts = self
            .facts
            .iter()
            .filter(|(id, answer)| !rules.fact(id).is_some_and(|fact| fact.lists(answer)))
            .map(|(id, _)| id);
        decisions.chain(facts).map(String::as_str).collect()
    }

    pub(super) fn check(&self, today: &[Applied]) -> Result<(), CaseError> {
        if self.decisions.len() > MAX_REVIEW_ENTRIES || self.facts.len() > MAX_REVIEW_ENTRIES {
            return Err(CaseError::BadReview(format!(
                "at most {MAX_REVIEW_ENTRIES} decisions and {MAX_REVIEW_ENTRIES} facts are allowed"
            )));
        }
        for key in self.decisions.keys().chain(self.facts.keys()) {
            if key.trim().is_empty() || key.chars().count() > MAX_REVIEW_NAME_CHARS {
                return Err(CaseError::BadReview(format!(
                    "names must contain 1 to {MAX_REVIEW_NAME_CHARS} characters"
                )));
            }
        }
        for answer in self.facts.values() {
            if answer.trim().is_empty() || answer.chars().count() > MAX_REVIEW_ANSWER_CHARS {
                return Err(CaseError::BadReview(format!(
                    "fact answers must contain 1 to {MAX_REVIEW_ANSWER_CHARS} characters"
                )));
            }
        }
        for item in today {
            if self.decisions.contains_key(&item.code) {
                return Err(CaseError::BadReview(format!("{} is both counted and excluded or deferred", item.code)));
            }
        }
        Ok(())
    }
}
