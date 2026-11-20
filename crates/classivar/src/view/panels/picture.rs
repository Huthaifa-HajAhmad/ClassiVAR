//! The signature picture of the Evidence view: the class drawn as a dial, a
//! column or a track (the curator picks one; the choice is remembered), with
//! the class readout and the keyboard path to every block.
//!
//! All three pictures draw the same numbers from the engine. The readout says
//! the class in words with its review status and rule version, so the class is
//! never read from colour alone and never shown without its status.

use classivar_core::Assessment;

use super::evidence_stack::{Step, steps};
use super::point_axis::points_text;
use super::*;
use crate::look::{MONO, display, eyebrow, figure, segment, segmented};
use crate::settings::Picture;

impl Workbench {
    /// The switch between the three pictures.
    pub(super) fn picture_switch(&self, cx: &mut Context<Self>) -> Div {
        let buttons: Vec<_> = Picture::ALL
            .into_iter()
            .map(|picture| {
                let id = format!("picture-{}", picture.name().to_lowercase());
                segment(id, picture.name(), self.picture == picture, cx)
                    .accessibility_label(format!("Show the class as a {}", picture.name().to_lowercase()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.picture = picture;
                        crate::settings::save_picture(picture);
                        cx.notify();
                    }))
            })
            .collect();
        segmented(buttons, cx)
    }

    /// The class in the chosen picture, the readout, and the buttons that select a block.
    pub(super) fn signature(&self, result: &Assessment, class: Class, cx: &mut Context<Self>) -> Div {
        let (width, height) = (self.stage_width.get(), self.stage_height.get());
        let centred = self.picture == Picture::Dial;
        let readout = self.readout(result, class, centred, cx);
        let picture = match self.picture {
            Picture::Dial => {
                let dial = self.dial(result, Some(class), (width, height), true, cx);
                div().flex().flex_col().items_center().gap_2().child(dial).child(readout)
            }
            Picture::Column => div()
                .flex()
                .items_center()
                .gap_8()
                .child(readout.flex_1().min_w(px(200.)))
                .child(self.column(result, class, cx)),
            Picture::Track => {
                div().flex().flex_col().gap_4().child(readout).child(self.evidence_track(result, class, cx))
            }
        };
        let steps = steps(&result.items);
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(picture)
            .when(self.picture != Picture::Track, |d| d.child(self.block_chips(&steps, cx)))
    }

    /// The class in words, the total, how far the next class is, and the review status.
    fn readout(&self, result: &Assessment, class: Class, centred: bool, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let sign = if result.total_points > 0 { "+" } else { "" };
        let distance = result.to_upgrade.map(|next| {
            let n = next.points_needed;
            format!("{n} point{} below {}", plural(n), next.class.label())
        });
        let words = div()
            .flex()
            .flex_col()
            .gap_1()
            .when(centred, |d| d.items_center())
            .child(eyebrow(self.scope_words(), cx))
            .child(display().child(class.label()))
            .when_some(result.vus_tier.clone(), |d, tier| d.child(div().text_base().child(tier)))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(figure(28.).child(format!("{sign}{}", points_text(result.total_points))))
                    .child(
                        div().text_sm().text_color(muted).child(format!("point{}", plural(result.total_points.abs()))),
                    )
                    .when_some(distance, |row, words| {
                        row.child(div().text_sm().text_color(muted).child(format!("· {words}")))
                    }),
            )
            .child(div().pt_1().child(status_pill(self.rules, cx)))
            .with_animation(
                SharedString::from(format!("readout-{class:?}")),
                Animation::new(std::time::Duration::from_millis(320)).with_easing(ease_out_quint()),
                |el, t| el.opacity(0.3 + 0.7 * t),
            );
        div().flex().flex_col().when(centred, |d| d.items_center()).child(words)
    }

    /// Whose evidence the class is for.
    pub(super) fn scope_words(&self) -> String {
        match self.editing {
            Editing::Today => "Today's assessment · your evidence".to_string(),
            Editing::Record(i) => format!("Earlier report · {} · {}", self.records[i].label, self.records[i].year),
        }
    }

    /// One chip per counted criterion: the keyboard way to the same choice as a click on the picture.
    pub(super) fn block_chips(&self, steps: &[Step], cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (ink, muted, border) = (theme.foreground, theme.muted_foreground, theme.border);
        let mut row = div().flex().flex_wrap().justify_center().gap_2();
        for step in steps {
            let code = step.code.clone();
            let selected = self.inspected.as_ref() == Some(&code);
            let colour = direction_colour(step.direction);
            row = row.child(
                Button::new(SharedString::from(format!("inspect-{code}")))
                    .small()
                    .ghost()
                    .rounded(px(999.))
                    .border_1()
                    .border_color(if selected { ink } else { border })
                    .accessibility_label(format!("Inspect {code}"))
                    .selected(selected)
                    .toggled(selected)
                    .child(div().size(px(8.)).rounded_full().bg(colour))
                    .child(figure(11.).text_color(ink).child(format!("{} {}", code, step.change())))
                    .child(div().text_xs().text_color(muted).child(step.strength.label()))
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

/// The review status and rule version, with a small attention dot while the rules are unverified.
pub(super) fn status_pill(rules: &RuleSet, cx: &App) -> Div {
    let unverified = rules.meta.status == ReviewStatus::Unverified;
    crate::look::pill(cx)
        .when(unverified, |pill| {
            pill.child(div().size(px(8.)).rounded_full().bg(warning_fill()).border_1().border_color(warning_ink()))
        })
        .child(div().font_family(MONO).child(rules_tag(rules)))
}
