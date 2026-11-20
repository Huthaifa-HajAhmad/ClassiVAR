//! Dropdowns for text boxes: a floating list of suggestions under a box while
//! you type, and a menu button beside it that lists every choice.
//!
//! Both only fill the box. They are typing aids, never a check: whatever is
//! typed stays allowed. The floating list is for the pointer; the menu button
//! is the keyboard path to the same names.

use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

use super::*;
use crate::look::lift;

/// A text box with a floating list of `items` under it. Picking one puts it in the box.
pub(super) fn with_suggestions(
    id: &'static str,
    field: impl IntoElement,
    items: Vec<String>,
    note: &'static str,
    input: &Entity<InputState>,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let wrapper = div().relative().flex_1().min_w_0().child(div().w_full().child(field));
    if items.is_empty() {
        return wrapper;
    }
    let mut list = div()
        .id(SharedString::from(format!("{id}-list")))
        .occlude()
        .flex()
        .flex_col()
        .p_1()
        .max_h(px(240.))
        .overflow_y_scroll()
        .rounded(px(crate::look::CARD_RADIUS))
        .bg(theme.popover)
        .border_1()
        .border_color(theme.border)
        .shadow(crate::look::elevation(theme.is_dark()));
    for item in items {
        let input = input.clone();
        let text = item.clone();
        list = list.child(
            Button::new(SharedString::from(format!("{id}-{item}")))
                .accessibility_label(item.clone())
                .child(div().w_full().text_left().child(item.clone()))
                .ghost()
                .small()
                .w_full()
                .on_mouse_down(MouseButton::Left, {
                    let (input, text) = (input.clone(), text.clone());
                    move |_, window, cx| fill(&input, &text, window, cx)
                })
                .on_click(move |_, window, cx| fill(&input, &text, window, cx)),
        );
    }
    list = list.child(div().px_2().pt_1().pb_0p5().child(caption(theme.muted_foreground).child(note)));
    wrapper.child(div().absolute().top(relative(1.)).left_0().right_0().mt_1().child(deferred(list).with_priority(1)))
}

fn fill(input: &Entity<InputState>, text: &str, window: &mut Window, cx: &mut App) {
    let text = text.to_string();
    input.update(cx, |input, cx| input.set_value(text, window, cx));
}

/// A small button beside a box that opens a menu of every choice; picking one fills the box.
pub(super) fn choice_menu(
    id: &'static str,
    label: &'static str,
    groups: Vec<(&'static str, Vec<String>)>,
    input: &Entity<InputState>,
    cx: &App,
) -> impl IntoElement {
    let input = input.clone();
    Button::new(id)
        .label("▾")
        .accessibility_label(label)
        .small()
        .ghost()
        .border_1()
        .border_color(cx.theme().border)
        .shadow(lift(false))
        .dropdown_menu_with_anchor(Anchor::TopRight, move |mut menu, _, _| {
            for (title, names) in &groups {
                if names.is_empty() {
                    continue;
                }
                menu = menu.label(*title);
                for name in names {
                    let (input, text) = (input.clone(), name.clone());
                    menu = menu.item(
                        PopupMenuItem::new(name.clone()).on_click(move |_, window, cx| fill(&input, &text, window, cx)),
                    );
                }
            }
            menu.max_h(px(320.)).scrollable(true)
        })
}
