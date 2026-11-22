//! The variant and condition rows of the case form: the check against the
//! ClinGen Allele Registry, the identity it found, the build choice for a
//! bare position, the condition and its inheritance, and the ClinVar import.
//!
//! The words are plain functions with tests; the drawing only lays them out.

use classivar_core::variant::{Build, VariantIdentity};
use classivar_core::{Condition, Inheritance};
use classivar_sources::allele_registry::Lookup as Found;

use super::*;
use crate::view::variant::curation_is_chosen;

/// A warning when the checked variant lies in another gene than the case's.
pub(super) fn gene_mismatch(identity: Option<&VariantIdentity>, case_gene: &str) -> Option<String> {
    let gene = identity?.gene.as_deref()?;
    (!case_gene.is_empty() && !gene.eq_ignore_ascii_case(case_gene))
        .then(|| format!("The checked variant is in {gene}, but the case's gene is {case_gene}. Check the gene box."))
}

impl Workbench {
    /// The variant row with its Check button, and what the check found under it.
    pub(super) fn variant_rows(&self, cx: &mut Context<Self>) -> Div {
        let row = div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_1().min_w_0().child(Input::new(&self.boxes.variant).aria_label("Variant name")));
        let check = Button::new("variant-check")
            .label("Check")
            .accessibility_label("Check the variant with the ClinGen Allele Registry")
            .small()
            .outline()
            .rounded(px(999.))
            .on_click(cx.listener(|this, _, _, cx| this.check_variant(cx)));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(super::records::form_row("Description", row.child(check), cx))
            .when(self.variant_needs_build(cx), |d| d.child(self.build_choice(cx)))
            .children(self.variant.problem.clone().map(|p| notice(p, warning_fill(), warning_ink())))
            .child(self.variant_answer(cx))
    }

    /// "Which build?" for a bare position. Nothing is chosen until the curator chooses.
    fn build_choice(&self, cx: &mut Context<Self>) -> Div {
        let segments: Vec<Button> = Build::ALL
            .into_iter()
            .map(|build| {
                crate::look::segment(format!("build-{build:?}"), build.label(), self.variant.build == Some(build), cx)
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_build(build, cx)))
            })
            .collect();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(caption(cx.theme().muted_foreground).child("Which genome build is this position on?"))
            .child(crate::look::segmented(segments, cx))
    }

    fn variant_answer(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let typed = self.variant_name(cx);
        let changed = self.variant.checked_text.is_some() && self.variant.checked_text != typed;
        let mut column = div().flex().flex_col().gap_1();
        match &self.variant.lookup {
            Lookup::Loading => {
                column = column.child(caption(muted).child("Checking with the ClinGen Allele Registry…"))
            }
            Lookup::Failed(message) => {
                column = column.child(notice(
                    format!("Could not check the variant: {message}"),
                    warning_fill(),
                    warning_ink(),
                ))
            }
            Lookup::Done(found) => match &found.value {
                Found::NotFound => {
                    column =
                        column.child(notice("The Allele Registry has no such allele.", warning_fill(), warning_ink()))
                }
                Found::Found(list) if list.len() > 1 => column = column.child(self.allele_choices(list, cx)),
                Found::Found(_) => {}
            },
            Lookup::Idle => {}
        }
        if let Some(identity) = &self.variant.identity {
            column = column.child(self.identity_card(identity, changed, cx));
        }
        column
            .children(
                gene_mismatch(self.variant.identity.as_ref(), &self.case_gene)
                    .map(|w| notice(w, warning_fill(), warning_ink())),
            )
            .into_any_element()
    }

    /// Several alleles for one id (a multi-allelic rsID): the curator picks one.
    fn allele_choices(&self, list: &[VariantIdentity], cx: &mut Context<Self>) -> Div {
        let mut column = div().flex().flex_col().gap_1().child(
            caption(cx.theme().muted_foreground)
                .child("This id names several different variants. None is chosen for you; pick the one you mean."),
        );
        for (i, identity) in list.iter().enumerate() {
            let name = identity.display_name().unwrap_or("an allele without HGVS").to_string();
            let chosen = identity.clone();
            column = column.child(
                Button::new(SharedString::from(format!("allele-choice-{i}")))
                    .label(name.clone())
                    .accessibility_label(format!("Use {name} as the case's variant"))
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_identity(chosen.clone(), cx))),
            );
        }
        column
    }

    /// The condition row: a name, its inheritance, and the ClinGen curations of
    /// the checked gene to choose from.
    pub(super) fn condition_rows(&self, cx: &mut Context<Self>) -> Div {
        let chosen = self.variant.inheritance();
        let segments: Vec<Button> = Inheritance::ALL
            .into_iter()
            .map(|inheritance| {
                crate::look::segment(
                    format!("inheritance-{}", inheritance.short()),
                    inheritance.short(),
                    chosen == inheritance,
                    cx,
                )
                .tooltip(inheritance.label())
                .accessibility_label(format!("Inheritance: {}", inheritance.label()))
                .on_click(cx.listener(move |this, _, _, cx| this.set_inheritance(inheritance, cx)))
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(super::records::form_row(
                "Name",
                div().w_full().child(Input::new(&self.variant.condition_input).aria_label("Condition")),
                cx,
            ))
            .child(super::records::form_row("Inheritance", crate::look::segmented(segments, cx), cx))
            .children(self.condition_from_clingen(cx))
    }

    /// Buttons that take a ClinGen curation of the checked gene as the condition.
    fn condition_from_clingen(&self, cx: &mut Context<Self>) -> Option<Div> {
        let Lookup::Done(found) = &self.gene.clingen else { return None };
        let curations = &found.value.curations;
        if curations.is_empty() {
            return None;
        }
        let chosen_now = self.variant.condition.as_ref();
        let mut pills = div().flex().flex_wrap().gap_2();
        for (i, curation) in curations.iter().take(4).enumerate() {
            let label = format!("{} ({}, {})", curation.disease, curation.inheritance, curation.classification);
            // The pill of the condition now set is filled and ticked; the others offer "+".
            let chosen = curation_is_chosen(chosen_now, curation);
            let picked = curation.clone();
            pills = pills.child(
                crate::look::choice_pill(format!("condition-clingen-{i}"), &label, chosen, cx)
                    .accessibility_label(format!("Use {label} as the case's condition"))
                    .tooltip(label.clone())
                    .on_click(cx.listener(move |this, _, window, cx| this.use_curation(&picked, window, cx))),
            );
        }
        let row = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(caption(cx.theme().muted_foreground).child("Or choose a condition ClinGen rated for this gene:"))
            .child(pills);
        Some(row)
    }

    /// The ClinVar import button and its state, for the reported-variant path.
    pub(super) fn clinvar_import_row(&self, cx: &mut Context<Self>) -> Option<Div> {
        let id = self.clinvar_variation_id()?;
        let muted = cx.theme().muted_foreground;
        let state = match &self.variant.clinvar {
            Lookup::Loading => Some(caption(muted).child("Reading ClinVar…")),
            Lookup::Failed(message) => {
                Some(notice(format!("Could not read ClinVar: {message}"), warning_fill(), warning_ink()))
            }
            _ => None,
        };
        let label = format!("Import labs from ClinVar VCV{id:09}");
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("clinvar-import")
                        .label(label.clone())
                        .accessibility_label(format!("{label} as earlier records"))
                        .small()
                        .outline()
                        .on_click(cx.listener(|this, _, _, cx| this.import_clinvar(cx))),
                )
                .children(state),
        )
    }

    /// Under the record form: the record's condition, how it compares with the
    /// case's, and where an imported record came from.
    pub(super) fn record_condition_rows(&self, index: usize, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let Some(record) = self.records.get(index) else { return div() };
        let compared = Condition::compare(self.variant.condition.as_ref(), record.condition.as_ref());
        let note = match compared {
            classivar_core::ConditionMatch::Different => Some(notice(
                "This record is for a different condition than the case, so it is shown but never compared.",
                warning_fill(),
                warning_ink(),
            )),
            classivar_core::ConditionMatch::Unknown
                if record.condition.is_some() && self.variant.condition.is_some() =>
            {
                Some(caption(muted).child("The names differ: check that this is the case's condition."))
            }
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(super::records::form_row(
                "Condition",
                div()
                    .w_full()
                    .child(Input::new(&self.variant.record_condition_input).aria_label("Condition the lab named")),
                cx,
            ))
            .children(note)
            .children(record.source.as_ref().map(|source| caption(muted).child(source_text(source))))
    }
}

