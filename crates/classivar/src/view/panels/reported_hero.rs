//! The middle panel for an earlier report that lists no criteria (most ClinVar
//! imports): the class the lab stated, said as the lab's own, never as a class
//! ClassiVAR worked out.

use super::*;

impl Workbench {
    /// The open earlier report, when the lab stated a class but no criteria are applied to it yet.
    pub(in crate::view) fn reported_only(&self) -> Option<&Record> {
        match self.editing {
            Editing::Record(i) => self.records.get(i).filter(|r| r.class.is_some() && r.evidence.is_empty()),
            Editing::Today => None,
        }
    }

    /// The stated class in the same place and type as "No class yet", in ink and not on a
    /// class colour, so it never reads as a class ClassiVAR gave; then the way on.
    pub(super) fn hero_reported(&self, hero: Div, cx: &mut Context<Self>) -> Div {
        let Some(record) = self.reported_only() else { return hero };
        let Some(class) = record.class else { return hero };
        let muted = cx.theme().muted_foreground;
        hero.child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(crate::look::display().child(class.label()))
                .children(record.tier.clone().map(|tier| div().text_base().child(tier)))
                .child(div().pt_1().child(crate::look::pill(cx).child("Stated by the lab · not scored")))
                .child(div().pt_2().text_sm().text_center().text_color(muted).child(
                    "No criteria are applied yet. Choose a kind of evidence below to enter the ones the lab used.",
                ))
                .children(record.source.as_ref().map(|source| clinvar_card(source, cx))),
        )
        .child(self.category_picker(cx))
    }
}

/// Where an imported report came from, as labelled rows: easier to read than one sentence.
fn clinvar_card(source: &RecordSource, cx: &App) -> Div {
    let muted = cx.theme().muted_foreground;
    let named = source.stated_criteria.clone().unwrap_or_else(|| "None listed in ClinVar".to_string());
    let rows = [
        ("Submission", source.accession.clone()),
        ("Review status", source.review_status.clone()),
        ("Last evaluated", source.last_evaluated.clone().unwrap_or_else(|| "Not given".to_string())),
        ("Criteria named", named),
        ("Imported", source.imported_on.clone()),
    ];
    let mut card = crate::look::card(cx).mt_3().w_full().max_w(px(520.)).flex().flex_col().gap_1p5().text_sm().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .pb_1()
            .child(crate::icons::glyph(IconName::Inbox, cx))
            .child(crate::look::eyebrow("From ClinVar", cx)),
    );
    for (label, value) in rows {
        card = card.child(
            div()
                .flex()
                .gap_3()
                .child(div().w(px(120.)).flex_none().text_color(muted).child(label))
                .child(div().flex_1().min_w_0().whitespace_normal().child(value)),
        );
    }
    card
}
