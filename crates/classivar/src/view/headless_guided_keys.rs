//! Keyboard tests for the guided review: Next and Previous, the optional context
//! questions, and a helper's Replace, Keep and Reconsider buttons. Real key events
//! go to the real window; nothing appears on screen.
//!
//! Explicit imports on purpose (see `headless.rs`).

use classivar_core::Strength;
use gpui::{FocusHandle, TestAppContext, VisualTestContext};
use gpui_kit::component::ThemeMode;

use super::Choice;
use super::headless::draw;
use super::headless_guided::guided;
use super::headless_keys::{focused, press, press_enter};

/// Every Tab stop in the window, in order, starting from the top.
fn tab_stops(cx: &mut VisualTestContext, start: &FocusHandle) -> Vec<FocusHandle> {
    cx.update(|window, cx| window.focus(start, cx));
    draw(cx);
    let mut stops = Vec::new();
    for _ in 0..600 {
        press(cx, "tab");
        let Some(now) = focused(cx) else { break };
        if stops.contains(&now) {
            break;
        }
        stops.push(now);
    }
    stops
}

/// The stops in `after` that were not in `before`: the controls that just appeared.
fn new_stops(before: &[FocusHandle], after: &[FocusHandle]) -> Vec<FocusHandle> {
    after.iter().filter(|stop| !before.contains(stop)).cloned().collect()
}

fn tab_to(cx: &mut VisualTestContext, target: &FocusHandle) {
    for _ in 0..600 {
        if focused(cx).as_ref() == Some(target) {
            return;
        }
        press(cx, "tab");
    }
    panic!("Tab never reaches the control");
}

#[gpui::test]
fn the_keyboard_moves_through_the_steps_with_next_and_previous(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    let step = |cx: &mut VisualTestContext| workbench.read_with(cx, |this, _| this.guided);
    let (previous, next) =
        workbench.read_with(cx, |this, _| (this.handles.step_previous.clone(), this.handles.step_next.clone()));
    tab_to(cx, &next);
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
fn the_keyboard_opens_the_context_questions_and_answers_and_clears_them(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    let (page, answers) = workbench.read_with(cx, |this, _| {
        (this.handles.page_focus.clone(), this.rules.facts.iter().map(|fact| fact.answers.len()).sum::<usize>())
    });
    let open = |cx: &mut VisualTestContext| workbench.read_with(cx, |this, _| this.facts_open);
    let consequence =
        |cx: &mut VisualTestContext| workbench.read_with(cx, |this, _| this.review.facts.get("consequence").cloned());
    // The toggle is the stop just before the first answer that appears when the card opens.
    let closed = tab_stops(cx, &page);
    workbench.update_in(cx, |this, _, cx| {
        this.facts_open = true;
        cx.notify();
    });
    let opened = tab_stops(cx, &page);
    let chips = new_stops(&closed, &opened);
    assert_eq!(chips.len(), answers, "every answer is a Tab stop");
    let toggle = opened[opened.iter().position(|stop| *stop == chips[0]).unwrap() - 1].clone();
    workbench.update_in(cx, |this, _, cx| {
        this.facts_open = false;
        cx.notify();
    });
    draw(cx);

    tab_to(cx, &toggle);
    press_enter(cx);
    assert!(open(cx), "Enter opens the questions");
    press(cx, "tab"); // the first answer of the first question
    press_enter(cx);
    assert_eq!(consequence(cx).as_deref(), Some("null"));
    press(cx, "tab");
    press_enter(cx);
    assert_eq!(consequence(cx).as_deref(), Some("missense"), "another answer replaces the first");
    press_enter(cx);
    assert_eq!(consequence(cx), None, "Enter on the chosen answer clears it");
    tab_to(cx, &toggle);
    press_enter(cx);
    assert!(!open(cx), "Enter on the toggle hides the questions again");
    workbench.read_with(cx, |this, _| assert!(this.today.is_empty(), "answers never count as evidence"));
}

#[gpui::test]
fn the_keyboard_keeps_reconsiders_and_replaces_a_contradicting_proposal(cx: &mut TestAppContext) {
    let (workbench, cx) = guided(cx, ThemeMode::Light);
    let page = workbench.read_with(cx, |this, _| this.handles.page_focus.clone());
    workbench.update_in(cx, |this, _, cx| {
        this.count_today("BA1".into(), Choice::new(Strength::StandAlone));
        this.count_today("BS1".into(), Choice::new(Strength::Strong));
        this.helpers.population.open = true;
        this.helpers.population.absent = true;
        cx.notify();
    });
    let kept = |cx: &mut VisualTestContext| workbench.read_with(cx, |this, _| this.kept.contains("PM2"));
    let offered = tab_stops(cx, &page);
    workbench.update_in(cx, |this, _, cx| {
        this.keep_rival("PM2");
        cx.notify();
    });
    let set_aside = tab_stops(cx, &page);
    let reconsider = new_stops(&offered, &set_aside);
    assert_eq!(reconsider.len(), 1, "a kept proposal offers only Reconsider");

    tab_to(cx, &reconsider[0]);
    press_enter(cx);
    assert!(!kept(cx), "Enter on Reconsider shows the proposal again");
    let offered = tab_stops(cx, &page);
    let replace_and_keep = new_stops(&set_aside, &offered);
    assert_eq!(replace_and_keep.len(), 2, "Replace and Keep come back");
    tab_to(cx, &replace_and_keep[1]);
    press_enter(cx);
    assert!(kept(cx), "Enter on Keep sets the proposal aside");

    // Reconsider once more, then take the other answer.
    let set_aside = tab_stops(cx, &page);
    tab_to(cx, &new_stops(&offered, &set_aside)[0]);
    press_enter(cx);
    let offered = tab_stops(cx, &page);
    tab_to(cx, &new_stops(&set_aside, &offered)[0]);
    press_enter(cx);
    workbench.read_with(cx, |this, _| {
        assert!(this.today.contains_key("PM2"), "Enter on Replace applies PM2");
        assert!(!this.today.contains_key("BA1") && !this.today.contains_key("BS1"));
    });
    press(cx, "escape");
    workbench.read_with(cx, |this, _| {
        assert!(!this.helpers.population.open, "Escape closes the helper");
        assert!(this.today.contains_key("PM2"), "and leaves the evidence alone");
    });
}
