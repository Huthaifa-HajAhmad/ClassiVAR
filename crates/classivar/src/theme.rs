//! The ClassiVAR look: one light and one dark theme, kept in
//! `assets/classivar-theme.json` and installed over the GPUI Kit defaults.
//!
//! The chrome is neutral ink on paper. The only hues in the app are the ones
//! that carry meaning (see `design.rs`), so a selected button is shown by its
//! fill and border, never by a colour that could be mistaken for a class.

use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeMode, ThemeSet};
use gpui_kit::*;

const THEMES: &str = include_str!("../assets/classivar-theme.json");

/// Parse a ClassiVAR theme JSON string. It needs a light and a dark theme: with
/// only one, the other mode would silently keep the GPUI Kit's own colours.
pub fn parse_themes(json: &str) -> Result<ThemeSet, String> {
    let set: ThemeSet = serde_json::from_str(json).map_err(|error| format!("invalid theme JSON: {error}"))?;
    let has_dark = set.themes.iter().any(|config| config.mode.is_dark());
    let has_light = set.themes.iter().any(|config| !config.mode.is_dark());
    if !(has_dark && has_light) {
        return Err("theme file needs one light and one dark theme".to_string());
    }
    Ok(set)
}

/// Apply an already-parsed theme set and switch to `mode`.
pub fn apply_theme_set(set: ThemeSet, mode: ThemeMode, cx: &mut App) {
    Theme::update(cx, |theme| {
        for config in set.themes {
            if config.mode.is_dark() {
                theme.dark_theme = Rc::new(config);
            } else {
                theme.light_theme = Rc::new(config);
            }
        }
    });
    Theme::change(mode, None, cx);
}

/// Install the ClassiVAR themes, then switch to `mode`. The file is built into
/// the program and a test parses it, so it cannot be invalid in a release.
pub fn install(mode: ThemeMode, cx: &mut App) {
    let set = parse_themes(THEMES).expect("the built-in theme file is valid");
    apply_theme_set(set, mode, cx);
}

/// The pairs the screens depend on, and the lowest contrast each may have:
/// 4.5 for text (WCAG AA), 3 for the edge of a control and the focus ring.
#[cfg(any(test, debug_assertions))]
const PAIRS: [(&str, &str, f32); 9] = [
    ("foreground", "background", 7.0),
    ("muted.foreground", "background", 4.5),
    ("muted.foreground", "muted.background", 4.5),
    ("accent.foreground", "accent.background", 7.0),
    ("secondary.foreground", "secondary.background", 7.0),
    ("primary.foreground", "primary.background", 7.0),
    ("popover.foreground", "popover.background", 7.0),
    ("input.border", "background", 3.0),
    ("ring", "background", 3.0),
];

#[cfg(any(test, debug_assertions))]
fn channel(hex: &str, at: usize) -> Option<f32> {
    u8::from_str_radix(hex.get(at..at + 2)?, 16).ok().map(|v| f32::from(v) / 255.0)
}

#[cfg(any(test, debug_assertions))]
fn luminance(hex: &str) -> Option<f32> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return None;
    }
    let linear = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    Some(0.2126 * linear(channel(hex, 1)?) + 0.7152 * linear(channel(hex, 3)?) + 0.0722 * linear(channel(hex, 5)?))
}

#[cfg(any(test, debug_assertions))]
fn contrast(a: &str, b: &str) -> Option<f32> {
    let (x, y) = (luminance(a)?, luminance(b)?);
    Some((x.max(y) + 0.05) / (x.min(y) + 0.05))
}

/// The text colour laid over `ground` at `strength`, as `#rrggbb`.
#[cfg(any(test, debug_assertions))]
fn blend(ink: &str, ground: &str, strength: f32) -> Option<String> {
    let mix = |at| {
        let value = channel(ink, at)? * strength + channel(ground, at)? * (1.0 - strength);
        Some((value * 255.0).round() as u8)
    };
    Some(format!("#{:02x}{:02x}{:02x}", mix(1)?, mix(3)?, mix(5)?))
}

#[cfg(any(test, debug_assertions))]
fn check_theme_contrast(theme: &serde_json::Value, warnings: &mut Vec<String>) {
    let name = theme["name"].as_str().unwrap_or("unnamed theme");
    let colour = |key: &str| theme["colors"][key].as_str();
    for (ink, ground, least) in PAIRS {
        match (colour(ink), colour(ground)) {
            (Some(a), Some(b)) => match contrast(a, b) {
                Some(ratio) if ratio < least => {
                    warnings.push(format!("{name}: {ink} on {ground} is {ratio:.2}:1 (needs {least}:1)"));
                }
                None => warnings.push(format!("{name}: invalid hex in {ink} or {ground}")),
                _ => {}
            },
            _ => warnings.push(format!("{name}: missing {ink} or {ground}")),
        }
    }
    for ground in ["background", "muted.background", "popover.background"] {
        if let (Some(fg), Some(bg)) = (colour("foreground"), colour(ground))
            && let Some(edge) = blend(fg, bg, crate::look::EDGE_STRENGTH)
            && let Some(ratio) = contrast(&edge, bg)
            && ratio < 3.0
        {
            warnings.push(format!("{name}: emphasis edge on {ground} is {ratio:.2}:1 (needs 3.0:1)"));
        }
    }
}

