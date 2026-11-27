//! Headless window tests for the guided review: the welcome entry, every step in
//! both themes and window sizes, decisions, the labelled replacement, the Guided
//! and Free switch, set-aside decisions, and the keyboard through a step.
//!
//! Explicit imports on purpose (see `headless.rs`).

use classivar_core::case::{ReviewDecision, ReviewState};
use classivar_core::{EvidenceReading, LocusChoice, Strength};
use gpui::{Entity, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext};
use gpui_kit::component::ThemeMode;

use super::headless::{draw, open};
use super::{Choice, Raise, Workbench};

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    draw(cx);
    let bounds = cx.debug_bounds(selector).unwrap_or_else(|| panic!("{selector} is on screen"));
    cx.simulate_click(bounds.center(), Default::default());
    draw(cx);
}

/// A window in the guided review, started from the welcome screen as a curator would.
fn guided(cx: &mut TestAppContext, theme: ThemeMode) -> (Entity<Workbench>, &mut VisualTestContext) {
    let (workbench, cx) = open(cx, theme);
    click(cx, "welcome-guided");
    assert_eq!(workbench.read_with(cx, |this, _| this.guided), Some(0), "the welcome entry opens step 1");
    (workbench, cx)
}

fn state(workbench: &Entity<Workbench>, cx: &mut VisualTestContext, code: &str) -> ReviewState {
    workbench.read_with(cx, |this, _| this.review_state(code))
}

#[gpui::test]
fn every_step_draws_in_both_themes_and_both_window_sizes(cx: &mut TestAppContext) {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = guided(cx, theme);
        for size in [(1240., 900.), (960., 600.)] {
            cx.simulate_resize(gpui::size(gpui::px(size.0), gpui::px(size.1)));
            for step in 0..10 {
                workbench.update_in(cx, |this, _, _| this.go_to_step(step));
                draw(cx);
                assert!(cx.debug_bounds("guided-step-page").is_some(), "step {step} draws its page");
                assert!(cx.debug_bounds("review-strip").is_some(), "the summary shows review progress");
            }
        }
    }
}

#[gpui::test]
fn does_not_apply_defer_and_clear_record_and_forget_a_decision(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    click(cx, "guided-does-not-apply-BA1");
    click(cx, "guided-defer-BS1");
    assert_eq!(state(&workbench, cx, "BA1"), ReviewState::DoesNotApply);
    assert_eq!(state(&workbench, cx, "BS1"), ReviewState::Deferred);
    click(cx, "guided-clear-BS1");
    assert_eq!(state(&workbench, cx, "BS1"), ReviewState::NotYetReviewed);
    let progress = workbench.read_with(cx, |this, _| this.review.total_progress(this.rules, &[]));
    assert_eq!(progress.reviewed, 1, "only BA1 is reviewed now");
    // A decision is never counted evidence, so no class appears from it.
    assert!(workbench.read_with(cx, |this, _| this.score_on_screen().unwrap().class().is_none()));
}

#[gpui::test]
fn marking_an_applied_criterion_as_not_applying_takes_it_off_the_evidence(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
    });
    click(cx, "guided-does-not-apply-PM2");
    workbench.read_with(cx, |this, _| {
        assert!(!this.today.contains_key("PM2"));
        assert_eq!(this.review.decisions.get("PM2"), Some(&ReviewDecision::DoesNotApply));
    });
}

/// Each way of counting a criterion answers its review at once, so the screen and the
/// saved file never show a criterion as both counted and deferred.
#[gpui::test]
fn every_way_of_counting_a_criterion_ends_its_deferral_at_once(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        let codes = ["PM2", "PVS1", "PM1", "BS1", "PP1", "PP4", "PS3"];
        this.load_real_case(window, cx);
        this.today.clear();
        for code in codes {
            this.decide(code, Some(ReviewDecision::Deferred));
        }
        let pm2 = this.rules.criterion("PM2").unwrap().clone();
        this.toggle(&pm2);
        this.answer_guide("PVS1", Some(Strength::VeryStrong), "nmd".into());
        this.raising = Some(Raise { code: "PM1".into(), to: Strength::Strong, note: false });
        this.boxes.reason.update(cx, |input, cx| input.set_value("a stated reason", window, cx));
        this.confirm_raise(cx);
        let bs1 = EvidenceReading { code: "BS1", published: Strength::Strong, strength: Strength::Strong };
        this.apply_reading(&bs1, "frequency".into());
        let both = LocusChoice { pp1: Some(Strength::Moderate), pp4: Some(Strength::Supporting) };
        this.apply_locus_choice(both, "locus".into());
        for code in &codes[..6] {
            assert!(!this.review.decisions.contains_key(*code), "{code} is counted, so no longer deferred");
        }
        // PS3 is in the published report, so copying it counts it.
        this.start_from_latest();
        assert!(!this.review.decisions.contains_key("PS3"));
        assert!(this.case_file(cx).check(this.rules).is_ok(), "the app never writes a file it would refuse");
        this.toggle(&pm2);
        assert_eq!(this.review_state("PM2"), ReviewState::NotYetReviewed, "the old deferral does not return");
    });
}

