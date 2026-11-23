//! The small helpers that sit inside the criteria list: the predictor score
//! (PP3/BP4), the functional assay (PS3/BS3), family and phenotype points
//! (PP1/PP4), non-segregation (BS4), de novo patients (PS2 or PM6), recessive patients
//! (PM3), the dated BA1 exceptions reference, and the reason prompt for a raised
//! strength. Each one only fills in a criterion; the curator still decides.
//!
//! One file per helper. Its state lives in `view/helper_state.rs`.

use super::*;

mod de_novo;
mod functional;
mod locus;
mod locus_read;
mod pm3;
mod population;

pub(in crate::view) use population::POPULATION_BOXES;
mod predictor;
mod raise;
mod segregation;

pub(super) use super::focus::page_part as part;

/// One labelled box of a helper: the label above, the box under it. In `fields`, two share a row.
pub(super) fn field(label: impl Into<SharedString>, input: impl IntoElement, muted: Hsla) -> Div {
    div()
        .flex_1()
        .min_w(px(180.))
        .flex()
        .flex_col()
        // A label that wraps to two lines must not push its box below its neighbour's.
        .justify_between()
        .gap_1()
        .child(caption(muted).whitespace_normal().child(label.into()))
        .child(input)
}

/// Labelled boxes, two to a row on a wide panel and one to a row on a narrow one.
pub(super) fn fields(items: impl IntoIterator<Item = Div>) -> Div {
    div().flex().flex_wrap().gap_3().children(items)
}

/// A question with its answers in one segmented control, the question above.
pub(super) fn question(label: impl Into<SharedString>, answers: Vec<Button>, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .items_start()
        .gap_1()
        .child(caption(cx.theme().muted_foreground).child(label.into()))
        .child(crate::look::segmented(answers, cx).flex_wrap())
}

/// One answer of a `question`.
pub(super) fn answer(id: impl Into<SharedString>, label: impl Into<SharedString>, chosen: bool, cx: &App) -> Button {
    crate::look::segment(id, label, chosen, cx)
}

/// A statement the curator confirms, as a box that is ticked or not: the shape says it, not only the colour.
pub(super) fn tick(id: impl Into<SharedString>, label: impl Into<SharedString>, on: bool, cx: &App) -> Button {
    let theme = cx.theme();
    let style = ButtonCustomVariant::new(cx)
        .color(theme.transparent)
        .foreground(theme.foreground)
        .hover(theme.accent)
        .active(theme.accent);
    let mark = if on {
        Icon::new(IconName::CircleCheck).w(px(16.)).h(px(16.)).into_any_element()
    } else {
        div().size(px(14.)).m(px(1.)).rounded(px(4.)).border_1().border_color(theme.muted_foreground).into_any_element()
    };
    Button::new(id.into()).custom(style).h_auto().py_1().px_2().rounded(px(7.)).selected(on).toggled(on).child(
        div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_none().pt(px(2.)).child(mark))
            .child(div().text_sm().text_left().whitespace_normal().child(label.into())),
    )
}

/// The result of a helper: what the numbers give, and the Apply buttons, on a card.
pub(super) fn result_card(message: impl IntoElement, cx: &App) -> Div {
    crate::look::card(cx).flex().flex_col().gap_2().child(crate::look::eyebrow("Result", cx)).child(message)
}

/// The rule file's notes for a helper, as a list under "What to check".
pub(super) fn notes_part(notes: impl IntoIterator<Item = String>, cx: &App) -> Option<Div> {
    let muted = cx.theme().muted_foreground;
    let items: Vec<Div> = notes
        .into_iter()
        .map(|note| {
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(div().flex_none().mt(px(8.)).size(px(4.)).rounded_full().bg(muted))
                .child(super::focus::reading(note).flex_1().min_w_0())
        })
        .collect();
    (!items.is_empty()).then(|| part("What to check", cx).child(div().flex().flex_col().gap_2().children(items)))
}

/// Where a helper's numbers come from.
pub(super) fn source_part(citation: Option<String>, cx: &App) -> Option<Div> {
    let muted = cx.theme().muted_foreground;
    citation.map(|text| part("Source", cx).child(super::focus::reading(text).text_color(muted)))
}

/// A helper's opening sentence: what to do before using it.
pub(super) fn reading_note(text: impl Into<SharedString>, muted: Hsla) -> Div {
    super::focus::reading(text).text_color(muted)
}
