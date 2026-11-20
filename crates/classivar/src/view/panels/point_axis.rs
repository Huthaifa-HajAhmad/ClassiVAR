//! Bars drawn on the points scale's own axis. "How the points add up" and the
//! "what if" lines under "What would change it" and the records timeline all use
//! them, so a point is the same width in each, and the class limits sit at the same point values as on
//! the scale above. When the bars run past the scale the axis is widened (see
//! `widened`), so the picture is wider than the scale and the class limits are
//! read from the point values printed under the zones rather than from their
//! position.
//!
//! A bar can be filled (evidence that is applied) or outlined (a move that is only
//! possible). That is the one visual difference, and each bar also says it in words.

use classivar_core::Direction;

use super::*;
use crate::view::describe::Scale;

/// Width of the column with a bar's label and note, and of the one with where it ends.
const LABEL_WIDTH: f32 = 96.0;
const END_WIDTH: f32 = 72.0;

/// A number of points with a real minus sign when it is negative ("−1", not "-1"),
/// the same way the rest of the screen writes benign points.
pub(super) fn points_text(points: i32) -> String {
    if points < 0 { format!("−{}", points.unsigned_abs()) } else { points.to_string() }
}

/// One bar from `from` points to `to` points (either may be the larger).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Bar {
    /// Bold, first line of the label column, for example "PS1 +4" or "Up +2".
    pub label: String,
    /// Quiet second line, for example "Strong" or "to LP".
    pub note: String,
    pub direction: Direction,
    pub from: i32,
    pub to: i32,
    /// At the right of the bar, for example "= 5".
    pub end: String,
    /// Outlined instead of filled: shows a move that is possible, not evidence that is applied.
    pub hollow: bool,
    /// Drawn in the quiet text colour instead of a direction colour: for a total of
    /// zero, which favours neither side.
    pub neutral: bool,
}

/// The points scale, made wider where the bars go past it. A total of 19 points
/// is past the scale's top at 12; cutting those bars would hide how much each
/// one added. The first and last class zones stretch to the new edges, so the
/// limits between classes stay where the rules put them.
pub(super) fn widened(scale: &Scale, bars: &[Bar]) -> Scale {
    let reached = bars.iter().flat_map(|bar| [bar.from, bar.to]);
    let min = reached.clone().min().map_or(scale.min, |low| low.min(scale.min));
    let max = reached.max().map_or(scale.max, |high| high.max(scale.max));
    let mut zones = scale.zones.clone();
    if let Some(first) = zones.first_mut() {
        first.1 = min;
    }
    if let Some(last) = zones.last_mut() {
        last.2 = max;
    }
    Scale { min, max, zones }
}

/// The class zones above the bars, on the same axis as the points scale.
fn zone_header(scale: &Scale, border: Hsla, muted: Hsla) -> Div {
    let mut zones = div().flex().pl(px(LABEL_WIDTH));
    for (class, from, to) in &scale.zones {
        zones = zones.child(
            div()
                .w(px((to - from) as f32 * SCALE_UNIT))
                .flex()
                .justify_center()
                .overflow_hidden()
                .border_l_1()
                .border_color(border)
                .child(caption(muted).child(class.short())),
        );
    }
    zones
}

/// The point value at each class limit, under the zones, so a limit can be read
/// off the picture the way it can be on the scale above. The first limit is left
/// out: its label would hang off the left edge of the axis.
fn tick_values(scale: &Scale) -> Vec<i32> {
    scale.zones.iter().skip(1).map(|(_, from, _)| *from).collect()
}

fn ticks(scale: &Scale, muted: Hsla) -> Div {
    let x_of = |points: i32| (points - scale.min) as f32 * SCALE_UNIT;
    let mut marks = div().relative().w(px(x_of(scale.max))).h(px(16.));
    for value in tick_values(scale) {
        marks = marks.child(
            div()
                .absolute()
                .left(px(x_of(value) - 10.))
                .w(px(20.))
                .flex()
                .justify_center()
                .text_xs()
                .text_color(muted)
                .child(value.to_string()),
        );
    }
    div().pl(px(LABEL_WIDTH)).child(marks)
}

