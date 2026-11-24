//! Headless GPUI tests for debug-only live theme reload, live rules reload,
//! and startup `--open` case loading.
//!
//! Explicit imports on purpose (see `headless.rs`).

use classivar_core::{RuleSet, Strength};
use gpui::TestAppContext;
use gpui_kit::component::{ActiveTheme, ThemeMode};

use super::headless::{draw, open};
use super::types::{Choice, Raise};
use crate::dev_reload::{apply_rules_poll, apply_theme_poll};
use crate::status::Tone;
use crate::theme;

const THEME_JSON: &str = include_str!("../../assets/classivar-theme.json");
const RULES_TOML: &str = include_str!("../../../../rules/acmg-amp-points.toml");

fn edited(text: &str, from: &str, to: &str) -> String {
    let out = text.replace(from, to);
    assert_ne!(out, text, "the fixture holds {from:?}");
    out
}

/// The built-in rules under another version, leaked like a rule file read from disk.
fn rules_with_version(version: &str) -> &'static RuleSet {
    let text = edited(RULES_TOML, "version = \"0.14.0-draft\"", &format!("version = \"{version}\""));
    Box::leak(Box::new(RuleSet::parse(&text).expect("valid rules")))
}

#[gpui::test]
fn live_theme_reload_updates_active_colours_without_panicking(cx: &mut TestAppContext) {
    let (_workbench, vcx) = open(cx, ThemeMode::Light);
    let before = vcx.update(|_, cx| cx.theme().background);
    let set = theme::parse_themes(&edited(THEME_JSON, "#eeede9", "#f4f1ea")).expect("edited theme parses");
    vcx.update(|_, cx| theme::apply_theme_set(set, ThemeMode::Light, cx));
    draw(vcx);
    let after = vcx.update(|_, cx| cx.theme().background);
    assert_ne!(before, after, "applying a reloaded ThemeSet must update active colours");
}

#[gpui::test]
fn a_theme_poll_applies_an_edit_and_keeps_the_old_theme_when_it_fails(cx: &mut TestAppContext) {
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    let path = std::path::Path::new("theme.json");
    let before = vcx.update(|_, cx| cx.theme().background);

    let set = theme::parse_themes(&edited(THEME_JSON, "#eeede9", "#f4f1ea")).expect("edited theme parses");
    workbench.update_in(vcx, |this, _, cx| apply_theme_poll(this, path, Ok((set, Vec::new())), cx));
    let after = vcx.update(|_, cx| cx.theme().background);
    assert_ne!(before, after);
    workbench.read_with(vcx, |this, _| {
        let status = this.file_status.as_ref().expect("the reload is announced");
        assert_eq!((status.tone, status.text.as_str()), (Tone::Done, "Reloaded theme from theme.json."));
    });

    let set = theme::parse_themes(THEME_JSON).expect("the built-in theme parses");
    let warnings = vec!["Light: foreground on background is 1.20:1 (needs 7:1)".to_string()];
    workbench.update_in(vcx, |this, _, cx| apply_theme_poll(this, path, Ok((set, warnings)), cx));
    workbench.read_with(vcx, |this, _| {
        let status = this.file_status.as_ref().expect("the warning is announced");
        assert_eq!(status.tone, Tone::Attention);
        assert!(status.text.contains("contrast warning: Light: foreground on background"), "{}", status.text);
    });

    let failed = Err("invalid theme JSON: expected value".to_string());
    workbench.update_in(vcx, |this, _, cx| apply_theme_poll(this, path, failed, cx));
    workbench.read_with(vcx, |this, _| {
        let status = this.file_status.as_ref().expect("the failure is announced");
        assert_eq!(status.tone, Tone::Attention);
        assert_eq!(status.text, "Theme reload skipped: invalid theme JSON: expected value");
    });
    assert_eq!(vcx.update(|_, cx| cx.theme().background), before, "the last theme applied stays (the built-in one)");
}

#[gpui::test]
fn a_rules_poll_applies_good_rules_and_keeps_the_old_ones_when_it_fails(cx: &mut TestAppContext) {
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    let original = workbench.read_with(vcx, |this, _| this.rules.meta.version.clone());

    let loaded = Ok(rules_with_version("0.14.0-poll"));
    workbench.update_in(vcx, |this, window, cx| apply_rules_poll(this, loaded, window, cx));
    draw(vcx);
    workbench.read_with(vcx, |this, _| {
        assert_eq!(this.rules.meta.version, "0.14.0-poll");
        assert!(this.file_status.as_ref().is_some_and(|s| s.text.contains("Reloaded rules")));
    });

    let failed = Err("rules.toml is not a valid rule file: expected a table".to_string());
    workbench.update_in(vcx, |this, window, cx| apply_rules_poll(this, failed, window, cx));
    workbench.read_with(vcx, |this, _| {
        assert_ne!(this.rules.meta.version, original, "the last good rules stay");
        assert_eq!(this.rules.meta.version, "0.14.0-poll");
        let status = this.file_status.as_ref().expect("the failure is announced");
        assert_eq!(status.tone, Tone::Attention);
        assert_eq!(status.text, "Rules reload skipped: rules.toml is not a valid rule file: expected a table");
    });
}

