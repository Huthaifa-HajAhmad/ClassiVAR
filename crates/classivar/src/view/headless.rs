//! Tests that open the real window in GPUI's headless test platform: nothing
//! appears on screen and no mouse or keyboard is used, so they can run while the
//! machine is in use. They draw the real views for the states people reach, so a
//! panic in a render function shows up here. The navigation test also clicks
//! the rendered controls using GPUI's simulated pointer.
//!
//! Explicit imports on purpose: `use gpui_kit::*` also brings GPUI's `test` macro
//! into scope, and `#[gpui::test]` then expands into itself without end.

use classivar_core::{Class, GeneDecision, RuleSet, Strength};
use gpui::{Entity, TestAppContext, VisualTestContext, WindowOptions};
use gpui_kit::component::ThemeMode;
use gpui_kit::{AppContext, init, open_window};

use super::{Choice, Editing, Mode, Workbench};
use crate::settings::Picture;
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
            open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| {
                    let mut workbench = Workbench::new(rules(), window, cx);
                    // The real settings file may name another picture; tests start from the dial.
                    workbench.picture = crate::settings::Picture::Dial;
                    workbench
                })
            })
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
fn the_published_case_draws_without_panicking_for_today_and_every_record(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.set_mode(Mode::ReportedVariant);
    });
    draw(cx);
    // Every criterion's page ("More") draws, with its strength ladder.
    let codes: Vec<String> =
        workbench.read_with(cx, |this, _| this.rules.criteria.iter().map(|c| c.code.clone()).collect());
    for code in codes {
        workbench.update_in(cx, |this, _, _| this.about = Some(code));
        draw(cx);
    }
    workbench.update_in(cx, |this, _, _| this.about = None);
    workbench.update_in(cx, |this, _, _| this.comparison = true);
    draw(cx);
    let records = workbench.read_with(cx, |this, _| this.records.len());
    assert!(records >= 2, "the published case has two panel records to draw");
    for index in 0..records {
        workbench.update_in(cx, |this, _, _| this.editing = Editing::Record(index));
        draw(cx);
    }
    workbench.update_in(cx, |this, _, _| this.editing = Editing::Today);
    draw(cx);
}

#[gpui::test]
fn the_published_case_draws_without_panicking_in_the_dark_theme_and_in_new_variant_mode(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Dark);
    workbench.update_in(cx, |this, window, cx| this.load_real_case(window, cx));
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
                let level = c.gene_level.first().map(|level| level.id.clone());
                this.today.insert(c.code.clone(), Choice::guided(strength, id, level));
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
        this.welcome = false;
        let helpers = &mut this.helpers;
        helpers.predictor.open = true;
        helpers.functional.open = true;
        helpers.locus.open = true;
        helpers.segregation.open = true;
        helpers.pm3.open = true;
        helpers.ba1_open = true;
        helpers.de_novo.open = true;
        helpers.population.open = true;
        helpers.population.absent = true;
    });
    draw(cx);
    // The population helper with numbers typed: its findings and Apply buttons draw too.
    workbench.update_in(cx, |this, window, cx| {
        for (input, text) in this.helpers.population.inputs.iter().zip(["0.06", "30000", "0.05", "500", "0.5", "0.02"])
        {
            input.update(cx, |input, cx| input.set_value(text, window, cx));
        }
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

#[gpui::test]
fn mixed_evidence_with_a_previewed_block_draws_in_both_themes_and_keeps_the_evidence(cx: &mut TestAppContext) {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = open(cx, theme);
        workbench.update_in(cx, |this, _, _| {
            for (code, strength) in
                [("PS1", Strength::Strong), ("PM2", Strength::Supporting), ("BP7", Strength::Supporting)]
            {
                this.today.insert(code.into(), Choice::new(strength));
            }
            this.inspected = Some("PS1".into());
            this.preview = true;
        });
        draw(cx);
        workbench.read_with(cx, |this, _| {
            assert_eq!(this.today.len(), 3, "a preview never removes evidence");
            assert_eq!(this.stage.needle.get().1, 4, "the marker is heading for the real total, not the preview's");
        });
    }
}

#[gpui::test]
fn previewing_the_only_criterion_preserves_the_case(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        this.today.insert("PS1".into(), Choice::new(classivar_core::Strength::Strong));
        let preview = this.preview_without("PS1").unwrap();
        assert!(!preview.has_evidence);
        assert!(preview.class().is_none());
        assert!(this.today.contains_key("PS1"));
    });
}

