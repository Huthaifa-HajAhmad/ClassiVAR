//! Where the column's parts go for a stage of a given width. Plain functions
//! with tests; the drawing in `column.rs` only follows them.

use super::{EARLIER_PITCH, LABEL_ROOM, TODAY_WIDTH};
use crate::view::panels::block_tip::TIP_WIDTH;

/// The least room the class readout keeps beside the column, and the gap between them.
pub(super) const READOUT_MIN: f32 = 200.;
pub(super) const GAP: f32 = 32.;
/// How far an earlier report's caption hangs left of its slim column; the scroll box keeps
/// this much room so the caption is not cut off.
pub(super) const CAPTION_OVERHANG: f32 = 17.;
/// How far the card may reach past the stage's edge: the page keeps 32 px of padding there.
pub(super) const SPILL: f32 = 32.;
/// Between the hover card and the capsule.
const TIP_GAP: f32 = 12.;

/// How the readout and the column share the stage.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum Arrangement {
    /// The readout at the left, the column at the right.
    Beside,
    /// The readout above the column; the column scrolls sideways if it is still too wide.
    Stacked,
}

/// The width of the whole picture: one slim column per earlier report, today's capsule and the zone names.
pub(super) fn picture_width(earlier: usize) -> f32 {
    earlier as f32 * EARLIER_PITCH + TODAY_WIDTH + LABEL_ROOM
}

/// Beside while the readout keeps its minimum room, stacked otherwise, so nothing is cut off.
pub(super) fn arrange(stage: f32, earlier: usize) -> Arrangement {
    if stage >= READOUT_MIN + GAP + picture_width(earlier) { Arrangement::Beside } else { Arrangement::Stacked }
}

/// The left edge of the hover card in the picture's own coordinates. The card goes on
/// the side of today's capsule where it covers the capsule least (the left when both
/// are the same), and is kept inside the stage, with the page margin to spare.
pub(super) fn tip_left(earlier: usize, stage: f32) -> f32 {
    let width = picture_width(earlier);
    let capsule = earlier as f32 * EARLIER_PITCH;
    // The part of the stage the picture can show, in the picture's own coordinates.
    let (lo, hi) = match arrange(stage, earlier) {
        Arrangement::Beside => (width - stage - SPILL, width + SPILL),
        Arrangement::Stacked => (-CAPTION_OVERHANG - SPILL, (stage - CAPTION_OVERHANG).max(width) + SPILL),
    };
    let place = |wanted: f32| wanted.clamp(lo, (hi - TIP_WIDTH).max(lo));
    let covered = |left: f32| ((left + TIP_WIDTH).min(capsule + TODAY_WIDTH) - left.max(capsule)).max(0.);
    let (left_side, right_side) = (place(capsule - TIP_WIDTH - TIP_GAP), place(capsule + TODAY_WIDTH + TIP_GAP));
    if covered(right_side) < covered(left_side) { right_side } else { left_side }
}

#[cfg(test)]
mod tests {
    use super::{
        Arrangement, CAPTION_OVERHANG, EARLIER_PITCH, LABEL_ROOM, SPILL, TIP_WIDTH, TODAY_WIDTH, arrange,
        picture_width, tip_left,
    };

    #[test]
    fn the_picture_is_as_wide_as_its_earlier_columns_plus_today_and_the_labels() {
        assert_eq!(picture_width(0), TODAY_WIDTH + LABEL_ROOM);
        assert_eq!(picture_width(2), 2. * EARLIER_PITCH + TODAY_WIDTH + LABEL_ROOM);
    }

    #[test]
    fn the_readout_sits_beside_the_column_only_while_it_keeps_its_minimum_room() {
        assert_eq!(
            arrange(504., 0),
            Arrangement::Beside,
            "no earlier report: 212 px of column leaves 292 for the readout (at least 200 needed)"
        );
        assert_eq!(arrange(504., 1), Arrangement::Stacked, "one earlier report needs 276 + 200 + 32 = 508 px");
        assert_eq!(arrange(508., 1), Arrangement::Beside);
        assert_eq!(arrange(504., 2), Arrangement::Stacked, "two earlier reports would squeeze the readout");
        assert_eq!(arrange(256., 0), Arrangement::Stacked, "the narrowest stage");
    }

    #[test]
    fn the_hover_card_stays_within_the_stage_and_the_page_margin_beside_it() {
        for stage in [256., 400., 504., 892.] {
            for earlier in 0..5 {
                let width = picture_width(earlier);
                // Where the picture starts in the stage: at the right edge beside the readout,
                // otherwise after the room kept for the captions.
                let start =
                    if arrange(stage, earlier) == Arrangement::Beside { stage - width } else { CAPTION_OVERHANG };
                let left = start + tip_left(earlier, stage);
                assert!(left >= -SPILL - 0.01, "stage {stage}, {earlier} earlier: the card starts at {left}");
                if stage >= TIP_WIDTH + 2. * SPILL && width <= stage {
                    let end = left + TIP_WIDTH;
                    assert!(end <= stage + SPILL + 0.01, "stage {stage}, {earlier} earlier: the card ends at {end}");
                }
            }
        }
    }

    #[test]
    fn the_hover_card_never_covers_the_capsule_when_a_free_side_can_hold_it() {
        // The stage beside an open editor at 1240 px is 504 px wide; two reports stack the readout above.
        for (stage, earlier) in [(504., 2), (504., 1), (508., 1), (400., 0), (892., 3), (892., 0)] {
            let capsule = earlier as f32 * EARLIER_PITCH;
            let left = tip_left(earlier, stage);
            let clear = left + TIP_WIDTH <= capsule || left >= capsule + TODAY_WIDTH;
            assert!(clear, "stage {stage}, {earlier} earlier: the card at {left} covers the capsule at {capsule}");
        }
    }

    #[test]
    fn the_hover_card_prefers_the_free_side_left_of_the_capsule() {
        let beside = tip_left(0, 892.);
        assert!(beside < 0., "with room on the left the card hangs left of the capsule, at {beside}");
        let (left, capsule) = (tip_left(3, 892.), 3. * EARLIER_PITCH);
        assert!(
            left + TIP_WIDTH <= capsule,
            "it does not cover the capsule: ends at {}, capsule at {capsule}",
            left + TIP_WIDTH
        );
    }
}
