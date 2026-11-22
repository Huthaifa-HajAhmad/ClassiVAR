//! The evidence track: the main picture of the Evidence view.
//!
//! One wide band holds the five class zones, coloured by class, on the points
//! axis. Each counted criterion is a block laid end to end from zero:
//! pathogenic blocks in the upper lane build to the right, benign blocks in the
//! lower lane take points back to the left. A marker shows the total and slides
//! to the new total when the evidence changes (no motion when the system asks
//! for reduced motion; GPUI handles that).
//!
//! Clicking a block selects it, and its explanation opens under the picture.
//! Clicking the selected block again previews the result without it: the
//! block turns to an outline and a second, outlined marker shows where the
//! total would be. The buttons under the band do the same from the keyboard.
//!
//! The track is drawn only when a class is shown, so it never suggests a
//! class the engine withholds. Positions are fractions of the width, so the
//! picture fills any window.

use std::time::Duration;

use classivar_core::{Assessment, Direction};

use super::block_tip::floating_at;
use super::evidence_stack::{Step, bars, steps, summary};
use super::point_axis::{fraction, name_fits, signed, tick_row, widened, zone_labels};
use super::*;
use crate::view::describe::Scale;

const LANE_HEIGHT: f32 = 40.;
/// Space above the first lane, and from the top of one lane to the next.
const LANE_START: f32 = 16.;
const LANE_PITCH: f32 = 48.;
/// Room under the lanes for the marker labels.
const PILL_ROOM: f32 = 36.;
const PILL_WIDTH: f32 = 64.;

/// Two lanes when both directions have evidence; otherwise one, so no empty lane is drawn.
fn lane_count(steps: &[Step]) -> usize {
    let has = |direction| steps.iter().any(|step| step.direction == direction);
    if has(Direction::Pathogenic) && has(Direction::Benign) { 2 } else { 1 }
}

pub(super) fn band_height(lanes: usize) -> f32 {
    LANE_START + lanes as f32 * LANE_PITCH + PILL_ROOM
}

/// One counted criterion as a block on the track.
#[derive(Debug, Clone, PartialEq)]
struct Block {
    code: String,
    /// For example "PM5 +2".
    label: String,
    direction: Direction,
    /// Running totals before and after this block, in points.
    from: i32,
    to: i32,
    /// Left edge and width as fractions of the track (0.0 to 1.0).
    left: f32,
    width: f32,
}

/// The words on a block in a track `track_width` pixels wide: the whole label where it
/// fits, else the code alone, else nothing (the hover card still names it), so no
/// label is ever cut in half.
fn block_text(block: &Block, track_width: f32) -> &str {
    if name_fits(&block.label, block.width, track_width) {
        &block.label
    } else if name_fits(&block.code, block.width, track_width) {
        &block.code
    } else {
        ""
    }
}

fn blocks(steps: &[Step], scale: &Scale) -> Vec<Block> {
    steps
        .iter()
        .map(|step| {
            let (low, high) = (step.from.min(step.to), step.from.max(step.to));
            let left = fraction(low, scale);
            Block {
                code: step.code.clone(),
                label: format!("{} {}", step.code, step.change()),
                direction: step.direction,
                from: step.from,
                to: step.to,
                left,
                width: fraction(high, scale) - left,
            }
        })
        .collect()
}

/// How far left of its anchor a marker's label starts, so it stays inside the band at either edge.
fn pill_offset(at: f32) -> f32 {
    if at < 0.06 {
        0.
    } else if at > 0.94 {
        -PILL_WIDTH
    } else {
        -PILL_WIDTH / 2.
    }
}

/// Where a block's hover card is anchored, as a share of the track, and whether
/// the card ends there (true) or starts there (false). A block in the left half
/// opens its card rightward from its left edge; one in the right half ends its
/// card at its right edge. Either way the card stays inside the track.
fn tip_anchor(block: &Block) -> (f32, bool) {
    if block.left + block.width / 2. < 0.5 { (block.left, false) } else { (block.left + block.width, true) }
}