#[gpui::test]
fn a_draft_record_leaves_completed_reports_available(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Dark);
    workbench.update_in(cx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.set_mode(Mode::ReportedVariant);
        let complete = this.records.len();
        this.add_record(window, cx);
        let (missing, comparison) = this.compare_with_records();
        assert_eq!(missing.len(), 1);
        assert_eq!(comparison.unwrap().records.len(), complete);
    });
}

#[gpui::test]
fn reading_and_comparison_controls_preserve_the_editing_scope(cx: &mut TestAppContext) {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        let (workbench, cx) = open(cx, theme);
        workbench.update_in(cx, |this, window, cx| {
            this.load_real_case(window, cx);
            this.set_mode(Mode::ReportedVariant);
            this.show_record(0, window, cx);
        });
        cx.simulate_resize(gpui::size(gpui::px(960.), gpui::px(600.)));
        for (control, reading, comparison) in [
            ("reading-view", true, false),
            ("view-compare", true, true),
            ("report-0", true, true),
            ("view-evidence", true, false),
            ("reading-view", false, false),
        ] {
            draw(cx);
            let bounds = cx.debug_bounds(control).expect("the control is visible");
            cx.simulate_click(bounds.center(), Default::default());
            workbench.read_with(cx, |this, _| {
                assert_eq!((this.reading, this.comparison), (reading, comparison), "{control}");
                assert!(this.editing == Editing::Record(0));
                if control == "report-0" {
                    assert_eq!(this.baseline, Some(0));
                }
            });
        }
    }
}

#[gpui::test]
fn the_opening_screen_offers_no_fictional_case(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(gpui::size(gpui::px(960.), gpui::px(600.)));
    draw(cx);
    assert!(cx.debug_bounds("welcome-example").is_none(), "no demo case is offered on the opening screen");
    let preview = workbench.read_with(cx, |this, _| format!("welcome-preview-{}", this.picture.name().to_lowercase()));
    let preview: &'static str = Box::leak(preview.into_boxed_str());
    assert!(cx.debug_bounds(preview).is_some(), "the opening screen draws the chosen picture, empty, instead");
}

/// The published RYR1 case with the reported mode on, in a window of the given size.
fn real_case_at(
    cx: &mut TestAppContext,
    mode: ThemeMode,
    (w, h): (f32, f32),
) -> (Entity<Workbench>, &mut VisualTestContext) {
    let (workbench, cx) = open(cx, mode);
    workbench.update_in(cx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.set_mode(Mode::ReportedVariant);
    });
    cx.simulate_resize(gpui::size(gpui::px(w), gpui::px(h)));
    (workbench, cx)
}

#[gpui::test]
fn every_picture_draws_with_a_previewed_block_in_both_themes_and_at_both_window_sizes(cx: &mut TestAppContext) {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        for size in [(1240., 900.), (960., 600.)] {
            let (workbench, cx) = real_case_at(cx, theme, size);
            for picture in Picture::ALL {
                workbench.update_in(cx, |this, _, _| {
                    this.picture = picture;
                    this.inspected = Some("PS4".into());
                    this.preview = true;
                });
                draw(cx);
                workbench.read_with(cx, |this, _| {
                    assert_eq!(this.today.len(), 5, "a preview never removes evidence");
                    assert_eq!(this.stage.rail_shown.get(), size.0 >= 1180., "the rail folds away in a narrow window");
                });
            }
        }
    }
}

