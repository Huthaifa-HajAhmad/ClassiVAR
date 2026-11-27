//! Guided review: the rule file's steps, one question at a time, on the same
//! evidence and review decisions as the free list. Nothing here counts evidence;
//! it only moves between steps and records "does not apply" or "deferred".

use classivar_core::RuleSet;
use classivar_core::case::{ReviewDecision, ReviewState};

use super::{Choice, Editing, Workbench, point, px};

/// Every criterion applied today that the rules say cannot be used with `code`, in
/// rule order. A helper reading for `code` then offers a labelled replacement for all
/// of them instead of a plain Apply.
pub(super) fn rivals<'a>(rules: &'a RuleSet, applied: impl Fn(&str) -> bool, code: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    for pair in &rules.exclusive_pairs {
        let other = if pair.a == code {
            pair.b.as_str()
        } else if pair.b == code {
            pair.a.as_str()
        } else {
            continue;
        };
        if applied(other) && !found.contains(&other) {
            found.push(other);
        }
    }
    found
}

/// Join one or more criterion codes in plain prose: "PM2", "BA1 and BS1", "A, B and C".
pub(super) fn join_codes(codes: &[&str]) -> String {
    match codes {
        [] => String::new(),
        [only] => (*only).to_string(),
        [a, b] => format!("{a} and {b}"),
        [most @ .., last] => format!("{} and {last}", most.join(", ")),
    }
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
            self.handles.criteria_scroll.set_offset(point(px(0.), px(0.)));
        }
    }

    /// Record a decision about a criterion, or clear it with `None`. A decision takes the
    /// criterion off today's evidence: "does not apply" and "applied" cannot both hold.
    pub(super) fn decide(&mut self, code: &str, decision: Option<ReviewDecision>) {
        self.remove_criterion(code);
        self.kept.remove(code);
        match decision {
            Some(decision) => self.review.decisions.insert(code.to_string(), decision),
            None => self.review.decisions.remove(code),
        };
    }

    /// Record or clear an answer to an optional context fact. Facts add hints only
    /// and never count as evidence.
    pub(super) fn set_fact(&mut self, id: &str, answer: Option<String>) {
        match answer {
            Some(answer) => self.review.facts.insert(id.to_string(), answer),
            None => self.review.facts.remove(id),
        };
    }

    /// The labelled replacement: take every contradicting `rival` off today's evidence
    /// and apply the reading. Replaced rivals return to "not yet reviewed"; nothing else changes.
    pub(super) fn replace_with_reading(
        &mut self,
        rivals: &[&str],
        reading: &classivar_core::EvidenceReading,
        reason: String,
    ) {
        for rival in rivals {
            self.remove_criterion(rival);
            self.review.decisions.remove(*rival);
        }
        self.kept.remove(reading.code);
        self.apply_reading(reading, reason);
    }

    /// Keep the applied criterion (or criteria) and set the helper's contradicting proposal aside.
    pub(super) fn keep_rival(&mut self, proposal: &str) {
        self.kept.insert(proposal.to_string());
    }

    /// Show a previously set-aside contradicting helper proposal again.
    pub(super) fn reconsider_proposal(&mut self, proposal: &str) {
        self.kept.remove(proposal);
    }

    /// Forget kept-aside proposals whose rival changed, or the criterion itself.
    pub(super) fn clear_kept_for(&mut self, code: &str) {
        self.kept.remove(code);
        for pair in &self.rules.exclusive_pairs {
            if pair.a == code {
                self.kept.remove(pair.b.as_str());
            } else if pair.b == code {
                self.kept.remove(pair.a.as_str());
            }
        }
    }

    /// Count a criterion on today's evidence. Counting answers its review, so a "does
    /// not apply" or "deferred" decision for it ends here. Every way of adding today's
    /// evidence goes through this, which keeps the screen and the saved file in step.
    pub(super) fn count_today(&mut self, code: String, choice: Choice) {
        self.preview = false;
        self.clear_kept_for(&code);
        self.review.decisions.remove(&code);
        self.today.insert(code, choice);
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;
    use classivar_core::case::ReviewState;

    use super::{join_codes, rivals, state_words};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    #[test]
    fn a_reading_that_contradicts_applied_evidence_names_every_rival() {
        assert_eq!(rivals(rules(), |code| code == "PM2", "BS1"), vec!["PM2"]);
        assert_eq!(rivals(rules(), |code| code == "PM2", "BA1"), vec!["PM2"]);
        assert_eq!(rivals(rules(), |code| code == "BA1" || code == "BS1", "PM2"), vec!["BA1", "BS1"]);
        assert_eq!(rivals(rules(), |code| code == "BP4", "PP3"), vec!["BP4"]);
        assert!(rivals(rules(), |_| false, "BS1").is_empty(), "nothing applied, nothing to replace");
        assert!(rivals(rules(), |code| code == "BA1", "BS1").is_empty(), "BA1 and BS1 may stand together");
    }

    #[test]
    fn rival_names_read_naturally_for_one_two_or_more_codes() {
        assert_eq!(join_codes(&[]), "");
        assert_eq!(join_codes(&["PM2"]), "PM2");
        assert_eq!(join_codes(&["BA1", "BS1"]), "BA1 and BS1");
        assert_eq!(join_codes(&["BA1", "BS1", "BS2"]), "BA1, BS1 and BS2");
    }

    #[test]
    fn every_state_has_its_own_words() {
        use ReviewState::*;
        let words: Vec<&str> =
            [Applied, DoesNotApply, Deferred, NotYetReviewed, ReferenceOnly].into_iter().map(state_words).collect();
        assert_eq!(words, ["Applied", "Does not apply", "Deferred", "Not yet reviewed", "Reference only"]);
    }
}
