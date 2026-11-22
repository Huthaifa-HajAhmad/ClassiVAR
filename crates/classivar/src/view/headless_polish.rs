//! Headless window tests for the polish pass: Clear case, the lab's stated class
//! on an earlier report without criteria, and the gene-sources drawer.
//!
//! Explicit imports on purpose (see `headless.rs`).

use classivar_core::Inheritance;
use classivar_core::variant::VariantIdentity;
use classivar_core::{Class, Strength};
use classivar_sources::Fetched;
use classivar_sources::allele_registry::Lookup as Found;
use classivar_sources::clingen::{ClinGenGene, Curation};
use classivar_sources::panelapp::{Confidence, PanelAppGene, PanelEntry};
use gpui::TestAppContext;
use gpui_kit::component::ThemeMode;

use super::headless::{draw, open};
use super::{Choice, Confirm, Editing, Lookup, Mode};
use crate::settings::Picture;

fn curation() -> Curation {
    Curation {
        disease: "hereditary breast ovarian cancer syndrome".into(),
        disease_id: "MONDO:0003582".into(),
        inheritance: "AD".into(),
        classification: "Definitive".into(),
        report_url: String::new(),
        classified_on: "2020-01-01".into(),
        expert_panel: "Hereditary Breast Cancer GCEP".into(),
    }
}

fn ratings() -> Lookup<ClinGenGene> {
    Lookup::Done(Fetched {
        source: "ClinGen",
        fetched_on: "2026-11-22".into(),
        value: ClinGenGene { symbol: "BRCA1".into(), curations: vec![curation()] },
    })
}

#[gpui::test]
fn clear_case_empties_the_whole_form_and_not_only_the_criteria(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.set_mode(Mode::ReportedVariant);
        this.boxes.variant.update(cx, |input, cx| input.set_value("NM_007294.4:c.5266dup", window, cx));
        this.boxes.gene.update(cx, |input, cx| input.set_value("BRCA1", window, cx));
        this.case_gene = "BRCA1".into();
        this.gene.clingen = ratings();
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
        this.add_record(window, cx);
        assert!(this.has_case_data(cx));
        this.ask(Confirm::ClearCase);
        this.apply_confirm(window, cx);
        assert!(!this.has_case_data(cx), "nothing is left to clear");
        assert!(this.today.is_empty() && this.records.is_empty());
        assert!(this.case_gene.is_empty() && this.boxes.gene.read(cx).value().is_empty());
        assert!(matches!(this.gene.clingen, Lookup::Idle), "the gene's answers go with the case");
    });
    draw(cx);
}

#[gpui::test]
fn an_earlier_report_without_criteria_shows_the_class_the_lab_stated(cx: &mut TestAppContext) {
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = open(cx, mode);
        workbench.update_in(cx, |this, window, cx| {
            this.set_mode(Mode::ReportedVariant);
            this.add_record(window, cx);
            this.records[0].class = Some(Class::LikelyPathogenic);
            this.editing = Editing::Record(0);
            assert!(this.reported_only().is_some(), "a stated class with no criteria has its own view");
            this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
            this.editing = Editing::Today;
            assert!(this.reported_only().is_none(), "today's own evidence never shows a lab's class");
            this.editing = Editing::Record(0);
            this.records[0].evidence.insert("PM2".into(), Choice::new(Strength::Supporting));
            assert!(this.reported_only().is_none(), "once criteria are applied, the engine scores them");
            this.records[0].evidence.clear();
        });
        draw(cx);
    }
}

#[gpui::test]
fn the_gene_drawer_opens_over_the_sheet_and_takes_a_rating_as_the_condition(cx: &mut TestAppContext) {
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = open(cx, mode);
        workbench.update_in(cx, |this, window, cx| {
            this.set_mode(Mode::ReportedVariant);
            this.case_gene = "BRCA1".into();
            this.gene.clingen = ratings();
            this.add_record(window, cx);
            this.editing = Editing::Record(0);
            this.open_sources(Editing::Record(0), cx);
        });
        draw(cx);
        workbench.update_in(cx, |this, window, cx| {
            let rating = curation();
            this.set_scope_condition(super::variant::condition_from_curation(&rating), window, cx);
            let record = this.records[0].condition.as_ref().expect("the record has a condition now");
            assert_eq!(record.mondo.as_deref(), Some("MONDO:0003582"));
            assert_eq!(
                this.variant.record_condition_input.read(cx).value().to_string(),
                "hereditary breast ovarian cancer syndrome"
            );
            assert!(this.variant.condition.is_none(), "the case's own condition is not touched");
            this.use_curation(&rating, window, cx);
            assert!(this.variant.condition.is_some());
            this.gene_cards_open = false;
        });
        draw(cx);
    }
}

