//! Tests that open the real window in GPUI's headless test platform: nothing
//! appears on screen and no mouse or keyboard is used, so they can run while the
//! machine is in use. They draw the real views for the states people reach, so a
//! panic in a render function shows up here. They only check that drawing does
//! not panic (and a few facts about the state); they do not check what is on the
//! screen. They set the state directly; what a click does is still checked by the
//! Windows smoke script.
//!
//! Explicit imports on purpose: `use gpui_kit::*` also brings GPUI's `test` macro
//! into scope, and `#[gpui::test]` then expands into itself without end.

use classivar_core::{Class, RuleSet};
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpui_kit::component::ThemeMode;
use gpui_kit::init;

use super::{Choice, Editing, Mode, Workbench};
use crate::theme;

fn rules() -> &'static RuleSet {
    RuleSet::builtin().expect("the built-in rules are valid")
}

/// A window with a new workbench in it, in the given theme.
fn open(cx: &mut TestAppContext, theme_mode: ThemeMode) -> (Entity<Workbench>, &mut VisualTestContext) {
    cx.update(|cx| {
        init(cx);
        theme::install(theme_mode, cx);
    });
    cx.add_window_view(|window, cx| Workbench::new(rules(), window, cx))
}

/// Draw the window once, as the screen would.
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let drawn = window.draw(cx);
        drawn.clear(cx);
    });
}

#[gpui::test]
fn the_empty_window_draws_without_panicking_in_both_themes(cx: &mut TestAppContext) {
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let (_workbench, cx) = open(cx, mode);
        draw(cx);
    }
}

#[gpui::test]
fn the_example_draws_without_panicking_for_today_and_every_record(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.load_example(window, cx);
        this.set_mode(Mode::ReportedVariant);
        // Every criterion's "More" panel is open, so the strength ladder is drawn for all of them.
        this.open_details = this.rules.criteria.iter().map(|c| c.code.clone()).collect();
    });
    draw(cx);
    let records = workbench.read_with(cx, |this, _| this.records.len());
    assert!(records >= 2, "the example has earlier records to draw");
    for index in 0..records {
        workbench.update_in(cx, |this, _, _| this.editing = Editing::Record(index));
        draw(cx);
    }
    workbench.update_in(cx, |this, _, _| this.editing = Editing::Today);
    draw(cx);
}

#[gpui::test]
fn the_example_draws_without_panicking_in_the_dark_theme_and_in_new_variant_mode(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Dark);
    workbench.update_in(cx, |this, window, cx| this.load_example(window, cx));
    draw(cx);
    workbench.update_in(cx, |this, _, _| this.set_mode(Mode::NewVariant));
    draw(cx);
}

/// Apply every criterion at its default strength (a guided one with its first answer).
fn apply_all(this: &mut Workbench) {
    for c in &this.rules.criteria {
        // A guide answer that sets no strength means "does not apply"; the first answer that sets one is used.
        let answer = c.guide.iter().find_map(|option| option.strength.map(|strength| (strength, option.id.clone())));
        match (c.guide.is_empty(), answer) {
            (true, _) => {
                this.today.insert(c.code.clone(), Choice::new(c.strength));
            }
            (false, Some((strength, id))) => {
                this.today.insert(c.code.clone(), Choice::guided(strength, id));
            }
            (false, None) => {}
        }
    }
}

#[gpui::test]
fn contradicting_evidence_draws_without_panicking_and_has_no_class(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| apply_all(this));
    draw(cx);
    let class = workbench.read_with(cx, |this, _| {
        this.rules.assess(&super::to_applied(&this.today)).expect("the evidence can be scored").class()
    });
    assert_eq!(class, None, "every criterion applied at once contradicts itself, so no class is shown");
}

#[gpui::test]
fn every_helper_open_at_once_draws_without_panicking(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        let helpers = &mut this.helpers;
        helpers.predictor.open = true;
        helpers.functional.open = true;
        helpers.locus.open = true;
        helpers.segregation.open = true;
        helpers.pm3.open = true;
        helpers.ba1_open = true;
        helpers.de_novo.open = true;
    });
    draw(cx);
}

#[gpui::test]
fn a_pathogenic_result_draws_without_panicking(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        for code in ["PS1", "PS2", "PM1", "PM2", "PP3"] {
            let strength = this.rules.criterion(code).expect("criterion exists").strength;
            this.today.insert(code.to_string(), Choice::new(strength));
        }
    });
    draw(cx);
    let class = workbench.read_with(cx, |this, _| {
        this.rules.assess(&super::to_applied(&this.today)).expect("the evidence can be scored").class()
    });
    assert!(matches!(class, Some(Class::Pathogenic | Class::LikelyPathogenic)), "{class:?}");
}
