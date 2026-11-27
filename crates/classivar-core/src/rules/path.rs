//! Optional review order. Steps guide navigation, never select evidence.

use super::{Hint, ReviewItem, RuleError, RuleSet};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewStep {
    pub id: String,
    pub question: String,
    pub criteria: Vec<String>,
    pub helper: Option<String>,
    #[serde(default)]
    pub hints: Vec<Hint>,
}

/// Criteria owned by each evidence helper. References do not apply evidence.
pub const HELPER_CRITERIA: &[(&str, &[&str])] = &[
    ("population", &["BA1", "BS1", "PM2"]),
    ("predictor", &["PP3", "BP4"]),
    ("functional", &["PS3", "BS3"]),
    ("locus", &["PP1", "PP4"]),
    ("segregation", &["BS4"]),
    ("de-novo", &["PS2", "PM6"]),
    ("pm3", &["PM3"]),
];

impl RuleSet {
    pub(super) fn check_path(&self) -> Result<(), RuleError> {
        self.check_facts()?;
        let Some(path) = &self.path else { return Ok(()) };
        let mut ids = HashSet::new();
        let mut codes = HashSet::new();
        for step in path {
            if step.id.trim().is_empty()
                || !ids.insert(&step.id)
                || step.question.trim().is_empty()
                || step.criteria.is_empty()
            {
                return Err(RuleError::BadPath("steps need unique nonempty ids, questions and criteria".into()));
            }
            for code in &step.criteria {
                if self.criterion(code).is_none() || !codes.insert(code.as_str()) {
                    return Err(RuleError::BadPath(format!("unknown or repeated criterion {code}")));
                }
            }
            check_helper(step)?;
            self.check_hints(step)?;
        }
        if codes.len() != self.criteria.len() {
            return Err(RuleError::BadPath("the path must cover every criterion once".into()));
        }
        Ok(())
    }
}

impl RuleSet {
    /// The sign-off entries of the guided review: facts, steps and hints.
    pub(super) fn guided_items(&self) -> Vec<ReviewItem> {
        let facts = self.facts.iter().map(|fact| ReviewItem {
            id: format!("fact:{}", fact.id),
            summary: format!("{} ({})", fact.question, fact.answers.join(" / ")),
        });
        let steps = self.path.iter().flatten().flat_map(|step| {
            let question = ReviewItem { id: format!("path:{}", step.id), summary: step.question.clone() };
            std::iter::once(question).chain(step.hints.iter().map(|hint| ReviewItem {
                id: format!("hint:{}:{}={}", step.id, hint.fact, hint.answer),
                summary: hint.text.clone(),
            }))
        });
        facts.chain(steps).collect()
    }
}

/// A helper fills all its criteria at once, so they must all be on its step.
fn check_helper(step: &ReviewStep) -> Result<(), RuleError> {
    let Some(helper) = &step.helper else { return Ok(()) };
    let owned = HELPER_CRITERIA.iter().find(|(key, _)| key == helper);
    if owned.is_some_and(|(_, owned)| owned.iter().all(|c| step.criteria.iter().any(|s| s == c))) {
        return Ok(());
    }
    Err(RuleError::BadPath(format!(
        "{} names an unknown helper, or one whose criteria are not all on this step",
        step.id
    )))
}

#[cfg(test)]
mod tests {
    use super::{super::HELPER_CODES, HELPER_CRITERIA};

    /// The load check that refuses guides on helper criteria and the path's helper
    /// ownership must name the same criteria; population sets strengths without a guide.
    #[test]
    fn helper_ownership_matches_the_criteria_that_refuse_guides() {
        let mut owned: Vec<&str> = HELPER_CRITERIA
            .iter()
            .filter(|(helper, _)| *helper != "population")
            .flat_map(|(_, codes)| codes.iter().copied())
            .collect();
        let mut guided_out = HELPER_CODES.to_vec();
        owned.sort_unstable();
        guided_out.sort_unstable();
        assert_eq!(owned, guided_out);
    }
}
