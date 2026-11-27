//! How far the review has got: one state per criterion, counted per step and in total.
//! Progress says what has been looked at. It never says the class is settled.

use super::{Applied, CaseReview, ReviewDecision};
use crate::rules::RuleSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Applied,
    DoesNotApply,
    Deferred,
    NotYetReviewed,
    /// A discouraged criterion (PP5, BP6): shown for reference, outside progress.
    ReferenceOnly,
}

/// Reviewed criteria out of those that count towards progress. `deferred` ones are
/// part of `reviewed` but still open, so they are also counted apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub reviewed: usize,
    pub deferred: usize,
    pub total: usize,
}

impl CaseReview {
    /// Today's evidence wins over a decision; `CaseFile::check` keeps the two apart.
    pub fn state(&self, rules: &RuleSet, today: &[Applied], code: &str) -> ReviewState {
        if rules.criterion(code).is_some_and(|c| c.discouraged) {
            return ReviewState::ReferenceOnly;
        }
        if today.iter().any(|item| item.code == code) {
            return ReviewState::Applied;
        }
        match self.decisions.get(code) {
            Some(ReviewDecision::DoesNotApply) => ReviewState::DoesNotApply,
            Some(ReviewDecision::Deferred) => ReviewState::Deferred,
            None => ReviewState::NotYetReviewed,
        }
    }

    /// Progress over `codes` (one step's criteria, or every criterion of the rules).
    /// Codes the rules lack, such as set-aside decisions, are not counted.
    pub fn progress<S: AsRef<str>>(&self, rules: &RuleSet, today: &[Applied], codes: &[S]) -> Progress {
        let states: Vec<ReviewState> = codes
            .iter()
            .filter(|code| rules.criterion(code.as_ref()).is_some())
            .map(|code| self.state(rules, today, code.as_ref()))
            .filter(|state| *state != ReviewState::ReferenceOnly)
            .collect();
        let count = |wanted: fn(&ReviewState) -> bool| states.iter().filter(|state| wanted(state)).count();
        Progress {
            reviewed: count(|state| *state != ReviewState::NotYetReviewed),
            deferred: count(|state| *state == ReviewState::Deferred),
            total: states.len(),
        }
    }

    /// Progress over every criterion of the rules.
    pub fn total_progress(&self, rules: &RuleSet, today: &[Applied]) -> Progress {
        let codes: Vec<&str> = rules.criteria.iter().map(|c| c.code.as_str()).collect();
        self.progress(rules, today, &codes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strength;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn review(pairs: &[(&str, ReviewDecision)]) -> CaseReview {
        CaseReview { decisions: pairs.iter().map(|(c, d)| (c.to_string(), *d)).collect(), ..CaseReview::default() }
    }

    fn applied(code: &str) -> Applied {
        Applied::new(code, Strength::Moderate)
    }

    #[test]
    fn each_criterion_has_one_state() {
        let r = review(&[("BA1", ReviewDecision::DoesNotApply), ("BS1", ReviewDecision::Deferred)]);
        let today = [applied("PM2")];
        let states: Vec<ReviewState> =
            ["PM2", "BA1", "BS1", "PS3", "PP5"].iter().map(|c| r.state(rules(), &today, c)).collect();
        use ReviewState::*;
        assert_eq!(states, [Applied, DoesNotApply, Deferred, NotYetReviewed, ReferenceOnly]);
    }

    #[test]
    fn progress_counts_reviewed_criteria_and_leaves_out_references_and_set_aside() {
        let r = review(&[("BA1", ReviewDecision::DoesNotApply), ("OLD1", ReviewDecision::Deferred)]);
        let today = [applied("PM2")];
        assert_eq!(
            r.progress(rules(), &today, &["BA1", "BS1", "PM2"]),
            Progress { reviewed: 2, deferred: 0, total: 3 }
        );
        assert_eq!(
            r.progress(rules(), &today, &["PP5", "BP6", "OLD1"]),
            Progress { reviewed: 0, deferred: 0, total: 0 }
        );
        let all = r.total_progress(rules(), &today);
        assert_eq!(all, Progress { reviewed: 2, deferred: 0, total: rules().criteria.len() - 2 });
    }

    #[test]
    fn deferred_criteria_count_as_reviewed_and_are_also_counted_apart() {
        let r = review(&[("BA1", ReviewDecision::DoesNotApply), ("BS1", ReviewDecision::Deferred)]);
        let progress = r.progress(rules(), &[], &["BA1", "BS1", "PM2"]);
        assert_eq!(progress, Progress { reviewed: 2, deferred: 1, total: 3 });
    }

    #[test]
    fn every_built_in_step_sums_to_the_total() {
        let r = CaseReview::default();
        let path = rules().path.as_ref().expect("built-in path");
        let steps: usize = path.iter().map(|s| r.progress(rules(), &[], &s.criteria).total).sum();
        assert_eq!(steps, r.total_progress(rules(), &[]).total);
    }
}
