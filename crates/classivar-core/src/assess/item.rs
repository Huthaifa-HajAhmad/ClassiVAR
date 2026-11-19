//! Checking and scoring one criterion the curator applied.

use crate::model::Direction;
use crate::rules::{Criterion, RuleSet};

use super::{Applied, AssessError, Counting, Item, MAX_REASON_CHARS, Warning};

/// Refuse what the engine must not score: a bad reason, a strength the rules do
/// not allow, or (in a new assessment) a guided criterion without a matching answer.
fn check_applied(a: &Applied, c: &Criterion, counting: Counting) -> Result<(), AssessError> {
    if let Some(reason) = &a.reason
        && (reason.trim().is_empty() || reason.chars().count() > MAX_REASON_CHARS)
    {
        return Err(AssessError::BadReason { code: a.code.clone() });
    }
    // Today's strength limits apply to new assessments only; an old record may predate them.
    let allowed = match counting {
        Counting::Current => c.allowed_strengths(),
        Counting::AsRecorded => c.recorded_strengths(),
    };
    if !allowed.contains(&a.strength) {
        return Err(AssessError::StrengthNotAllowed { code: a.code.clone(), strength: a.strength });
    }
    // A guide answer is required for a new assessment: the engine must not
    // fall back to a strength the tree did not give. Earlier records may
    // predate the guide, so their saved strength is read as it was made.
    if counting == Counting::Current && !c.guide.is_empty() {
        let Some(situation) = &a.situation else {
            return Err(AssessError::GuideAnswerRequired { code: a.code.clone() });
        };
        let matches = c.guide.iter().find(|g| g.id == *situation).is_some_and(|g| g.strength == Some(a.strength));
        if !matches {
            return Err(AssessError::GuideMismatch { code: a.code.clone(), situation: situation.clone() });
        }
    }
    Ok(())
}

impl RuleSet {
    /// Check one applied criterion and score it. The notes it raises on its own
    /// (a changed strength, a criterion the experts no longer recommend) come with it.
    pub(super) fn score_item(
        &self,
        a: &Applied,
        c: &Criterion,
        counting: Counting,
    ) -> Result<(Item, Vec<Warning>), AssessError> {
        check_applied(a, c, counting)?;
        let counted = !(c.discouraged && counting == Counting::Current);
        let magnitude = if counted { self.magnitude(a.strength) } else { 0 };
        let points = match c.direction {
            Direction::Pathogenic => magnitude,
            Direction::Benign => -magnitude,
        };
        let mut warnings = Vec::new();
        // A guide answer explains the strength (the criterion row shows
        // which answer was chosen); this warning is for a manual change.
        if a.strength != c.strength && counting == Counting::Current && a.situation.is_none() {
            warnings.push(Warning::StrengthChanged {
                code: a.code.clone(),
                from: c.strength,
                to: a.strength,
                reason: a.reason.clone(),
            });
        }
        if c.discouraged {
            warnings.push(Warning::Discouraged { code: a.code.clone(), note: c.note.clone(), counted });
        }
        let item = Item {
            code: a.code.clone(),
            direction: c.direction,
            strength: a.strength,
            default_strength: c.strength,
            situation: a.situation.clone(),
            points,
            counted,
        };
        Ok((item, warnings))
    }
}
