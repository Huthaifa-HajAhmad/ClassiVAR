//! The right panel: the case form and the criteria list, or one focus page in their
//! place (the gene sources, a strength guide, a criterion's page, an open helper).

use super::*;

impl Workbench {
    pub(super) fn criteria_panel(&self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        if let Some(page) = self.focus_for(width, cx) {
            return page;
        }
        let list = div()
            .id("criteria")
            .track_scroll(&self.handles.criteria_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.case_panel(cx))
            // Child 1: the category chips under the picture scroll the sheet here.
            .child(self.criteria_bar(cx))
            .child(self.criteria_tabs(cx));

        crate::look::panel(cx)
            .flex()
            .flex_col()
            .w(px(width))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .child(self.criteria_header(cx))
            .child(list)
            .into_any_element()
    }

    /// The page that takes the panel, if one is open: the gene sources first, then the
    /// strength guide (today's evidence only), a criterion's page, and an open helper.
    fn focus_for(&self, width: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        if let Some(sources) = self.gene_drawer(cx) {
            return Some(
                crate::look::panel(cx)
                    .flex()
                    .flex_col()
                    .w(px(width))
                    .h_full()
                    .flex_none()
                    .overflow_hidden()
                    .child(sources)
                    .into_any_element(),
            );
        }
        let guided = self.guide_open.as_deref().filter(|_| self.editing == Editing::Today);
        if let Some(c) = guided.and_then(|code| self.rules.criterion(code)) {
            return Some(self.guide_page(c, width, cx));
        }
        if let Some(c) = self.about.as_deref().and_then(|code| self.rules.criterion(code)) {
            return Some(self.criterion_page(c, width, cx));
        }
        self.helper_page(width, cx)
    }

    /// The title of the list: whose evidence is being edited. The lab's name is in the
    /// rail and the record form, so the label names only the year.
    fn criteria_header(&self, cx: &mut Context<Self>) -> Div {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let whose = match self.editing {
            Editing::Today => "Today (you)".to_string(),
            Editing::Record(i) => format!("Earlier report · {}", self.records[i].year),
        };
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .px_5()
            .pt_4()
            .pb_3()
            .border_b_1()
            .border_color(border)
            .child(crate::look::eyebrow(whose, cx))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Evidence"))
            .child(caption(muted).child("Click a code to apply it. You decide which criteria apply."))
    }
}
