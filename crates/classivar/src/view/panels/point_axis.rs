//! The points axis that the pictures share: the track, the column, the "what if"
//! moves under "What would change it" and the records timeline. Every one places
//! points with `fraction`, so a class limit sits at the same share of the width
//! in each. When the bars run past the scale the axis is widened (see `widened`),
//! and the point value at each limit is printed under the zones.
//!
//! A bar can be filled (evidence that is applied) or outlined (a move that is only
//! possible). That is the one visual difference, and each bar also says it in words.

use classivar_core::Direction;

use super::*;
use crate::view::describe::Scale;

/// A number of points with a real minus sign when it is negative ("−1", not "-1"),
/// the same way the rest of the screen writes benign points.
pub(super) fn points_text(points: i32) -> String {
    if points < 0 { format!("−{}", points.unsigned_abs()) } else { points.to_string() }
}

/// A signed number of points: "+4", "−4" or "0".
pub(super) fn signed(points: i32) -> String {
    if points > 0 { format!("+{points}") } else { points_text(points) }
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

/// A thin line at every class limit, across a fluid row, so a bar can be read against the limits.
pub(super) fn limit_lines(scale: &Scale, colour: Hsla) -> Vec<Div> {
    scale
        .zones
        .iter()
        .skip(1)
        .map(|(_, from, _)| {
            div().absolute().top_0().h_full().left(relative(fraction(*from, scale))).w(px(1.)).bg(colour)
        })
        .collect()
}

/// The class zones as one rounded strip across a fluid row, on the panel colour
/// so a pale class still shows on a card. The class shown is in full colour with
/// an ink edge (like the dial's rim); the others are quieter.
pub(super) fn class_strip(scale: &Scale, class: Class, (paper, ink): (Hsla, Hsla), height: f32) -> Div {
    let dark = paper.l < 0.5;
    let mut strip = div().relative().w_full().h(px(height)).rounded_full().overflow_hidden().bg(paper);
    for (zone, from, to) in &scale.zones {
        let (left, width) = reach(*from, *to, scale);
        let active = class == *zone;
        strip = strip.child(
            div()
                .absolute()
                .top_0()
                .h_full()
                .left(relative(left))
                .w(relative(width))
                .bg(Rgba { a: if active { 1.0 } else { 0.4 }, ..class_fill(*zone, dark) })
                .when(active, |zone| zone.border_2().border_color(ink.opacity(crate::look::EDGE_STRENGTH))),
        );
    }
    strip
}

/// A bar from `from` to `to` across a fluid row, `height` pixels tall, with a
/// dot where it ends. Filled for evidence that is applied; a dashed outline on a
/// light tint for a move that is only possible.
pub(super) fn fluid_bar(bar: &Bar, scale: &Scale, (paper, muted): (Hsla, Hsla), height: f32) -> Div {
    let colour: Hsla = if bar.neutral { muted } else { direction_colour(bar.direction).into() };
    let (left, width) = reach(bar.from, bar.to, scale);
    let span = div()
        .absolute()
        .top_0()
        .h_full()
        .left(relative(left))
        .w(relative(width))
        .min_w(px(3.))
        .rounded_full()
        .when(!bar.hollow, |fill| fill.bg(colour))
        .when(bar.hollow, |outline| outline.bg(colour.opacity(0.12)).border_2().border_dashed().border_color(colour));
    let dot = height + 6.;
    let end = div()
        .absolute()
        .top(px(-3.))
        .left(relative(fraction(bar.to, scale)))
        .ml(px(-dot / 2.))
        .size(px(dot))
        .rounded_full()
        .bg(colour)
        .border_2()
        .border_color(paper);
    div().relative().w_full().h(px(height)).child(span).child(end)
}

/// The span between two totals on a fluid axis, as its left edge and its width
/// (shares of the axis). Either total may be the larger.
pub(super) fn reach(from: i32, to: i32, scale: &Scale) -> (f32, f32) {
    let left = fraction(from.min(to), scale);
    (left, fraction(from.max(to), scale) - left)
}

/// Where `points` sits across a fluid axis (the evidence track): 0.0 at the left edge, 1.0 at the right.
pub(super) fn fraction(points: i32, scale: &Scale) -> f32 {
    let span = (scale.max - scale.min).max(1) as f32;
    ((points - scale.min) as f32 / span).clamp(0.0, 1.0)
}

/// Whether a zone name fits in a zone that takes `share` of a picture `width` pixels wide
/// (about 6.5 px per letter of 12 px text, plus a little room at each side).
pub(super) fn name_fits(name: &str, share: f32, width: f32) -> bool {
    share * width >= name.chars().count() as f32 * 6.5 + 12.
}

/// The class names over the zones; the class shown is in full ink. With the picture's `width` in pixels, each zone
/// gets its full name where it fits and its short one (B, LB, VUS, LP, P) where it
/// does not; without it, every zone gets the short name in the monospaced face.
pub(super) fn zone_labels(scale: &Scale, class: Class, (ink, muted): (Hsla, Hsla), width: Option<f32>) -> Div {
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
                .when(width.is_none(), |name| name.font_family(crate::look::MONO).text_size(px(11.)))
                .text_color(if active { ink } else { muted })
                .font_weight(if active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .child(match width {
                    Some(width) if name_fits(zone.label(), right - left, width) => zone.label(),
                    _ => zone.short(),
                }),
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

    use super::{fraction, name_fits, points_text, reach, scale_for};

    #[test]
    fn a_reach_covers_its_points_whichever_way_it_runs() {
        let scale = scale_for(RuleSet::builtin().expect("the built-in rules are valid"));
        let (left, width) = reach(5, -1, &scale);
        assert_eq!(reach(-1, 5, &scale), (left, width), "a move down covers the same span as one up");
        assert!((left - fraction(-1, &scale)).abs() < 1e-6);
        assert!((left + width - fraction(5, &scale)).abs() < 1e-6);
    }

    #[test]
    fn a_total_is_written_the_same_way_everywhere_on_the_axis() {
        assert_eq!(points_text(0), "0");
        assert_eq!(points_text(4), "4");
        assert_eq!(points_text(-6), "−6", "a real minus sign, not a hyphen");
    }

    #[test]
    fn a_full_zone_name_is_used_only_where_it_fits() {
        let name = "Uncertain significance (VUS)";
        assert!(!name_fits(name, 0.3, 500.), "150 px is too narrow for the long name");
        assert!(name_fits(name, 0.3, 1000.), "300 px holds it");
        assert!(name_fits("Benign", 0.1, 600.));
        assert!(!name_fits("Benign", 0.05, 600.), "30 px is too narrow even for a short name");
    }
}
