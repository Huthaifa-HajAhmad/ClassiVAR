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
        let panel = self.helper_start();
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
        let muted = cx.theme().muted_foreground;
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        panel
            .gap_6()
            .child(reading_note(
                "Fill in one or both parts. An empty box counts as 0 points. The paper divides the yield, never the points, when several variants share an allele.",
                muted,
            ))
            .child(self.locus_phenotype_part(&inputs, cx))
            .child(self.locus_family_part(rules, &inputs, cx))
            .child(result_card(self.locus_result(&inputs, cx), cx))
            .children(notes_part(rules.notes.iter().cloned(), cx))
            .children(source_part(citation, cx))
            .into_any_element()
    }

    /// The PP4 half: the diagnostic yield, how many variants share the allele, and the paper's cautions.
    fn locus_phenotype_part(&self, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let locus = &self.helpers.locus;
        part("The phenotype · PP4", cx)
            .gap_3()
            .child(fields([
                field(
                    "Diagnostic yield of testing for the phenotype (%)",
                    Input::new(&locus.yield_input).aria_label("Diagnostic yield, percent"),
                    muted,
                ),
                field(
                    "Variants on the same allele (1 if alone)",
                    Input::new(&locus.variants_input).aria_label("Variants sharing the allele"),
                    muted,
                ),
            ]))
            .child(super::super::focus::reading(inputs.pp4_note.clone()))
    }

    /// The PP1 half: how many genes can cause the phenotype (it decides whether PP1 counts),
    /// the kind of family, and how many informative members there are.
    fn locus_family_part(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let state = &self.helpers.locus;
        part("The family · PP1", cx)
            .gap_3()
            .child(self.locus_genes_question(cx))
            .when(state.genes == PhenotypeGenes::One, |d| d.child(reading_note(rules.homogeneity_text.clone(), muted)))
            .child(self.locus_kinds(rules, cx))
            .when_some(rules.segregation[state.kind].note.clone(), |d, note| d.child(reading_note(note, muted)))
            .child(fields([field(
                "Informative family members (see the note)",
                Input::new(&state.segregations_input).aria_label("Informative family member count"),
                muted,
            )]))
            .child(super::super::focus::reading(inputs.pp1_note.clone()))
    }

    /// "Can other genes cause this phenotype?" It decides whether PP1 counts at all.
    fn locus_genes_question(&self, cx: &mut Context<Self>) -> Div {
        let answers = [
            ("locus-genes-several", "Several genes can cause it", PhenotypeGenes::Several),
            ("locus-genes-one", "Only this gene causes it", PhenotypeGenes::One),
        ]
        .into_iter()
        .map(|(id, label, genes)| {
            answer(id, label, self.helpers.locus.genes == genes, cx).on_click(cx.listener(move |this, _, _, cx| {
                this.helpers.locus.genes = genes;
                cx.notify();
            }))
        })
        .collect();
        question("Can other genes cause this phenotype?", answers, cx)
    }

    /// One button for each kind of family evidence the rules give points for.
    fn locus_kinds(&self, rules: &LocusRules, cx: &mut Context<Self>) -> Div {
        let mut kinds = Vec::new();
        for (i, kind) in rules.segregation.iter().enumerate() {
            let chosen = self.helpers.locus.kind == i;
            kinds.push(answer(format!("locus-kind-{}", kind.id), kind.label.clone(), chosen, cx).on_click(
                cx.listener(move |this, _, _, cx| {
                    this.helpers.locus.kind = i;
                    cx.notify();
                }),
            ));
        }
        question("What kind of family?", kinds, cx)
    }

    /// The total and the choices the paper allows for it.
    fn locus_result(&self, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let Some(rules) = self.rules.locus_evidence.as_ref() else { return div() };
        let reading = (!inputs.unusable && inputs.typed_any)
            .then(|| self.rules.read_locus(inputs.pp1, inputs.pp4, inputs.pp1_allowed))
            .flatten();
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
            result = result.child(super::super::focus::reading(summary)).child(choices);
        } else if !inputs.unusable {
            result = result.child(super::super::focus::reading("Fill in at least one box to see the points."));
        }
        result
    }
}
