//! The lower half of the opening screen: pick how the class is drawn, what the
//! app does and does not do, and the rules it is running.

use std::f32::consts::PI;

use super::*;
use crate::look::eyebrow;
use crate::settings::Picture;

/// A small drawing of each picture, using the class colours in scale order.
fn sketch(picture: Picture, dark: bool) -> AnyElement {
    let fills: Vec<Hsla> = CLASSES.iter().map(|class| class_fill(*class, dark).into()).collect();
    match picture {
        Picture::Dial => canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                let centre = point(bounds.origin.x + px(w / 2.), bounds.origin.y + px(h - 2.));
                let (outer, inner) = (h - 4., h - 14.);
                let at = |r: f32, a: f32| point(centre.x + px(r * a.cos()), centre.y - px(r * a.sin()));
                for (i, fill) in fills.iter().enumerate() {
                    let (from, to) = (PI * (1. - i as f32 / 5.) - 0.02, PI * (1. - (i + 1) as f32 / 5.) + 0.02);
                    let mut path = PathBuilder::fill();
                    path.move_to(at(outer, from));
                    path.arc_to(point(px(outer), px(outer)), px(0.), false, true, at(outer, to));
                    path.line_to(at(inner, to));
                    path.arc_to(point(px(inner), px(inner)), px(0.), false, false, at(inner, from));
                    path.close();
                    if let Ok(path) = path.build() {
                        window.paint_path(path, *fill);
                    }
                }
            },
        )
        .w(px(96.))
        .h(px(52.))
        .into_any_element(),
        Picture::Column => div()
            .flex()
            .flex_col_reverse()
            .w(px(30.))
            .h(px(52.))
            .rounded(px(8.))
            .overflow_hidden()
            .children(fills.into_iter().map(|fill| div().flex_1().bg(fill)))
            .into_any_element(),
        Picture::Track => div()
            .flex()
            .w(px(96.))
            .h(px(18.))
            .my(px(17.))
            .rounded(px(5.))
            .overflow_hidden()
            .children(fills.into_iter().map(|fill| div().flex_1().bg(fill)))
            .into_any_element(),
    }
}

fn picture_note(picture: Picture) -> &'static str {
    match picture {
        Picture::Dial => "A needle swings to the total across the class bands.",
        Picture::Column => "The evidence stacks up, beside earlier lab reports.",
        Picture::Track => "The evidence laid end to end along the points scale.",
    }
}

/// The name of a picture (marked when chosen) and a line about it.
fn picture_words(picture: Picture, chosen: bool, (ink, muted): (Hsla, Hsla)) -> Div {
    let name = if chosen { format!("{} · chosen", picture.name()) } else { picture.name().to_string() };
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_start()
        .gap_1()
        .child(div().text_base().font_weight(FontWeight::SEMIBOLD).text_color(ink).child(name))
        .child(div().w_full().text_xs().text_color(muted).whitespace_normal().text_left().child(picture_note(picture)))
}

impl Workbench {
    /// Three cards, one per picture; the one chosen is marked, and the choice is remembered.
    pub(super) fn picture_choices(&self, cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().flex_wrap().gap_4();
        for picture in Picture::ALL {
            row = row.child(self.picture_card(picture, cx));
        }
        div().flex().flex_col().gap_3().child(eyebrow("Choose how the class is drawn", cx)).child(row)
    }

    /// One picture's card: a small drawing, its name and a line about it.
    fn picture_card(&self, picture: Picture, cx: &mut Context<Self>) -> Button {
        let theme = cx.theme();
        let (ink, muted, line, dark) = (theme.foreground, theme.muted_foreground, theme.border, theme.is_dark());
        let chosen = self.picture == picture;
        Button::new(SharedString::from(format!("welcome-picture-{}", picture.name().to_lowercase())))
            .ghost()
            .flex_1()
            .min_w(px(200.))
            .h_auto()
            .p_4()
            .rounded(px(crate::look::CARD_RADIUS))
            .bg(theme.popover)
            .border_1()
            .border_color(if chosen { crate::look::edge(cx) } else { line })
            .accessibility_label(format!("Draw the class as a {}", picture.name().to_lowercase()))
            .selected(chosen)
            .toggled(chosen)
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(div().w(px(96.)).flex().justify_center().child(sketch(picture, dark)))
                    .child(picture_words(picture, chosen, (ink, muted))),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.choose_picture(picture, cx)))
    }

    /// What the app does and what it does not, in four short statements.
    pub(super) fn promises(&self, cx: &App) -> Div {
        let muted = cx.theme().muted_foreground;
        let statement = |label: &'static str, title: &'static str, body: &'static str| {
            div()
                .flex_1()
                .min_w(px(200.))
                .flex()
                .flex_col()
                .gap_1()
                .pt_3()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(eyebrow(label, cx))
                .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(div().text_sm().text_color(muted).child(body))
        };
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(statement(
                "You decide",
                "Every criterion is your choice",
                "You pick each criterion and its strength. ClassiVAR never applies one for you.",
            ))
            .child(statement(
                "It explains",
                "How the evidence makes the class",
                "It adds up the points, shows what moves the class, and compares today with earlier reports.",
            ))
            .child(statement(
                "Not for clinical reports",
                "Research and learning only",
                "The rules stay UNVERIFIED until a qualified curator reviews them, and every class says so.",
            ))
            .child(statement(
                "Scope",
                "Germline sequence variants",
                "Not built for copy-number variants (ACMG/ClinGen CNV points), mitochondrial variants, somatic or cancer variants, or low-penetrance risk alleles: those use other frameworks.",
            ))
    }

    /// The rules the app is running, at the foot of the opening screen.
    pub(super) fn welcome_footer(&self, cx: &App) -> Div {
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(super::picture::status_pill(self.rules, cx))
            .child(caption(muted).child(format!("Rule file: {} ({})", self.rules.meta.name, self.rules.meta.id)))
    }
}
