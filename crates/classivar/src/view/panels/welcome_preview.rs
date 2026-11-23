//! The preview on the opening screen: the chosen picture with no evidence on it,
//! and a key of the five classes. Pointing at a class in the key (or clicking it)
//! colours that class in the picture and says which totals give it. Nothing here
//! is a variant or a class for one: the picture stays empty of evidence.

use super::point_axis::points_text;
use super::*;
use crate::look::{eyebrow, figure};
use crate::settings::Picture;
use crate::view::describe::{Scale, scale_for};

/// The height the preview keeps for every picture, so choosing another does not move the page.
const PREVIEW_HEIGHT: f32 = 400.;
/// The width the dial and the track may use inside the preview card.
const PREVIEW_ROOM: (f32, f32) = (560., 380.);

/// The totals that give a class, in words: "6 to 9 points", "−7 or fewer points".
/// `from` is the class's first point and `to` the next class's first point, as in `Scale::zones`.
fn zone_range(from: i32, to: i32, scale: &Scale) -> String {
    if from == scale.min {
        format!("{} or fewer points", points_text(to - 1))
    } else if to == scale.max {
        format!("{} or more points", points_text(from))
    } else {
        format!("{} to {} points", points_text(from), points_text(to - 1))
    }
}

/// What the preview says under the picture: the class pointed at and its totals, or how the picture moves.
fn preview_words(picture: Picture, pointed: Option<(Class, String)>) -> String {
    if let Some((class, range)) = pointed {
        return format!("{}: {range}. A total in this range gives this class.", class.label());
    }
    let moves = match picture {
        Picture::Dial => "Each criterion you choose moves the needle.",
        Picture::Column => "Each criterion you choose stacks a block on the column.",
        Picture::Track => "Each criterion you choose lays a block along the scale.",
    };
    format!("{moves} No class is shown until you choose one. Point at a class below to see its points.")
}

impl Workbench {
    /// The chosen picture with no evidence on it, so the opening screen shows the
    /// instrument without suggesting any class, and the key under it.
    pub(super) fn empty_figure(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let card = crate::look::panel(cx)
            .flex_1()
            .min_w_0()
            .p_6()
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .child(eyebrow("How the evidence adds up", cx));
        let Ok(result) = self.rules.assess(&[]) else {
            return card;
        };
        let picture = match self.picture {
            Picture::Dial => self.dial(&result, None, PREVIEW_ROOM, false, cx),
            Picture::Column => self.empty_column(cx),
            Picture::Track => self.empty_track(PREVIEW_ROOM.0, cx),
        };
        let id = format!("welcome-preview-{}", self.picture.name().to_lowercase());
        let scale = scale_for(self.rules);
        let pointed = self.pointed_zone.and_then(|class| {
            let (_, from, to) = scale.zones.iter().find(|(zone, _, _)| *zone == class)?;
            Some((class, zone_range(*from, *to, &scale)))
        });
        card.child(
            div()
                .debug_selector(move || id.clone())
                .w_full()
                .h(px(PREVIEW_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .child(picture),
        )
        .child(self.zone_key(&scale, cx))
        .child(caption(muted).text_center().child(preview_words(self.picture, pointed)))
        .child(super::picture::status_pill(self.rules, cx))
    }

    /// One small button per class, in scale order. Pointing at one (or clicking it) marks that class.
    fn zone_key(&self, scale: &Scale, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (ink, line, dark) = (theme.foreground, theme.border, theme.is_dark());
        let mut key = div().flex().flex_wrap().justify_center().gap_2();
        for (zone, from, to) in &scale.zones {
            let (zone, chosen) = (*zone, self.pointed_zone == Some(*zone));
            let range = zone_range(*from, *to, scale);
            let id = format!("welcome-zone-{}", zone.short().to_lowercase());
            let selector = id.clone();
            let button = Button::new(SharedString::from(id))
                .small()
                .ghost()
                .rounded(px(999.))
                .border_1()
                .border_color(if chosen { crate::look::edge(cx) } else { line })
                .accessibility_label(format!("{}: {range}", zone.label()))
                .selected(chosen)
                .toggled(chosen)
                .child(div().size(px(10.)).rounded_full().bg(class_fill(zone, dark)))
                .child(figure(11.).text_color(ink).child(zone.short()))
                .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                    if *over && this.pointed_zone != Some(zone) {
                        this.pointed_zone = Some(zone);
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.pointed_zone = Some(zone);
                    cx.notify();
                }));
            key = key.child(div().debug_selector(move || selector).child(button));
        }
        key
    }
}

#[cfg(test)]
mod tests {
    use super::{preview_words, zone_range};
    use crate::settings::Picture;
    use crate::view::describe::scale_for;
    use classivar_core::{Class, RuleSet};

    #[test]
    fn each_class_names_the_totals_that_give_it() {
        let rules = RuleSet::builtin().expect("the built-in rules are valid");
        let scale = scale_for(rules);
        let words: Vec<String> = scale.zones.iter().map(|(_, from, to)| zone_range(*from, *to, &scale)).collect();
        let t = &rules.class_min_points;
        assert_eq!(words[0], format!("−{} or fewer points", -(t.likely_benign - 1)));
        assert_eq!(words[2], format!("{} to {} points", t.vus, t.likely_pathogenic - 1));
        assert_eq!(words[4], format!("{} or more points", t.pathogenic));
    }

    #[test]
    fn the_words_follow_the_picture_until_a_class_is_pointed_at() {
        assert!(preview_words(Picture::Dial, None).contains("needle"));
        assert!(preview_words(Picture::Column, None).contains("column"));
        assert!(preview_words(Picture::Track, None).contains("along the scale"));
        let pointed = preview_words(Picture::Track, Some((Class::LikelyPathogenic, "6 to 9 points".into())));
        assert!(pointed.starts_with("Likely pathogenic: 6 to 9 points."));
    }
}
