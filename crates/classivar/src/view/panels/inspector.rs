//! Inspect counted evidence and explore its effect on today's result.
use classivar_core::Item;

use super::point_axis::points_text;
use super::*;

impl Workbench {
    pub(in crate::view) fn preview_without(&self, code: &str) -> Result<Assessment, classivar_core::AssessError> {
        let mut evidence = to_applied(&self.today);
        evidence.retain(|item| item.code != code);
        self.rules.assess(&evidence)
    }

    pub(super) fn inspector(&self, assessment: &Assessment, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let item = assessment.items.iter().find(|item| item.counted && Some(&item.code) == self.inspected.as_ref());
        let found =
            item.and_then(|item| Some((item, self.rules.criterion(&item.code)?, self.current().get(&item.code)?)));
        let Some((item, criterion, choice)) = found else {
            let hint = if self.editing == Editing::Today {
                "Click a block to see why it counts. Click it again to see the class without it."
            } else {
                "Click a block to see why it counts."
            };
            return div().flex().justify_center().child(caption(muted).child(hint));
        };
        let mut body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(inspector_title(item))
            .child(div().text_base().child(criterion.summary.clone()));
        if let Some(reason) = &choice.reason {
            body = body.child(div().text_sm().child(format!("Recorded reason: {reason}")));
        }
        if let Some(source) = self.rules.source(&criterion.source) {
            body = body.child(caption(muted).child(source.citation.clone()));
        }
        if self.editing == Editing::Today {
            body = body.child(self.preview_controls(assessment, &criterion.code, cx));
        }
        div()
            .flex()
            .rounded(px(crate::look::CARD_RADIUS))
            .overflow_hidden()
            .bg(cx.theme().muted)
            .child(div().w(px(4.)).flex_none().bg(direction_colour(item.direction)))
            .child(body.p_4())
    }

    /// The preview switch and, while it is on, today's result next to the result without the criterion.
    fn preview_controls(&self, assessment: &Assessment, code: &str, cx: &mut Context<Self>) -> Div {
        let toggle = Button::new("preview-without")
            .small()
            .outline()
            .label(if self.preview { "Close preview".to_string() } else { format!("Preview without {code}") })
            .on_click(cx.listener(|this, _, _, cx| {
                this.preview = !this.preview;
                cx.notify();
            }));
        let panel = div().flex().flex_col().gap_3().pt_1().child(div().flex().child(toggle));
        if !self.preview {
            return panel;
        }
        let outcome = self.preview_without(code);
        let without = match &outcome {
            Ok(result) if result.class().is_none() && result.has_evidence => {
                return panel.child(self.hero(result, Some("Preview"), cx));
            }
            other => outcome_tile(&format!("Without {code}"), other, cx),
        };
        panel.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(outcome_tile("Your evidence", &Ok(assessment.clone()), cx))
                .child(div().text_xl().child("→"))
                .child(without)
                .child(caption(cx.theme().muted_foreground).child(format!("Preview only. {}", rules_tag(self.rules)))),
        )
    }
}

/// "PM5" on its direction colour, then the strength and the points it adds.
fn inspector_title(item: &Item) -> Div {
    let sign = if item.points > 0 { "+" } else { "" };
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            crate::look::figure(13.)
                .px_2()
                .py(px(2.))
                .rounded(px(7.))
                .bg(direction_colour(item.direction))
                .text_color(direction_ink())
                .child(item.code.clone()),
        )
        .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(format!(
            "{} · {sign}{} point{}",
            item.strength.label(),
            points_text(item.points),
            plural(item.points.abs())
        )))
}

/// One result in the preview: its class on the class colour and its points, or why there is none.
fn outcome_tile(
    title: &str,
    outcome: &Result<Assessment, classivar_core::AssessError>,
    cx: &Context<Workbench>,
) -> Div {
    let dark = cx.theme().is_dark();
    let tile = div().min_w(px(150.)).px_3().py_2().rounded(px(crate::look::CARD_RADIUS)).flex().flex_col().gap_1();
    let (class, line) = match outcome {
        Ok(result) => match result.class() {
            Some(class) => (Some(class), format!("{} points", points_text(result.total_points))),
            None => (None, "No class: no counted evidence".to_string()),
        },
        Err(error) => (None, format!("Cannot be scored: {error}")),
    };
    let tile = match class {
        Some(class) => tile.bg(class_fill(class, dark)).text_color(class_ink(class, dark)),
        None => tile.bg(warning_fill()).text_color(warning_ink()),
    };
    tile.child(div().text_xs().child(title.to_string()))
        .child(
            div().font_weight(FontWeight::BOLD).child(class.map_or("No class".to_string(), |c| c.label().to_string())),
        )
        .child(div().text_xs().child(line))
}
