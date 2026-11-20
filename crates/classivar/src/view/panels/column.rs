//! The column: one of the three pictures of the class (see `settings::Picture`).
//!
//! A tall capsule of class bands, most pathogenic at the top. Each counted
//! criterion is a block stacked from zero (pathogenic blocks climb, benign
//! blocks come back down), and a line marks the total. Earlier reports whose
//! class is shown stand beside it as slim columns, each with a dot at its own
//! total, joined to today by a dotted line, so the picture shows the class move
//! over time. Clicking a block selects it; clicking it again previews the class
//! without it.
//!
//! Positions are shares of the column height, from `point_axis::fraction`, so
//! the column, the track and the dial agree on every number.

use std::rc::Rc;
use std::time::Duration;

use classivar_core::Assessment;

use super::evidence_stack::{Step, bars, steps};
use super::point_axis::{fraction, points_text, widened};
use super::*;
use crate::look::{MONO, figure};
use crate::view::describe::Scale;

const HEIGHT: f32 = 360.;
const TODAY_WIDTH: f32 = 112.;
const EARLIER_WIDTH: f32 = 30.;
/// From the left edge of one earlier column to the next.
const EARLIER_PITCH: f32 = 64.;
const LABEL_ROOM: f32 = 100.;
/// Room under the columns for their names.
const FOOT: f32 = 40.;

/// The top edge and height of a span of points, as shares of the column (0 at the top).
fn span(low: i32, high: i32, scale: &Scale) -> (f32, f32) {
    let (top, bottom) = (1. - fraction(high, scale), 1. - fraction(low, scale));
    (top, bottom - top)
}

/// Where a total sits, as a share of the column from the top.
fn level(points: i32, scale: &Scale) -> f32 {
    1. - fraction(points, scale)
}

/// An earlier report as the column draws it: its name and its total.
pub(super) struct Earlier {
    pub name: String,
    pub year: u16,
    pub total: i32,
}

impl Workbench {
    pub(super) fn column(&self, result: &Assessment, class: Class, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (dark, ink, muted) = (theme.is_dark(), theme.foreground, theme.muted_foreground);
        let steps = steps(&result.items);
        let earlier = self.earlier_totals();
        let scale = widened(&scale_for(self.rules), &bars(&steps));
        let left = earlier.len() as f32 * EARLIER_PITCH;
        let width = left + TODAY_WIDTH + LABEL_ROOM;
        let mut picture = div().relative().flex_none().w(px(width)).h(px(HEIGHT + FOOT));
        for (i, report) in earlier.iter().enumerate() {
            picture = picture.child(earlier_column(report, i as f32 * EARLIER_PITCH, &scale, dark, (ink, muted)));
        }
        let today_level = level(result.total_points, &scale);
        if !earlier.is_empty() {
            let dots: Vec<(f32, f32)> = earlier
                .iter()
                .enumerate()
                .map(|(i, report)| {
                    (i as f32 * EARLIER_PITCH + EARLIER_WIDTH / 2., level(report.total, &scale) * HEIGHT)
                })
                .chain([(left, today_level * HEIGHT)])
                .collect();
            picture = picture.child(trail(dots, ink));
        }
        picture
            .child(self.capsule(&steps, result.total_points, &scale, class, left, cx))
            .child(zone_names(&scale, class, left + TODAY_WIDTH + 12., (ink, muted)))
            .child(foot("Today", "your evidence", left, TODAY_WIDTH, ink, muted))
            .into_any_element()
    }

    /// Today's capsule: the class bands, the stacked blocks, the total and the preview line.
    fn capsule(
        &self,
        steps: &[Step],
        total: i32,
        scale: &Scale,
        class: Class,
        left: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = cx.theme();
        let (dark, ink, paper) = (theme.is_dark(), theme.foreground, theme.popover);
        let mut capsule = div()
            .absolute()
            .left(px(left))
            .top_0()
            .w(px(TODAY_WIDTH))
            .h(px(HEIGHT))
            .rounded(px(28.))
            .overflow_hidden()
            .children(bands(scale, Some(class), dark, 28.));
        for step in steps {
            capsule = capsule.child(self.column_block(step, scale, cx));
        }
        let ghost = self
            .inspected
            .as_deref()
            .filter(|_| self.preview && self.editing == Editing::Today)
            .and_then(|code| self.preview_without(code).ok())
            .filter(|outcome| outcome.class().is_some())
            .map(|outcome| outcome.total_points);
        div().child(capsule).children(ghost.map(|points| marker(points, scale, left, ink, None))).child(marker(
            total,
            scale,
            left,
            ink,
            Some(paper),
        ))
    }

