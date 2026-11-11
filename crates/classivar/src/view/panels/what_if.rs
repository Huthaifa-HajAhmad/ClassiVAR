//! The "what if" picture under "What would change it": from the current total, how
//! many points up reach the next class and how many down reach the one below, as
//! outlined bars on the same axis as the points scale. The bars are only possible
//! moves; nothing here applies evidence, and no criterion is suggested.
//!
//! The numbers come from the engine (`to_upgrade`, `to_downgrade`), so the picture
//! and the text lines above it cannot disagree.

use classivar_core::{Assessment, Direction};

use super::point_axis::{Bar, axis, points_text, widened};
use super::*;

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
        }
    });
    [up, down].into_iter().flatten().collect()
}

impl Workbench {
    /// The outlined bars with the class zones, and a line saying what they mean.
    pub(super) fn what_if(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let bars = moves(result);
        if bars.is_empty() {
            return div().into_any_element();
        }
        let scale = widened(&scale_for(self.rules), &bars);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(axis("what-if-scroll", &scale, &bars, border, muted))
            .child(caption(muted).child(
                "Outlined bars show how far the total would have to move to reach the next class. Nothing is applied.",
            ))
            .into_any_element()
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
