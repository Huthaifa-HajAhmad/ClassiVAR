//! The guided review in the editor sheet: the list of steps with their progress, then
//! the open step: its question, the helpers for its criteria, one row per criterion,
//! and Previous and Next. It edits the same evidence as the free list.

use classivar_core::Progress;
use classivar_core::rules::ReviewStep;

use super::focus::tagged;
use super::guided_row::state_cell;
use super::*;

impl Workbench {
    /// The steps, the optional context questions, and the open step, for the sheet's scrolling list.
    pub(super) fn guided_body(&self, path: &'static [ReviewStep], index: usize, cx: &mut Context<Self>) -> Div {
        let step = &path[index.min(path.len() - 1)];
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.guided_steps(path, index, cx))
            .children(self.guided_facts_card(cx))
            .child(self.guided_step(step, index, path.len(), cx))
    }

    /// One small button per step, in a row that wraps: its number and how far it is
    /// reviewed. Its name and progress are in its accessible name and on the step page.
    fn guided_steps(&self, path: &'static [ReviewStep], open: usize, cx: &mut Context<Self>) -> Div {
        let total = self.review.total_progress(self.rules, &to_applied(&self.today));
        let muted = cx.theme().muted_foreground;
        let mut list =
            div().flex().flex_wrap().items_center().gap_1().child(
                div().w_full().child(caption(muted).child(format!("Guided review · {}", progress_words(total)))),
            );
        for (i, step) in path.iter().enumerate() {
            list = list.child(self.guided_step_button(step, i, i == open, cx));
        }
        list
    }

    fn guided_step_button(&self, step: &ReviewStep, i: usize, open: bool, cx: &mut Context<Self>) -> Div {
        let progress = self.review.progress(self.rules, &to_applied(&self.today), &step.criteria);
        let hint = progress_words(progress);
        let cells = step
            .criteria
            .iter()
            .filter_map(|code| self.rules.criterion(code))
            .map(|c| state_cell(self.review_state(&c.code), c.direction, 6., cx));
        let name = step_name(&step.id);
        let muted = cx.theme().muted_foreground;
        let button = crate::look::tab(format!("guided-step-{}", step.id), open, cx)
            .accessibility_label(format!("Step {}, {name}: {hint}", i + 1))
            .tooltip(format!("{name}: {hint}"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div().text_color(if open { cx.theme().foreground } else { muted }).child(format!("{}", i + 1)),
                    )
                    .child(div().flex().gap(px(2.)).children(cells)),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.go_to_step(i);
                cx.notify();
            }));
        let selector = format!("guided-step-{}", step.id);
        div().debug_selector(move || selector).child(button)
    }

    /// The open step: its question, its helpers, its criteria, and Previous and Next.
    fn guided_step(&self, step: &'static ReviewStep, index: usize, steps: usize, cx: &mut Context<Self>) -> Div {
        let border = cx.theme().border;
        let mut rows = div().flex().flex_col().gap_2();
        for c in step.criteria.iter().filter_map(|code| self.rules.criterion(code)) {
            rows = rows.child(self.guided_row(c, cx));
        }
        div()
            .debug_selector(|| "guided-step-page".to_string())
            .flex()
            .flex_col()
            .gap_3()
            .pt_3()
            .border_t_1()
            .border_color(border)
            .child(crate::look::eyebrow(format!("Step {} of {steps} · {}", index + 1, step_name(&step.id)), cx))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(step.question.clone()))
            .children(self.step_hints(step, cx))
            .children(self.helper_cards_for(&step.criteria, cx))
            .child(rows)
            .child(self.guided_nav(index, steps, cx))
    }

    fn guided_nav(&self, index: usize, steps: usize, cx: &mut Context<Self>) -> Div {
        let previous = Button::new("guided-previous")
            .label("Previous")
            .icon(Icon::new(IconName::ChevronLeft))
            .accessibility_label("Previous step")
            .track_focus(&self.handles.step_previous)
            .small()
            .outline()
            .disabled(index == 0)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.go_to_step(index.saturating_sub(1));
                cx.notify();
            }));
        let next = Button::new("guided-next")
            .label("Next")
            .icon(Icon::new(IconName::ChevronRight))
            .accessibility_label("Next step")
            .track_focus(&self.handles.step_next)
            .small()
            .primary()
            .disabled(index + 1 >= steps)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.go_to_step(index + 1);
                cx.notify();
            }));
        let (previous, next) = (tagged("guided-previous".into(), previous), tagged("guided-next".into(), next));
        div().flex().gap_2().pt_2().child(previous).child(div().flex_1()).child(next)
    }
}

/// A step's name from its id: "variant-type" reads "Variant type".
pub(super) fn step_name(id: &str) -> String {
    let words = id.replace('-', " ");
    let mut chars = words.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// "2 of 3 reviewed", with the deferred ones named so that a full count is not read
/// as a finished review; "Reference only" for a step with nothing to review.
pub(super) fn progress_words(progress: Progress) -> String {
    let Progress { reviewed, deferred, total } = progress;
    match (total, deferred) {
        (0, _) => "Reference only".to_string(),
        (_, 0) => format!("{reviewed} of {total} reviewed"),
        _ => format!("{reviewed} of {total} reviewed, {deferred} deferred"),
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::Progress;

    use super::{progress_words, step_name};

    #[test]
    fn a_step_is_named_from_its_id_and_counts_what_is_reviewed() {
        assert_eq!(step_name("variant-type"), "Variant type");
        assert_eq!(step_name("de-novo"), "De novo");
        let words = |reviewed, deferred, total| progress_words(Progress { reviewed, deferred, total });
        assert_eq!(words(2, 0, 3), "2 of 3 reviewed");
        assert_eq!(words(3, 1, 3), "3 of 3 reviewed, 1 deferred");
        assert_eq!(words(0, 0, 0), "Reference only");
    }
}
