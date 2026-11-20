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
    ActiveTheme, Selectable, Sizable,
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
