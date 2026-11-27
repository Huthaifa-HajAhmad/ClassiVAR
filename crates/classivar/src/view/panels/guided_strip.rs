//! The review progress under the class: one cell per criterion, how many are
//! reviewed, how many are deferred, and any saved answers these rules set aside.
//! It shows how far the review has got, never that the class is settled.

use super::guided_page::progress_words;
use super::guided_row::state_cell;
use super::*;

impl Workbench {
    /// The strip, when the rules have a guided path or the case holds review answers.
    pub(super) fn review_strip(&self, cx: &mut Context<Self>) -> Option<Div> {
        let set_aside = self.review.set_aside(self.rules);
        if self.rules.path.is_none() && self.review.is_empty() {
            return None;
        }
        let today = to_applied(&self.today);
        let progress = self.review.total_progress(self.rules, &today);
        let cells = self.rules.criteria.iter().map(|c| state_cell(self.review_state(&c.code), c.direction, 10., cx));
        let muted = cx.theme().muted_foreground;
        let mut summary = progress_words(progress);
        if progress.deferred > 0 {
            summary.push_str(". The result can change when deferred criteria are answered.");
        }
        let strip = div()
            .debug_selector(|| "review-strip".to_string())
            .flex()
            .flex_col()
            .gap_1()
            .child(crate::look::eyebrow("Review", cx))
            .child(div().flex().flex_wrap().gap(px(3.)).children(cells))
            .child(caption(muted).child(summary))
            .when(!set_aside.is_empty(), |strip| {
                strip.child(
                    caption(muted)
                        .child(format!("Set aside (not in these rules, not counted): {}", set_aside.join(", "))),
                )
            });
        Some(strip)
    }
}
