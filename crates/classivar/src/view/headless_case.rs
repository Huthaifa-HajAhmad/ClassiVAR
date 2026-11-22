//! Headless window tests for the case's variant, condition and ClinVar records:
//! every state the new rows reach is drawn in both themes, and the saved state
//! follows them. No network: answers are put in place as a lookup would.
//!
//! Explicit imports on purpose (see `headless.rs`): `use gpui_kit::*` brings in
//! GPUI's `test` macro, which then expands into itself.

use classivar_core::variant::VariantIdentity;
use classivar_core::{Class, Condition, ConditionMatch, GeneDecision, Inheritance, Strength, Verdict};
use classivar_sources::Fetched;
use classivar_sources::allele_registry::Lookup as Found;
use classivar_sources::clinvar::{ClinVarSubmission, ClinVarVariation};
use gpui::TestAppContext;
use gpui_kit::component::ThemeMode;

use super::headless::{draw, open};
use super::{Choice, Editing, Lookup, Mode};

fn brca1() -> VariantIdentity {
    VariantIdentity {
        allele_id: Some("CA003404".into()),
        grch38: Some("NC_000017.11:g.43057065dup".into()),
        grch37: Some("NC_000017.10:g.41209082dup".into()),
        mane_select: Some("NM_007294.4:c.5266dup".into()),
        coding: Vec::new(),
        protein: Some("NP_009225.1:p.Gln1756ProfsTer?".into()),
        gene: Some("BRCA1".into()),
        clinvar_variation: Some(17677),
        dbsnp: Some(80357906),
        checked_with: "ClinGen Allele Registry".into(),
        checked_on: "2026-11-21".into(),
    }
}

fn fetched<T>(value: T) -> Fetched<T> {
    Fetched { source: "test", fetched_on: "2026-11-21".into(), value }
}

fn hboc(mondo: &str) -> Condition {
    Condition {
        name: "Hereditary breast ovarian cancer".into(),
        mondo: Some(mondo.into()),
        inheritance: Inheritance::AutosomalDominant,
    }
}

fn variation(submissions: Vec<ClinVarSubmission>) -> ClinVarVariation {
    ClinVarVariation { accession: "VCV000017677".into(), name: "test".into(), submissions }
}

fn submission(accession: &str, class: Class, mondo: &str) -> ClinVarSubmission {
    ClinVarSubmission {
        accession: accession.into(),
        submitter: format!("Lab {accession}"),
        last_evaluated: Some("2022-11-23".into()),
        classification: class.label().into(),
        class: Some(class),
        review_status: "criteria provided, single submitter".into(),
        condition: Some(hboc(mondo)),
        stated_criteria: Some("PS4, PVS1".into()),
    }
}

#[gpui::test]
fn every_variant_check_state_draws_in_both_themes(cx: &mut TestAppContext) {
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = open(cx, mode);
        workbench.update_in(cx, |this, window, cx| {
            this.set_mode(Mode::NewVariant);
            this.boxes.variant.update(cx, |input, cx| input.set_value("17-43057062-T-TG", window, cx));
            this.check_variant(cx);
            assert!(
                this.variant.problem.as_deref().is_some_and(|p| p.contains("build")),
                "a bare position asks for its build"
            );
        });
        draw(cx);
        workbench.update_in(cx, |this, _, _| this.variant.lookup = Lookup::Loading);
        draw(cx);
        workbench.update_in(cx, |this, _, _| {
            this.variant.lookup = Lookup::Failed("the service did not answer in time".into())
        });
        draw(cx);
        workbench.update_in(cx, |this, _, _| this.variant.lookup = Lookup::Done(fetched(Found::NotFound)));
        draw(cx);
        workbench.update_in(cx, |this, _, _| {
            let other = VariantIdentity { allele_id: None, ..brca1() };
            this.finish_registry_lookup(Ok(fetched(Found::Found(vec![brca1(), other]))));
            assert!(this.variant.identity.is_none(), "several alleles wait for the curator's choice");
        });
        draw(cx);
        workbench.update_in(cx, |this, _, _| {
            this.finish_registry_lookup(Ok(fetched(Found::Found(vec![brca1()]))));
            assert_eq!(this.variant.identity, Some(brca1()), "one allele is the answer");
            this.case_gene = "BRCA2".into();
        });
        draw(cx);
    }
}

