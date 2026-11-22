//! The "what if" picture under "What would change it": from the current total, how
//! many points up reach the next class and how many down reach the one below, as
//! outlined bars on the same axis as the points scale. The bars are only possible
//! moves; nothing here applies evidence, and no criterion is suggested.
//!
//! The numbers come from the engine (`to_upgrade`, `to_downgrade`), so the picture
//! and the text lines above it cannot disagree.

use classivar_core::{Assessment, Direction};

use super::point_axis::{
    Bar, class_strip, fluid_bar, fraction, limit_lines, points_text, signed, tick_row, widened, zone_labels,
};
use super::*;
use crate::look::{eyebrow, figure};

/// The possible moves as outlined bars, up first. Empty when no class is known
/// (the engine gives no moves then), or when the class has nowhere to go.
fn moves(result: &Assessment) -> Vec<Bar> {
    let total = result.total_points;
    let up = result.to_upgrade.map(|next| {
        let to = total + next.points_needed;
        Bar {
            label: format!("Up +{}", next.points_needed),
            note: format!("to {}", next.class.short()),
            direction: Direction::Pathogenic,
            from: total,
            to,
            end: format!("= {}", points_text(to)),
            hollow: true,
            neutral: false,
        }
    });
    let down = result.to_downgrade.map(|next| {
        let to = total - next.points_needed;
        Bar {
            label: format!("Down −{}", next.points_needed),
            note: format!("to {}", next.class.short()),
            direction: Direction::Benign,
            from: total,
            to,
            end: format!("= {}", points_text(to)),
            hollow: true,
            neutral: false,
        }
    });
    [up, down].into_iter().flatten().collect()
}

/// Width of the column that names each move, and the height of one row.
const LABEL_WIDTH: f32 = 92.;
const ROW: f32 = 34.;

/// One move in words, for example "Up: 1 more point of pathogenic evidence reaches Likely pathogenic."
fn move_sentence(way: &str, kind: &str, next: classivar_core::NextClass) -> String {
    let n = next.points_needed;
    let verb = if n == 1 { "reaches" } else { "reach" };
    format!("{way}: {n} more point{} of {kind} evidence {verb} {}.", plural(n), next.class.label())
}

impl Workbench {
    /// What would change the class: the moves in words, then as outlined bars from the total.
    pub(super) fn what_changes(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let up = result.to_upgrade.map(|next| move_sentence("Up", "pathogenic", next));
        let down = result.to_downgrade.map(|next| move_sentence("Down", "benign", next));
        let bars = moves(result);
        crate::look::card(cx)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(div().flex().flex_col().gap_1().text_sm().children(up).children(down))
            .when_some(result.class().filter(|_| !bars.is_empty()), |card, class| {
                card.child(self.moves_chart(result.total_points, class, &bars, cx))
            })
            .child(caption(muted).child(
                "Outlined bars are moves that are possible, not evidence that is applied. Evidence that could move the class would come from kinds of evidence you have not used yet.",
            ))
            .into_any_element()
    }

