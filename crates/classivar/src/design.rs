//! The colours ClassiVAR owns, and the one job each of them does.
//!
//! Chrome (backgrounds, borders, ordinary text) comes from the GPUI Kit theme, so
//! it follows the light and dark switch. A colour that *means* something is
//! defined here instead, because the kit's own pairs are not checked for contrast.
//! Every fill below is paired with an ink, and the test at the bottom checks
//! each pair against the WCAG AA contrast of 4.5:1 in both themes.
//!
//! | Job | Colours | Where it is used |
//! |---|---|---|
//! | A class | `class_fill` / `class_ink` | the class badge and the points scale |
//! | A direction | `direction_colour` / `direction_ink` | the red and blue dots, evidence blocks and chips |
//! | A blocking state | `warning_fill` / `warning_ink` | a withheld class, a failed lookup, a confirm bar, a missing record class, a message that needs attention |
//! | Something done | `confirm_fill` / `confirm_ink` | a file saved or opened with nothing to fix |
//!
//! Colour is never the only clue: each mark also carries a word or a sign.
//!
//! The one type role that repeats enough to be named here is the caption.

use classivar_core::{Class, Direction};
use gpui_kit::*;

/// The dim layer under the confirmation window: a wash of black over the page.
/// It carries no meaning, so it has no ink pair and no contrast test.
pub fn scrim() -> Hsla {
    hsla(0.0, 0.0, 0.0, 0.45)
}

/// Small muted text: a caption, a note, a source line. Add `.child(...)` to it.
pub fn caption(muted: Hsla) -> Div {
    div().text_xs().text_color(muted)
}

/// The five classes from most benign to most pathogenic, in the order the points scale draws them.
pub const CLASSES: [Class; 5] =
    [Class::Benign, Class::LikelyBenign, Class::Vus, Class::LikelyPathogenic, Class::Pathogenic];

fn pathogenic_colour() -> Rgba {
    rgb(0xc62828)
}

fn benign_colour() -> Rgba {
    rgb(0x1f5fb5)
}

/// The colour of a direction of evidence: red for disease, blue against it.
pub fn direction_colour(direction: Direction) -> Rgba {
    match direction {
        Direction::Pathogenic => pathogenic_colour(),
        Direction::Benign => benign_colour(),
    }
}

/// Text on a `direction_colour` fill.
pub fn direction_ink() -> Rgba {
    rgb(0xffffff)
}

/// The fill of a class. The two confident classes use the direction colours; the
/// likely and uncertain classes are quieter tints (pale in light, deep in dark).
pub fn class_fill(class: Class, dark: bool) -> Rgba {
    if dark {
        return match class {
            Class::Pathogenic => pathogenic_colour(),
            Class::LikelyPathogenic => rgb(0x5f1b1b),
            Class::Vus => rgb(0x33332f),
            Class::LikelyBenign => rgb(0x173a5e),
            Class::Benign => benign_colour(),
        };
    }
    match class {
        Class::Pathogenic => pathogenic_colour(),
        Class::LikelyPathogenic => rgb(0xf6c9c8),
        Class::Vus => rgb(0xe6e5e0),
        Class::LikelyBenign => rgb(0xcde2fb),
        Class::Benign => benign_colour(),
    }
}

/// Text on a `class_fill`.
pub fn class_ink(class: Class, dark: bool) -> Rgba {
    if dark {
        return match class {
            Class::Pathogenic | Class::Benign => rgb(0xffffff),
            Class::LikelyPathogenic => rgb(0xffd9d5),
            Class::Vus => rgb(0xe8e7e1),
            Class::LikelyBenign => rgb(0xd6e8ff),
        };
    }
    if matches!(class, Class::Pathogenic | Class::Benign) { rgb(0xffffff) } else { rgb(0x0b0b0b) }
}

/// The fill of every warning and blocking notice. The kit's own warning pair is
/// hard to read (about 1.8:1 in the light theme), so the app uses its own pair,
/// which reads the same in both themes.
pub fn warning_fill() -> Rgba {
    rgb(0xffe9a8)
}

/// Text on `warning_fill`.
pub fn warning_ink() -> Rgba {
    rgb(0x2b2100)
}

/// The fill of a message that says something finished as expected (a case was
/// saved, or opened with nothing to fix). A pale green, apart from the red and
/// blue that mean pathogenic and benign, and from the yellow that means attention.
pub fn confirm_fill() -> Rgba {
    rgb(0xd6efdc)
}

/// Text on `confirm_fill`.
pub fn confirm_ink() -> Rgba {
    rgb(0x0b2a14)
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose: `use super::*` would also bring in GPUI's own
    // `#[test]` macro (through `gpui_kit::*`), which replaces Rust's.
    use super::{
        CLASSES, Direction, Rgba, class_fill, class_ink, confirm_fill, confirm_ink, direction_colour, direction_ink,
        warning_fill, warning_ink,
    };

    /// WCAG relative luminance of a colour.
    fn luminance(c: Rgba) -> f32 {
        let linear = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }

    fn contrast(a: Rgba, b: Rgba) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn class_colours_reach_wcag_aa_contrast_for_their_text() {
        for dark in [false, true] {
            for class in CLASSES {
                let ratio = contrast(class_fill(class, dark), class_ink(class, dark));
                assert!(ratio >= 4.5, "{class:?} (dark={dark}): {ratio:.2}:1");
            }
        }
    }

    #[test]
    fn direction_colours_reach_wcag_aa_contrast_for_their_text() {
        for direction in [Direction::Pathogenic, Direction::Benign] {
            let ratio = contrast(direction_colour(direction), direction_ink());
            assert!(ratio >= 4.5, "{direction:?} chip text: {ratio:.2}:1");
        }
    }

    #[test]
    fn the_warning_and_confirmation_pairs_reach_wcag_aa_contrast() {
        for (name, fill, ink) in
            [("warning", warning_fill(), warning_ink()), ("confirmation", confirm_fill(), confirm_ink())]
        {
            let ratio = contrast(fill, ink);
            assert!(ratio >= 4.5, "{name} text: {ratio:.2}:1");
        }
    }
}
