//! The ClassiVAR look: type roles, surfaces and the few custom controls that
//! carry the design. Colours that *mean* something stay in `design.rs`; the
//! chrome colours come from the theme (`assets/classivar-theme.json`).
//!
//! The idea is "a calm instrument". Panels float on a soft canvas with real
//! shadows. Numbers, criterion codes and small labels use a monospaced face
//! (JetBrains Mono), so values line up and read as measurements; everything
//! else uses the system's own interface font. One accent per state, never two.
//!
//! Spacing follows a 4 px grid: 4, 8, 12, 16, 24, 32. Corners: 16 for panels,
//! 10 for controls and cards inside a panel, 6 for chips.

use std::borrow::Cow;

use gpui_kit::component::{
    ActiveTheme, Selectable, Sizable, Theme,
    button::{Button, ButtonCustomVariant, ButtonVariants},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// The monospaced face for numbers, codes and small labels. It is built into the program.
pub const MONO: &str = "JetBrains Mono";

/// Corner radii, in pixels.
pub const PANEL_RADIUS: f32 = 16.;
pub const CARD_RADIUS: f32 = 10.;

/// Make the built-in fonts available. A failure only means the system font is used instead.
pub fn install_fonts(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf")),
    ];
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        eprintln!("ClassiVAR: the built-in fonts could not be loaded ({error}); using the system font.");
    }
}

/// A soft, two-layer shadow that lifts a panel off the canvas. Shadows carry no
/// meaning, so they are plain black at low opacity, a little stronger in the dark theme.
pub fn elevation(dark: bool) -> Vec<BoxShadow> {
    let (near, far) = if dark { (0.5, 0.45) } else { (0.06, 0.10) };
    vec![
        BoxShadow {
            color: hsla(0., 0., 0., near),
            offset: point(px(0.), px(1.)),
            blur_radius: px(2.),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: hsla(0., 0., 0., far),
            offset: point(px(0.), px(12.)),
            blur_radius: px(32.),
            spread_radius: px(-12.),
            inset: false,
        },
    ]
}

/// A small lift for a selected segment or a raised chip.
pub fn lift(dark: bool) -> Vec<BoxShadow> {
    let alpha = if dark { 0.55 } else { 0.12 };
    vec![BoxShadow {
        color: hsla(0., 0., 0., alpha),
        offset: point(px(0.), px(1.)),
        blur_radius: px(3.),
        spread_radius: px(0.),
        inset: false,
    }]
}

/// A floating panel: the surface colour, a hairline edge and the soft shadow.
pub fn panel(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .bg(theme.popover)
        .text_color(theme.popover_foreground)
        .rounded(px(PANEL_RADIUS))
        .border_1()
        .border_color(theme.border)
        .shadow(elevation(theme.is_dark()))
}

/// How strong an emphasis edge is, as a share of the text colour. A full-strength
/// line reads as a bright white stroke in the dark theme; this keeps the outline
/// clear (it is a shape cue, not only a colour) without shouting.
pub const EDGE_STRENGTH: f32 = 0.55;

/// The faint edge on a dial slice under the pointer. It stays lighter than `EDGE_STRENGTH`,
/// so a slice that is only hovered never looks as marked as one that is chosen.
pub const HOVER_STRENGTH: f32 = 0.3;

/// The line around the one thing in a group that is marked: the class shown,
/// the picture chosen, a history cell that changed.
pub fn edge(cx: &App) -> Hsla {
    cx.theme().foreground.opacity(EDGE_STRENGTH)
}

/// A card inside a panel: a quieter fill, no shadow.
pub fn card(cx: &App) -> Div {
    div().bg(cx.theme().muted).rounded(px(CARD_RADIUS)).p_3()
}