fn panels() -> Lookup<PanelAppGene> {
    Lookup::Done(Fetched {
        source: "PanelApp",
        fetched_on: "2026-11-22".into(),
        value: PanelAppGene {
            symbol: "SSR4".into(),
            entries: vec![PanelEntry {
                panel_id: 1,
                panel_name: "Congenital disorders of glycosylation".into(),
                panel_version: "8.8".into(),
                confidence: Confidence::Green,
                mode_of_inheritance: "X-LINKED: hemizygous mutation in males, biallelic mutations in females".into(),
                phenotypes: Vec::new(),
            }],
        },
    })
}

#[gpui::test]
fn clicking_a_panelapp_record_in_the_drawer_makes_it_the_condition_of_the_open_report(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.set_mode(Mode::ReportedVariant);
        this.case_gene = "SSR4".into();
        this.gene.clingen = ratings();
        this.gene.england = panels();
        this.add_record(window, cx);
        this.editing = Editing::Record(0);
        this.open_sources(Editing::Record(0), cx);
    });
    draw(cx);
    let bounds = cx.debug_bounds("pick-england-0").expect("the PanelApp record is a button in the drawer");
    cx.simulate_click(bounds.center(), Default::default());
    workbench.update_in(cx, |this, window, cx| {
        let condition = this.records[0].condition.clone().expect("the report has the panel as its condition");
        assert_eq!(condition.name, "Congenital disorders of glycosylation");
        assert_eq!(condition.inheritance, Inheritance::XLinked);
        assert!(this.variant.condition.is_none(), "the case's own condition is not touched");
        assert!(!this.gene_cards_open, "the drawer closes after a pick");
        this.gene_cards_open = true;
        this.escape(window, cx);
        assert!(!this.gene_cards_open, "Escape closes the drawer");
    });
}

#[gpui::test]
fn a_checked_variant_with_a_gene_checks_that_gene_when_none_is_set(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, cx| {
        let identity = VariantIdentity {
            allele_id: Some("CA1139667862".into()),
            grch38: None,
            grch37: None,
            mane_select: Some("NM_006280.3:c.141dup".into()),
            coding: Vec::new(),
            protein: None,
            gene: Some("SSR4".into()),
            clinvar_variation: None,
            dbsnp: None,
            checked_with: "ClinGen Allele Registry".into(),
            checked_on: "2026-11-22".into(),
        };
        this.finish_registry_lookup(Ok(Fetched {
            source: "ClinGen Allele Registry",
            fetched_on: "2026-11-22".into(),
            value: Found::Found(vec![identity]),
        }));
        this.adopt_variant_gene(cx);
        assert_eq!(this.case_gene, "SSR4", "the gene is checked without typing it again");
        assert!(matches!(this.gene.clingen, Lookup::Loading));
        // No network in these tests: drop the lookups before they run.
        this._lookup_tasks.clear();
    });
    draw(cx);
    workbench.update_in(cx, |this, _, cx| {
        assert_eq!(this.boxes.gene.read(cx).value().as_ref(), "SSR4", "the gene box shows it after the draw");
        this.case_gene = "BRCA1".into();
        this.adopt_variant_gene(cx);
        assert_eq!(this.case_gene, "BRCA1", "a gene already checked is never replaced");
    });
}

#[gpui::test]
fn every_picture_draws_empty_and_for_a_report_without_criteria(cx: &mut TestAppContext) {
    for picture in [Picture::Dial, Picture::Column, Picture::Track] {
        let (workbench, cx) = open(cx, ThemeMode::Dark);
        workbench.update_in(cx, |this, window, cx| {
            this.picture = picture;
            this.set_mode(Mode::ReportedVariant);
            this.add_record(window, cx);
        });
        draw(cx);
        workbench.update_in(cx, |this, _, _| {
            this.records[0].class = Some(Class::Pathogenic);
            this.editing = Editing::Record(0);
        });
        draw(cx);
    }
}

