//! A focus page of the right panel: a criterion's page ("More"), its strength
//! guide, or an evidence helper. It takes the whole panel, with a head (Back,
//! what it is about), a body that scrolls, and a bar at the foot for the action.
//! The list comes back with Back or Escape. The gene sources work the same way.

use super::*;

/// What a focus page shows. `id` is its selector for the window tests.
pub(super) struct Page {
    pub id: String,
    pub back: Div,
    pub kicker: String,
    pub title: AnyElement,
    pub intro: Option<String>,
    pub body: Div,
    pub footer: Option<Div>,
}

impl Workbench {
    pub(super) fn focus_page(&self, width: f32, page: Page, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let head = div()
            .flex()
            .flex_col()
            .gap_2()
            .px_5()
            .pt_3()
            .pb_4()
            .border_b_1()
            .border_color(border)
            .child(div().flex().child(page.back))
            .child(crate::look::eyebrow(page.kicker, cx))
            .child(page.title)
            .when_some(page.intro, |head, intro| head.child(reading(intro).text_color(muted)));
        let body = div()
            .id("focus-body")
            .track_scroll(&self.handles.criteria_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(page.body.px_5().py_5());
        let id = page.id;
        crate::look::panel(cx)
            .debug_selector(move || id)
            .flex()
            .flex_col()
            .w(px(width))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .child(head)
            .child(body)
            .when_some(page.footer, |panel, footer| {
                panel.child(
                    footer
                        .flex_none()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .px_5()
                        .py_3()
                        .border_t_1()
                        .border_color(border),
                )
            })
            .into_any_element()
    }

    /// The Back button at the top of a page. `selector` is for the window tests.
    pub(super) fn back_button(
        &self,
        (id, selector): (String, String),
        accessible: String,
        leave: impl Fn(&mut Self) + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        tagged(
            selector,
            Button::new(SharedString::from(id))
                .icon(Icon::new(IconName::ChevronLeft))
                .label("Back")
                .accessibility_label(accessible)
                .small()
                .ghost()
                .on_click(cx.listener(move |this, _, _, cx| {
                    leave(this);
                    cx.notify();
                })),
        )
    }
}

/// An element wrapped so the window tests can find it by `selector`.
pub(super) fn tagged(selector: String, element: impl IntoElement) -> Div {
    div().debug_selector(move || selector).child(element)
}

/// Text meant to be read, not scanned: the body size with room between the lines.
pub(super) fn reading(text: impl Into<SharedString>) -> Div {
    div().text_sm().line_height(px(21.)).whitespace_normal().child(text.into())
}

/// One part of a page: a small label over its content.
pub(super) fn page_part(label: impl AsRef<str>, cx: &App) -> Div {
    div().flex().flex_col().gap_2().child(crate::look::eyebrow(label, cx))
}

/// A criterion's code, in its direction's colour, and its title: the head of its pages.
pub(super) fn criterion_heading(c: &Criterion) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap_3()
        .child(
            crate::look::figure(13.)
                .flex_none()
                .mt(px(2.))
                .px_2()
                .py(px(2.))
                .rounded(px(6.))
                .bg(direction_colour(c.direction))
                .text_color(direction_ink())
                .child(c.code.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(18.))
                .line_height(px(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .whitespace_normal()
                .child(c.title.clone()),
        )
        .into_any_element()
}

/// "Variant type · for disease": where a criterion sits, over its title.
pub(super) fn criterion_kicker(c: &Criterion, prefix: Option<&str>) -> String {
    let side = match c.direction {
        Direction::Pathogenic => "for disease",
        Direction::Benign => "against disease",
    };
    let place = format!("{} · {side}", c.category.label());
    prefix.map_or(place.clone(), |prefix| format!("{prefix} · {place}"))
}

/// One answer to choose, as a whole card: its name, what it means, and what it gives
/// (a strength, or that the criterion does not apply). The chosen card is marked by
/// its fill, edge and tick, so the choice never rests on colour alone.
pub(super) fn choice_card(id: String, words: (String, String, String), chosen: bool, cx: &App) -> Button {
    let theme = cx.theme();
    let (title, body, gives) = words;
    let style = ButtonCustomVariant::new(cx)
        .color(if chosen { theme.muted } else { theme.transparent })
        .foreground(theme.foreground)
        .hover(if chosen { theme.muted } else { theme.accent })
        .active(theme.muted);
    let mark = if chosen {
        Icon::new(IconName::CircleCheck).w(px(16.)).h(px(16.)).into_any_element()
    } else {
        div().size(px(14.)).m(px(1.)).rounded_full().border_1().border_color(theme.muted_foreground).into_any_element()
    };
    Button::new(SharedString::from(id))
        .custom(style)
        .w_full()
        .h_auto()
        .p_3()
        .rounded(px(crate::look::CARD_RADIUS))
        .border_1()
        .border_color(if chosen { crate::look::edge(cx) } else { theme.border })
        .selected(chosen)
        .toggled(chosen)
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .gap_3()
                .child(div().flex_none().pt(px(2.)).child(mark))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_left()
                        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).whitespace_normal().child(title))
                        .child(reading(body).text_color(theme.muted_foreground)),
                )
                .child(
                    crate::look::figure(11.)
                        .flex_none()
                        .whitespace_nowrap()
                        .px_2()
                        .py(px(2.))
                        .rounded_full()
                        .border_1()
                        .border_color(theme.border)
                        .child(gives),
                ),
        )
}
