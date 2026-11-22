//! Small icons from the kit's built-in set, drawn beside labels, and the thin
//! line that separates two groups in a row.

use gpui_kit::component::{ActiveTheme, Icon, IconName};
use gpui_kit::*;

/// A 14 px icon in the quiet text colour.
pub fn glyph(name: IconName, cx: &App) -> Icon {
    Icon::new(name).w(px(14.)).h(px(14.)).flex_none().text_color(cx.theme().muted_foreground)
}

/// A thin vertical line between two groups of controls in a row.
pub fn rule(cx: &App) -> Div {
    div().w(px(1.)).h(px(16.)).flex_none().bg(cx.theme().border)
}