    fn column_block(&self, step: &Step, scale: &Scale, cx: &mut Context<Self>) -> AnyElement {
        let (top, height) = span(step.from.min(step.to), step.from.max(step.to), scale);
        let selected = self.inspected.as_ref() == Some(&step.code);
        let set_aside = selected && self.preview && self.editing == Editing::Today;
        let colour = direction_colour(step.direction);
        let ink = cx.theme().foreground;
        let tall = height * HEIGHT >= 15.;
        let code = step.code.clone();
        div()
            .id(SharedString::from(format!("column-{}", step.code)))
            .absolute()
            .left(px(10.))
            .right(px(10.))
            .top(relative(top))
            .h(relative(height))
            .py(px(1.))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.choose_block(&code, cx)))
            .child(
                div()
                    .size_full()
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .overflow_hidden()
                    .font_family(MONO)
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .when(!set_aside, |face| face.bg(colour).text_color(direction_ink()))
                    .when(set_aside, |face| face.border_2().border_dashed().border_color(colour).text_color(ink))
                    .when(selected && !set_aside, |face| face.border_2().border_color(ink))
                    .when(tall, |face| face.child(step.code.clone()).child(step.change())),
            )
            .with_animation(
                SharedString::from(format!("rise-{}-{}-{}", step.code, step.from, step.to)),
                Animation::new(Duration::from_millis(420)).with_easing(ease_out_quint()),
                move |el, t| el.opacity(t),
            )
            .into_any_element()
    }

    /// The total of each earlier report whose class is shown, oldest first. A
    /// report with a withheld class or no counted evidence is left out, so the
    /// picture never places a class the engine does not give.
    pub(super) fn earlier_totals(&self) -> Vec<Earlier> {
        if self.mode != Mode::ReportedVariant {
            return Vec::new();
        }
        let mut out: Vec<Earlier> = self
            .records
            .iter()
            .filter_map(|record| {
                let counting = classivar_core::Counting::AsRecorded;
                let scored = self.rules.assess_as(&to_applied(&record.evidence), counting).ok()?;
                scored.class()?;
                Some(Earlier { name: record.label.clone(), year: record.year, total: scored.total_points })
            })
            .collect();
        out.sort_by_key(|report| report.year);
        out
    }
}

/// The class bands as stacked fills, the class shown (if any) in full colour.
/// The end bands carry the rounded corners, since a clip does not round its children.
fn bands(scale: &Scale, class: Option<Class>, dark: bool, corner: f32) -> Vec<Div> {
    let last = scale.zones.len().saturating_sub(1);
    scale
        .zones
        .iter()
        .enumerate()
        .map(|(i, (zone, from, to))| {
            let (top, height) = span(*from, *to, scale);
            let alpha = if class == Some(*zone) {
                1.0
            } else if class.is_some() {
                0.6
            } else {
                0.4
            };
            div()
                .absolute()
                .left_0()
                .w_full()
                .top(relative(top))
                .h(relative(height))
                .bg(Rgba { a: alpha, ..class_fill(*zone, dark) })
                .when(i == last, |band| band.rounded_t(px(corner)))
                .when(i == 0, |band| band.rounded_b(px(corner)))
        })
        .collect()
}

fn earlier_column(report: &Earlier, left: f32, scale: &Scale, dark: bool, (ink, muted): (Hsla, Hsla)) -> Div {
    let at = level(report.total, scale) * HEIGHT;
    div()
        .child(
            div()
                .absolute()
                .left(px(left))
                .top_0()
                .w(px(EARLIER_WIDTH))
                .h(px(HEIGHT))
                .rounded(px(15.))
                .overflow_hidden()
                .children(bands(scale, None, dark, 15.)),
        )
        .child(
            div()
                .absolute()
                .left(px(left + EARLIER_WIDTH / 2. - 6.))
                .top(px(at - 6.))
                .size(px(12.))
                .rounded_full()
                .bg(ink),
        )
        .child(
            figure(11.)
                .absolute()
                .left(px(left - 10.))
                .w(px(EARLIER_WIDTH + 20.))
                .top(px(at - 24.))
                .flex()
                .justify_center()
                .text_color(ink)
                .child(points_text(report.total)),
        )
        .child(foot(&report.name, &report.year.to_string(), left - 17., EARLIER_WIDTH + 34., ink, muted))
}