    /// The class strip with the total, the move up above it and the move down below it.
    fn moves_chart(&self, total: i32, class: Class, bars: &[Bar], cx: &App) -> Div {
        let theme = cx.theme();
        let (ink, muted, paper, border) = (theme.foreground, theme.muted_foreground, theme.muted, theme.border);
        let scale = widened(&scale_for(self.rules), bars);
        let at = fraction(total, &scale);
        let row = |label: Div, track: Div| {
            div().flex().items_center().h(px(ROW)).child(label.w(px(LABEL_WIDTH)).flex_none()).child(
                track.relative().flex_1().h_full().flex().items_center().children(limit_lines(&scale, border)).child(
                    div().absolute().top_0().h_full().left(relative(at)).ml(px(-1.)).w(px(2.)).bg(ink.opacity(0.35)),
                ),
            )
        };
        let lane = |bar: &Bar| {
            let label = div()
                .flex()
                .flex_col()
                .child(figure(12.).child(bar.label.clone()))
                .child(caption(muted).child(bar.note.clone()));
            row(label, div().child(fluid_bar(bar, &scale, (paper, muted), 14.)))
        };
        let pill = figure(12.)
            .absolute()
            .left(relative(at))
            .ml(px(-20.))
            .w(px(40.))
            .h(px(20.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(ink)
            .text_color(theme.popover)
            .child(signed(total));
        let strip =
            row(eyebrow("Now", cx), div().child(class_strip(&scale, class, (theme.popover, ink), 12.)).child(pill));
        let (ups, downs): (Vec<&Bar>, Vec<&Bar>) = bars.iter().partition(|bar| bar.to > bar.from);
        div()
            .flex()
            .flex_col()
            .child(div().pl(px(LABEL_WIDTH)).child(zone_labels(&scale, Some(class), (ink, muted), None)))
            .children(ups.into_iter().map(lane))
            .child(strip)
            .children(downs.into_iter().map(lane))
            .child(div().pl(px(LABEL_WIDTH)).child(tick_row(&scale, muted)))
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, Direction, RuleSet, Strength};

    use super::moves;

    fn assessed(applied: &[Applied]) -> classivar_core::Assessment {
        RuleSet::builtin().expect("the built-in rules are valid").assess(applied).expect("valid evidence")
    }

    #[test]
    fn a_vus_can_move_up_one_point_and_down_six() {
        // 1 + 1 + 1 + 2 = 5 points: one short of Likely pathogenic, six above Likely benign.
        let result = assessed(&[
            Applied::new("PM2", Strength::Supporting),
            Applied::new("PP3", Strength::Supporting),
            Applied::new("PP4", Strength::Supporting),
            Applied::new("PM5", Strength::Moderate),
        ]);
        let bars = moves(&result);
        assert_eq!(bars.len(), 2);
        let (up, down) = (&bars[0], &bars[1]);
        assert_eq!((up.label.as_str(), up.note.as_str(), up.direction), ("Up +1", "to LP", Direction::Pathogenic));
        assert_eq!((up.from, up.to, up.end.as_str()), (5, 6, "= 6"));
        assert_eq!((down.label.as_str(), down.note.as_str(), down.direction), ("Down −6", "to LB", Direction::Benign));
        assert_eq!((down.from, down.to, down.end.as_str()), (5, -1, "= −1"));
        assert!(bars.iter().all(|bar| bar.hollow), "a possible move is outlined, never filled");
    }

    #[test]
    fn the_bars_end_exactly_on_the_class_limits() {
        // Total 4 points: Likely pathogenic starts at 6, and Likely benign ends at -1.
        let result = assessed(&[Applied::new("PS1", Strength::Strong)]);
        let bars = moves(&result);
        assert_eq!(bars[0].to, 6);
        assert_eq!(bars[1].to, -1);
        let rules = RuleSet::builtin().unwrap();
        assert_eq!(rules.class_for_points(bars[0].to), classivar_core::Class::LikelyPathogenic);
        assert_eq!(rules.class_for_points(bars[1].to), classivar_core::Class::LikelyBenign);
    }

    #[test]
    fn a_bar_that_ends_below_zero_is_written_with_a_real_minus_sign() {
        // Two Strong benign criteria: Benign at -8, and the move up ends at -6.
        // Every other total on the screen uses the real minus sign, so this one must too.
        let result = assessed(&[Applied::new("BS1", Strength::Strong), Applied::new("BS2", Strength::Strong)]);
        let bars = moves(&result);
        assert_eq!(bars.len(), 1, "Benign has no class below it");
        assert_eq!(bars[0].label, "Up +2");
        assert_eq!(bars[0].note, "to LB");
        assert_eq!(bars[0].end, "= −6");
    }

    #[test]
    fn the_top_class_can_only_move_down_and_stand_alone_benign_cannot_move() {
        let top = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS2", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
        ]);
        let bars = moves(&top);
        assert_eq!(bars.len(), 1, "Pathogenic has no class above it");
        assert_eq!(bars[0].direction, Direction::Benign);
        // BA1 fixes the class at Benign, and Benign has no class below it.
        let ba1 = assessed(&[Applied::new("BA1", Strength::StandAlone)]);
        assert!(moves(&ba1).is_empty());
    }

    #[test]
    fn a_withheld_class_has_no_moves() {
        let both = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS2", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
            Applied::new("PS4", Strength::Strong),
            Applied::new("BS1", Strength::Strong),
            Applied::new("BS2", Strength::Strong),
        ]);
        assert!(both.class().is_none(), "this example must be withheld");
        assert!(moves(&both).is_empty());
    }
}
