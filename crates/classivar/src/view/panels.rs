//! The parts of the main screen, drawn from the state kept in `view.rs`.
//!
//! The screen has four areas: a top bar (what this is, which mode, save and open),
//! a case panel (variant, gene, gene sources, earlier records), the criteria list on
//! the right, and the result on the left. The result is the first thing the eye
//! should land on, so it gets the largest type and the strongest colour on screen.
//! Yellow is kept for things that block a result; everything else is quiet.
//!
//! Most functions here return an `AnyElement` (a few small helpers return a `Div`
//! or a `String`). Keeping each one small also keeps the stack use of a debug
//! build low (a single huge `render` once overflowed the stack and closed the app).

use super::*;

mod block_tip;
mod column;
mod combining_table;
mod criteria;
mod criterion_row;
mod dial;
mod dial_geometry;
mod evidence_stack;
mod frame;
mod gene_cards;
mod helper_bar;
mod helpers;
mod hero;
mod history;
mod insights;
mod inspector;
mod interactions;
mod lanes;
mod makeup;
mod picture;
mod point_axis;
mod rail;
mod record_fields;
mod records;
mod report_focus;
mod results;
mod strength_ladder;
mod suggest;
mod timeline;
mod top;
mod track;
mod verdict;
mod welcome;
mod welcome_more;
mod what_if;

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
    /// comparison for complete records. An unfinished report still blocks the overall verdict.
    pub(super) fn compare_with_records(&self) -> (Vec<String>, Option<Reevaluation>) {
        let incomplete: Vec<String> =
            self.records.iter().filter(|r| r.class.is_none()).map(|r| format!("{} · {}", r.label, r.year)).collect();
        if self.mode != Mode::ReportedVariant {
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

impl Workbench {
    fn workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.welcome && !self.has_case_data(cx) {
            return self.welcome_screen(cx);
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
        let (rail, sheet) = self.measure_stage(window);
        let middle = div().flex_1().min_w_0().h_full().flex().flex_col().child(self.results_panel(
            &result,
            reevaluation.as_ref(),
            &incomplete,
            variant.as_deref(),
            cx,
        ));

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .child(self.top_bar(unsaved, cx))
            .children(self.file_status_banner(unsaved, cx))
            .child(self.stage_row(&result, middle, (rail, sheet), variant.as_deref(), cx))
            .children(self.confirm_modal(cx))
            .into_any_element()
    }
}

impl Workbench {
    /// While a question waits on screen, it owns the keyboard: its window takes
    /// the focus and traps Tab (see `confirm_modal`). Enter and Space keep their
    /// normal meaning on the answer that has the focus; Escape (and the window's
    /// Cancel button) is handled by the global key observer. With no focus at all
    /// Tab has nowhere to start from (a fresh window, or one whose question just
    /// closed), so the page holds it until a control does.
    fn settle_focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm.is_some() && !self.confirm_focus.contains_focused(window, cx) {
            window.focus(&self.confirm_focus, cx);
        } else if self.confirm.is_none() && window.focused(cx).is_none() {
            window.focus(&self.page_focus, cx);
        }
    }

    /// Note how much room the picture and the rail have in a window this size.
    /// Returns whether the rail shows and how wide the editor sheet is (0 when hidden).
    fn measure_stage(&self, window: &Window) -> (bool, f32) {
        let (width, height) = (f32::from(window.viewport_size().width), f32::from(window.viewport_size().height));
        let (rail, sheet) = (width >= rail::RAIL_FROM, if self.reading { 0. } else { sheet_width(width) });
        self.rail_shown.set(rail);
        self.stage_width.set(stage_width(width, rail, sheet));
        // The top bar, the banner, the stage toolbar and the class readout under the dial take about 330 px.
        self.stage_height.set(height - 330.);
        (rail, sheet)
    }

    /// The row under the top bar: the rail, the middle panel and the editor sheet.
    fn stage_row(
        &self,
        result: &Assessment,
        middle: Div,
        (rail, sheet): (bool, f32),
        variant: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .gap_4()
            .px_4()
            .pb_4()
            .when(rail, |body| body.child(self.rail(variant, cx)))
            .child(middle)
            .when(!self.reading, |body| body.child(self.criteria_panel(result, sheet, cx)))
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.settle_focus(window, cx);
        let frame = self.frame(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.page_focus)
            .child(frame)
            .child(div().flex_1().min_h_0().child(self.workspace(window, cx)))
    }
}

/// A section of the explanation inside the stage: its label and title, then its
/// content (usually a card). Space separates sections, not lines.
fn explanation_section(label: &str, title: &str, cx: &Context<Workbench>) -> Div {
    div().pt_4().flex().flex_col().gap_3().child(crate::look::section_head(label, title, cx))
}

/// The width of the criteria sheet at the right for a window `window` pixels wide.
fn sheet_width(window: f32) -> f32 {
    (window * 0.3).clamp(340., 420.)
}

/// The room left for the picture inside the middle panel.
fn stage_width(window: f32, rail: bool, sheet: f32) -> f32 {
    let rail = if rail { rail::RAIL_WIDTH + 16. } else { 0. };
    let sheet = if sheet > 0. { sheet + 16. } else { 0. };
    // The page margins (32) and the stage's own padding (64).
    (window - 32. - rail - sheet - 64.).max(240.)
}
