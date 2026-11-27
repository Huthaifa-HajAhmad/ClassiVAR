//! Guided review: the rule file's steps, one question at a time, on the same
//! evidence and review decisions as the free list. Nothing here counts evidence;
//! it only moves between steps and records "does not apply" or "deferred".

use classivar_core::RuleSet;
use classivar_core::case::{ReviewDecision, ReviewState};

use super::{Choice, Editing, Workbench};

/// The criterion applied today that the rules say cannot be used with `code`, if any.
/// A helper reading for `code` then offers a labelled replacement instead of a plain Apply.
pub(super) fn rival<'a>(rules: &'a RuleSet, applied: impl Fn(&str) -> bool, code: &str) -> Option<&'a str> {
    rules.exclusive_pairs.iter().find_map(|pair| {
        let other = if pair.a == code {
            pair.b.as_str()
        } else if pair.b == code {
            pair.a.as_str()
        } else {
            return None;
        };
        applied(other).then_some(other)
    })
}

/// The words for a criterion's review state, as the row and the cells name it.
pub(super) fn state_words(state: ReviewState) -> &'static str {
    match state {
        ReviewState::Applied => "Applied",
        ReviewState::DoesNotApply => "Does not apply",
        ReviewState::Deferred => "Deferred",
        ReviewState::NotYetReviewed => "Not yet reviewed",
        ReviewState::ReferenceOnly => "Reference only",
    }
}

impl Workbench {
    /// Start the guided review at its first step, from the welcome screen.
    pub(super) fn start_guided(&mut self) {
        self.set_mode(super::Mode::NewVariant);
        self.guided = self.rules.path.as_ref().map(|_| 0);
    }

    /// Switch between the guided steps and the free list. Both edit the same evidence.
    pub(super) fn set_guided(&mut self, on: bool) {
        self.close_pages();
        self.editing = Editing::Today;
        self.guided = if on { self.rules.path.as_ref().map(|_| self.guided.unwrap_or(0)) } else { None };
    }

    /// Open step `index` of the path; an index past either end is ignored.
    pub(super) fn go_to_step(&mut self, index: usize) {
        if self.rules.path.as_ref().is_some_and(|path| index < path.len()) {
            self.close_pages();
            self.guided = Some(index);
        }
    }

    /// Record a decision about a criterion, or clear it with `None`. A decision takes the
    /// criterion off today's evidence: "does not apply" and "applied" cannot both hold.
    pub(super) fn decide(&mut self, code: &str, decision: Option<ReviewDecision>) {
        self.remove_criterion(code);
        match decision {
            Some(decision) => self.review.decisions.insert(code.to_string(), decision),
            None => self.review.decisions.remove(code),
        };
    }

    /// The labelled replacement: take `rival` off today's evidence and apply the reading.
    /// The rival returns to "not yet reviewed"; nothing else changes.
    pub(super) fn replace_with_reading(
        &mut self,
        rival: &str,
        reading: &classivar_core::EvidenceReading,
        reason: String,
    ) {
        self.remove_criterion(rival);
        self.review.decisions.remove(rival);
        self.kept.remove(reading.code);
        self.apply_reading(reading, reason);
    }

    /// Keep the applied criterion and set the helper's contradicting proposal aside.
    pub(super) fn keep_rival(&mut self, proposal: &str) {
        self.kept.insert(proposal.to_string());
    }

    /// Count a criterion on today's evidence. Counting answers its review, so a "does
    /// not apply" or "deferred" decision for it ends here. Every way of adding today's
    /// evidence goes through this, which keeps the screen and the saved file in step.
    pub(super) fn count_today(&mut self, code: String, choice: Choice) {
        self.review.decisions.remove(&code);
        self.today.insert(code, choice);
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;
    use classivar_core::case::ReviewState;

    use super::{rival, state_words};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    #[test]
    fn a_reading_that_contradicts_applied_evidence_names_its_rival() {
        assert_eq!(rival(rules(), |code| code == "PM2", "BS1"), Some("PM2"));
        assert_eq!(rival(rules(), |code| code == "PM2", "BA1"), Some("PM2"));
        assert_eq!(rival(rules(), |code| code == "BP4", "PP3"), Some("BP4"));
        assert_eq!(rival(rules(), |_| false, "BS1"), None, "nothing applied, nothing to replace");
        assert_eq!(rival(rules(), |code| code == "BA1", "BS1"), None, "BA1 and BS1 may stand together");
    }

    #[test]
    fn every_state_has_its_own_words() {
        use ReviewState::*;
        let words: Vec<&str> =
            [Applied, DoesNotApply, Deferred, NotYetReviewed, ReferenceOnly].into_iter().map(state_words).collect();
        assert_eq!(words, ["Applied", "Does not apply", "Deferred", "Not yet reviewed", "Reference only"]);
    }
}
