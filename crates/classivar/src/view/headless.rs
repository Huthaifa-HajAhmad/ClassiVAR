//! Tests that open the real window in GPUI's headless test platform: nothing
//! appears on screen and no mouse or keyboard is used, so they can run while the
//! machine is in use. They draw the real views for the states people reach, so a
//! panic in a render function shows up here. The navigation test also clicks
//! the rendered controls using GPUI's simulated pointer.
//!
//! Explicit imports on purpose: `use gpui_kit::*` also brings GPUI's `test` macro
//! into scope, and `#[gpui::test]` then expands into itself without end.

use classivar_core::{Class, RuleSet, Strength};
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpui_kit::component::ThemeMode;
use gpui_kit::init;

use super::{Choice, Editing, Mode, Workbench};
use crate::settings::Picture;
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
    workbench.update_in(cx, |this, _, _| this.comparison = true);
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
        this.welcome = false;
        this.show_helpers = true;
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
            assert_eq!(this.needle.get().1, 4, "the marker is heading for the real total, not the preview's");
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
        this.load_example(window, cx);
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
            this.load_example(window, cx);
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
fn opening_example_starts_a_fictional_reported_case(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(gpui::size(gpui::px(960.), gpui::px(600.)));
    draw(cx);
    let button = cx.debug_bounds("welcome-example").expect("the example is visible on first launch");
    cx.simulate_click(button.center(), Default::default());
    workbench.read_with(cx, |this, _| {
        assert!(this.example && this.mode == Mode::ReportedVariant);
        assert_eq!(this.today.len(), 4);
    });
}

/// The example with the reported mode on, in a window of the given size.
fn example_at(
    cx: &mut TestAppContext,
    mode: ThemeMode,
    (w, h): (f32, f32),
) -> (Entity<Workbench>, &mut VisualTestContext) {
    let (workbench, cx) = open(cx, mode);
    workbench.update_in(cx, |this, window, cx| {
        this.load_example(window, cx);
        this.set_mode(Mode::ReportedVariant);
    });
    cx.simulate_resize(gpui::size(gpui::px(w), gpui::px(h)));
    (workbench, cx)
}

#[gpui::test]
fn every_picture_draws_with_a_previewed_block_in_both_themes_and_at_both_window_sizes(cx: &mut TestAppContext) {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        for size in [(1240., 900.), (960., 600.)] {
            let (workbench, cx) = example_at(cx, theme, size);
            for picture in Picture::ALL {
                workbench.update_in(cx, |this, _, _| {
                    this.picture = picture;
                    this.inspected = Some("PM5".into());
                    this.preview = true;
                });
                draw(cx);
                workbench.read_with(cx, |this, _| {
                    assert_eq!(this.today.len(), 4, "a preview never removes evidence");
                    assert_eq!(this.rail_shown.get(), size.0 >= 1180., "the rail folds away in a narrow window");
                });
            }
        }
    }
}

#[gpui::test]
fn clicking_a_dial_slice_selects_it_and_a_second_click_previews_without_it(cx: &mut TestAppContext) {
    let (workbench, cx) = example_at(cx, ThemeMode::Light, (1240., 900.));
    workbench.update_in(cx, |this, _, _| this.picture = Picture::Dial);
    draw(cx);
    let target = workbench.update_in(cx, |this, _, cx| {
        let result = this.rules.assess(&super::to_applied(&this.today)).expect("the example can be scored");
        this.slice_centre(&result, "PM5", cx)
    });
    let target = target.expect("the dial was painted with a PM5 slice");
    cx.simulate_click(target, Default::default());
    workbench.read_with(cx, |this, _| {
        assert_eq!(this.inspected.as_deref(), Some("PM5"));
        assert!(!this.preview);
    });
    draw(cx);
    cx.simulate_click(target, Default::default());
    workbench.read_with(cx, |this, _| {
        assert!(this.preview, "a second click previews the class without the slice");
        assert!(this.today.contains_key("PM5"), "the evidence is kept");
    });
}

#[gpui::test]
fn the_rail_opens_an_earlier_report_and_returns_to_today(cx: &mut TestAppContext) {
    let (workbench, cx) = example_at(cx, ThemeMode::Dark, (1240., 900.));
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
    let (workbench, cx) = example_at(cx, ThemeMode::Light, (1240., 900.));
    workbench.update_in(cx, |this, window, cx| {
        let pm5 = this.rules.criterion("PM5").expect("PM5 exists");
        this.choose_strength(pm5, Strength::Strong, window, cx);
        assert!(this.raising.as_ref().is_some_and(|raise| raise.code == "PM5"), "a raise waits for its reason");
        assert_eq!(this.today["PM5"].strength, Strength::Moderate, "nothing changes before the reason");
        this.raising = None;
        this.choose_strength(pm5, Strength::Supporting, window, cx);
        assert_eq!(this.today["PM5"].strength, Strength::Supporting);
        assert!(this.raising.is_none());
    });
    draw(cx);
}
