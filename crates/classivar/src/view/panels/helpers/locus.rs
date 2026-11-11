//! The family and phenotype helper: PP1 and PP4 points and the strengths the paper allows.
//! What the boxes mean is worked out in `locus_read.rs`; this file draws.

use super::locus_read::{LocusInputs, LocusTyped, locus_summary, read_locus_inputs};
use super::*;
use classivar_core::{LocusChoice, LocusRules};

/// The button text and the stable id part for one strength choice, for example "PP1 Strong + PP4 Supporting".
fn choice_label(choice: LocusChoice) -> String {
    [("PP1", choice.pp1), ("PP4", choice.pp4)]
        .into_iter()
        .filter_map(|(code, strength)| strength.map(|s| format!("{code} {}", s.label())))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn choice_id(choice: LocusChoice) -> String {
    let slug = |strength: Option<Strength>| {
        strength.map_or("none".to_string(), |s| s.label().to_lowercase().replace(' ', "-"))
    };
    format!("locus-choice-{}-{}", slug(choice.pp1), slug(choice.pp4))
}

impl Workbench {
    /// Turns family and phenotype facts into PP1 and PP4 strengths with the
    /// arithmetic of ClinGen 2024. It lists the choices the paper allows for the
    /// total; the curator picks one. Like the other helpers, it only fills in criteria.
    pub(in crate::view::panels) fn locus_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let panel = self.helper_start(
            ("locus", "Family and phenotype → PP1 and PP4"),
            "co-segregation and phenotype helper",
            |this| this.helpers.locus.open = !this.helpers.locus.open,
            cx,
        );
        let Some(rules) = self.rules.locus_evidence.as_ref() else {
            return panel.into_any_element();
        };
        let state = &self.helpers.locus;
        let typed = LocusTyped {
            yield_percent: self.typed_number(&state.yield_input, cx),
            family: self.typed_count(&state.segregations_input, cx),
            variants: self.typed_count(&state.variants_input, cx),
            genes: state.genes,
            kind: state.kind,
        };
        let inputs = read_locus_inputs(self.rules, rules, &typed);
        panel
            .child(self.locus_phenotype_part(rules, &inputs, cx))
            .child(self.locus_family_part(rules, &inputs, cx))
            .child(self.locus_result(rules, &inputs, cx))
            .into_any_element()
    }

    /// The PP4 half: the diagnostic yield, how many variants share the allele, and the paper's cautions.
    fn locus_phenotype_part(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let row = |label: &'static str, input: &Entity<InputState>, aria: &'static str| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(caption(muted).child(label))
                .child(div().w(px(120.)).child(Input::new(input).aria_label(aria)))
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                caption(muted).child(
                    "Fill in one or both parts. An empty box counts as 0 points. The paper divides the yield, never the points, when several variants share an allele.",
                ),
            )
            .child(row(
                "Diagnostic yield of testing for the phenotype (%)",
                &self.helpers.locus.yield_input,
                "Diagnostic yield, percent",
            ))
            .child(row(
                "Variants on the same allele (1 if alone)",
                &self.helpers.locus.variants_input,
                "Variants sharing the allele",
            ))
            .child(div().text_sm().child(inputs.pp4_note.clone()))
            .children(rules.notes.iter().map(|note| caption(muted).child(note.clone())))
    }

    /// The PP1 half: how many genes can cause the phenotype (it decides whether PP1 counts),
    /// the kind of family, and how many informative members there are.
    fn locus_family_part(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let state = &self.helpers.locus;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.locus_genes_question(cx))
            .when(state.genes == PhenotypeGenes::One, |d| d.child(caption(muted).child(rules.homogeneity_text.clone())))
            .child(self.locus_kinds(rules, cx))
            .when_some(rules.segregation[state.kind].note.clone(), |d, note| d.child(caption(muted).child(note)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(caption(muted).child("Informative family members (see the note)"))
                    .child(
                        div()
                            .w(px(120.))
                            .child(Input::new(&state.segregations_input).aria_label("Informative family member count")),
                    ),
            )
            .child(div().text_sm().child(inputs.pp1_note.clone()))
    }

    /// "Can other genes cause this phenotype?" It decides whether PP1 counts at all.
    fn locus_genes_question(&self, cx: &mut Context<Self>) -> Div {
        let genes_button = |id: &'static str, label: &'static str, genes: PhenotypeGenes, cx: &mut Context<Self>| {
            let chosen = self.helpers.locus.genes == genes;
            Button::new(id).label(label).small().selected(chosen).toggled(chosen).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.helpers.locus.genes = genes;
                    cx.notify();
                },
            ))
        };
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(caption(cx.theme().muted_foreground).child("Can other genes cause this phenotype?"))
            .child(genes_button("locus-genes-several", "Several genes can cause it", PhenotypeGenes::Several, cx))
            .child(genes_button("locus-genes-one", "Only this gene causes it", PhenotypeGenes::One, cx))
    }

    /// One button for each kind of family evidence the rules give points for.
    fn locus_kinds(&self, rules: &LocusRules, cx: &mut Context<Self>) -> Div {
        let mut kinds =
            div().flex().flex_wrap().items_center().gap_2().child(caption(cx.theme().muted_foreground).child("Family"));
        for (i, kind) in rules.segregation.iter().enumerate() {
            let chosen = self.helpers.locus.kind == i;
            kinds = kinds.child(
                Button::new(SharedString::from(format!("locus-kind-{}", kind.id)))
                    .label(kind.label.clone())
                    .small()
                    .selected(chosen)
                    .toggled(chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.helpers.locus.kind = i;
                        cx.notify();
                    })),
            );
        }
        kinds
    }

    /// The total, the choices the paper allows for it, and the source.
    fn locus_result(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let reading = (!inputs.unusable && inputs.typed_any)
            .then(|| self.rules.read_locus(inputs.pp1, inputs.pp4, inputs.pp1_allowed))
            .flatten();
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        let mut result = div().flex().flex_col().gap_2();
        if let Some(reading) = reading {
            let (summary, reason) = locus_summary(rules, &reading, inputs);
            let mut choices = div().flex().flex_wrap().gap_2();
            for choice in reading.choices {
                let (label, reason) = (choice_label(choice), reason.clone());
                choices = choices.child(
                    Button::new(SharedString::from(choice_id(choice)))
                        .label(format!("Apply {label}"))
                        .accessibility_label(format!("Apply {label} to today's evidence"))
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_locus_choice(choice, reason.clone());
                            cx.notify();
                        })),
                );
            }
            result = result.child(div().text_sm().child(summary)).child(choices);
        } else if !inputs.unusable {
            result = result.child(div().text_sm().child("Fill in at least one box to see the points."));
        }
        result.when_some(citation, |d, citation| d.child(caption(muted).child(format!("Source: {citation}"))))
    }
}
