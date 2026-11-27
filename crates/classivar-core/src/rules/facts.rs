//! Optional fact questions and the hints their answers add to a review step.
//! A hint is only words for the curator: it never applies, excludes or scores evidence.

use std::collections::HashSet;

use serde::Deserialize;

use super::{ReviewStep, RuleError, RuleSet};
use crate::case::{MAX_REVIEW_ANSWER_CHARS, MAX_REVIEW_NAME_CHARS};

/// Every fact lets the curator say that they do not know, or that no answer fits.
const OPEN_ANSWERS: [&str; 2] = ["unknown", "other"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub id: String,
    pub question: String,
    pub answers: Vec<String>,
}

/// Words shown on a step when the curator gave `answer` to the fact `fact`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hint {
    pub fact: String,
    pub answer: String,
    pub text: String,
}

impl Fact {
    pub fn lists(&self, answer: &str) -> bool {
        self.answers.iter().any(|listed| listed == answer)
    }

    /// A case file must be able to hold the id and every answer.
    fn is_well_formed(&self) -> bool {
        let fits = |text: &str, max: usize| !text.trim().is_empty() && text.chars().count() <= max;
        let mut seen = HashSet::new();
        fits(&self.id, MAX_REVIEW_NAME_CHARS)
            && !self.question.trim().is_empty()
            && self.answers.iter().all(|a| fits(a, MAX_REVIEW_ANSWER_CHARS) && seen.insert(a))
            && OPEN_ANSWERS.iter().all(|open| self.lists(open))
    }
}

impl RuleSet {
    pub fn fact(&self, id: &str) -> Option<&Fact> {
        self.facts.iter().find(|fact| fact.id == id)
    }

    pub(super) fn check_facts(&self) -> Result<(), RuleError> {
        let mut ids = HashSet::new();
        match self.facts.iter().find(|fact| !fact.is_well_formed() || !ids.insert(&fact.id)) {
            None => Ok(()),
            Some(fact) => Err(RuleError::BadPath(format!(
                "fact '{}' needs a unique id, a question, and different answers that include unknown and other",
                fact.id
            ))),
        }
    }

    pub(super) fn check_hints(&self, step: &ReviewStep) -> Result<(), RuleError> {
        let mut seen = HashSet::new();
        let sound = |hint: &Hint| {
            !hint.text.trim().is_empty() && self.fact(&hint.fact).is_some_and(|fact| fact.lists(&hint.answer))
        };
        if step.hints.iter().all(|hint| sound(hint) && seen.insert((&hint.fact, &hint.answer))) {
            return Ok(());
        }
        Err(RuleError::BadPath(format!("{}: each hint needs text and its own listed fact and answer", step.id)))
    }
}
