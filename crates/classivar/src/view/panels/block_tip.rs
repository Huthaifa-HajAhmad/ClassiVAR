//! The card that floats beside a dial slice or a column block while the pointer
//! is over it: the criterion code, its strength, its points and the rule summary.
//! It only repeats what a click opens in the inspector, so the keyboard path
//! (the chips under the picture) loses nothing.
//!
//! What the card says is a plain function (`tip`) with tests.

use classivar_core::Item;

use super::point_axis::points_text;
use super::*;
use crate::look::figure;

/// The words on the card.
#[derive(Debug, PartialEq, Eq)]
struct Tip {
    code: String,
    strength: &'static str,
    /// For example "+4 points" or "−1 point".
    points: String,
    summary: String,
}

fn tip(item: &Item, summary: &str) -> Tip {
    let sign = if item.points > 0 { "+" } else { "" };
    Tip {
        code: item.code.clone(),
        strength: item.strength.label(),
        points: format!("{sign}{} point{}", points_text(item.points), plural(item.points.abs())),
        summary: summary.to_string(),
    }
}

/// The card's width, so a picture can keep it inside its own edges.
pub(super) const TIP_WIDTH: f32 = 264.;

impl Workbench {
    /// The hover card for `code`, if it is a counted criterion of `result`.
    pub(super) fn block_tip(&self, result: &Assessment, code: &str, cx: &App) -> Option<Div> {
        let item = result.items.iter().find(|item| item.counted && item.code == code)?;
        let words = tip(item, &self.rules.criterion(code)?.summary);
        let theme = cx.theme();
        let next = match (self.inspected.as_deref() == Some(code), self.editing == Editing::Today) {
            (false, _) => "Click to see why it counts.",
            (true, true) if self.preview => "Click again to close the preview.",
            (true, true) => "Click again to see the class without it.",
            (true, false) => "Its explanation is open below.",
        };
        let head = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                figure(12.)
                    .px_2()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .bg(direction_colour(item.direction))
                    .text_color(direction_ink())
                    .child(words.code),
            )
            .child(div().flex_1().text_sm().child(words.strength))
            .child(figure(12.).child(words.points));
        Some(
            crate::look::panel(cx)
                .rounded(px(crate::look::CARD_RADIUS))
                .w(px(TIP_WIDTH))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(head)
                .child(div().text_sm().child(words.summary))
                .child(caption(theme.muted_foreground).child(next)),
        )
    }

    /// The pointer entered (`over`) or left the block for `code`. Leaving only
    /// clears the card if it is still this block's: the next block may have taken it.
    pub(super) fn hover_over(&mut self, code: &str, over: bool, cx: &mut Context<Self>) {
        if over {
            self.hover_block(Some(code.to_string()), cx);
        } else if self.hovered.as_deref() == Some(code) {
            self.hover_block(None, cx);
        }
    }

    /// Remember which criterion is under the pointer, and redraw only when it changes.
    pub(super) fn hover_block(&mut self, code: Option<String>, cx: &mut Context<Self>) {
        if self.hovered != code {
            self.hovered = code;
            cx.notify();
        }
    }
}

/// A hover card placed at `(left, top)` inside its picture and drawn above everything else.
pub(super) fn floating(card: Div, (left, top): (f32, f32)) -> AnyElement {
    div().absolute().left(px(left)).top(px(top)).child(deferred(card).with_priority(1)).into_any_element()
}

/// A hover card on a picture whose width is not known in pixels (the track).
/// `at` is a share of the width; the card starts there, or ends there when
/// `ends_there` is true (see `track::tip_anchor`).
pub(super) fn floating_at(card: Div, (at, ends_there): (f32, bool), top: f32) -> AnyElement {
    let shift = if ends_there { -TIP_WIDTH } else { 0. };
    div()
        .absolute()
        .top(px(top))
        .left(relative(at))
        .child(div().absolute().left(px(shift)).child(deferred(card).with_priority(1)))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, RuleSet, Strength};

    use super::{Tip, tip};

    #[test]
    fn the_card_says_code_strength_signed_points_and_the_rule() {
        let rules = RuleSet::builtin().expect("the built-in rules are valid");
        let result =
            rules.assess(&[Applied::new("PS3", Strength::Strong), Applied::new("BP7", Strength::Supporting)]).unwrap();
        let card = |code: &str| {
            let item = result.items.iter().find(|item| item.code == code).unwrap();
            tip(item, &rules.criterion(code).unwrap().summary)
        };
        let ps3 = card("PS3");
        assert_eq!(
            ps3,
            Tip {
                code: "PS3".into(),
                strength: "Strong",
                points: "+4 points".into(),
                summary: rules.criterion("PS3").unwrap().summary.clone(),
            }
        );
        assert_eq!(card("BP7").points, "−1 point", "benign points carry a real minus sign and a singular");
    }
}