/// Check WCAG contrast on a theme JSON document, returning a warning for every failing pair.
#[cfg(any(test, debug_assertions))]
pub fn contrast_warnings(json: &str) -> Vec<String> {
    let Ok(file) = serde_json::from_str::<serde_json::Value>(json) else {
        return vec!["theme file is not valid JSON".to_string()];
    };
    let Some(themes) = file["themes"].as_array() else {
        return vec!["theme file has no `themes` array".to_string()];
    };
    let mut warnings = Vec::new();
    for theme in themes {
        check_theme_contrast(theme, &mut warnings);
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::{PAIRS, THEMES, blend, contrast, contrast_warnings, parse_themes};
    use serde_json::Value;

    #[test]
    fn both_themes_parse_and_reach_the_contrast_the_screens_need() {
        let file: Value = serde_json::from_str(THEMES).expect("the theme file is JSON");
        let themes = file["themes"].as_array().expect("a list of themes");
        assert_eq!(themes.len(), 2, "one light and one dark theme");
        for theme in themes {
            let name = theme["name"].as_str().unwrap();
            let colour = |key: &str| theme["colors"][key].as_str().unwrap_or_else(|| panic!("{name}: no {key}"));
            for (ink, ground, least) in PAIRS {
                let ratio = contrast(colour(ink), colour(ground)).expect("valid #rrggbb hex");
                assert!(ratio >= least, "{name}: {ink} on {ground} is {ratio:.2}:1, needs {least}");
            }
        }
        assert!(contrast_warnings(THEMES).is_empty());
    }

    #[test]
    fn the_soft_emphasis_edge_still_reaches_3_to_1_on_every_surface_it_marks() {
        let file: Value = serde_json::from_str(THEMES).expect("the theme file is JSON");
        for theme in file["themes"].as_array().expect("a list of themes") {
            let name = theme["name"].as_str().unwrap();
            let colour = |key: &str| theme["colors"][key].as_str().unwrap_or_else(|| panic!("{name}: no {key}"));
            for ground in ["background", "muted.background", "popover.background"] {
                let edge = blend(colour("foreground"), colour(ground), crate::look::EDGE_STRENGTH).unwrap();
                let ratio = contrast(&edge, colour(ground)).unwrap();
                assert!(ratio >= 3.0, "{name}: the edge on {ground} is {ratio:.2}:1, needs 3");
            }
        }
    }

    #[test]
    fn contrast_warnings_reports_low_contrast_pairs() {
        let low_contrast = THEMES.replace("\"foreground\": \"#141414\"", "\"foreground\": \"#dcdad3\"");
        assert_ne!(low_contrast, THEMES, "the light foreground is in the theme file");
        let warnings = contrast_warnings(&low_contrast);
        assert!(!warnings.is_empty(), "a near-invisible foreground must trigger contrast warnings");
        assert!(warnings.iter().any(|w| w.contains("foreground on background")), "{warnings:?}");
    }

    #[test]
    fn contrast_warnings_name_a_broken_theme_file() {
        assert_eq!(contrast_warnings("{ nope"), ["theme file is not valid JSON"]);
        assert_eq!(contrast_warnings("{}"), ["theme file has no `themes` array"]);
        let mut file: Value = serde_json::from_str(THEMES).expect("the theme file is JSON");
        file["themes"][0]["colors"]["foreground"] = Value::String("red".into());
        file["themes"][0]["colors"].as_object_mut().expect("a colour table").remove("ring");
        let warnings = contrast_warnings(&file.to_string());
        assert!(warnings.iter().any(|w| w.contains("invalid hex in foreground")), "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("missing ring or background")), "{warnings:?}");
    }

    #[test]
    fn a_theme_file_needs_valid_json_and_both_a_light_and_a_dark_theme() {
        assert!(parse_themes(THEMES).is_ok());
        assert!(parse_themes("{ nope").unwrap_err().contains("invalid theme JSON"));
        let file: Value = serde_json::from_str(THEMES).expect("the theme file is JSON");
        let all = file["themes"].as_array().expect("a list of themes").clone();
        let with = |themes: &[Value]| {
            let mut copy = file.clone();
            copy["themes"] = Value::Array(themes.to_vec());
            copy.to_string()
        };
        assert!(parse_themes(&with(&[])).unwrap_err().contains("one light and one dark"));
        for one in &all {
            let error = parse_themes(&with(std::slice::from_ref(one))).unwrap_err();
            assert!(error.contains("one light and one dark"), "{error}");
        }
    }
}
