//! The non-segregation helper (BS4), the BA1 reference, and the frame every helper panel starts from.

use super::*;
use classivar_core::{BA1_SOURCE, NonSegregationRules};

impl Workbench {
    /// The column every helper panel starts from. Its title and Back button are in the
    /// head of the helper's page (`helper_cards.rs`).
    pub(in crate::view::panels) fn helper_start(&self) -> Div {
        div().flex().flex_col().gap_3()
    }

    /// Asks which situation fits and applies BS4 only where ClinGen 2024 supports
    /// it. Where it does not, the panel says why. Like the other helpers, it only
    /// fills in the criterion.
    pub(in crate::view::panels) fn non_segregation_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let panel = self.helper_start();
        let Some(rules) = self.rules.non_segregation.as_ref() else {
            return panel.into_any_element();
        };

        let situation = &rules.situations[self.helpers.segregation.situation];
        let apply = self.rules.read_non_segregation(&situation.id).map(|reading| {
            let reason = format!(
                "Non-segregation: {} ({}, −{} points)",
                situation.label,
                rules.source,
                self.rules.magnitude(reading.strength)
            );
            self.reading_apply("segregation-apply", reading, reason, cx)
        });
        let result = match apply {
            Some(button) => div().child(button),
            None => reading_note("This situation gives no BS4: the paper does not support it here.", muted),
        };
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        panel
            .gap_6()
            .child(self.segregation_situations(rules, cx))
            .child(result_card(result, cx))
            .children(source_part(citation, cx))
            .into_any_element()
    }

    /// One card for each family situation the paper discusses, with what it gives.
    fn segregation_situations(&self, rules: &NonSegregationRules, cx: &mut Context<Self>) -> Div {
        let mut situations = part("What fits the family?", cx);
        for (i, situation) in rules.situations.iter().enumerate() {
            let chosen = self.helpers.segregation.situation == i;
            let gives = self.rules.read_non_segregation(&situation.id).map_or("No BS4".to_string(), |reading| {
                format!("{} {} · −{}", reading.code, reading.strength.label(), self.rules.magnitude(reading.strength))
            });
            situations = situations.child(
                super::super::focus::choice_card(
                    format!("segregation-situation-{}", situation.id),
                    (situation.label.clone(), situation.text.clone(), gives),
                    chosen,
                    cx,
                )
                .accessibility_label(situation.label.clone())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.helpers.segregation.situation = i;
                    cx.notify();
                })),
            );
        }
        situations
    }

    /// Shows the dated source list for the curator to check manually: one line per variant.
    pub(in crate::view::panels) fn ba1_reference(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        let rows = self.rules.ba1_exceptions.iter().map(|row| {
            div()
                .flex()
                .items_baseline()
                .gap_3()
                .py_1p5()
                .border_b_1()
                .border_color(border)
                .child(div().w(px(64.)).flex_none().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.gene.clone()))
                .child(div().flex_1().min_w_0().text_sm().whitespace_normal().child(row.variant.clone()))
                .child(
                    crate::look::figure(crate::look::SMALL_TEXT)
                        .flex_none()
                        .text_color(muted)
                        .font_weight(FontWeight::NORMAL)
                        .child(format!("ClinVar {} · {}", row.clinvar_id, row.allele_id)),
                )
        });
        let citation = self.rules.source(BA1_SOURCE).map(|source| source.citation.clone());
        self.helper_start()
            .gap_6()
            .child(reading_note(
                "Manual reference, not a variant lookup. Check these exceptions before using BA1. Absence from this dated list does not establish eligibility. The list includes uncertain variants; HFE entries have reduced penetrance. It does not set a class or change your evidence.",
                muted,
            ))
            .child(part("Variants that are common but not benign", cx).gap_0().children(rows))
            .children(source_part(citation, cx))
            .into_any_element()
    }
}
