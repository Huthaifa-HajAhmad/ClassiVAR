//! Keyboard tests in GPUI's headless test platform: a real Tab or Enter key is
//! sent to the real window and the tests read which element holds the focus and
//! what changed. Nothing appears on screen and no real keyboard is used.
//!
//! The window is opened the way `main` opens it, so the toolkit's root (which
//! turns Tab into a focus move) is part of what is tested.

use gpui::{Entity, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext};
use gpui_kit::Focusable;
use gpui_kit::component::ThemeMode;

use super::headless::{draw, open};
use super::{Confirm, Workbench};

/// A window past the opening screen, where the editor and its text boxes are.
fn open_editor(cx: &mut TestAppContext) -> (Entity<Workbench>, &mut VisualTestContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| this.welcome = false);
    draw(cx);
    (workbench, cx)
}

/// Press a key, then draw, as a real session does between key presses.
fn press(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    draw(cx);
}

/// Press and release Enter, as a keyboard does: a button answers on the release.
fn press_enter(cx: &mut VisualTestContext) {
    let keystroke = Keystroke::parse("enter").expect("enter is a key");
    cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
    cx.simulate_event(KeyUpEvent { keystroke });
    draw(cx);
}

fn focused(cx: &mut VisualTestContext) -> Option<FocusHandle> {
    cx.update(|window, cx| window.focused(cx))
}

fn question_window(workbench: &Entity<Workbench>, cx: &mut VisualTestContext) -> FocusHandle {
    workbench.read_with(cx, |this, _| this.confirm_focus.clone())
}

fn question_holds_focus(question: &FocusHandle, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| question.contains_focused(window, cx))
}

/// Open the "load the fictional example" question and draw it.
fn ask_to_load_the_example(workbench: &Entity<Workbench>, cx: &mut VisualTestContext) -> FocusHandle {
    workbench.update_in(cx, |this, _, _| this.ask(Confirm::LoadExample));
    draw(cx);
    question_window(workbench, cx)
}

#[gpui::test]
fn tab_and_shift_tab_stay_inside_a_question_and_reach_both_answers(cx: &mut TestAppContext) {
    let (workbench, cx) = open_editor(cx);
    let question = ask_to_load_the_example(&workbench, cx);
    assert!(question_holds_focus(&question, cx), "a question takes the keyboard when it opens");

    let mut reached: Vec<FocusHandle> = Vec::new();
    for key in ["tab", "tab", "tab", "tab", "shift-tab", "shift-tab", "shift-tab"] {
        press(cx, key);
        assert!(question_holds_focus(&question, cx), "{key} left the question for the page behind it");
        let now = focused(cx).expect("something inside the question holds the focus");
        if now != question && !reached.contains(&now) {
            reached.push(now);
        }
    }
    assert_eq!(reached.len(), 2, "Tab reaches exactly the two answers, Cancel and Yes");
}

#[gpui::test]
fn the_keyboard_alone_can_cancel_or_confirm_a_question(cx: &mut TestAppContext) {
    let (workbench, cx) = open_editor(cx);

    ask_to_load_the_example(&workbench, cx);
    press(cx, "tab"); // Cancel is the first answer
    press_enter(cx);
    assert!(workbench.read_with(cx, |this, _| this.confirm.is_none() && !this.example), "Enter on Cancel closes it");

    ask_to_load_the_example(&workbench, cx);
    press(cx, "tab");
    press(cx, "tab"); // "Yes, continue" is the second
    press_enter(cx);
    assert!(workbench.read_with(cx, |this, _| this.confirm.is_none() && this.example), "Enter on Yes answers it");
}

#[gpui::test]
fn tab_starts_from_a_fresh_window_and_from_after_a_question_closes(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    draw(cx);
    press(cx, "tab");
    assert!(focused(cx).is_some(), "the first Tab in a fresh window must land on a control");

    ask_to_load_the_example(&workbench, cx);
    press(cx, "escape");
    assert!(workbench.read_with(cx, |this, _| this.confirm.is_none()));
    press(cx, "tab");
    assert!(focused(cx).is_some(), "Tab must still work after a question is answered");
}

#[gpui::test]
fn the_tab_order_reaches_both_text_boxes_and_a_stop_for_every_criterion(cx: &mut TestAppContext) {
    let (workbench, cx) = open_editor(cx);
    draw(cx);
    let (variant, gene, criteria) = workbench.read_with(cx, |this, app| {
        (
            this.variant_input.read(app).focus_handle(app),
            this.gene_input.read(app).focus_handle(app),
            this.rules.criteria.len(),
        )
    });
    let mut stops: Vec<FocusHandle> = Vec::new();
    for _ in 0..600 {
        press(cx, "tab");
        let Some(now) = focused(cx) else { break };
        if stops.contains(&now) {
            break;
        }
        stops.push(now);
    }
    assert!(stops.contains(&variant), "the variant box is in the Tab order");
    assert!(stops.contains(&gene), "the gene box is in the Tab order");
    assert!(stops.len() >= criteria + 2, "{} stops for {criteria} criteria and two boxes", stops.len());
}

#[gpui::test]
fn a_question_opened_while_a_page_control_has_the_focus_takes_it(cx: &mut TestAppContext) {
    let (workbench, cx) = open_editor(cx);
    draw(cx);
    press(cx, "tab");
    press(cx, "tab");
    let page_control = focused(cx).expect("the page's second control has the focus");

    let question = ask_to_load_the_example(&workbench, cx);
    assert_ne!(focused(cx), Some(page_control), "the control behind the question must lose the focus");
    assert!(question_holds_focus(&question, cx));
}

#[gpui::test]
fn escape_cancels_a_question_even_while_an_answer_has_the_focus(cx: &mut TestAppContext) {
    let (workbench, cx) = open_editor(cx);
    ask_to_load_the_example(&workbench, cx);
    press(cx, "tab");
    press(cx, "tab"); // on "Yes, continue"
    press(cx, "escape");
    assert!(workbench.read_with(cx, |this, _| this.confirm.is_none() && !this.example), "Escape means Cancel");
}