#[gpui::test]
fn live_rules_reload_updates_workbench_and_sanitizes_focus_state(cx: &mut TestAppContext) {
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    workbench.update_in(vcx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.about = Some("PS4".into());
        this.inspected = Some("PS4".into());
        this.reload_rules(rules_with_version("0.14.0-live"), window, cx);
    });
    draw(vcx);
    workbench.read_with(vcx, |this, _| {
        assert_eq!(this.rules.meta.version, "0.14.0-live");
        assert_eq!(this.about.as_deref(), Some("PS4"));
        assert_eq!(this.inspected.as_deref(), Some("PS4"));
        assert!(this.file_status.as_ref().is_some_and(|s| s.text.contains("Reloaded rules")));
    });

    // Stale focus state pointing to a non-existent criterion is dropped cleanly.
    workbench.update_in(vcx, |this, window, cx| {
        this.about = Some("NO_SUCH".into());
        this.inspected = Some("NO_SUCH".into());
        this.hovered = Some("NO_SUCH".into());
        this.preview = true;
        this.reload_rules(this.rules, window, cx);
    });
    draw(vcx);
    workbench.read_with(vcx, |this, _| {
        assert_eq!(this.about, None);
        assert_eq!(this.inspected, None);
        assert_eq!(this.hovered, None);
        assert!(!this.preview, "the preview of a dropped selection ends");
    });
}

#[gpui::test]
fn a_rules_reload_drops_the_open_guide_and_the_helper_state_that_no_longer_fits(cx: &mut TestAppContext) {
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    workbench.update_in(vcx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.raising = Some(Raise { code: "PS4".into(), to: Strength::Strong, note: false });
        this.guide_open = Some("PVS1".into());
        this.guide_pick = Some("an-answer-of-the-old-rules".into());
        this.helpers.predictor.tool = 99;
        this.helpers.locus.kind = 99;
        this.helpers.segregation.situation = 99;
        assert!(this.helpers.pm3.inputs.pop().is_some(), "the built-in rules have PM3 rows");
        this.reload_rules(this.rules, window, cx);
    });
    draw(vcx);
    workbench.read_with(vcx, |this, _| {
        assert!(this.raising.is_none());
        assert_eq!(this.guide_open, None, "a guide answer may not exist in the new rules");
        assert_eq!(this.guide_pick, None);
        assert_eq!(this.helpers.predictor.tool, 0);
        assert_eq!(this.helpers.locus.kind, 0);
        assert_eq!(this.helpers.segregation.situation, 0);
        let pm3_rows = this.rules.pm3.as_ref().map_or(0, |table| table.rows.len());
        assert_eq!(this.helpers.pm3.inputs.len(), pm3_rows, "one box for each PM3 row again");
    });
}

#[gpui::test]
fn a_rules_reload_never_rewrites_an_earlier_report(cx: &mut TestAppContext) {
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    workbench.update_in(vcx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.records[0].evidence.insert("ZZ9".into(), Choice::new(Strength::Supporting));
        this.reload_rules(this.rules, window, cx);
    });
    draw(vcx);
    workbench.read_with(vcx, |this, _| {
        assert!(this.records[0].evidence.contains_key("ZZ9"), "the report keeps the evidence it recorded");
        let status = this.file_status.as_ref().expect("the reload is announced");
        assert_eq!(status.tone, Tone::Attention);
        assert!(status.text.contains("ZZ9") && status.text.contains("cannot be scored"), "{}", status.text);
    });
}

#[gpui::test]
fn a_rules_reload_is_not_a_change_the_curator_made(cx: &mut TestAppContext) {
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    workbench.update_in(vcx, |this, window, cx| {
        this.load_real_case(window, cx);
        let saved = this.case_file(cx);
        this.remember_saved(saved);
        assert!(!this.has_unsaved_changes(cx));
        this.reload_rules(rules_with_version("0.14.0-live"), window, cx);
        assert!(!this.has_unsaved_changes(cx), "new rules alone do not make the case unsaved");
        this.today.insert("ZZ9".into(), Choice::new(Strength::Supporting));
        assert!(this.has_unsaved_changes(cx), "a change the curator makes still does");
    });
}

#[gpui::test]
fn startup_case_file_loads_through_read_case_file_and_draws(cx: &mut TestAppContext) {
    let path = std::env::temp_dir().join(format!("classivar-startup-case-{}.classivar.json", std::process::id()));
    let (workbench, vcx) = open(cx, ThemeMode::Light);
    let json = workbench.update_in(vcx, |this, window, cx| {
        this.load_real_case(window, cx);
        this.case_file(cx).to_json()
    });
    std::fs::write(&path, json).expect("write temp case");

    let (fresh, fresh_cx) = open(cx, ThemeMode::Light);
    fresh.update_in(fresh_cx, |this, window, cx| {
        let case = super::read_case_file(&path, this.rules).expect("temp case file parses");
        this.apply_case(case, window, cx);
    });
    draw(fresh_cx);
    fresh.read_with(fresh_cx, |this, _| {
        assert!(!this.welcome);
        assert_eq!(this.records.len(), 2);
    });
    std::fs::remove_file(path).ok();
}
