//! Load-time checks of the prediction tools used for PP3 and BP4.

use std::collections::HashSet;

use crate::model::{Direction, Strength};

use super::check::{check_strength_ladder, limits_move_away_from_the_middle, needs_source};
use super::{Predictor, RuleError, RuleSet};

/// Strength order of a limit, to compare a limit with the calibrated strength.
fn rank(strength: Strength) -> Option<usize> {
    [Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong]
        .iter()
        .position(|s| *s == strength)
}

impl RuleSet {
    /// Every predictor needs a known source, and its limits must run in order, so
    /// a stronger level can never be reached by a score that misses a weaker one.
    pub(super) fn check_predictors(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        if self.predictors.is_empty() {
            return Ok(());
        }
        // `read_predictor` maps a high score to PP3 and a low one to BP4, so the
        // codes must sit that way, and PP3/BP4 cannot be discouraged pair members.
        let shared =
            |problem: &str| RuleError::BadPredictor { id: "<predictors>".to_string(), problem: problem.to_string() };
        let (Some(pp3), Some(bp4)) = (self.criterion("PP3"), self.criterion("BP4")) else {
            return Err(shared("predictors need the criteria PP3 and BP4"));
        };
        let counted_pair = pp3.direction == Direction::Pathogenic
            && !pp3.discouraged
            && bp4.direction == Direction::Benign
            && !bp4.discouraged;
        if !counted_pair {
            return Err(shared("PP3 must be a counted pathogenic criterion and BP4 a counted benign one"));
        }
        let mut ids = HashSet::new();
        for tool in &self.predictors {
            let unique = ids.insert(tool.id.as_str());
            if let Err(problem) = check_predictor(tool, unique, known) {
                return Err(match problem {
                    PredictorFault::Source(error) => error,
                    PredictorFault::Problem(problem) => RuleError::BadPredictor { id: tool.id.clone(), problem },
                });
            }
        }
        Ok(())
    }
}

/// What is wrong with one predictor: a source the file does not list, or a problem in words.
enum PredictorFault {
    Source(RuleError),
    Problem(String),
}

impl From<&str> for PredictorFault {
    fn from(problem: &str) -> Self {
        Self::Problem(problem.to_string())
    }
}

impl From<RuleError> for PredictorFault {
    fn from(error: RuleError) -> Self {
        Self::Source(error)
    }
}

fn check_predictor(tool: &Predictor, unique: bool, known: &HashSet<&str>) -> Result<(), PredictorFault> {
    if tool.id.trim().is_empty() || tool.name.trim().is_empty() || tool.reference.trim().is_empty() || !unique {
        return Err("the id, name and reference must be filled in, and the id must be unique".into());
    }
    needs_source(known, format!("predictor {}", tool.id), &tool.source)?;
    if tool.pathogenic.is_empty() && tool.benign.is_empty() {
        return Err("it has no score limits".into());
    }
    if tool.eligibility.as_deref().is_some_and(|text| text.trim().is_empty()) {
        return Err("the eligibility text is empty".into());
    }
    match (tool.min_score, tool.max_score) {
        (None, None) => {}
        (Some(min), Some(max)) if min.is_finite() && max.is_finite() && min < max => {}
        _ => {
            return Err(
                "min_score and max_score must be set together, be finite, and put min_score below max_score".into()
            );
        }
    }
    if let Some(calibrated) = tool.calibrated {
        let Some(calibrated_rank) = rank(calibrated) else {
            return Err("the calibrated strength must be supporting, moderate, strong or very_strong".into());
        };
        if tool.pathogenic.iter().chain(&tool.benign).any(|cut| rank(cut.strength).is_some_and(|r| r > calibrated_rank))
        {
            return Err("a limit is stronger than the calibrated evidence it describes".into());
        }
    }
    for (limits, rising) in [(&tool.pathogenic, true), (&tool.benign, false)] {
        check_strength_ladder(limits)?;
        if limits.iter().any(|cut| !cut.at.is_finite()) {
            return Err("a score limit is not a number".into());
        }
        if !limits_move_away_from_the_middle(limits, rising) {
            return Err("stronger limits must lie further from the middle of the scale".into());
        }
    }
    // A score must never be both PP3 and BP4.
    if let (Some(p), Some(b)) = (tool.pathogenic.first(), tool.benign.first())
        && b.at >= p.at
    {
        return Err("the benign and pathogenic limits overlap".into());
    }
    Ok(())
}
