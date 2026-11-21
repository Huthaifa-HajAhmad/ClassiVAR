//! The top bar and the case panel (variant, gene, gene decision).

use super::unsaved::save_button_label;
use super::*;

impl Workbench {
    pub(super) fn top_bar(&self, unsaved: bool, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .child(self.mode_buttons(cx))
            .child(caption(muted).child("Research and learning only"))
            .child(div().flex_1())
            .child(super::picture::status_pill(self.rules, cx))
            .child(self.file_buttons(unsaved, cx))
            .child(self.theme_button(cx))
            .child(self.learning_button(cx))
            .into_any_element()
    }

    /// New variant, or a variant earlier labs already reported.
    fn mode_buttons(&self, cx: &mut Context<Self>) -> Div {
        let mut buttons = Vec::new();
        for (id, label, mode) in [
            ("mode-new", "New variant", Mode::NewVariant),
            ("mode-reported", "Reported variant", Mode::ReportedVariant),
        ] {
            buttons.push(crate::look::segment(id, label, self.mode == mode, cx).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.set_mode(mode);
                    cx.notify();
                },
            )));
        }
        crate::look::segmented(buttons, cx)
    }

    fn file_buttons(&self, unsaved: bool, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .items_center()
            .gap_2()
            .when(unsaved, |row| row.child(caption(muted).child("Changed since saved")))
            .child(
                Button::new("open-rules")
                    .label("Rules…")
                    .accessibility_label("Open a rule file in a new window")
                    .tooltip("Try an expert panel's gene-specific rule file. It opens in a new window; this case is not changed.")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| this.open_rules(window, cx))),
            )
            .child(Button::new("open-case").label("Open").accessibility_label("Open case…").small().ghost().on_click(
                cx.listener(|this, _, window, cx| {
                    this.open_case(window, cx);
                }),
            ))
            .child(
                Button::new("save-case")
                    .label("Save")
                    .accessibility_label(save_button_label(unsaved))
                    .small()
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.save_case(cx);
                    })),
            )
    }

    pub(super) fn theme_button(&self, cx: &mut Context<Self>) -> Button {
        let dark = cx.theme().is_dark();
        Button::new("theme-toggle")
            .label(if dark { "Light" } else { "Dark" })
            .accessibility_label(if dark { "Theme: dark. Switch to light." } else { "Theme: light. Switch to dark." })
            .toggled(dark)
            .small()
            .ghost()
            .on_click(cx.listener(|this, _, window, cx| this.toggle_theme(window, cx)))
    }

    fn learning_button(&self, cx: &mut Context<Self>) -> Button {
        let theme = cx.theme();
        // A small switch drawn beside the word, so on and off read at a glance.
        let knob = div()
            .w(px(22.))
            .h(px(12.))
            .p(px(2.))
            .rounded_full()
            .flex()
            .when(self.learning, |track| track.justify_end())
            .bg(if self.learning { theme.foreground } else { theme.muted })
            .border_1()
            .border_color(theme.border)
            .child(div().size(px(8.)).rounded_full().bg(if self.learning {
                theme.background
            } else {
                theme.muted_foreground
            }));
        Button::new("learning")
            .accessibility_label(if self.learning { "Learning on" } else { "Learning off" })
            .child(div().flex().items_center().gap_2().child(knob).child("Learning"))
            .small()
            .ghost()
            .selected(self.learning)
            .toggled(self.learning)
            .on_click(cx.listener(|this, _, _, cx| {
                this.learning = !this.learning;
                cx.notify();
            }))
    }

    /// The case at the top of the editor sheet: variant, gene, the curator's gene
    /// decision, the gene sources, and the earlier record being edited.
    pub(super) fn case_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let lookups_started = !matches!(self.gene.clingen, Lookup::Idle);
        div()
            .id("case-panel")
            .flex()
            .flex_col()
            .gap_3()
            .px_2()
            .pb_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(crate::look::eyebrow("Case", cx))
            .child(self.variant_rows(cx))
            .child(super::records::form_row(
                "Gene",
                div().w_full().flex().items_center().gap_2().child(self.gene_box(cx)).child(
                    Button::new("gene-check").label("Check gene").small().outline().rounded(px(999.)).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.check_gene(cx);
                        }),
                    ),
                ),
                cx,
            ))
            .children(self.gene_error.as_ref().map(|error| notice(error.clone(), warning_fill(), warning_ink())))
            .child(self.gene_decision_buttons(cx))
            .child(self.condition_rows(cx))
            .when(lookups_started, |d| d.child(self.gene_summary(cx)))
            .when(lookups_started && self.gene_cards_open, |d| d.child(self.gene_cards(cx)))
            .when(self.mode == Mode::ReportedVariant, |d| {
                d.children(self.clinvar_import_row(cx)).child(self.records_bar(cx))
            })
            .into_any_element()
    }

    /// The curator's own answer to: does this gene cause this disease?
    fn gene_decision_buttons(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let mut choices = Vec::new();
        for (choice, text) in [
            (GeneDecision::Established, "Yes"),
            (GeneDecision::NotEstablished, "Doubtful"),
            (GeneDecision::NotDecided, "Not decided"),
        ] {
            choices.push(
                crate::look::segment(format!("gene-decision-{text}"), text, self.gene_link == choice, cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.gene_link = choice;
                        cx.notify();
                    }),
                ),
            );
        }
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(caption(muted).child("Gene causes this disease?"))
            .child(crate::look::segmented(choices, cx))
    }

    /// The gene box, with matching symbols in a floating list under it. A typing
    /// aid only: picking a symbol fills the box, and "Check gene" (or Enter) starts the lookups.
    fn gene_box(&self, cx: &mut Context<Self>) -> Div {
        let typed = self.boxes.gene.read(cx).value();
        let matches: Vec<String> = crate::genes::suggestions_for(&typed, 6).into_iter().map(String::from).collect();
        super::suggest::with_suggestions(
            "gene-suggestion",
            Input::new(&self.boxes.gene).aria_label("Gene symbol"),
            matches,
            "For typing only, not a check that the gene is right.",
            &self.boxes.gene,
            cx,
        )
    }
}
