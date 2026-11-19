//! The non-segregation helper (BS4), the BA1 reference, and the frame every helper panel starts from.

use super::*;
use classivar_core::{BA1_SOURCE, NonSegregationRules};

impl Workbench {
    /// The frame of an open helper panel: its title and a Close button. The
    /// helper is opened from the bar (`helper_bar.rs`); `toggle` flips its open flag.
    /// `ids` is the stem of the button id (`{stem}-close`) and the panel title.
    pub(in crate::view::panels) fn helper_start(
        &self,
        ids: (&'static str, &'static str),
        what: &'static str,
        toggle: fn(&mut Self),
        cx: &mut Context<Self>,
    ) -> Div {
        let (border, background) = (cx.theme().border, cx.theme().background);
        let (key, title) = ids;
        div().p_3().rounded_md().border_1().border_color(border).bg(background).flex().flex_col().gap_2().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(
                    Button::new(SharedString::from(format!("{key}-close")))
                        .label("Close")
                        .accessibility_label(format!("Close the {what}"))
                        .small()
                        .outline()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            toggle(this);
                            cx.notify();
                        })),
                ),
        )
    }

    /// Asks which situation fits and applies BS4 only where ClinGen 2024 supports
    /// it. Where it does not, the panel says why. Like the other helpers, it only
    /// fills in the criterion.
    pub(in crate::view::panels) fn non_segregation_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let panel = self.helper_start(
            ("segregation", "Non-segregation → BS4"),
            "non-segregation helper",
            |this| this.helpers.segregation.open = !this.helpers.segregation.open,
            cx,
        );
        let Some(rules) = self.rules.non_segregation.as_ref() else {
            return panel.into_any_element();
        };

        let situations = self.segregation_situations(rules, cx);
        let situation = &rules.situations[self.helpers.segregation.situation];
        let mut panel = panel.child(situations).child(div().text_sm().child(situation.text.clone()));
        if let Some(reading) = self.rules.read_non_segregation(&situation.id) {
            let label = format!("Apply {} {}", reading.code, reading.strength.label());
            let reason = format!(
                "Non-segregation: {} ({}, −{} points)",
                situation.label,
                rules.source,
                self.rules.magnitude(reading.strength)
            );
            panel = panel.child(
                div().child(
                    Button::new("segregation-apply")
                        .label(label.clone())
                        .accessibility_label(format!("{label} to today's evidence"))
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_reading(&reading, reason.clone());
                            cx.notify();
                        })),
                ),
            );
        }
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        panel
            .when_some(citation, |d, citation| d.child(caption(muted).child(format!("Source: {citation}"))))
            .into_any_element()
    }

    /// One button for each family situation the paper discusses.
    fn segregation_situations(&self, rules: &NonSegregationRules, cx: &mut Context<Self>) -> Div {
        let mut situations = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(caption(cx.theme().muted_foreground).child("What fits the family"));
        for (i, situation) in rules.situations.iter().enumerate() {
            let chosen = self.helpers.segregation.situation == i;
            situations = situations.child(
                Button::new(SharedString::from(format!("segregation-situation-{}", situation.id)))
                    .label(situation.label.clone())
                    .small()
                    .selected(chosen)
                    .toggled(chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.helpers.segregation.situation = i;
                        cx.notify();
                    })),
            );
        }
        situations
    }

    /// Shows the dated source list for the curator to check manually.
    pub(in crate::view::panels) fn ba1_reference(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut panel = self
            .helper_start(
                ("ba1-reference", "BA1 exceptions (July 2018)"),
                "BA1 exceptions reference",
                |this| this.helpers.ba1_open = !this.helpers.ba1_open,
                cx,
            )
            .child(caption(muted).child(
                "Manual reference, not a variant lookup. Check these exceptions before using BA1. Absence from this dated list does not establish eligibility. The list includes uncertain variants; HFE entries have reduced penetrance. It does not set a class or change your evidence."));
        for row in &self.rules.ba1_exceptions {
            panel = panel.child(
                div()
                    .text_xs()
                    .child(format!("{}: {} | ClinVar {} | {}", row.gene, row.variant, row.clinvar_id, row.allele_id)),
            );
        }
        if let Some(source) = self.rules.source(BA1_SOURCE) {
            panel = panel.child(caption(muted).child(format!("Source: {}", source.citation)));
        }
        panel.into_any_element()
    }
}
