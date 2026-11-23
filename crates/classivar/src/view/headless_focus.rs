//! Headless window tests for the focus pages of the right panel: a criterion's
//! strength guide, its "More" page, and the evidence helpers inside their tabs.
//!
//! Explicit imports on purpose (see `headless.rs`).

use classivar_core::{Category, Strength};
use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_kit::component::ThemeMode;

use super::headless::{draw, open};
use super::{Mode, Workbench};

/// A selector built at run time. The test context only looks up `'static` names, and a test may leak a few bytes.
fn selector(name: String) -> &'static str {
    Box::leak(name.into_boxed_str())
}

fn click(cx: &mut VisualTestContext, name: &str) {
    let bounds = cx.debug_bounds(selector(name.to_string())).unwrap_or_else(|| panic!("{name} is drawn"));
    cx.simulate_click(bounds.center(), Modifiers::default());
    draw(cx);
}

fn drawn(cx: &mut VisualTestContext, name: &str) -> bool {
    cx.debug_bounds(selector(name.to_string())).is_some()
}

fn new_variant(cx: &mut TestAppContext) -> (gpui::Entity<Workbench>, &mut VisualTestContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(size(px(1920.), px(1080.)));
    workbench.update_in(cx, |this, _, _| this.set_mode(Mode::NewVariant));
    draw(cx);
    (workbench, cx)
}

#[gpui::test]
fn the_pvs1_guide_takes_the_panel_and_applies_only_on_apply(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    click(cx, "criterion-code-PVS1");
    assert!(drawn(cx, "guide-page-PVS1"), "the guide has the whole panel");
    assert!(!drawn(cx, "criterion-PS1"), "the list waits behind it");
    assert!(!drawn(cx, "guide-option-PVS1-0"), "the variant step waits for the gene step");

    click(cx, "gene-level-option-PVS1-0");
    click(cx, "guide-option-PVS1-0");
    workbench.read_with(cx, |this, _| {
        assert!(!this.today.contains_key("PVS1"), "choosing an answer does not apply it");
        assert_eq!(this.guide_pick.as_deref(), Some("nmd"));
    });

    click(cx, "guide-apply-PVS1");
    workbench.read_with(cx, |this, _| {
        let choice = &this.today["PVS1"];
        assert_eq!(choice.strength, Strength::VeryStrong);
        assert_eq!(choice.situation.as_deref(), Some("nmd"));
        assert!(choice.gene_level.is_some());
        assert!(this.guide_open.is_none());
    });
    assert!(drawn(cx, "criterion-PS1"), "Apply returns to the list");
}

#[gpui::test]
fn going_back_from_the_guide_applies_nothing(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    click(cx, "criterion-code-PVS1");
    click(cx, "gene-level-option-PVS1-0");
    click(cx, "guide-option-PVS1-0");
    click(cx, "guide-back-PVS1");
    workbench.read_with(cx, |this, _| {
        assert!(this.today.is_empty());
        assert!(this.guide_open.is_none() && this.guide_pick.is_none() && this.guide_gene_level.is_none());
    });
    assert!(drawn(cx, "criterion-PVS1"));
}

#[gpui::test]
fn more_opens_the_criterion_page_and_back_returns(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    click(cx, "criterion-more-PS1");
    assert!(drawn(cx, "about-page-PS1"));
    assert!(!drawn(cx, "criterion-PVS1"));
    click(cx, "about-apply-PS1");
    workbench.read_with(cx, |this, _| assert_eq!(this.today["PS1"].strength, Strength::Strong));
    assert!(drawn(cx, "about-page-PS1"), "the page stays open and now offers Remove");
    click(cx, "about-back-PS1");
    workbench.read_with(cx, |this, _| assert!(this.about.is_none()));
    assert!(drawn(cx, "criterion-PVS1"));
}

#[gpui::test]
fn a_guided_criterion_page_leads_to_its_guide(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    click(cx, "criterion-more-PVS1");
    click(cx, "about-apply-PVS1");
    workbench.read_with(cx, |this, _| {
        assert!(this.today.is_empty(), "a guided criterion is never applied without its answer");
        assert_eq!(this.guide_open.as_deref(), Some("PVS1"));
        assert!(this.about.is_none());
    });
    assert!(drawn(cx, "guide-page-PVS1"));
}

#[gpui::test]
fn each_helper_sits_in_the_tab_of_the_criteria_it_sets(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    let tab = |kind: Category| format!("criteria-tab-{}", kind.label().to_lowercase().replace(' ', "-"));
    let expected: [(Category, &[&str]); 5] = [
        (Category::VariantType, &[]),
        (Category::Population, &["population", "ba1-reference"]),
        (Category::Computational, &["predictor"]),
        (Category::FamilyCases, &["locus", "segregation", "pm3", "de-novo"]),
        (Category::Phenotype, &["locus"]),
    ];
    let all = ["population", "ba1-reference", "predictor", "functional", "locus", "segregation", "pm3", "de-novo"];
    for (kind, keys) in expected {
        click(cx, &tab(kind));
        for key in all {
            let card = format!("helper-card-{key}");
            assert_eq!(drawn(cx, &card), keys.contains(&key), "{card} in the {} tab", kind.label());
        }
    }
    click(cx, &tab(Category::Functional));
    assert!(drawn(cx, "helper-card-functional"));
    let _ = workbench;
}

#[gpui::test]
fn a_helper_opens_alone_in_the_panel_and_back_closes_it(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    click(cx, "criteria-tab-population");
    click(cx, "helper-open-population");
    assert!(drawn(cx, "helper-page-population"));
    assert!(!drawn(cx, "criterion-PM2"), "the list waits behind the helper");
    workbench.update_in(cx, |this, _, cx| {
        // Another helper opened from elsewhere replaces the first.
        this.open_helper("predictor");
        cx.notify();
    });
    draw(cx);
    workbench.read_with(cx, |this, _| {
        assert!(this.helpers.predictor.open);
        assert!(!this.helpers.population.open);
    });
    click(cx, "helper-back-predictor");
    workbench.read_with(cx, |this, _| assert!(!this.helpers.predictor.open));
    assert!(drawn(cx, "criterion-PM2"));
}

#[gpui::test]
fn leaving_today_for_an_earlier_report_closes_every_open_page(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    workbench.update_in(cx, |this, window, cx| {
        this.set_mode(Mode::ReportedVariant);
        this.add_record(window, cx);
        this.set_mode(Mode::NewVariant);
        this.open_guide_for_test("PVS1");
        this.about = Some("PS1".into());
        this.open_helper("predictor");
        this.show_record(0, window, cx);
        assert!(this.guide_open.is_none() && this.guide_pick.is_none() && this.guide_gene_level.is_none());
        assert!(this.about.is_none(), "a page of today's evidence does not follow us to a report");
        assert!(!this.helpers.predictor.open, "a helper that only works on today's evidence is closed too");
    });
}

#[gpui::test]
fn changing_the_mode_closes_every_open_page_and_escape_has_nothing_hidden_to_close(cx: &mut TestAppContext) {
    let (workbench, cx) = new_variant(cx);
    workbench.update_in(cx, |this, window, cx| {
        this.open_guide_for_test("PVS1");
        this.set_mode(Mode::ReportedVariant);
        assert!(this.guide_open.is_none() && this.about.is_none());
        this.escape(window, cx);
    });
}