fn motion(ms: u64) -> Animation {
    Animation::new(Duration::from_millis(ms)).with_easing(ease_out_quint())
}

impl Workbench {
    pub(super) fn evidence_track(&self, result: &Assessment, class: Class, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (dark, border, muted, ink, paper) =
            (theme.is_dark(), theme.border, theme.muted_foreground, theme.foreground, theme.background);
        let steps = steps(&result.items);
        let scale = widened(&scale_for(self.rules), &bars(&steps));
        let lanes = lane_count(&steps);
        let height = band_height(lanes);
        let mut band = div().relative().w_full().h(px(height)).rounded_lg().overflow_hidden();
        for (zone, from, to) in &scale.zones {
            let (left, right) = (fraction(*from, &scale), fraction(*to, &scale));
            let fill = class_fill(*zone, dark);
            let fill = if *zone == class { fill } else { Rgba { a: 0.4, ..fill } };
            band =
                band.child(div().absolute().top_0().h_full().left(relative(left)).w(relative(right - left)).bg(fill));
            band = band.child(div().absolute().top_0().h_full().left(relative(left)).w(px(1.)).bg(border));
        }
        let blocks = blocks(&steps, &scale);
        let band = band
            // The preview line goes under the blocks; the total's line goes over them.
            .children(self.ghost_marker(&scale, height, ink))
            .children(blocks.iter().map(|block| self.block(block, lanes, cx)))
            .child(self.marker(result.total_points, &scale, height, (ink, paper)));
        // The card hangs under the band, outside its clipped edges, so it never hides a block.
        let tip = self.hovered.as_deref().and_then(|code| {
            let block = blocks.iter().find(|block| block.code == code)?;
            Some(floating_at(self.block_tip(result, code, cx)?, tip_anchor(block), height + 6.))
        });
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(zone_labels(&scale, Some(class), (ink, muted), Some(self.stage.width.get())))
            .child(div().relative().child(band).children(tip))
            .child(tick_row(&scale, muted))
            .child(self.track_buttons(&steps, cx))
            .child(caption(muted).child(summary(&steps)))
            .into_any_element()
    }