/// One bar with its label, its note and where it ends. Thin lines mark the class
/// limits so the bar can be read against them.
pub(super) fn bar_row(bar: &Bar, scale: &Scale, border: Hsla, muted: Hsla, label: impl IntoElement) -> Div {
    let x_of = |points: i32| (points - scale.min) as f32 * SCALE_UNIT;
    let (left, right) = (bar.from.min(bar.to), bar.from.max(bar.to));
    let mut track = div().relative().flex_none().w(px(x_of(scale.max))).h(px(36.));
    for edge in scale.zones.iter().map(|(_, from, _)| *from) {
        track = track.child(div().absolute().top(px(0.)).left(px(x_of(edge))).w(px(1.)).h_full().bg(border));
    }
    let colour: Rgba = if bar.neutral { muted.into() } else { direction_colour(bar.direction) };
    track = track.child(
        div()
            .absolute()
            .top(px(9.))
            .left(px(x_of(left)))
            .w(px(((right - left) as f32 * SCALE_UNIT).max(3.)))
            .h(px(18.))
            .rounded_sm()
            .when(!bar.hollow, |fill| fill.bg(colour))
            .when(bar.hollow, |outline| outline.border_2().border_color(colour)),
    );
    div()
        .flex()
        .items_center()
        .child(
            div()
                .w(px(LABEL_WIDTH))
                .flex_none()
                .flex()
                .flex_col()
                .child(label)
                .child(caption(muted).child(bar.note.clone())),
        )
        .child(track)
        .child(div().w(px(END_WIDTH)).flex_none().pl_2().child(caption(muted).child(bar.end.clone())))
}

/// The zones, the point value at each limit, and one row per bar, in a box that
/// scrolls sideways when the window is narrow.
pub(super) fn axis(id: &'static str, scale: &Scale, bars: &[Bar], border: Hsla, muted: Hsla) -> AnyElement {
    axis_rows(
        id,
        scale,
        border,
        muted,
        bars.iter().map(|bar| {
            bar_row(bar, scale, border, muted, div().text_xs().font_weight(FontWeight::BOLD).child(bar.label.clone()))
        }),
    )
}

pub(super) fn axis_rows(
    id: &'static str,
    scale: &Scale,
    border: Hsla,
    muted: Hsla,
    bars: impl IntoIterator<Item = impl IntoElement>,
) -> AnyElement {
    let mut rows = div().flex().flex_col().child(zone_header(scale, border, muted)).child(ticks(scale, muted));
    for bar in bars {
        rows = rows.child(bar);
    }
    div().id(id).overflow_x_scroll().child(rows).into_any_element()
}

/// Where `points` sits across a fluid axis (the evidence track): 0.0 at the left edge, 1.0 at the right.
pub(super) fn fraction(points: i32, scale: &Scale) -> f32 {
    let span = (scale.max - scale.min).max(1) as f32;
    ((points - scale.min) as f32 / span).clamp(0.0, 1.0)
}

/// The class names above their zones; the class shown is in full ink.
pub(super) fn zone_labels(scale: &Scale, class: Class, ink: Hsla, muted: Hsla) -> Div {
    let mut row = div().relative().w_full().h(px(18.));
    for (zone, from, to) in &scale.zones {
        let (left, right) = (fraction(*from, scale), fraction(*to, scale));
        let active = *zone == class;
        row = row.child(
            div()
                .absolute()
                .left(relative(left))
                .w(relative(right - left))
                .flex()
                .justify_center()
                .overflow_hidden()
                .text_xs()
                .text_color(if active { ink } else { muted })
                .font_weight(if active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .child(if right - left > 0.12 { zone.label() } else { zone.short() }),
        );
    }
    row
}

/// The point value at each class limit.
pub(super) fn tick_row(scale: &Scale, muted: Hsla) -> Div {
    let mut row = div().relative().w_full().h(px(16.));
    for (_, from, _) in scale.zones.iter().skip(1) {
        row = row.child(
            div()
                .absolute()
                .left(relative(fraction(*from, scale)))
                .ml(px(-12.))
                .w(px(24.))
                .flex()
                .justify_center()
                .text_xs()
                .text_color(muted)
                .child(points_text(*from)),
        );
    }
    row
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::RuleSet;

    use super::{points_text, scale_for, tick_values};

    #[test]
    fn the_ticks_name_every_class_limit_but_the_first() {
        let scale = scale_for(RuleSet::builtin().expect("the built-in rules are valid"));
        let limits: Vec<i32> = scale.zones.iter().map(|(_, from, _)| *from).collect();
        let ticks = tick_values(&scale);
        assert!(!ticks.is_empty());
        assert_eq!(ticks, limits[1..], "every limit but the one off the left edge");
    }

    #[test]
    fn a_total_is_written_the_same_way_everywhere_on_the_axis() {
        assert_eq!(points_text(0), "0");
        assert_eq!(points_text(4), "4");
        assert_eq!(points_text(-6), "−6", "a real minus sign, not a hyphen");
    }
}