#[gpui::test]
fn clicking_a_dial_slice_selects_it_and_a_second_click_previews_without_it(cx: &mut TestAppContext) {
    let (workbench, cx) = real_case_at(cx, ThemeMode::Light, (1240., 900.));
    workbench.update_in(cx, |this, _, _| this.picture = Picture::Dial);
    draw(cx);
    let target = workbench.update_in(cx, |this, _, cx| {
        let result = this.rules.assess(&super::to_applied(&this.today)).expect("the published case can be scored");
        this.slice_centre(&result, "PS4", cx)
    });
    let target = target.expect("the dial was painted with a PS4 slice");
    cx.simulate_click(target, Default::default());
    workbench.read_with(cx, |this, _| {
        assert_eq!(this.inspected.as_deref(), Some("PS4"));
        assert!(!this.preview);
    });
    draw(cx);
    cx.simulate_click(target, Default::default());
    workbench.read_with(cx, |this, _| {
        assert!(this.preview, "a second click previews the class without the slice");
        assert!(this.today.contains_key("PS4"), "the evidence is kept");
    });
}

#[gpui::test]
fn hovering_a_dial_slice_or_a_column_or_track_block_shows_its_card_and_leaving_hides_it(cx: &mut TestAppContext) {
    let (workbench, cx) = real_case_at(cx, ThemeMode::Dark, (960., 600.));
    workbench.update_in(cx, |this, _, _| this.picture = Picture::Dial);
    draw(cx);
    let target = workbench.update_in(cx, |this, _, cx| {
        let result = this.rules.assess(&super::to_applied(&this.today)).expect("the published case can be scored");
        this.slice_centre(&result, "PS4", cx)
    });
    let away = gpui::point(gpui::px(4.), gpui::px(4.));
    cx.simulate_mouse_move(target.expect("the dial was painted with a PS4 slice"), None, Default::default());
    workbench.read_with(cx, |this, _| assert_eq!(this.hovered.as_deref(), Some("PS4")));
    draw(cx);
    cx.simulate_mouse_move(away, None, Default::default());
    workbench.read_with(cx, |this, _| assert_eq!(this.hovered, None, "leaving the dial hides the card"));

    workbench.update_in(cx, |this, _, _| this.picture = Picture::Column);
    draw(cx);
    // Two earlier reports do not fit beside the readout here, so the column sits below it: scroll to it.
    workbench
        .update_in(cx, |this, _, _| this.handles.result_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(-300.))));
    draw(cx);
    let block = cx.debug_bounds("column-PS4").expect("the column has a PS4 block");
    cx.simulate_mouse_move(block.center(), None, Default::default());
    workbench.read_with(cx, |this, _| assert_eq!(this.hovered.as_deref(), Some("PS4")));
    draw(cx);
    cx.simulate_mouse_move(away, None, Default::default());
    workbench.read_with(cx, |this, _| assert_eq!(this.hovered, None));

    workbench.update_in(cx, |this, _, _| {
        this.picture = Picture::Track;
        // Back to the top: the column step scrolled the stage down.
        this.handles.result_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
    });
    draw(cx);
    let block = cx.debug_bounds("block-PS4").expect("the track has a PS4 block");
    cx.simulate_mouse_move(block.center(), None, Default::default());
    workbench.read_with(cx, |this, _| assert_eq!(this.hovered.as_deref(), Some("PS4")));
    draw(cx);
    cx.simulate_mouse_move(away, None, Default::default());
    workbench.read_with(cx, |this, _| assert_eq!(this.hovered, None));
    workbench.read_with(cx, |this, _| assert_eq!(this.inspected, None, "hovering never selects"));
}

