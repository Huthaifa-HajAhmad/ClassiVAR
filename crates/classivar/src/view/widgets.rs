//! Small drawing pieces shared by several panels.

use super::*;

/// A short banner text on the warning colours.
pub(super) fn notice(text: impl Into<SharedString>, background: impl Into<Hsla>, foreground: impl Into<Hsla>) -> Div {
    let foreground: Hsla = foreground.into();
    div()
        .flex()
        .items_start()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_sm()
        .bg(background.into())
        .text_color(foreground)
        .text_sm()
        .child(
            div()
                .pt(px(2.))
                .child(Icon::new(IconName::TriangleAlert).w(px(14.)).h(px(14.)).flex_none().text_color(foreground)),
        )
        .child(div().flex_1().min_w_0().child(text.into()))
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