#[gpui::test]
fn clicking_an_imported_lab_in_the_rail_opens_it(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(gpui::size(gpui::px(1920.), gpui::px(1080.)));
    workbench.update_in(cx, |this, window, cx| {
        this.set_mode(Mode::ReportedVariant);
        this.case_gene = "SSR4".into();
        this.gene.england = panels();
        this.gene.clingen = ratings();
        this.add_record(window, cx);
        let record = &mut this.records[0];
        record.label = "Institute of Human Genetics Munich, TUM University Hospital".into();
        record.year = 2020;
        record.class = Some(Class::Pathogenic);
        record.source = Some(classivar_core::RecordSource {
            accession: "SCV001430077".into(),
            review_status: "criteria provided, single submitter".into(),
            last_evaluated: Some("2020-06-05".into()),
            stated_criteria: None,
            imported_on: "2026-11-22".into(),
        });
        record.condition = Some(classivar_core::Condition {
            name: "Congenital disorder of glycosylation, type 1y".into(),
            mondo: Some("MONDO:0010466".into()),
            inheritance: Inheritance::Unknown,
        });
        this.category = Some(classivar_core::Category::ALL[0]);
        this.handles.criteria_scroll.scroll_to_top_of_item(1);
        this.picture = Picture::Column;
    });
    draw(cx);
    for selector in ["rail-record-0", "rail-record-today", "rail-record-0"] {
        let bounds = cx.debug_bounds(selector).expect("the rail stop is drawn");
        cx.simulate_click(bounds.center(), Default::default());
        draw(cx);
    }
    workbench.update_in(cx, |this, _, _| assert!(this.editing == Editing::Record(0)));
}

#[gpui::test]
fn picking_a_panel_for_todays_case_and_typing_a_lab_name_do_not_crash(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(gpui::size(gpui::px(1920.), gpui::px(1080.)));
    workbench.update_in(cx, |this, _, _| {
        this.set_mode(Mode::NewVariant);
        this.case_gene = "SSR4".into();
        this.gene.clingen = ratings();
        this.gene.england = panels();
        this.gene_cards_open = true;
    });
    draw(cx);
    let bounds = cx.debug_bounds("pick-england-0").expect("drawn");
    cx.simulate_click(bounds.center(), Default::default());
    draw(cx);
    workbench.update_in(cx, |this, window, cx| {
        assert!(this.variant.condition.is_some());
        this.set_mode(Mode::ReportedVariant);
        this.add_record(window, cx);
        this.show_record(0, window, cx);
        this.boxes
            .record_name
            .update(cx, |input, cx| input.set_value("Institute of Human Genetics Munich, TUM", window, cx));
    });
    draw(cx);
    draw(cx);
}

#[gpui::test]
fn the_criteria_draw_folded_and_open_by_kind_of_evidence(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        this.set_mode(Mode::NewVariant);
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
        this.folded = classivar_core::Category::ALL.into_iter().collect();
    });
    draw(cx);
    workbench.read_with(cx, |this, _| assert_eq!(this.folded.len(), classivar_core::Category::ALL.len()));
    workbench.update_in(cx, |this, _, cx| {
        this.folded.clear();
        cx.notify();
    });
    draw(cx);
}

#[gpui::test]
fn sources_opened_from_todays_condition_choose_todays_condition_even_while_a_report_is_open(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.set_mode(Mode::ReportedVariant);
        this.case_gene = "SSR4".into();
        this.gene.clingen = ratings();
        this.gene.england = panels();
        this.add_record(window, cx);
        this.editing = Editing::Record(0);
        this.open_sources(Editing::Today, cx);
    });
    draw(cx);
    let bounds = cx.debug_bounds("pick-england-0").expect("drawn");
    cx.simulate_click(bounds.center(), Default::default());
    workbench.update_in(cx, |this, _, _| {
        assert!(this.variant.condition.is_some(), "the case's condition was set");
        assert!(this.records[0].condition.is_none(), "the open report's condition was not touched");
    });
}

#[gpui::test]
fn a_gene_queued_for_the_box_never_outlives_clear_case_or_a_newly_opened_case(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.pending_gene_box = Some("SSR4".into());
        this.typing_condition = true;
        this.ask(Confirm::ClearCase);
        this.apply_confirm(window, cx);
        assert!(this.pending_gene_box.is_none() && !this.typing_condition);
        this.pending_gene_box = Some("SSR4".into());
        this.typing_condition = true;
        let case = this.case_file(cx);
        this.apply_case(case, window, cx);
        assert!(this.pending_gene_box.is_none() && !this.typing_condition);
    });
    draw(cx);
    workbench.update_in(cx, |this, _, cx| {
        assert!(this.boxes.gene.read(cx).value().is_empty(), "a cleared case stays cleared across a draw");
    });
}