/// Where an imported record came from, in one sentence.
pub(super) fn source_text(source: &classivar_core::RecordSource) -> String {
    let evaluated = source.last_evaluated.as_deref().map(|d| format!(", last evaluated {d}")).unwrap_or_default();
    let stated = source
        .stated_criteria
        .as_deref()
        .map(|codes| format!(" The lab's comment names: {codes}. Apply only those you can check."))
        .unwrap_or_else(|| " ClinVar does not list the criteria this lab used.".to_string());
    format!(
        "From ClinVar {} ({}{evaluated}), imported {}.{stated}",
        source.accession, source.review_status, source.imported_on
    )
}

#[cfg(test)]
mod tests {
    // Explicit imports: `use super::*` would bring in GPUI's own `#[test]` macro.
    use classivar_core::variant::VariantIdentity;

    use super::super::identity_card::identity_view;
    use super::{gene_mismatch, source_text};

    fn brca1() -> VariantIdentity {
        VariantIdentity {
            allele_id: Some("CA003404".into()),
            grch38: Some("NC_000017.11:g.43057065dup".into()),
            grch37: None,
            mane_select: Some("NM_007294.4:c.5266dup".into()),
            protein: None,
            gene: Some("BRCA1".into()),
            clinvar_variation: Some(17677),
            dbsnp: Some(80357906),
            checked_with: "ClinGen Allele Registry".into(),
            checked_on: "2026-11-21".into(),
        }
    }

