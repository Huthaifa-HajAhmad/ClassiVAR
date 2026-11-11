//! Small drawing pieces shared by several panels.

use super::*;

/// A short banner text on the warning colours.
pub(super) fn notice(text: impl Into<SharedString>, background: impl Into<Hsla>, foreground: impl Into<Hsla>) -> Div {
    div().px_2().py_1().rounded_sm().bg(background.into()).text_color(foreground.into()).text_sm().child(text.into())
}

/// A class with its review status and rule version, so a screenshot of the
/// class alone still shows how far it can be trusted.
pub(super) fn class_badge(class: Class, rules: &RuleSet, prefix: &str, border: Hsla, dark: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(border)
        .bg(class_fill(class, dark))
        .text_color(class_ink(class, dark))
        .child(div().font_weight(FontWeight::BOLD).child(format!("{prefix}{}", class.label())))
        .child(div().text_xs().child(rules_tag(rules)))
}
