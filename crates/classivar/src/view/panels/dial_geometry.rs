//! The geometry of the dial, as plain functions so it can be tested without a window.
//!
//! The dial is a half circle. The lowest point value of the scale sits at the
//! left end (an angle of pi, in radians), the highest at the right end (angle 0),
//! so more pathogenic evidence turns the needle to the right, the same way the
//! track reads from left to right. Angles grow anticlockwise from the right, as
//! in school maths; screen y grows downwards, so `polar` flips the sign of y.

use std::f32::consts::PI;

use classivar_core::Direction;

use super::evidence_stack::Step;
use crate::view::describe::Scale;

/// The smallest and largest outer radius of the dial, in pixels.
const MIN_RADIUS: f32 = 120.;
const MAX_RADIUS: f32 = 250.;

/// The angle of a point value on the dial.
pub(super) fn angle_of(points: f32, scale: &Scale) -> f32 {
    let span = (scale.max - scale.min).max(1) as f32;
    let share = ((points - scale.min as f32) / span).clamp(0.0, 1.0);
    PI * (1.0 - share)
}

/// A position `radius` away from `centre` at `angle`, in screen coordinates.
pub(super) fn polar(centre: (f32, f32), radius: f32, angle: f32) -> (f32, f32) {
    (centre.0 + radius * angle.cos(), centre.1 - radius * angle.sin())
}

/// The outer radius that fits a stage `width` wide and `height` tall, leaving room for the labels.
pub(super) fn radius_for(width: f32, height: f32) -> f32 {
    ((width - 96.) / 2.).min(height - 120.).clamp(MIN_RADIUS, MAX_RADIUS)
}

/// One counted criterion as a slice of the inner ring.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Segment {
    pub code: String,
    pub label: String,
    pub direction: Direction,
    /// The angles at the start and end of the slice; `start` is the larger (further left).
    pub start: f32,
    pub end: f32,
}

/// The slices of the inner ring, one per counted criterion, laid end to end from zero like the track's blocks.
pub(super) fn segments(steps: &[Step], scale: &Scale) -> Vec<Segment> {
    steps
        .iter()
        .map(|step| {
            let (low, high) = (step.from.min(step.to), step.from.max(step.to));
            Segment {
                code: step.code.clone(),
                label: format!("{} {}", step.code, step.change()),
                direction: step.direction,
                start: angle_of(low as f32, scale),
                end: angle_of(high as f32, scale),
            }
        })
        .collect()
}

/// The slices that get their code written on them: wide enough for the text, and not
/// covered by a slice painted after them. Opposite evidence covers the same angles near
/// zero and the later slice is drawn on top, so its code is the one that is readable; a
/// code written over it would show two codes on one slice. The hover card, the chips
/// under the picture and the track still name every slice.
pub(super) fn labelled(slices: &[Segment], radius: f32, needs: f32) -> Vec<&Segment> {
    let mut chosen: Vec<&Segment> = Vec::new();
    for slice in slices.iter().rev().filter(|s| (s.start - s.end) * radius >= needs) {
        let free = chosen.iter().all(|kept| slice.end >= kept.start || slice.start <= kept.end);
        if free {
            chosen.push(slice);
        }
    }
    chosen.reverse();
    chosen
}

/// Which slice a pointer at `(x, y)` is over, if any. `ring` is the inner and outer radius of the evidence ring.
pub(super) fn segment_at(
    segments: &[Segment],
    centre: (f32, f32),
    ring: (f32, f32),
    (x, y): (f32, f32),
) -> Option<&Segment> {
    let (dx, dy) = (x - centre.0, centre.1 - y);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < ring.0 || distance > ring.1 || dy < 0. {
        return None;
    }
    let angle = dy.atan2(dx);
    segments.iter().find(|segment| angle <= segment.start && angle >= segment.end)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use classivar_core::{Applied, RuleSet, Strength};

    use super::super::evidence_stack::steps;
    use super::{angle_of, labelled, polar, radius_for, segment_at, segments};
    use crate::view::describe::Scale;
    use crate::view::describe::scale_for;

    /// The point value at an angle (the inverse of `angle_of`).
    fn points_at(angle: f32, scale: &Scale) -> f32 {
        let span = (scale.max - scale.min).max(1) as f32;
        scale.min as f32 + (1.0 - angle / PI) * span
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn the_lowest_points_sit_at_the_left_end_and_the_highest_at_the_right() {
        let scale = scale_for(RuleSet::builtin().unwrap());
        assert!(close(angle_of(scale.min as f32, &scale), PI));
        assert!(close(angle_of(scale.max as f32, &scale), 0.0));
        assert!(close(angle_of(scale.max as f32 + 40., &scale), 0.0), "past the end is clamped to the dial");
        assert!(close(points_at(angle_of(3.0, &scale), &scale), 3.0), "angle and points convert both ways");
    }

    #[test]
    fn a_point_straight_up_is_above_the_centre_on_screen() {
        let (x, y) = polar((100., 100.), 50., PI / 2.);
        assert!(close(x, 100.) && close(y, 50.));
    }

    #[test]
    fn the_dial_grows_with_the_stage_but_stays_readable_and_bounded() {
        assert_eq!(radius_for(200., 200.), super::MIN_RADIUS);
        assert_eq!(radius_for(2000., 2000.), super::MAX_RADIUS);
        let middle = radius_for(500., 400.);
        assert!(middle > super::MIN_RADIUS && middle < super::MAX_RADIUS);
    }

    #[test]
    fn two_slices_over_the_same_angles_never_both_get_their_code_written() {
        let rules = RuleSet::builtin().unwrap();
        let scale = scale_for(rules);
        let result =
            rules.assess(&[Applied::new("PM2", Strength::Supporting), Applied::new("BS4", Strength::Strong)]).unwrap();
        let slices = segments(&steps(&result.items), &scale);
        let shown = labelled(&slices, 180., 0.);
        assert_eq!(shown.iter().map(|s| s.code.as_str()).collect::<Vec<_>>(), ["BS4"], "BS4 is painted over PM2");
        let apart =
            rules.assess(&[Applied::new("PS1", Strength::Strong), Applied::new("PM2", Strength::Supporting)]).unwrap();
        assert_eq!(
            labelled(&segments(&steps(&apart.items), &scale), 180., 0.).len(),
            2,
            "end-to-end slices both keep theirs"
        );
    }

    #[test]
    fn slices_sit_end_to_end_and_a_click_finds_the_slice_under_it() {
        let rules = RuleSet::builtin().unwrap();
        let scale = scale_for(rules);
        let result =
            rules.assess(&[Applied::new("PS1", Strength::Strong), Applied::new("PM2", Strength::Supporting)]).unwrap();
        let slices = segments(&steps(&result.items), &scale);
        assert_eq!(slices.len(), 2);
        assert!(close(slices[0].start, angle_of(0.0, &scale)), "the first slice starts at zero");
        assert!(close(slices[0].end, slices[1].start), "the second starts where the first ends");
        let centre = (300., 300.);
        let ring = (200., 240.);
        let middle_of_first = (slices[0].start + slices[0].end) / 2.;
        let hit = segment_at(&slices, centre, ring, polar(centre, 220., middle_of_first));
        assert_eq!(hit.map(|s| s.code.as_str()), Some("PS1"));
        assert!(segment_at(&slices, centre, ring, polar(centre, 150., middle_of_first)).is_none(), "inside the ring");
        assert!(segment_at(&slices, centre, ring, (300., 320.)).is_none(), "below the centre line");
    }
}