#[gpui::test]
fn the_identity_and_the_condition_are_saved_and_reopened(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
        this.variant.identity = Some(brca1());
        this.variant.condition = Some(hboc("MONDO:0003582"));
        let case = this.case_file(cx);
        assert_eq!(case.identity, Some(brca1()));
        this.apply_case(case, window, cx);
        assert_eq!(this.variant.identity, Some(brca1()), "opening restores the checked identity");
        assert_eq!(this.variant.inheritance(), Inheritance::AutosomalDominant);
        assert!(!this.has_unsaved_changes(cx));
        this.set_inheritance(Inheritance::AutosomalRecessive, cx);
        assert!(this.has_unsaved_changes(cx), "the condition is part of what Save writes");
        this.clear_case(window, cx);
        assert!(this.variant.identity.is_none() && this.variant.condition.is_none(), "clearing forgets both");
    });
    draw(cx);
}

#[gpui::test]
fn imported_records_for_another_condition_are_shown_but_never_compared(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Dark);
    workbench.update_in(cx, |this, _, cx| {
        this.set_mode(Mode::ReportedVariant);
        this.variant.identity = Some(brca1());
        this.variant.condition = Some(hboc("MONDO:0003582"));
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
        let found = variation(vec![submission("SCV1", Class::Pathogenic, "MONDO:0015356")]);
        this.finish_clinvar_import(Ok(fetched(Some(found))), cx);
        assert_eq!(this.records.len(), 1);
        assert!(this.records[0].evidence.is_empty(), "criteria are never invented from ClinVar");
        let (_, comparison) = this.compare_with_records();
        assert_eq!(comparison.unwrap().verdict, Some(classivar_core::Verdict::DifferentCondition));
    });
    draw(cx);
    workbench.update_in(cx, |this, window, cx| this.show_record(0, window, cx));
    draw(cx);
    workbench.update_in(cx, |this, _, cx| {
        // A second import of the same variation adds nothing.
        let again = variation(vec![submission("SCV1", Class::Pathogenic, "MONDO:0015356")]);
        this.finish_clinvar_import(Ok(fetched(Some(again))), cx);
        assert_eq!(this.records.len(), 1);
        this.finish_clinvar_import(Err(classivar_sources::SourceError::Timeout), cx);
        assert!(matches!(this.variant.clinvar, Lookup::Failed(_)), "a failed import is said, never shown as empty");
        this.editing = Editing::Today;
    });
    draw(cx);
}

#[gpui::test]
fn a_doubtful_gene_shows_a_pathogenic_result_as_vus_and_draws(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        this.set_mode(Mode::NewVariant);
        for (code, strength) in [
            ("PS3", Strength::Strong),
            ("PS4", Strength::Strong),
            ("PM2", Strength::Supporting),
            ("PP3", Strength::Supporting),
        ] {
            this.today.insert(code.into(), Choice::new(strength));
        }
        assert_eq!(this.score_on_screen().unwrap().class(), Some(Class::Pathogenic));
        this.gene_link = GeneDecision::NotEstablished;
        let result = this.score_on_screen().unwrap();
        assert_eq!(result.class(), Some(Class::Vus));
        assert!(result.lowered_for_gene());
    });
    draw(cx);
}

#[gpui::test]
fn the_pvs1_guide_asks_the_gene_level_question_first_and_lowers_the_answer(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        this.set_mode(Mode::NewVariant);
        let pvs1 = this.rules.criterion("PVS1").expect("PVS1 exists").clone();
        this.toggle(&pvs1);
        assert_eq!(this.guide_open.as_deref(), Some("PVS1"));
        assert_eq!(this.guide_gene_level, None, "nothing is assumed about the gene");
    });
    draw(cx);
    workbench.update_in(cx, |this, window, cx| {
        this.guide_gene_level = Some("lower_one".into());
        let pvs1 = this.rules.criterion("PVS1").unwrap().clone();
        let strength = pvs1.strength_for("nmd", this.guide_gene_level.as_deref());
        assert_eq!(strength, Some(Strength::Strong), "NMD lowered one level");
        this.answer_guide("PVS1", strength, "nmd".into());
        let choice = &this.today["PVS1"];
        assert_eq!((choice.strength, choice.gene_level.as_deref()), (Strength::Strong, Some("lower_one")));
        assert_eq!(this.score_on_screen().unwrap().total_points, 4);
        // Reopening shows the saved gene-level answer.
        this.guide_gene_level = None;
        this.next_strength(&pvs1, window, cx);
        assert_eq!(this.guide_gene_level.as_deref(), Some("lower_one"));
    });
    draw(cx);
    workbench.update_in(cx, |this, _, _| {
        // "No LoF mechanism" removes PVS1: it does not apply at any strength.
        this.guide_gene_level = Some("no_lof_mechanism".into());
        this.answer_guide("PVS1", None, String::new());
        assert!(!this.today.contains_key("PVS1"));
    });
    draw(cx);
}