/// A small label above a value: monospaced capitals in the quiet text colour.
pub fn eyebrow(text: impl AsRef<str>, cx: &App) -> Div {
    div()
        .font_family(MONO)
        .text_size(px(10.5))
        .line_height(px(14.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(cx.theme().muted_foreground)
        .child(text.as_ref().to_uppercase())
}

/// One titled group of the editor's form, for example "Variant" or "Gene": a small
/// label over its rows. `first` leaves out the hairline above, which separates groups.
pub fn form_section(label: &str, first: bool, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .when(!first, |d| d.pt_5().border_t_1().border_color(cx.theme().border))
        .child(eyebrow(label, cx))
}

/// The head of a section: a small label over a title, for example "2015 TABLE"
/// over "What the 2015 table says".
pub fn section_head(label: &str, title: &str, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(eyebrow(label, cx))
        .child(div().text_size(px(18.)).line_height(px(24.)).font_weight(FontWeight::SEMIBOLD).child(title.to_string()))
}

/// Monospaced text for numbers and codes, at `size` pixels.
pub fn figure(size: f32) -> Div {
    div().font_family(MONO).text_size(px(size)).font_weight(FontWeight::SEMIBOLD)
}

/// The largest words on screen: the class.
pub fn display() -> Div {
    div().text_size(px(34.)).line_height(px(40.)).font_weight(FontWeight::SEMIBOLD)
}

/// One choice of a segmented control. Pass the buttons to `segmented`.
pub fn segment(id: impl Into<SharedString>, label: impl Into<SharedString>, chosen: bool, cx: &App) -> Button {
    let theme = cx.theme();
    // In the dark theme the panel colour is too close to the track, so a lighter fill marks the choice.
    let raised = if theme.is_dark() { theme.accent } else { theme.popover };
    let style = ButtonCustomVariant::new(cx)
        .color(if chosen { raised } else { theme.transparent })
        .foreground(if chosen { theme.foreground } else { theme.muted_foreground })
        .hover(if chosen { raised } else { theme.accent })
        .active(raised);
    let id: SharedString = id.into();
    Button::new(id)
        .label(label)
        .small()
        .custom(style)
        .rounded(px(7.))
        .selected(chosen)
        .toggled(chosen)
        .when(chosen, |button| button.shadow(lift(theme.is_dark())))
}

/// The four colours of a `choice_pill`. The kit paints a selected button with its
/// `active` colour as the fill and its `foreground` as the text, so for a chosen pill
/// all of fill, hover and active are the ink colour: nothing may fall back to a quiet fill.
#[derive(Debug, Clone, Copy)]
struct PillColours {
    fill: Hsla,
    hover: Hsla,
    active: Hsla,
    text: Hsla,
}

fn pill_colours(chosen: bool, theme: &Theme) -> PillColours {
    if chosen {
        PillColours {
            fill: theme.foreground,
            hover: theme.foreground,
            active: theme.foreground,
            text: theme.background,
        }
    } else {
        PillColours { fill: theme.muted, hover: theme.accent, active: theme.accent, text: theme.foreground }
    }
}

/// A rounded choice that adds something, for example a suggested condition: "+" and a
/// quiet fill when open, filled in ink with a tick when chosen. Shaped as a pill so it
/// never reads as a text box. A long label wraps inside its container (the kit's own
/// `label` would cut it short, so the text is a child).
pub fn choice_pill(id: impl Into<SharedString>, label: &str, chosen: bool, cx: &App) -> Button {
    let colours = pill_colours(chosen, cx.theme());
    let style = ButtonCustomVariant::new(cx)
        .color(colours.fill)
        .foreground(colours.text)
        .hover(colours.hover)
        .active(colours.active);
    let id: SharedString = id.into();
    let text = format!("{} {label}", if chosen { "✓" } else { "+" });
    Button::new(id)
        .small()
        .custom(style)
        .rounded(px(999.))
        .px_3()
        .max_w_full()
        .h_auto()
        .py_1()
        .child(div().min_w_0().whitespace_normal().text_left().child(text))
        .selected(chosen)
        .toggled(chosen)
}

/// A row of `segment`s on a recessed track.
pub fn segmented(segments: impl IntoIterator<Item = impl IntoElement>, cx: &App) -> Div {
    div().flex().items_center().gap(px(2.)).p(px(3.)).rounded(px(CARD_RADIUS)).bg(cx.theme().muted).children(segments)
}

/// A quiet rounded label, for example the review status in the top bar.
pub fn pill(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1()
        .rounded_full()
        .bg(theme.popover)
        .border_1()
        .border_color(theme.border)
        .font_family(MONO)
        .text_size(px(11.))
        .text_color(theme.muted_foreground)
}

#[cfg(test)]
mod tests {
    // Explicit imports: `use gpui_kit::*` would bring in GPUI's own `#[test]` macro.
    use gpui::{Rgba, TestAppContext};
    use gpui_kit::component::{ActiveTheme, ThemeMode};
    use gpui_kit::init;

    use super::pill_colours;
    use crate::design::contrast;
    use crate::theme;

    #[gpui::test]
    fn a_condition_pill_is_readable_open_and_chosen_in_both_themes(cx: &mut TestAppContext) {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update(|cx| {
                init(cx);
                theme::install(mode, cx);
                let theme = cx.theme();
                for chosen in [false, true] {
                    let c = pill_colours(chosen, theme);
                    if chosen {
                        // The kit paints a selected button with its active colour, so it must be the ink fill.
                        assert_eq!((c.hover, c.active), (c.fill, c.fill), "chosen, {mode:?}");
                    }
                    // Fill, hover and pressed backgrounds each carry the pill's text.
                    for (state, background) in [("fill", c.fill), ("hover", c.hover), ("active", c.active)] {
                        let ratio = contrast(Rgba::from(background), Rgba::from(c.text));
                        assert!(ratio >= 4.5, "chosen={chosen} {state} ({mode:?}): {ratio:.2}:1");
                    }
                }
            });
        }
    }
}
