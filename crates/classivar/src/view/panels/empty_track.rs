//! The evidence track with nothing on it: the chosen picture before any evidence.

use super::point_axis::{fraction, tick_row, zone_labels};
use super::track::band_height;
use super::*;

impl Workbench {
    /// The track with no evidence: the class zones in grey, their names and the axis, nothing marked.
    pub(super) fn empty_track(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (ink, muted, fill, border) = (theme.foreground, theme.muted_foreground, theme.muted, theme.border);
        let scale = scale_for(self.rules);
        let mut band = div().relative().w_full().h(px(band_height(1))).rounded_lg().overflow_hidden().bg(fill);
        for (_, from, _) in scale.zones.iter().skip(1) {
            band = band
                .child(div().absolute().top_0().h_full().left(relative(fraction(*from, &scale))).w(px(1.)).bg(border));
        }
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(zone_labels(&scale, None, (ink, muted), Some(self.stage.width.get())))
            .child(band)
            .child(tick_row(&scale, muted))
            .into_any_element()
    }
}
