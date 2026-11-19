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

use classivar_core::{Class, GeneDecision, RuleSet, Strength};
use gpui::{Entity, TestAppContext, VisualTestContext, WindowOptions};
use gpui_kit::component::ThemeMode;
use gpui_kit::{AppContext, init, open_window};

use super::{Choice, Editing, Mode, Workbench};
use crate::status::Status;
use crate::theme;

fn rules() -> &'static RuleSet {
    RuleSet::builtin().expect("the built-in rules are valid")
}

/// A window with a new workbench in it, in the given theme.
pub(super) fn open(cx: &mut TestAppContext, theme_mode: ThemeMode) -> (Entity<Workbench>, &mut VisualTestContext) {
    cx.update(|cx| {
        init(cx);
        theme::install(theme_mode, cx);
    });
    // The same call `main` makes, so the window has the toolkit's root around it (which owns Tab).
    let (handle, workbench) = cx
        .update(|cx| {
            open_window(WindowOptions::default(), cx, |window, cx| cx.new(|cx| Workbench::new(rules(), window, cx)))
        })
        .expect("the window opens");
    (workbench, VisualTestContext::from_window(handle, cx).into_mut())
}

/// Draw the window once, as the screen would.
pub(super) fn draw(cx: &mut VisualTestContext) {
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

/// Apply one criterion at its default strength.
fn apply_default(this: &mut Workbench, code: &str) {
    let strength = this.rules.criterion(code).expect("criterion exists").strength;
    this.today.insert(code.to_string(), Choice::new(strength));
}

#[gpui::test]
fn a_change_after_saving_is_noticed_and_undoing_it_is_noticed_too(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        assert!(!this.has_unsaved_changes(cx), "nothing was saved yet, so there is nothing to compare with");
        apply_default(this, "PM2");
        let saved = this.case_file(cx);
        this.remember_saved(saved);
        assert!(!this.has_unsaved_changes(cx));

        apply_default(this, "PS1");
        assert!(this.has_unsaved_changes(cx), "a new criterion is a change");
        this.today.remove("PS1");
        assert!(!this.has_unsaved_changes(cx), "putting it back is not a change");

        this.today.insert("PM2".to_string(), Choice::new(Strength::Moderate));
        assert!(this.has_unsaved_changes(cx), "a different strength is a change");
        apply_default(this, "PM2");

        this.variant_input.update(cx, |input, cx| input.set_value("NM_000000.1:c.1A>G", window, cx));
        assert!(this.has_unsaved_changes(cx), "the variant name is part of the case");
    });
    draw(cx);
}

#[gpui::test]
fn an_opened_case_is_unchanged_until_it_is_edited_and_clearing_forgets_it(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        apply_default(this, "PM2");
        let case = this.case_file(cx);
        this.apply_case(case, window, cx);
        assert!(!this.has_unsaved_changes(cx), "a case just opened matches its file");
        apply_default(this, "PP3");
        assert!(this.has_unsaved_changes(cx));
        this.clear_case(window, cx);
        assert!(!this.has_unsaved_changes(cx), "a cleared case has no file to differ from");
    });
}

#[gpui::test]
fn a_failed_save_keeps_the_old_baseline_and_a_finished_one_moves_it(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, cx| {
        apply_default(this, "PM2");
        let before = this.case_file(cx);
        this.remember_saved(before.clone());
        apply_default(this, "PS1");
        let edited = this.case_file(cx);

        this.finish_save(Some(Status::attention("Could not save to a.json: disk full.")), edited.clone(), cx);
        assert!(this.has_unsaved_changes(cx), "a failed save wrote nothing, so the change is still unsaved");
        this.finish_save(None, edited.clone(), cx);
        assert!(this.has_unsaved_changes(cx), "a closed dialog wrote nothing either");

        this.finish_save(Some(Status::done("Saved to a.json.")), edited, cx);
        assert!(!this.has_unsaved_changes(cx), "a finished save makes what was written the baseline");
    });
}

#[gpui::test]
fn the_gene_decision_and_the_records_are_part_of_the_saved_state(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        apply_default(this, "PM2");
        let saved = this.case_file(cx);
        this.remember_saved(saved);

        this.gene_link = GeneDecision::Established;
        assert!(this.has_unsaved_changes(cx), "the gene decision is saved in the file");
        this.gene_link = GeneDecision::NotDecided;
        assert!(!this.has_unsaved_changes(cx));

        this.add_record(window, cx);
        assert!(this.has_unsaved_changes(cx), "a new earlier record is a change");
        this.remove_record(0, window, cx);
        assert!(!this.has_unsaved_changes(cx), "taking the record away again is not a change");
    });
}

#[gpui::test]
fn loading_the_example_removes_the_marker(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        apply_default(this, "PM2");
        let saved = this.case_file(cx);
        this.remember_saved(saved);
        apply_default(this, "PS1");
        assert!(this.has_unsaved_changes(cx));
        this.load_example(window, cx);
        assert!(!this.has_unsaved_changes(cx), "the example is never saved, so it has no file to differ from");
    });
}