#[gpui::test]
fn review_answers_survive_workbench_open_save_and_clear(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        let mut case = this.case_file(cx);
        case.review.decisions.insert("BS1".into(), ReviewDecision::Deferred);
        case.review.facts.insert("other-rule-fact".into(), "unknown".into());
        this.apply_case(case.clone(), window, cx);
        assert_eq!(this.case_file(cx).review, case.review);
        assert!(this.has_case_data(cx), "review answers are work worth saving");
        assert!(!this.has_unsaved_changes(cx));
        this.clear_case(window, cx);
        assert!(this.case_file(cx).review.is_empty());
    });
    draw(cx);
}

#[gpui::test]
fn a_contradicting_proposal_is_replaced_only_when_asked_and_keep_leaves_the_evidence(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    let bs1 = EvidenceReading { code: "BS1", published: Strength::Strong, strength: Strength::Strong };
    workbench.update_in(cx, |this, _, _| {
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
        this.decide("BA1", Some(ReviewDecision::Deferred));
        this.keep_rival("BS1");
    });
    draw(cx);
    workbench.read_with(cx, |this, _| assert!(this.today.contains_key("PM2") && !this.today.contains_key("BS1")));
    assert_eq!(state(&workbench, cx, "BS1"), ReviewState::NotYetReviewed, "kept aside is not reviewed");
    workbench.update_in(cx, |this, _, _| this.replace_with_reading(&["PM2"], &bs1, "frequency".into()));
    draw(cx);
    assert_eq!(state(&workbench, cx, "PM2"), ReviewState::NotYetReviewed, "the replaced criterion returns");
    assert_eq!(state(&workbench, cx, "BS1"), ReviewState::Applied);
    assert_eq!(state(&workbench, cx, "BA1"), ReviewState::Deferred, "BA1 is untouched");
}

#[gpui::test]
fn guided_and_free_give_the_same_assessment_for_the_same_confirmed_evidence(cx: &mut TestAppContext) {
    let pick = |this: &mut Workbench| {
        for (code, strength) in
            [("PM2", Strength::Supporting), ("PS4", Strength::Strong), ("PP3", Strength::Supporting)]
        {
            this.today.insert(code.into(), Choice::new(strength));
        }
    };
    let (guided_bench, guided_cx) = guided(cx, ThemeMode::Light);
    guided_bench.update_in(guided_cx, |this, _, _| {
        pick(this);
        this.decide("BA1", Some(ReviewDecision::DoesNotApply));
        this.decide("BS3", Some(ReviewDecision::Deferred));
    });
    click(guided_cx, "mode-free");
    let (guided_points, guided_class, still) = guided_bench.read_with(guided_cx, |this, _| {
        let result = this.score_on_screen().unwrap();
        (result.total_points, result.class(), this.today.len())
    });
    assert_eq!(still, 3, "switching to Free keeps the evidence");
    let (free_bench, free_cx) = open(cx, ThemeMode::Light);
    free_bench.update_in(free_cx, |this, _, _| {
        this.welcome = false;
        pick(this);
    });
    draw(free_cx);
    let (free_points, free_class) = free_bench.read_with(free_cx, |this, _| {
        let result = this.score_on_screen().unwrap();
        (result.total_points, result.class())
    });
    assert_eq!((guided_points, guided_class), (free_points, free_class));
}

#[gpui::test]
fn decisions_these_rules_lack_are_set_aside_and_named(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Dark);
    workbench.update_in(cx, |this, _, _| {
        this.review.decisions.insert("ZZ9".into(), ReviewDecision::Deferred);
    });
    draw(cx);
    workbench.read_with(cx, |this, _| {
        assert_eq!(this.review.set_aside(this.rules), ["ZZ9"]);
        assert_eq!(this.review.total_progress(this.rules, &[]).reviewed, 0, "set aside is not progress");
    });
}