    fn block(&self, block: &Block, lanes: usize, cx: &mut Context<Self>) -> AnyElement {
        let ink = cx.theme().foreground;
        let selected = self.inspected.as_ref() == Some(&block.code);
        let set_aside = selected && self.preview;
        let colour = direction_colour(block.direction);
        let lane = usize::from(lanes == 2 && block.direction == Direction::Benign);
        let face = div()
            .size_full()
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .when(!set_aside, |face| face.bg(colour).text_color(direction_ink()))
            .when(set_aside, |face| face.border_2().border_dashed().border_color(colour).text_color(ink))
            .when(selected && !set_aside, |face| face.border_2().border_color(ink))
            .child(block_text(block, self.stage.width.get()).to_string());
        let (code, hover) = (block.code.clone(), block.code.clone());
        let width = block.width;
        let selector = format!("block-{}", block.code);
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector)
            .absolute()
            .top(px(LANE_START + lane as f32 * LANE_PITCH))
            .h(px(LANE_HEIGHT))
            .left(relative(block.left))
            .px(px(1.))
            .cursor_pointer()
            .hover(|style| style.opacity(0.85))
            .on_click(cx.listener(move |this, _, _, cx| this.choose_block(&code, cx)))
            .on_hover(cx.listener(move |this, over: &bool, _, cx| this.hover_over(&hover, *over, cx)))
            .child(face)
            .with_animation(
                SharedString::from(format!("grow-{}-{}-{}", block.code, block.from, block.to)),
                motion(420),
                move |element, t| element.w(relative(width * t.clamp(0.0, 1.0))),
            )
            .into_any_element()
    }

    /// The first click selects a block; a second click on it turns the preview on or off.
    pub(super) fn choose_block(&mut self, code: &str, cx: &mut Context<Self>) {
        if self.inspected.as_deref() == Some(code) {
            self.preview = self.editing == Editing::Today && !self.preview;
        } else {
            self.inspected = Some(code.to_string());
            self.preview = false;
        }
        cx.notify();
    }

    /// The total: a line through the band with the points on a label, sliding from the last total shown.
    fn marker(&self, total: i32, scale: &Scale, height: f32, (ink, paper): (Hsla, Hsla)) -> AnyElement {
        let (mut from, shown) = self.stage.needle.get();
        if shown != total {
            from = shown;
            self.stage.needle.set((shown, total));
        }
        let (start, end) = (fraction(from, scale), fraction(total, scale));
        marker_shape(signed(total), end, height, ink, |pill| pill.bg(ink).text_color(paper))
            .with_animation(SharedString::from(format!("needle-{from}-{total}")), motion(560), move |element, t| {
                element.left(relative(start + (end - start) * t))
            })
            .into_any_element()
    }

    /// Where the total would be without the selected block, while the preview is on.
    fn ghost_marker(&self, scale: &Scale, height: f32, ink: Hsla) -> Option<AnyElement> {
        let code = self.inspected.as_ref().filter(|_| self.preview && self.editing == Editing::Today)?;
        let outcome = self.preview_without(code).ok()?;
        outcome.class()?;
        let at = fraction(outcome.total_points, scale);
        let shape = marker_shape(signed(outcome.total_points), at, height, ink, |pill| {
            pill.border_2().border_dashed().border_color(ink).text_color(ink)
        });
        Some(
            shape
                .with_animation(SharedString::from(format!("ghost-{code}")), motion(360), |el, t| el.opacity(t))
                .into_any_element(),
        )
    }

    /// The keyboard way to the same choice as a click on a block.
    fn track_buttons(&self, steps: &[Step], cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().flex_wrap().gap_2().pt_2();
        for step in steps {
            let code = step.code.clone();
            let selected = self.inspected.as_ref() == Some(&code);
            row = row.child(
                Button::new(SharedString::from(format!("inspect-{code}")))
                    .small()
                    .outline()
                    .label(format!("{} {} · {}", code, step.change(), step.strength.label()))
                    .accessibility_label(format!("Inspect {code}"))
                    .selected(selected)
                    .toggled(selected)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.inspected = Some(code.clone());
                        this.preview = false;
                        cx.notify();
                    })),
            );
        }
        row
    }
}