    #[test]
    fn the_identity_card_names_only_what_the_registry_gave() {
        let view = identity_view(&brca1());
        assert_eq!(view.heading, "MANE Select");
        assert_eq!(view.name.as_deref(), Some("NM_007294.4:c.5266dup"));
        assert_eq!(view.gene.as_deref(), Some("BRCA1"));
        assert_eq!(view.builds, vec![("GRCh38", "NC_000017.11:g.43057065dup".to_string())], "no GRCh37 was given");
        assert_eq!(view.ids, ["CA003404", "VCV000017677", "rs80357906"]);
        assert_eq!(view.checked, "ClinGen Allele Registry · 2026-11-21");
        let unregistered = VariantIdentity { allele_id: None, ..brca1() };
        assert!(identity_view(&unregistered).ids.iter().any(|id| id.contains("not yet registered")));
    }

    #[test]
    fn without_a_mane_select_transcript_the_genomic_name_leads_and_is_not_repeated() {
        let genomic = VariantIdentity { mane_select: None, gene: None, ..brca1() };
        let view = identity_view(&genomic);
        assert_eq!(view.heading, "Genomic, GRCh38");
        assert_eq!(view.name.as_deref(), Some("NC_000017.11:g.43057065dup"));
        assert!(view.builds.is_empty(), "the name is not repeated as a build row");
    }

    #[test]
    fn a_variant_in_another_gene_than_the_case_is_flagged() {
        let identity = brca1();
        assert!(gene_mismatch(Some(&identity), "BRCA2").is_some_and(|w| w.contains("BRCA1") && w.contains("BRCA2")));
        assert_eq!(gene_mismatch(Some(&identity), "brca1"), None);
        assert_eq!(gene_mismatch(Some(&identity), ""), None, "no gene checked yet");
        assert_eq!(gene_mismatch(None, "BRCA2"), None);
    }

    #[test]
    fn an_imported_record_says_where_it_came_from_and_what_the_lab_named() {
        let source = classivar_core::RecordSource {
            accession: "SCV000271320".into(),
            review_status: "criteria provided, single submitter".into(),
            last_evaluated: Some("2022-11-23".into()),
            stated_criteria: Some("PS4, PVS1".into()),
            imported_on: "2026-11-21".into(),
        };
        let text = source_text(&source);
        assert!(text.contains("SCV000271320") && text.contains("2022-11-23"), "{text}");
        assert!(text.contains("names: PS4, PVS1. Apply only those you can check."), "{text}");
        let silent = classivar_core::RecordSource { stated_criteria: None, ..source };
        assert!(source_text(&silent).contains("does not list the criteria"));
    }
}