#[gpui::test]
fn the_rail_opens_an_earlier_report_and_returns_to_today(cx: &mut TestAppContext) {
    let (workbench, cx) = real_case_at(cx, ThemeMode::Dark, (1240., 900.));
    for (selector, expected) in [("rail-record-0", Editing::Record(0)), ("rail-record-today", Editing::Today)] {
        draw(cx);
        let bounds = cx.debug_bounds(selector).expect("the stop is on the rail");
        cx.simulate_click(bounds.center(), Default::default());
        workbench.read_with(cx, |this, _| assert!(this.editing == expected, "{selector}"));
    }
}

#[gpui::test]
fn choosing_a_stronger_strength_from_the_menu_asks_for_a_reason_and_a_weaker_one_applies_at_once(
    cx: &mut TestAppContext,
) {
    let (workbench, cx) = real_case_at(cx, ThemeMode::Light, (1240., 900.));
    workbench.update_in(cx, |this, window, cx| {
        let pm2 = this.rules.criterion("PM2").expect("PM2 exists").clone();
        assert!(pm2.allowed_strengths().contains(&Strength::Moderate), "the raise must be one the menu offers");
        this.choose_strength(&pm2, Strength::Moderate, window, cx);
        assert!(this.raising.as_ref().is_some_and(|raise| raise.code == "PM2"), "a raise waits for its reason");
        assert_eq!(this.today["PM2"].strength, Strength::Supporting, "nothing changes before the reason");
        this.raising = None;
        let pm5 = this.rules.criterion("PM5").expect("PM5 exists").clone();
        this.choose_strength(&pm5, Strength::Supporting, window, cx);
        assert_eq!(this.today["PM5"].strength, Strength::Supporting);
        assert!(this.raising.is_none());
    });
    draw(cx);
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

        this.boxes.variant.update(cx, |input, cx| input.set_value("NM_000000.1:c.1A>G", window, cx));
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
fn closing_an_unchanged_case_just_closes(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    assert!(cx.simulate_close(), "nothing to lose, so the window closes");
    assert!(workbench.read_with(cx, |this, _| this.confirm.is_none()));

    workbench.update_in(cx, |this, _, cx| {
        apply_default(this, "PM2");
        let saved = this.case_file(cx);
        this.remember_saved(saved);
    });
    assert!(cx.simulate_close(), "a saved case closes without a question");
}

#[gpui::test]
fn closing_a_saved_case_that_changed_asks(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, cx| {
        apply_default(this, "PM2");
        let saved = this.case_file(cx);
        this.remember_saved(saved);
        apply_default(this, "PS1");
    });
    assert!(!cx.simulate_close(), "a change after saving would be lost");
}

#[gpui::test]
fn closing_a_changed_case_asks_first_and_cancel_keeps_the_window(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| apply_default(this, "PM2"));
    assert!(!cx.simulate_close(), "unsaved work must not be thrown away by one click on the close button");
    assert!(workbench.read_with(cx, |this, _| matches!(this.confirm, Some(super::Confirm::CloseWindow))));
    draw(cx);

    workbench.update_in(cx, |this, window, cx| this.cancel_confirm(window, cx));
    assert!(
        workbench.read_with(cx, |this, _| this.confirm.is_none() && !this.today.is_empty()),
        "Cancel keeps the case"
    );
}

#[gpui::test]
fn a_close_request_replaces_the_question_that_was_open_and_yes_closes_the_window(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| {
        apply_default(this, "PM2");
        this.ask(super::Confirm::Clear(Editing::Today));
    });
    assert!(!cx.simulate_close());
    assert!(
        workbench.read_with(cx, |this, _| matches!(this.confirm, Some(super::Confirm::CloseWindow))),
        "the question about closing must show, not stay hidden behind the older one"
    );
    assert!(
        workbench.read_with(cx, |this, _| this.today.contains_key("PM2")),
        "the older question was cancelled, not answered"
    );

    workbench.update_in(cx, |this, window, cx| this.apply_confirm(window, cx));
    assert!(cx.windows().is_empty(), "Yes closes the window");
}
