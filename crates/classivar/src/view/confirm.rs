//! The question window shown before work is thrown away.

use gpui_kit::component::FocusTrapElement;

use super::*;

impl Workbench {
    pub(super) fn apply_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(confirm) = self.confirm.take() else { return };
        match confirm {
            // An index, not a name: a record may have been removed while the
            // question waited, so never index without checking.
            Confirm::Clear(Editing::Today) => {
                self.today.clear();
                self.kept.clear();
            }
            Confirm::Clear(Editing::Record(i)) => {
                if let Some(record) = self.records.get_mut(i) {
                    record.evidence.clear();
                }
            }
            Confirm::ClearCase => self.start_over(window, cx),
            Confirm::StartFromLatest => self.start_from_latest(),
            Confirm::OpenCase(case) => self.apply_case(*case, window, cx),
            Confirm::RemoveRecord(index, _) => self.remove_record(index, window, cx),
            Confirm::CloseWindow => window.remove_window(),
            Confirm::NewGene(symbol) => {
                self.clear_case(window, cx);
                self.run_gene_check(symbol, cx);
            }
        }
        cx.notify();
    }

    /// Empty the whole form, not only the criteria: the case, then the gene and its lookups.
    fn start_over(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_case(window, cx);
        self.case_gene.clear();
        self.gene = GeneChecks::idle();
        self.lookup_id += 1;
        self._lookup_tasks.clear();
        self.gene_cards_open = false;
        self.typing_condition = false;
        self.boxes.gene.update(cx, |input, cx| input.set_value("", window, cx));
    }

    pub(super) fn cancel_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.confirm, Some(Confirm::NewGene(_))) {
            // Put the old gene back in the box, so the box and the case agree.
            let old = self.case_gene.clone();
            self.boxes.gene.update(cx, |input, cx| input.set_value(old, window, cx));
        }
        self.confirm = None;
        cx.notify();
    }

    /// The words of the question for each step that throws work away.
    fn confirm_question(&self, confirm: &Confirm) -> String {
        match confirm {
            Confirm::Clear(scope) => {
                format!("Clear every criterion from {}? This cannot be undone.", self.scope_label(*scope))
            }
            Confirm::ClearCase => {
                "Clear the whole case? The variant, gene, condition, every report and every criterion are removed. This cannot be undone."
                    .to_string()
            }
            Confirm::StartFromLatest => {
                "Replace today's evidence with a copy of the latest record's evidence? Today's criteria will be lost."
                    .to_string()
            }
            Confirm::OpenCase(case) => format!(
                "Open the case for {}? It replaces today's evidence, the records and the gene decision on screen.",
                if case.variant.is_empty() { "an unnamed variant" } else { case.variant.as_str() }
            ),
            Confirm::NewGene(symbol) => format!(
                "You are switching from {} to {}. The evidence, records, gene decision and variant name belong to {}. Start a new case for {}? This clears them.",
                self.case_gene, symbol, self.case_gene, symbol
            ),
            Confirm::CloseWindow => {
                "You have changed the case since it was saved. Close ClassiVAR and lose the change? Choose Cancel, then Save case, to keep it."
                    .to_string()
            }
            Confirm::RemoveRecord(_, label) => {
                format!("Remove {label} and every criterion it reported? This cannot be undone.")
            }
        }
    }

    /// The question as a window over the screen. The dim layer takes every mouse
    /// click, and the window is a focus trap holding the keyboard focus (set in
    /// `render`): Tab and Shift-Tab cycle between its two answers and cannot
    /// reach the controls behind it, so the question can be answered without a mouse.
    pub(super) fn confirm_modal(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let confirm = self.confirm.as_ref()?;
        let (background, foreground, border) = (cx.theme().background, cx.theme().foreground, cx.theme().border);
        let question = self.confirm_question(confirm);
        let buttons = div()
            .flex()
            .justify_end()
            .gap_2()
            .child(Button::new("confirm-no").label("Cancel").small().outline().on_click(cx.listener(
                |this, _, window, cx| {
                    this.cancel_confirm(window, cx);
                },
            )))
            .child(Button::new("confirm-yes").label("Yes, continue").small().on_click(cx.listener(
                |this, _, window, cx| {
                    this.apply_confirm(window, cx);
                },
            )));
        let card = div()
            .w(px(460.))
            .max_w_full()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(border)
            .bg(background)
            .text_color(foreground)
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_sm().child(question))
            .child(buttons);
        Some(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .p_4()
                .bg(scrim())
                .child(card)
                .focus_trap("confirm-trap", &self.handles.confirm_focus)
                .into_any_element(),
        )
    }
}
