//! The top bar and the case panel (variant, gene, gene decision).

use super::unsaved::save_button_label;
use super::*;

impl Workbench {
    pub(super) fn top_bar(&self, unsaved: bool, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (border, muted, background) = (theme.border, theme.muted_foreground, theme.background);
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_3()
            .px_5()
            .py_3()
            .border_b_1()
            .border_color(border)
            .bg(background)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_lg().font_weight(FontWeight::BOLD).child("ClassiVAR"))
                    .child(caption(muted).child("Research and learning only")),
            )
            .child(self.mode_buttons(cx))
            .child(div().flex_1())
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .text_xs()
                    .text_color(muted)
                    .child(rules_tag(self.rules)),
            )
            .child(self.file_buttons(unsaved, cx))
            .child(self.theme_button(cx))
            .child(self.learning_button(cx))
            .into_any_element()
    }

    /// New variant, or a variant earlier labs already reported.
    fn mode_buttons(&self, cx: &mut Context<Self>) -> Div {
        let mode_button = |id: &'static str, label: &'static str, mode: Mode, cx: &mut Context<Self>| {
            let chosen = self.mode == mode;
            Button::new(id).label(label).small().selected(chosen).toggled(chosen).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.set_mode(mode);
                    cx.notify();
                },
            ))
        };
        div().flex().gap_1().child(mode_button("mode-new", "New variant", Mode::NewVariant, cx)).child(mode_button(
            "mode-reported",
            "Reported variant",
            Mode::ReportedVariant,
            cx,
        ))
    }

    fn file_buttons(&self, unsaved: bool, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .items_center()
            .gap_3()
            .when(unsaved, |row| row.child(caption(muted).child("Changed since saved")))
            .child(
                Button::new("save-case")
                    .label("Save")
                    .accessibility_label(save_button_label(unsaved))
                    .small()
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.save_case(cx);
                    })),
            )
            .child(Button::new("open-case").label("Open").accessibility_label("Open case…").small().outline().on_click(
                cx.listener(|this, _, window, cx| {
                    this.open_case(window, cx);
                }),
            ))
    }

    fn theme_button(&self, cx: &mut Context<Self>) -> Button {
        let dark = cx.theme().is_dark();
        Button::new("theme-toggle")
            .label(if dark { "Light" } else { "Dark" })
            .accessibility_label(if dark { "Theme: dark. Switch to light." } else { "Theme: light. Switch to dark." })
            .toggled(dark)
            .small()
            .outline()
            .on_click(cx.listener(|this, _, window, cx| this.toggle_theme(window, cx)))
    }

    fn learning_button(&self, cx: &mut Context<Self>) -> Button {
        Button::new("learning")
            .label(if self.learning { "Learning on" } else { "Learning off" })
            .small()
            .selected(self.learning)
            .toggled(self.learning)
            .on_click(cx.listener(|this, _, _, cx| {
                this.learning = !this.learning;
                cx.notify();
            }))
    }

    pub(super) fn case_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (border, band) = (theme.border, theme.background);
        let label = |text: &'static str| {
            div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child(text)
        };
        let lookups_started = !matches!(self.gene.clingen, Lookup::Idle);
        // The panel can hold four source cards (up to 360 px) and thirty record
        // chips. A cap with its own scroll keeps a short window from pushing
        // the criteria and the result out of reach.
        div()
            .id("case-panel")
            .max_h(relative(0.42))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .px_5()
            .py_3()
            .bg(band)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(label("Variant"))
                    .child(div().w(px(300.)).child(Input::new(&self.variant_input).aria_label("Variant name")))
                    .child(label("Gene"))
                    .child(div().w(px(200.)).child(Input::new(&self.gene_input).aria_label("Gene symbol")))
                    .child(Button::new("gene-check").label("Check gene").small().on_click(cx.listener(
                        |this, _, _, cx| {
                            this.check_gene(cx);
                        },
                    )))
                    .child(div().flex_1())
                    .child(self.gene_decision_buttons(cx)),
            )
            .children(self.gene_suggestions(cx))
            .children(self.gene_error.as_ref().map(|error| notice(error.clone(), warning_fill(), warning_ink())))
            .when(lookups_started, |d| d.child(self.gene_summary(cx)))
            .when(lookups_started && self.gene_cards_open, |d| d.child(self.gene_cards(cx)))
            .when(self.mode == Mode::ReportedVariant, |d| d.child(self.records_bar(cx)))
            .into_any_element()
    }

    /// The curator's own answer to: does this gene cause this disease?
    fn gene_decision_buttons(&self, cx: &mut Context<Self>) -> Div {
        let label = div()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .child("Gene causes this disease?");
        let mut decision = div().flex().items_center().gap_2().child(label);
        for (choice, text) in [
            (GeneDecision::Established, "Yes"),
            (GeneDecision::NotEstablished, "Doubtful"),
            (GeneDecision::NotDecided, "Not decided"),
        ] {
            let chosen = self.gene_link == choice;
            decision = decision.child(
                Button::new(SharedString::from(format!("gene-decision-{text}")))
                    .label(text)
                    .small()
                    .selected(chosen)
                    .toggled(chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.gene_link = choice;
                        cx.notify();
                    })),
            );
        }
        decision
    }

    /// Symbols that start with what is in the gene box, so the curator can fill
    /// the field with one click. A typing aid only: it fills the box, and
    /// "Check gene" (or Enter) starts the lookups.
    pub(super) fn gene_suggestions(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let typed = self.gene_input.read(cx).value();
        let matches = crate::genes::suggestions_for(&typed, 6);
        if matches.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        let mut row =
            div().flex().flex_wrap().items_center().gap_2().child(caption(muted).child("Fill in the gene box:"));
        for symbol in matches {
            row = row.child(
                Button::new(SharedString::from(format!("gene-suggestion-{symbol}")))
                    .label(symbol)
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.gene_input.update(cx, |input, cx| input.set_value(symbol, window, cx));
                    })),
            );
        }
        row = row.child(caption(muted).child("A short list for typing only, not a check that the gene is right."));
        Some(row.into_any_element())
    }
}