/// The name under a column, cut short rather than wrapped.
fn foot(name: &str, note: &str, left: f32, width: f32, ink: Hsla, muted: Hsla) -> Div {
    div()
        .absolute()
        .left(px(left))
        .w(px(width))
        .top(px(HEIGHT + 8.))
        .flex()
        .flex_col()
        .items_center()
        .text_xs()
        .child(
            div()
                .w_full()
                .truncate()
                .text_center()
                .text_color(ink)
                .font_weight(FontWeight::SEMIBOLD)
                .child(name.to_string()),
        )
        .child(div().w_full().truncate().text_center().text_color(muted).child(note.to_string()))
}

/// The total as a line across the capsule with its value at the right; outlined when `paper` is `None` (the preview).
fn marker(points: i32, scale: &Scale, left: f32, ink: Hsla, paper: Option<Hsla>) -> Div {
    let at = level(points, scale) * HEIGHT;
    let sign = if points > 0 { "+" } else { "" };
    let tag = figure(13.)
        .absolute()
        .left(px(left + TODAY_WIDTH + 8.))
        .top(px(at - 11.))
        .px_2()
        .h(px(22.))
        .flex()
        .items_center()
        .rounded_full()
        .child(format!("{sign}{}", points_text(points)));
    let tag = match paper {
        Some(paper) => tag.bg(ink).text_color(paper),
        None => tag.border_1().border_dashed().border_color(ink).text_color(ink),
    };
    div()
        .child(
            div()
                .absolute()
                .left(px(left - 6.))
                .w(px(TODAY_WIDTH + 12.))
                .top(px(at - 1.5))
                .h(px(3.))
                .rounded_full()
                .bg(ink)
                .when(paper.is_none(), |line| line.opacity(0.5)),
        )
        .child(tag)
}

/// The band names beside the capsule, and the point value at each class limit.
fn zone_names(scale: &Scale, class: Class, left: f32, (ink, muted): (Hsla, Hsla)) -> Div {
    let mut names = div();
    for (zone, from, to) in &scale.zones {
        let middle = (level(*from, scale) + level(*to, scale)) / 2. * HEIGHT;
        let active = *zone == class;
        names = names.child(
            figure(11.)
                .absolute()
                .left(px(left + 40.))
                .top(px(middle - 8.))
                .whitespace_nowrap()
                .text_color(if active { ink } else { muted })
                .when(!active, |d| d.font_weight(FontWeight::NORMAL))
                .child(zone.short()),
        );
    }
    names
}

/// A dotted line joining the earlier reports' dots to today's total.
fn trail(dots: Vec<(f32, f32)>, ink: Hsla) -> impl IntoElement {
    let dots = Rc::new(dots);
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.5)).dash_array(&[px(2.), px(4.)]);
            for (i, (x, y)) in dots.iter().enumerate() {
                let at = point(bounds.origin.x + px(*x), bounds.origin.y + px(*y));
                if i == 0 { path.move_to(at) } else { path.line_to(at) }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, ink.opacity(0.6));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;

    use super::{level, span};
    use crate::view::describe::scale_for;

    #[test]
    fn higher_points_sit_higher_and_a_span_covers_exactly_its_points() {
        let scale = scale_for(RuleSet::builtin().unwrap());
        assert_eq!(level(scale.max, &scale), 0.0, "the top of the column is the most pathogenic");
        assert_eq!(level(scale.min, &scale), 1.0);
        assert!(level(6, &scale) < level(0, &scale));
        let (top, height) = span(0, 2, &scale);
        assert!((top - level(2, &scale)).abs() < 1e-6);
        assert!((top + height - level(0, &scale)).abs() < 1e-6, "the span ends at its lower value");
    }
}
