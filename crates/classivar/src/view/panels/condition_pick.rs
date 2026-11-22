//! The condition row when the gene sources have records: the picked condition with
//! Change and Type a name, or the button that opens the gene-sources drawer.

use super::gene_drawer::Picked;
use super::*;
use crate::view::variant::condition_from_curation;

/// The answer of a finished lookup.
fn done<T>(lookup: &Lookup<T>) -> Option<&Fetched<T>> {
    match lookup {
        Lookup::Done(found) => Some(found),
        _ => None,
    }
}

impl Workbench {
    /// How many records the gene sources gave that can be taken as a condition.
    pub(super) fn source_records(&self) -> usize {
        let g = &self.gene;
        let count = |n: Option<usize>| n.unwrap_or(0);
        count(done(&g.clingen).map(|f| f.value.curations.len()))
            + count(done(&g.g2p).map(|f| f.value.records.len()))
            + count(done(&g.england).map(|f| f.value.entries.len()))
            + count(done(&g.australia).map(|f| f.value.entries.len()))
    }

    /// The case's condition as a card (name, inheritance, MONDO id) with Change and Type a
    /// name; before one is picked, the button that opens the sources.
    pub(super) fn picked_condition(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let type_instead = Button::new("condition-type")
            .label("Type a name")
            .accessibility_label("Type the condition by hand")
            .small()
            .ghost()
            .on_click(cx.listener(|this, _, _, cx| {
                this.typing_condition = true;
                cx.notify();
            }));
        let Some(c) = self.variant.condition.as_ref().filter(|c| !c.name.is_empty()) else {
            return div()
                .flex()
                .items_center()
                .gap_2()
                .children(self.pick_button(Editing::Today, cx))
                .child(type_instead);
        };
        let about = [Some(c.inheritance.label().to_string()), c.mondo.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        let change = Button::new("condition-change")
            .icon(Icon::new(IconName::Globe))
            .label("Change")
            .accessibility_label("Choose another condition from the gene sources")
            .small()
            .ghost()
            .on_click(cx.listener(|this, _, _, cx| {
                this.open_sources(Editing::Today, cx);
            }));
        crate::look::card(cx)
            .flex()
            .items_start()
            .gap_3()
            .child(div().pt(px(2.)).child(crate::icons::glyph(IconName::CircleCheck, cx)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).whitespace_normal().child(c.name.clone()))
                    .child(caption(muted).child(about))
                    .child(div().flex().gap_1().pt_1().child(change).child(type_instead)),
            )
    }

    /// The button that opens the gene sources, once they have records to pick from.
    pub(super) fn pick_button(&self, scope: Editing, cx: &mut Context<Self>) -> Option<Div> {
        let count = self.source_records();
        if count == 0 {
            return None;
        }
        let label = format!("Pick from gene sources · {count} record{}", plural(count as i32));
        Some(
            div().child(
                Button::new("condition-from-sources")
                    .icon(Icon::new(IconName::Globe))
                    .label(label.clone())
                    .accessibility_label(label)
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, _, cx| this.open_sources(scope, cx))),
            ),
        )
    }

    /// Take a record from the drawer as the condition of what is being edited.
    pub(super) fn take_pick(&mut self, picked: Picked, window: &mut Window, cx: &mut Context<Self>) {
        let scope = self.sources_for;
        match (picked, scope) {
            (Picked::ClinGen(curation), Editing::Today) => self.use_curation(&curation, window, cx),
            (Picked::ClinGen(curation), Editing::Record(_)) => {
                self.set_scope_condition(condition_from_curation(&curation), window, cx)
            }
            (Picked::Named(condition, source), _) => {
                let note = format!(
                    "Condition set to {} ({}) from {source}. The inheritance is read from the source's wording; check it.",
                    condition.name,
                    condition.inheritance.label()
                );
                self.set_scope_condition(condition, window, cx);
                self.file_status = Some(Status::attention(note));
            }
        }
        self.gene_cards_open = false;
        if scope == Editing::Today {
            self.typing_condition = false;
        }
        cx.notify();
    }
}