#[gpui::test]
fn a_note_on_any_applied_criterion_keeps_its_strength_and_guide_answers(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.set_mode(Mode::NewVariant);
        this.today.insert("PVS1".into(), Choice::guided(Strength::Strong, "nmd".into(), Some("lower_one".into())));
        this.open_details.insert("PVS1".into());
        this.start_note("PVS1", window, cx);
        assert!(this.raising.as_ref().is_some_and(|r| r.note && r.code == "PVS1"));
    });
    draw(cx);
    workbench.update_in(cx, |this, window, cx| {
        this.boxes.reason.update(cx, |input, cx| input.set_value("RNA assay, PMID 1, read 2026-11-21", window, cx));
        this.confirm_raise(cx);
        let choice = &this.today["PVS1"];
        assert_eq!(choice.strength, Strength::Strong, "a note never changes the strength");
        assert_eq!(choice.situation.as_deref(), Some("nmd"));
        assert_eq!(choice.gene_level.as_deref(), Some("lower_one"));
        assert_eq!(choice.reason.as_deref(), Some("RNA assay, PMID 1, read 2026-11-21"));
        assert!(this.score_on_screen().is_ok());
    });
    draw(cx);
}

/// The two real ClinGen panels named different conditions. The earlier one is shown
/// but never compared with today's class; the verdict reads the later panel only.
#[gpui::test]
fn the_real_case_never_compares_a_record_for_another_condition(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.set_mode(Mode::ReportedVariant);
        let (_, comparison) = this.compare_with_records();
        let comparison = comparison.expect("reported mode compares");
        assert_eq!(comparison.records[0].condition, ConditionMatch::Different, "the 2023 panel's condition");
        assert_eq!(comparison.records[1].condition, ConditionMatch::Same, "the 2026 panel's condition");
        assert_eq!(comparison.latest_index, Some(1), "the verdict reads the same-condition record");

        // With only the other-condition record left there is nothing to compare with.
        this.records.remove(1);
        let (_, comparison) = this.compare_with_records();
        let comparison = comparison.expect("reported mode compares");
        assert_eq!(comparison.latest_index, None);
        assert_eq!(comparison.verdict, Some(Verdict::DifferentCondition));
    });
}

/// ClinGen's two RYR1 curations of one disease (recessive and dominant) are drawn as
/// pills; choosing one marks only that one, in both themes.
#[gpui::test]
fn the_clingen_condition_pills_draw_and_only_the_chosen_one_is_marked(cx: &mut TestAppContext) {
    use classivar_sources::clingen::{ClinGenGene, Curation};

    use super::real_case::{RYR1_MYOPATHY, condition};
    use super::variant::curation_is_chosen;

    let curation = |inheritance: &str| Curation {
        disease: RYR1_MYOPATHY.0.into(),
        disease_id: RYR1_MYOPATHY.1.into(),
        inheritance: inheritance.into(),
        classification: "Definitive".into(),
        report_url: String::new(),
        classified_on: "2024-01-01".into(),
        expert_panel: "Congenital Myopathies GCEP".into(),
    };
    let (recessive, dominant) = (curation("AR"), curation("AD"));
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = open(cx, mode);
        workbench.update_in(cx, |this, window, cx| {
            this.welcome = false;
            this.gene.clingen = Lookup::Done(fetched(ClinGenGene {
                symbol: "RYR1".into(),
                curations: vec![recessive.clone(), dominant.clone()],
            }));
            this.use_curation(&recessive, window, cx);
            let now = this.variant.condition.as_ref();
            assert!(curation_is_chosen(now, &recessive));
            assert!(!curation_is_chosen(now, &dominant));
            assert_eq!(now.map(|c| c.mondo.clone()), Some(condition(RYR1_MYOPATHY).mondo));
        });
        // Drawing is the check: the pills (open and chosen) render without a panic.
        draw(cx);
    }
}
