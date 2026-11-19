//! The parts of the main screen, drawn from the state kept in `view.rs`.
//!
//! The screen has four areas: a top bar (what this is, which mode, save and open),
//! a case panel (variant, gene, gene sources, earlier records), the criteria list on
//! the left, and the result on the right. The result is the first thing the eye
//! should land on, so it gets the largest type and the strongest colour on screen.
//! Yellow is kept for things that block a result; everything else is quiet.
//!
//! Most functions here return an `AnyElement` (a few small helpers return a `Div`
//! or a `String`). Keeping each one small also keeps the stack use of a debug
//! build low (a single huge `render` once overflowed the stack and closed the app).

use super::*;

mod combining_table;
mod criteria;
mod criterion_row;
mod evidence_stack;
mod gene_cards;
mod helper_bar;
mod helpers;
mod hero;
mod history;
mod lanes;
mod point_axis;
mod records;
mod results;
mod strength_ladder;
mod timeline;
mod top;
mod verdict;
mod what_if;

/// A section heading in the results pane.
fn heading(text: impl Into<SharedString>) -> Div {
    div().text_sm().font_weight(FontWeight::SEMIBOLD).pb_2().child(text.into())
}

impl Workbench {
    /// Today's evidence, scored. An earlier record is scored the way it was made, as in the history table.
    fn score_on_screen(&self) -> Result<Assessment, classivar_core::AssessError> {
        let counting = match self.editing {
            Editing::Today => classivar_core::Counting::Current,
            Editing::Record(_) => classivar_core::Counting::AsRecorded,
        };
        self.rules.assess_as(&to_applied(self.current()), counting)
    }

    /// The records that still lack a class (they are named in the verdict), and the
    /// comparison with today's evidence once every record has one. Records without a
    /// class cannot be compared, so there is no verdict until each has one.
    fn compare_with_records(&self) -> (Vec<String>, Option<Reevaluation>) {
        let incomplete: Vec<String> =
            self.records.iter().filter(|r| r.class.is_none()).map(|r| format!("{} · {}", r.label, r.year)).collect();
        if self.mode != Mode::ReportedVariant || !incomplete.is_empty() {
            return (incomplete, None);
        }
        let submissions: Vec<Submission> = self
            .records
            .iter()
            .filter_map(|r| {
                Some(Submission {
                    label: r.label.clone(),
                    year: r.year,
                    reported_class: r.class?,
                    reported_tier: r.tier.clone(),
                    rule_set: None,
                    criteria: to_applied(&r.evidence),
                })
            })
            .collect();
        let comparison = self.rules.reevaluate(&submissions, &to_applied(&self.today));
        (incomplete, Some(comparison))
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // While a question waits on screen, it owns the keyboard: its window takes
        // the focus and traps Tab (see `confirm_modal`). Enter and Space keep
        // their normal meaning on the answer that has the focus; Escape (and the
        // window's Cancel button) is handled by the global key observer.
        if self.confirm.is_some() && !self.confirm_focus.contains_focused(window, cx) {
            window.focus(&self.confirm_focus, cx);
        } else if self.confirm.is_none() && window.focused(cx).is_none() {
            // With no focus at all Tab has nowhere to start from: a fresh window,
            // or one whose question just closed. The page holds it until a control does.
            window.focus(&self.page_focus, cx);
        }
        let result = match self.score_on_screen() {
            Ok(result) => result,
            Err(error) => return self.error_screen(format!("The evidence on screen cannot be scored: {error}"), cx),
        };
        let (incomplete, reevaluation) = self.compare_with_records();
        let variant = self.variant_name(cx);
        // Worked out once per draw: the Save marker and the file message must agree.
        let unsaved = self.has_unsaved_changes(cx);
        let (background, foreground) = (cx.theme().background, cx.theme().foreground);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .track_focus(&self.page_focus)
            .child(self.top_bar(unsaved, cx))
            .children(self.file_status_banner(unsaved, cx))
            .child(self.case_panel(cx))
            .child(div().flex().flex_1().min_h_0().child(self.criteria_panel(&result, cx)).child(self.results_panel(
                &result,
                reevaluation.as_ref(),
                &incomplete,
                variant.as_deref(),
                cx,
            )))
            .children(self.confirm_modal(cx))
            .into_any_element()
    }
}
