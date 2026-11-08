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

/// Install the ClassiVAR themes, then switch to `mode`. The file is built into
/// the program and a test parses it, so it cannot be invalid in a release.
pub fn install(mode: ThemeMode, cx: &mut App) {
    let set: ThemeSet = serde_json::from_str(THEMES).expect("the built-in theme file is valid");
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

#[cfg(test)]
mod tests {
    use super::THEMES;
    use serde_json::Value;

    fn channel(hex: &str, at: usize) -> f32 {
        f32::from(u8::from_str_radix(&hex[at..at + 2], 16).expect("two hex digits")) / 255.0
    }

    fn luminance(hex: &str) -> f32 {
        assert!(hex.len() == 7 && hex.starts_with('#'), "{hex} is not #rrggbb");
        let linear = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        0.2126 * linear(channel(hex, 1)) + 0.7152 * linear(channel(hex, 3)) + 0.0722 * linear(channel(hex, 5))
    }

    fn contrast(a: &str, b: &str) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// The pairs the screens depend on, and the lowest contrast each may have:
    /// 4.5 for text (WCAG AA), 3 for the edge of a control and the focus ring.
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

    #[test]
    fn both_themes_parse_and_reach_the_contrast_the_screens_need() {
        let file: Value = serde_json::from_str(THEMES).expect("the theme file is JSON");
        let themes = file["themes"].as_array().expect("a list of themes");
        assert_eq!(themes.len(), 2, "one light and one dark theme");
        for theme in themes {
            let name = theme["name"].as_str().unwrap();
            let colour = |key: &str| theme["colors"][key].as_str().unwrap_or_else(|| panic!("{name}: no {key}"));
            for (ink, ground, least) in PAIRS {
                let ratio = contrast(colour(ink), colour(ground));
                assert!(ratio >= least, "{name}: {ink} on {ground} is {ratio:.2}:1, needs {least}");
            }
        }
    }
}