#[gpui::test]
fn the_pvs1_guide_opens_from_its_step_and_escape_returns_to_the_step(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    click(cx, "guided-step-variant-type");
    workbench.update_in(cx, |this, _, _| {
        let pvs1 = this.rules.criterion("PVS1").unwrap();
        this.toggle(pvs1);
    });
    draw(cx);
    assert_eq!(workbench.read_with(cx, |this, _| this.guide_open.clone()), Some("PVS1".into()));
    press(cx, "escape");
    workbench.read_with(cx, |this, _| assert!(this.guide_open.is_none() && this.guided == Some(1)));
    assert!(cx.debug_bounds("guided-step-page").is_some());
}

fn press(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    draw(cx);
}

fn press_enter(cx: &mut VisualTestContext) {
    let keystroke = Keystroke::parse("enter").expect("enter is a key");
    cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
    cx.simulate_event(KeyUpEvent { keystroke });
    draw(cx);
}

#[gpui::test]
fn the_keyboard_moves_through_the_steps_with_next_and_previous(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    let step = |cx: &mut VisualTestContext| workbench.read_with(cx, |this, _| this.guided);
    let (previous, next) =
        workbench.read_with(cx, |this, _| (this.handles.step_previous.clone(), this.handles.step_next.clone()));
    let focused = |cx: &mut VisualTestContext| cx.update(|window, cx| window.focused(cx));
    for _ in 0..200 {
        press(cx, "tab");
        if focused(cx).as_ref() == Some(&next) {
            break;
        }
    }
    assert_eq!(focused(cx), Some(next.clone()), "Tab reaches Next");
    press_enter(cx);
    assert_eq!(step(cx), Some(1));
    press_enter(cx);
    assert_eq!(step(cx), Some(2), "the focus stays on Next");
    press(cx, "shift-tab");
    assert_eq!(focused(cx), Some(previous), "Shift+Tab goes back to Previous");
    press_enter(cx);
    assert_eq!(step(cx), Some(1));
}

#[gpui::test]
fn the_population_helper_offers_keep_or_a_labelled_replacement_against_pm2(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    // Tall enough that the helper's buttons are inside the window, where a click lands.
    cx.simulate_resize(gpui::size(gpui::px(1240.), gpui::px(2400.)));
    workbench.update_in(cx, |this, window, cx| {
        this.set_inheritance(classivar_core::Inheritance::AutosomalDominant, cx);
        this.today.insert("PM2".into(), Choice::new(Strength::Supporting));
        this.decide("BA1", Some(ReviewDecision::DoesNotApply));
        this.helpers.population.open = true;
        let typed = ["0.0001", "120000", "0.00005", "500", "0.5", "0.02"];
        for (input, text) in this.helpers.population.inputs.iter().zip(typed) {
            input.update(cx, |input, cx| input.set_value(text, window, cx));
        }
    });
    click(cx, "population-apply-BS1-keep");
    workbench.read_with(cx, |this, _| {
        assert!(this.kept.contains("BS1"), "Keep sets the proposal aside");
        assert!(this.today.contains_key("PM2") && !this.today.contains_key("BS1"));
    });
    assert!(cx.debug_bounds("population-apply-BS1-replace").is_none(), "a kept proposal is set aside");
    workbench.update_in(cx, |this, _, _| this.kept.clear());
    click(cx, "population-apply-BS1-replace");
    assert_eq!(state(&workbench, cx, "BS1"), ReviewState::Applied);
    assert_eq!(state(&workbench, cx, "PM2"), ReviewState::NotYetReviewed);
    assert_eq!(state(&workbench, cx, "BA1"), ReviewState::DoesNotApply, "BA1 is untouched");
}

#[gpui::test]
fn answering_the_pvs1_guide_with_does_not_apply_records_the_review_decision(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        this.today
            .insert("PVS1".into(), Choice::guided(Strength::VeryStrong, "nmd".into(), Some("established".into())));
        this.guide_gene_level = Some("no_lof_mechanism".into());
        this.answer_guide("PVS1", None, String::new());
    });
    draw(cx);
    assert_eq!(state(&workbench, cx, "PVS1"), ReviewState::DoesNotApply);
    workbench.read_with(cx, |this, _| {
        assert!(!this.today.contains_key("PVS1"));
        assert_eq!(this.review.decisions.get("PVS1"), Some(&ReviewDecision::DoesNotApply));
        assert_eq!(this.review.total_progress(this.rules, &[]).reviewed, 1);
        assert!(this.score_on_screen().unwrap().class().is_none());
    });
}

