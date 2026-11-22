//! The criteria list and the strength guide.

use super::*;
use classivar_core::rules::StrengthGuide;

impl Workbench {
    pub(super) fn criteria_panel(&self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        // The gene sources take the whole sheet while open; Back returns to the evidence.
        if let Some(sources) = self.gene_drawer(cx) {
            return crate::look::panel(cx)
                .flex()
                .flex_col()
                .w(px(width))
                .h_full()
                .flex_none()
                .overflow_hidden()
                .child(sources)
                .into_any_element();
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
            .children(self.helper_section(cx))
            .children(self.criteria_sections(cx));

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

    /// The questions that set a criterion's strength. Nothing is applied until one is answered.
    pub(super) fn strength_guide(&self, c: &Criterion, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let citation = c.guide_source.as_deref().and_then(|id| self.rules.source(id)).map(|s| s.citation.clone());
        let heading = self.guide_heading(c, cx);
        // With a gene-level question, the situations wait until it is answered.
        let asks_gene = !c.gene_level.is_empty();
        let answers: Vec<Div> = if asks_gene && self.guide_gene_level.is_none() {
            vec![
                caption(muted)
                    .child("Answer the gene-level question first: it sets how far the strength below is lowered."),
            ]
        } else {
            c.guide.iter().enumerate().map(|(i, option)| self.guide_answer(c, i, option, cx)).collect()
        };
        div()
            .mt_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(background)
            .flex()
            .flex_col()
            .gap_2()
            .child(heading)
            .when(asks_gene, |d| d.child(self.gene_level_question(c, cx)))
            .children(answers)
            .when_some(citation, |d, citation| d.child(caption(muted).child(format!("Source: {citation}"))))
            .into_any_element()
    }

    /// The guide's question and its Cancel button.
    fn guide_heading(&self, c: &Criterion, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("Which fits this variant? The answer sets the strength of {}.", c.code)),
            )
            .child(
                Button::new(SharedString::from(format!("guide-cancel-{}", c.code)))
                    .label("Cancel")
                    .accessibility_label(format!("Close the {} strength guide", c.code))
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.guide_open = None;
                        this.guide_gene_level = None;
                        cx.notify();
                    })),
            )
    }

    /// The gene-level question (Abou Tayoun 2018, Table 1), asked before the
    /// situations. "No LoF mechanism" ends the guide: PVS1 does not apply.
    fn gene_level_question(&self, c: &Criterion, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let source = c.gene_level_source.as_deref().and_then(|id| self.rules.source(id)).map(|s| s.citation.clone());
        let mut column = div().flex().flex_col().gap_1().child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child("First, the gene: is loss of function how it causes this disease?"),
        );
        for (i, level) in c.gene_level.iter().enumerate() {
            let chosen = self.guide_gene_level.as_deref() == Some(level.id.as_str());
            let (code, id, steps) = (c.code.clone(), level.id.clone(), level.steps);
            let button = Button::new(SharedString::from(format!("gene-level-{}-{i}", c.code)))
                .label(level.short.clone())
                .accessibility_label(format!("{} gene level: {}", c.code, level.short))
                .small()
                .outline()
                .selected(chosen)
                .toggled(chosen)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.guide_gene_level = Some(id.clone());
                    if steps.is_none() {
                        this.answer_guide(&code, None, String::new());
                    }
                    cx.notify();
                }));
            column = column.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(div().w(px(180.)).flex_none().child(button))
                    .child(div().flex_1().min_w_0().text_xs().child(level.situation.clone())),
            );
        }
        column.when_some(source, |d, source| d.child(caption(muted).child(format!("Source: {source}"))))
    }

    /// One answer of a strength guide: the button that applies it, and the situation it describes.
    /// With a gene-level answer, the button names the strength after that answer's step.
    fn guide_answer(&self, c: &Criterion, i: usize, option: &StrengthGuide, cx: &mut Context<Self>) -> Div {
        let strength = c.strength_for(&option.id, self.guide_gene_level.as_deref());
        let (code, situation) = (c.code.clone(), option.id.clone());
        let chosen =
            self.current().get(&c.code).and_then(|choice| choice.situation.as_deref()) == Some(option.id.as_str());
        div()
            .flex()
            .items_start()
            .gap_2()
            .child(
                div().w(px(112.)).flex_none().child(
                    Button::new(SharedString::from(format!("guide-{}-{i}", c.code)))
                        .label(strength.map_or("Does not apply", Strength::label))
                        .accessibility_label(format!(
                            "{} {}: {}",
                            c.code,
                            strength.map_or("does not apply", Strength::label),
                            option.short
                        ))
                        .small()
                        .outline()
                        .selected(chosen)
                        .toggled(chosen)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.answer_guide(&code, strength, situation.clone());
                            cx.notify();
                        })),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(option.short.clone()))
                    .child(div().text_xs().child(option.situation.clone())),
            )
    }
}
