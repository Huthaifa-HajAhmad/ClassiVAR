//! The top bar and the case panel (variant, gene, gene decision).

use super::unsaved::save_button_label;
use super::*;

impl Workbench {
    /// The top bar: the mode and the research-use note at the left; the rules'
    /// status, the file buttons and the theme at the right, in groups with a line between.
    pub(super) fn top_bar(&self, unsaved: bool, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .child(self.mode_buttons(cx))
            // The note gives way when the window is narrow (the rail folds away at the same width).
            .when(self.stage.rail_shown.get(), |bar| {
                bar.child(caption(muted).child("Research use · not for clinical reporting"))
            })
            .child(div().flex_1())
            .child(super::picture::status_pill(self.rules, cx))
            .child(crate::icons::rule(cx))
            .child(self.file_buttons(unsaved, cx))
            .child(crate::icons::rule(cx))
            .child(self.theme_button(cx))
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

    /// Rules, Open and Save: one row of equal-height buttons, each with an icon.
    fn file_buttons(&self, unsaved: bool, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .items_center()
            .gap_1()
            .when(unsaved, |row| row.child(caption(muted).pr_2().child("Changed since saved")))
            .child(
                Button::new("open-rules")
                    .icon(Icon::new(IconName::Settings2))
                    .label("Rules")
                    .accessibility_label("Open a rule file in a new window")
                    .tooltip("Try an expert panel's gene-specific rule file. It opens in a new window; this case is not changed.")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| this.open_rules(window, cx))),
            )
            .child(
                Button::new("open-case")
                    .icon(Icon::new(IconName::FolderOpen))
                    .label("Open")
                    .accessibility_label("Open case…")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_case(window, cx);
                    })),
            )
            .child(
                Button::new("save-case")
                    .icon(Icon::new(IconName::HardDrive))
                    .label("Save")
                    .accessibility_label(save_button_label(unsaved))
                    .small()
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.save_case(cx);
                    })),
            )
    }

    /// Light or dark, as a two-part switch with a sun and a moon; the theme in use is the raised half.
    pub(super) fn theme_button(&self, cx: &mut Context<Self>) -> Div {
        let dark = cx.theme().is_dark();
        let half = |mode: ThemeMode, word: &'static str, icon: IconName, cx: &mut Context<Self>| {
            let chosen = (mode == ThemeMode::Dark) == dark;
            crate::look::segment_icon(format!("theme-{}", word.to_lowercase()), icon, word, chosen, cx)
                .accessibility_label(format!("{word} theme"))
                .on_click(cx.listener(move |this, _, window, cx| this.set_theme(mode, window, cx)))
        };
        let light = half(ThemeMode::Light, "Light", IconName::Sun, cx);
        let dark_half = half(ThemeMode::Dark, "Dark", IconName::Moon, cx);
        crate::look::segmented([light, dark_half], cx)
    }

    /// The case at the top of the editor sheet: variant, gene, the curator's gene
    /// decision, the gene sources, and the earlier record being edited.
    pub(super) fn case_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let lookups_started = !matches!(self.gene.clingen, Lookup::Idle);
        let gene = crate::look::form_section("Gene", false, cx)
            .child(super::records::form_row(
                "Symbol",
                div().w_full().flex().items_center().gap_2().child(self.gene_box(cx)).child(
                    Button::new("gene-check")
                        .icon(Icon::new(IconName::Search))
                        .label("Check")
                        .accessibility_label("Check gene")
                        .small()
                        .outline()
                        .rounded(px(999.))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.check_gene(cx);
                        })),
                ),
                cx,
            ))
            .children(self.gene_error.as_ref().map(|error| notice(error.clone(), warning_fill(), warning_ink())))
            .when(lookups_started, |d| d.child(self.gene_summary(cx)))
            .child(self.gene_decision_buttons(cx));
        div()
            .id("case-panel")
            .flex()
            .flex_col()
            .gap_5()
            .px_2()
            .pt_1()
            .pb_5()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(!self.stage.rail_shown.get(), |d| {
                d.child(div().flex().justify_end().child(self.clear_case_button(cx)))
            })
            .child(crate::look::form_section("Variant", true, cx).child(self.variant_rows(cx)))
            .child(gene)
            .child(crate::look::form_section("Condition", false, cx).child(self.condition_rows(cx)))
            .when(self.mode == Mode::ReportedVariant, |d| {
                d.child(
                    crate::look::form_section("Earlier reports", false, cx)
                        // With the rail shown, the import sits there, beside Add record.
                        .when(!self.stage.rail_shown.get(), |d| d.children(self.clinvar_import_row(false, cx)))
                        .child(self.records_bar(cx)),
                )
            })
            .into_any_element()
    }

    /// Beside the case's name: forget the variant, gene, condition, reports and criteria together.
    pub(super) fn clear_case_button(&self, cx: &mut Context<Self>) -> Div {
        div().child(
            Button::new("clear-case")
                .icon(Icon::new(IconName::Delete))
                .label("Clear case")
                .accessibility_label("Clear the whole case")
                .tooltip("Empty the variant, gene, condition, reports and criteria.")
                .small()
                .ghost()
                .disabled(!self.has_case_data(cx))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.ask(Confirm::ClearCase);
                    cx.notify();
                })),
        )
    }

    /// The curator's own answer to: does this gene cause this disease?
    fn gene_decision_buttons(&self, cx: &mut Context<Self>) -> Div {
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
        super::records::form_row("Causes the disease?", crate::look::segmented(choices, cx), cx)
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
