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

use super::block_tip::floating;
use super::evidence_stack::{Step, bars, steps};
use super::point_axis::{fraction, points_text, widened};
use super::*;
use crate::look::{MONO, figure};
use crate::view::describe::Scale;
mod layout;
mod parts;
use layout::{Arrangement, CAPTION_OVERHANG, GAP, READOUT_MIN, arrange, tip_left};
use parts::{bands, earlier_column, foot, marker, trail, zone_names};

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
    /// The class readout and the column. Beside each other while the readout keeps its
    /// room; otherwise the readout goes above and the column scrolls sideways if it must.
    pub(super) fn column_with_readout(
        &self,
        readout: Div,
        result: &Assessment,
        class: Class,
        cx: &mut Context<Self>,
    ) -> Div {
        let earlier = self.earlier_totals();
        let stage = self.stage_width.get();
        let column = self.column(result, class, &earlier, tip_left(earlier.len(), stage), cx);
        match arrange(stage, earlier.len()) {
            Arrangement::Beside => {
                div().flex().items_center().gap(px(GAP)).child(readout.flex_1().min_w(px(READOUT_MIN))).child(column)
            }
            Arrangement::Stacked => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(readout)
                .child(div().id("column-scroll").overflow_x_scroll().pl(px(CAPTION_OVERHANG)).child(column)),
        }
    }

    fn column(
        &self,
        result: &Assessment,
        class: Class,
        earlier: &[Earlier],
        tip_at: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (dark, ink, muted) = (theme.is_dark(), theme.foreground, theme.muted_foreground);
        let steps = steps(&result.items);
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
        let tip = self.column_tip(result, &steps, &scale, tip_at, cx);
        picture
            .child(self.capsule(&steps, result.total_points, &scale, class, left, cx))
            .children(tip)
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
        let (code, hover) = (step.code.clone(), step.code.clone());
        let selector = format!("column-{}", step.code);
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector)
            .absolute()
            .left(px(10.))
            .right(px(10.))
            .top(relative(top))
            .h(relative(height))
            .py(px(1.))
            .cursor_pointer()
            .hover(|style| style.opacity(0.88))
            .on_click(cx.listener(move |this, _, _, cx| this.choose_block(&code, cx)))
            .on_hover(cx.listener(move |this, over: &bool, _, cx| this.hover_over(&hover, *over, cx)))
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

    /// The hover card for the block under the pointer, at the block's height and `tip_at` across (see `layout::tip_left`).
    fn column_tip(
        &self,
        result: &Assessment,
        steps: &[Step],
        scale: &Scale,
        tip_at: f32,
        cx: &App,
    ) -> Option<AnyElement> {
        let code = self.hovered.as_deref()?;
        let step = steps.iter().find(|step| step.code == code)?;
        let card = self.block_tip(result, code, cx)?;
        let (top, height) = span(step.from.min(step.to), step.from.max(step.to), scale);
        let middle = (top + height / 2.) * HEIGHT;
        Some(floating(card, (tip_at, (middle - 48.).clamp(0., HEIGHT - 96.))))
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