/// A vertical line across the band with a label at the bottom. `face` styles the label.
fn marker_shape(text: String, at: f32, height: f32, ink: Hsla, face: impl FnOnce(Div) -> Div) -> Div {
    let pill = div()
        .absolute()
        .bottom(px(6.))
        .left(px(pill_offset(at)))
        .w(px(PILL_WIDTH))
        .py(px(2.))
        .rounded_full()
        .flex()
        .justify_center()
        .text_sm()
        .font_weight(FontWeight::BOLD)
        .child(text);
    div()
        .absolute()
        .top_0()
        .h_full()
        .w(px(0.))
        .left(relative(at))
        .child(div().absolute().top_0().h(px(height - PILL_ROOM + 4.)).w(px(3.)).ml(px(-1.5)).bg(ink))
        .child(face(pill))
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, Direction, RuleSet, Strength};

    use super::super::point_axis::fraction;
    use super::{band_height, block_text, blocks, lane_count, pill_offset, scale_for, steps, tip_anchor};

    #[test]
    fn a_narrow_block_shows_its_code_then_nothing_rather_than_a_cut_label() {
        let rules = RuleSet::builtin().unwrap();
        let result = rules.assess(&[Applied::new("PM2", Strength::Supporting)]).unwrap();
        let block = blocks(&steps(&result.items), &scale_for(rules)).remove(0);
        assert_eq!(block_text(&block, 4000.), "PM2 +1", "wide enough for the whole label");
        let code_only = (0..4000).map(|w| w as f32).find(|w| block_text(&block, *w) == "PM2 +1").unwrap() - 1.;
        assert_eq!(block_text(&block, code_only), "PM2", "just too narrow for the points: the code alone");
        assert_eq!(block_text(&block, 10.), "", "too narrow for the code: the hover card names it");
    }

    #[test]
    fn a_point_value_maps_across_the_track_and_stays_inside_it() {
        let scale = scale_for(RuleSet::builtin().unwrap());
        assert_eq!(fraction(scale.min, &scale), 0.0);
        assert_eq!(fraction(scale.max, &scale), 1.0);
        assert_eq!(fraction(scale.max + 50, &scale), 1.0, "past the end is clamped, never drawn outside");
        let mid = (scale.min + scale.max) / 2;
        assert!((fraction(mid, &scale) - 0.5).abs() < 0.05);
    }

    #[test]
    fn blocks_sit_end_to_end_and_the_last_ends_at_the_total() {
        let rules = RuleSet::builtin().unwrap();
        let result = rules
            .assess(&[
                Applied::new("PS1", Strength::Strong),
                Applied::new("PM2", Strength::Supporting),
                Applied::new("BP7", Strength::Supporting),
            ])
            .unwrap();
        let scale = scale_for(rules);
        let blocks = blocks(&steps(&result.items), &scale);
        assert_eq!(blocks[0].left, fraction(0, &scale), "the first block starts at zero");
        assert_eq!(blocks[0].label, "PS1 +4");
        // PM2 starts where PS1 ends.
        assert!((blocks[1].left - (blocks[0].left + blocks[0].width)).abs() < 1e-6);
        // BP7 is benign: it runs from the pathogenic sum (5) back to the total (4).
        let benign = &blocks[2];
        assert_eq!(benign.direction, Direction::Benign);
        assert!((benign.left - fraction(result.total_points, &scale)).abs() < 1e-6);
        assert!((benign.left + benign.width - fraction(5, &scale)).abs() < 1e-6);
    }

    #[test]
    fn a_marker_label_stays_inside_the_band_at_either_edge() {
        assert_eq!(pill_offset(0.0), 0.0);
        assert_eq!(pill_offset(1.0), -super::PILL_WIDTH);
        assert_eq!(pill_offset(0.5), -super::PILL_WIDTH / 2.0);
    }

    #[test]
    fn a_second_lane_is_drawn_only_when_both_directions_have_evidence() {
        let rules = RuleSet::builtin().unwrap();
        let lanes = |applied: &[Applied]| lane_count(&steps(&rules.assess(applied).unwrap().items));
        assert_eq!(lanes(&[Applied::new("PM2", Strength::Supporting)]), 1);
        assert_eq!(lanes(&[Applied::new("BP7", Strength::Supporting)]), 1);
        assert_eq!(lanes(&[Applied::new("PM2", Strength::Supporting), Applied::new("BP7", Strength::Supporting)]), 2);
        assert!(band_height(2) > band_height(1));
    }

    #[test]
    fn a_hover_card_opens_away_from_the_nearer_edge_of_the_track() {
        let rules = RuleSet::builtin().unwrap();
        let result = rules.assess(&[Applied::new("PM2", Strength::Supporting)]).unwrap();
        let block = blocks(&steps(&result.items), &scale_for(rules)).remove(0);
        let early = super::Block { left: 0.05, width: 0.2, ..block.clone() };
        assert_eq!(tip_anchor(&early), (0.05, false), "left half: the card starts at the block's left edge");
        let late = super::Block { left: 0.7, width: 0.25, ..block };
        let (at, ends_there) = tip_anchor(&late);
        assert!((at - 0.95).abs() < 1e-6 && ends_there, "right half: the card ends at the block's right edge");
    }
}
