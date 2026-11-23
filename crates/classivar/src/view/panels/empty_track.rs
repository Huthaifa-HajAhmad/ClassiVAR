//! The evidence track with nothing on it: the chosen picture before any evidence.

use super::point_axis::{fraction, tick_row, zone_labels};
use super::track::band_height;
use super::*;

impl Workbench {
    /// The track with no evidence: the class zones in grey, their names and the axis, nothing
    /// marked; `width` is the room for it, so a long zone name is shortened only when it must be.
    /// The class pointed at on the opening screen is coloured.
    pub(super) fn empty_track(&self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (pointed, dark) = (self.pointed_zone, theme.is_dark());
        let (ink, muted, fill, border) = (theme.foreground, theme.muted_foreground, theme.muted, theme.border);
        let scale = scale_for(self.rules);
        let mut band = div().relative().w_full().h(px(band_height(1))).rounded_lg().overflow_hidden().bg(fill);
        for (zone, from, to) in &scale.zones {
            if pointed == Some(*zone) {
                let (left, right) = (fraction(*from, &scale), fraction(*to, &scale));
                let colour: Hsla = class_fill(*zone, dark).into();
                band = band
                    .child(div().absolute().top_0().h_full().left(relative(left)).w(relative(right - left)).bg(colour));
            }
        }
        for (_, from, _) in scale.zones.iter().skip(1) {
            band = band
                .child(div().absolute().top_0().h_full().left(relative(fraction(*from, &scale))).w(px(1.)).bg(border));
        }
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(zone_labels(&scale, pointed, (ink, muted), Some(width)))
            .child(band)
            .child(tick_row(&scale, muted))
            .into_any_element()
    }
}
