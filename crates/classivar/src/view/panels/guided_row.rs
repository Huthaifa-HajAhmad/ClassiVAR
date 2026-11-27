//! A criterion on a guided step: the same row as the free list, its review state in
//! words, and Does not apply, Defer and Clear. Also the small progress cell, one per
//! criterion, that the step list and the summary strip share.

use classivar_core::case::{ReviewDecision, ReviewState};

use super::*;
use crate::view::guided::state_words;

impl Workbench {
    /// The review state of `code` today.
    pub(in crate::view) fn review_state(&self, code: &str) -> ReviewState {
        let today = to_applied(&self.today);
        self.review.state(self.rules, &today, code)
    }

    /// One criterion on a step: its row, then its state and the decision buttons.
    pub(super) fn guided_row(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Div {
        let state = self.review_state(&c.code);
        let muted = cx.theme().muted_foreground;
        let mut actions = div().flex().flex_wrap().items_center().gap_2().pl(px(74.)).pr_2();
        actions =
            actions.child(state_cell(state, c.direction, 10., cx)).child(caption(muted).child(state_words(state)));
        if state != ReviewState::ReferenceOnly {
            actions = actions.child(div().flex_1());
            for (label, decision) in
                [("Does not apply", ReviewDecision::DoesNotApply), ("Defer", ReviewDecision::Deferred)]
            {
                let chosen = self.review.decisions.get(&c.code) == Some(&decision);
                actions = actions.when(!chosen, |row| row.child(self.decision_button(c, label, Some(decision), cx)));
            }
            actions = actions.when(state != ReviewState::NotYetReviewed, |row| {
                row.child(self.decision_button(c, "Clear", None, cx))
            });
        }
        div().flex().flex_col().gap_1().child(self.criterion_row(c, cx)).child(actions)
    }

    fn decision_button(
        &self,
        c: &'static Criterion,
        label: &'static str,
        decision: Option<ReviewDecision>,
        cx: &mut Context<Self>,
    ) -> Div {
        let code = c.code.clone();
        let slug = label.to_lowercase().replace(' ', "-");
        let what = match decision {
            Some(ReviewDecision::DoesNotApply) => format!("Mark {} as does not apply", c.code),
            Some(ReviewDecision::Deferred) => format!("Mark {} as deferred", c.code),
            None => format!("Clear the review of {}: it returns to not yet reviewed", c.code),
        };
        let id = format!("guided-{slug}-{}", c.code);
        let button = Button::new(SharedString::from(id.clone()))
            .label(label)
            .accessibility_label(what)
            .xsmall()
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.decide(&code, decision);
                cx.notify();
            }));
        super::focus::tagged(id, button)
    }
}

/// A small square for one criterion's review state. Applied cells take the
/// direction's colour; the others use theme tokens and the checked warning fill.
/// The state is always also given in words next to it or in an accessible name.
pub(super) fn state_cell(state: ReviewState, direction: Direction, size: f32, cx: &App) -> Div {
    let theme = cx.theme();
    let cell = div().size(px(size)).flex_none().rounded(px(2.)).border_1();
    match state {
        ReviewState::Applied => cell.bg(direction_colour(direction)).border_color(direction_colour(direction)),
        ReviewState::DoesNotApply => cell.bg(theme.muted_foreground).border_color(theme.muted_foreground),
        ReviewState::Deferred => cell.bg(warning_fill()).border_color(warning_ink()),
        ReviewState::NotYetReviewed => cell.border_color(theme.muted_foreground),
        ReviewState::ReferenceOnly => cell.bg(theme.muted).border_color(theme.border),
    }
}