#[gpui::test]
fn replacing_both_ba1_and_bs1_with_pm2_clears_both_rivals_and_allows_reconsider(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    cx.simulate_resize(gpui::size(gpui::px(1240.), gpui::px(2400.)));
    workbench.update_in(cx, |this, _, _| {
        this.today.insert("BA1".into(), Choice::new(Strength::StandAlone));
        this.today.insert("BS1".into(), Choice::new(Strength::Strong));
        this.helpers.population.open = true;
        this.helpers.population.absent = true;
    });
    click(cx, "population-apply-PM2-keep");
    assert!(cx.debug_bounds("population-apply-PM2-replace").is_none(), "kept proposal is set aside");
    click(cx, "population-apply-PM2-reconsider");
    assert!(cx.debug_bounds("population-apply-PM2-replace").is_some(), "Reconsider restores the proposal");
    click(cx, "population-apply-PM2-replace");
    assert_eq!(state(&workbench, cx, "PM2"), ReviewState::Applied);
    assert_eq!(state(&workbench, cx, "BA1"), ReviewState::NotYetReviewed);
    assert_eq!(state(&workbench, cx, "BS1"), ReviewState::NotYetReviewed);
    workbench.read_with(cx, |this, _| {
        assert!(
            this.score_on_screen().unwrap().class().is_some(),
            "both rivals were removed, so class is not withheld"
        );
    });
}

#[gpui::test]
fn optional_fact_questions_show_matching_hints_without_scoring_evidence(cx: &mut TestAppContext) {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = guided(cx, theme);
        cx.simulate_resize(gpui::size(gpui::px(1240.), gpui::px(1800.)));
        click(cx, "guided-step-variant-type");
        assert!(cx.debug_bounds("guided-hints-variant-type").is_none());
        click(cx, "guided-facts-toggle");
        click(cx, "guided-fact-consequence-1");
        click(cx, "guided-fact-splicing-4");
        assert!(cx.debug_bounds("guided-hints-variant-type").is_some(), "matching hints appear on the step");
        workbench.read_with(cx, |this, _| {
            assert_eq!(this.review.facts.get("consequence").map(String::as_str), Some("missense"));
            assert_eq!(this.review.facts.get("splicing").map(String::as_str), Some("observed normal splicing (RNA)"));
            assert!(this.score_on_screen().unwrap().class().is_none(), "facts and hints never score evidence");
        });
        // Clicking the active answer again clears it.
        click(cx, "guided-fact-consequence-1");
        click(cx, "guided-fact-splicing-4");
        assert!(cx.debug_bounds("guided-hints-variant-type").is_none());
    }
}

#[gpui::test]
fn helper_bar_reads_its_criteria_from_the_shared_core_table(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.read_with(cx, |this, _| {
        let entries = this.helper_entry_codes();
        for &(key, codes) in classivar_core::rules::HELPER_CRITERIA {
            let (_, found) = entries.iter().find(|(k, _)| *k == key).unwrap_or_else(|| panic!("missing helper {key}"));
            assert_eq!(*found, codes, "helper {key} matches HELPER_CRITERIA");
        }
        for (key, codes) in &entries {
            if *key == "ba1-reference" {
                continue;
            }
            assert_eq!(
                classivar_core::rules::helper_criteria(key),
                Some(*codes),
                "helper {key} comes from HELPER_CRITERIA"
            );
        }
    });
}

#[gpui::test]
fn review_strip_hides_when_editing_an_earlier_report(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.editing = super::Editing::Record(0);
    });
    draw(cx);
    assert!(cx.debug_bounds("review-strip").is_none(), "today's review strip does not show under an earlier report");
    workbench.update_in(cx, |this, _, _| this.editing = super::Editing::Today);
    draw(cx);
    assert!(cx.debug_bounds("review-strip").is_some());
}

#[cfg(debug_assertions)]
#[gpui::test]
fn reloading_rules_without_a_path_clears_guided_step_and_kept_proposals(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    let mut without_path = classivar_core::RuleSet::builtin().unwrap().clone();
    without_path.path = None;
    let leaked: &'static classivar_core::RuleSet = Box::leak(Box::new(without_path));
    workbench.update_in(cx, |this, window, cx| {
        this.keep_rival("BS1");
        this.reload_rules(leaked, window, cx);
        assert_eq!(this.guided, None);
        assert!(this.kept.is_empty());
    });
    draw(cx);
}
