//! The evidence helpers on screen: a card at the top of each tab whose criteria a
//! helper sets, and, once opened, the helper on its own page over the list.

use super::focus::{Page, tagged};
use super::helper_bar::Entry;
use super::*;

impl Workbench {
    /// The cards of the helpers that set a criterion of this kind of evidence.
    pub(super) fn helper_cards(&self, kind: Category, cx: &mut Context<Self>) -> Vec<AnyElement> {
        self.helper_entries()
            .into_iter()
            .filter(|entry| {
                entry.codes.iter().any(|code| self.rules.criterion(code).is_some_and(|c| c.category == kind))
            })
            .map(|entry| self.helper_card(&entry, cx))
            .collect()
    }

    /// The cards of the helpers that set any of `codes`, for a guided step.
    pub(super) fn helper_cards_for(&self, codes: &[String], cx: &mut Context<Self>) -> Vec<AnyElement> {
        self.helper_entries()
            .into_iter()
            .filter(|entry| entry.codes.iter().any(|code| codes.iter().any(|c| c == code)))
            .map(|entry| self.helper_card(&entry, cx))
            .collect()
    }

    /// One helper as a compact card: its name, the codes it sets, and Open.
    fn helper_card(&self, entry: &Entry, cx: &mut Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let codes = entry.codes.iter().map(|code| {
            crate::look::figure(crate::look::SMALL_TEXT)
                .px(px(6.))
                .py(px(1.))
                .rounded(px(5.))
                .border_1()
                .border_color(border)
                .child(*code)
        });
        let key = entry.key;
        let open = Button::new(SharedString::from(format!("{key}-toggle")))
            .label("Open")
            .icon(Icon::new(IconName::ChevronRight))
            .accessibility_label(format!("Open the {}", entry.what))
            .tooltip(entry.about)
            .small()
            .outline()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_helper(key);
                cx.notify();
            }));
        // One line: the helper's name, the codes it sets, and Open. What it does is said on its page.
        let card = crate::look::card(cx)
            .py_2()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(crate::look::eyebrow("Helper", cx))
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(entry.label))
            .children(codes)
            .child(div().flex_1())
            .child(tagged(format!("helper-open-{key}"), open));
        tagged(format!("helper-card-{key}"), card).into_any_element()
    }

    /// The open helper on its own page. Only one opens at a time from the screen; if
    /// several are open, their panels follow one another under the first one's head.
    pub(super) fn helper_page(&self, width: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let entries: Vec<Entry> = self.helper_entries().into_iter().filter(|entry| entry.open).collect();
        let first = entries.first()?;
        let (key, toggle) = (first.key, first.toggle);
        let back = self.back_button(
            (format!("{key}-close"), format!("helper-back-{key}")),
            format!("Close the {}", first.what),
            toggle,
            cx,
        );
        let mut body = div().flex().flex_col().gap_6();
        for entry in &entries {
            body = body.child((entry.panel)(self, cx));
        }
        let page = Page {
            id: format!("helper-page-{key}"),
            back,
            kicker: "Evidence helper".to_string(),
            title: div()
                .text_size(px(18.))
                .line_height(px(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(first.title)
                .into_any_element(),
            intro: Some(format!(
                "{} It reads published tables and suggests a strength; you still choose what to apply.",
                first.about
            )),
            body,
            footer: None,
        };
        Some(self.focus_page(width, page, cx))
    }
}
