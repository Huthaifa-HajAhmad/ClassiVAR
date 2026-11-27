//! What a click or a key does to the evidence on screen.

use super::*;

impl Workbench {
    /// The variant name the curator typed, if any. Long pastes are cut to the
    /// same cap the saved case uses, so a display or a file cannot balloon.
    pub(super) fn variant_name(&self, cx: &App) -> Option<String> {
        let typed = self.boxes.variant.read(cx).value();
        let typed = typed.trim().chars().take(MAX_VARIANT_CHARS).collect::<String>();
        (!typed.is_empty()).then_some(typed)
    }

    pub(super) fn has_case_data(&self, cx: &App) -> bool {
        !self.today.is_empty()
            || !self.review.is_empty()
            || !self.records.is_empty()
            || self.variant_name(cx).is_some()
            || self.gene_link != GeneDecision::NotDecided
            || !self.boxes.gene.read(cx).value().trim().is_empty()
    }

    /// Ask a question, unless one is already waiting for an answer. A second
    /// question must never replace the first: the user may have read it already
    /// and could confirm something else by mistake.
    pub(super) fn ask(&mut self, confirm: Confirm) {
        if self.confirm.is_none() {
            self.confirm = Some(confirm);
        }
    }

    /// Scroll the pane under the mouse pointer (the criteria list when the
    /// pointer is over neither pane) with Page Up, Page Down, Home and End.
    /// Home and End belong to a text box while one has focus.
    pub(super) fn scroll_with_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        // The confirmation window covers the screen: keys must not scroll what is behind it.
        if self.confirm.is_some() {
            return;
        }
        if matches!(key, "home" | "end") {
            let own = [&self.boxes.variant, &self.boxes.gene, &self.boxes.record_name, &self.boxes.reason];
            if own
                .into_iter()
                .chain(self.helpers.text_boxes())
                .any(|input| input.read(cx).focus_handle(cx).is_focused(window))
            {
                return;
            }
        }
        let handle = if self.handles.result_scroll.bounds().contains(&window.mouse_position()) {
            &self.handles.result_scroll
        } else {
            &self.handles.criteria_scroll
        };
        let furthest = handle.max_offset().y.abs();
        let page = handle.bounds().size.height * 0.9;
        let now = handle.offset().y;
        let target = match key {
            "pagedown" => now - page,
            "pageup" => now + page,
            "home" => px(0.),
            _ => -furthest,
        };
        let target = target.max(-furthest).min(px(0.));
        handle.set_offset(point(handle.offset().x, target));
        cx.notify();
    }

    /// What Escape does: answer "Cancel" to a question, otherwise close the gene-sources
    /// drawer, the open strength guide or reason prompt, a criterion's page, or an open helper.
    pub(super) fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm.is_some() {
            self.cancel_confirm(window, cx);
        } else if self.gene_cards_open {
            self.gene_cards_open = false;
            cx.notify();
        } else if self.guide_open.is_some() {
            self.close_guide();
            cx.notify();
        } else if self.raising.take().is_some() || self.about.take().is_some() || self.close_helpers() {
            cx.notify();
        }
    }

    pub(super) fn evidence(&self, scope: Editing) -> &Evidence {
        match scope {
            Editing::Today => &self.today,
            Editing::Record(i) => &self.records[i].evidence,
        }
    }

    pub(super) fn evidence_mut(&mut self, scope: Editing) -> &mut Evidence {
        self.preview = false;
        match scope {
            Editing::Today => &mut self.today,
            Editing::Record(i) => &mut self.records[i].evidence,
        }
    }

    pub(super) fn current(&self) -> &Evidence {
        self.evidence(self.editing)
    }

    pub(super) fn scope_label(&self, scope: Editing) -> String {
        match scope {
            Editing::Today => "Today (you)".to_string(),
            Editing::Record(i) => format!("{} · {}", self.records[i].label, self.records[i].year),
        }
    }

    /// Apply, open the guide for, or remove a criterion. A guided criterion on
    /// today's evidence never disappears from this button: the first click opens
    /// the guide, a second closes it, and removal has its own button.
    pub(super) fn toggle(&mut self, criterion: &Criterion) {
        // Clicking the code ends any open reason prompt for it, so the prompt cannot outlive the criterion.
        if self.raising.as_ref().is_some_and(|raise| raise.code == criterion.code) {
            self.raising = None;
        }
        if self.guide_open.as_deref() == Some(criterion.code.as_str()) {
            self.close_guide();
            return;
        }
        let scope = self.editing;
        if let Some(choice) = self.evidence_mut(scope).remove(&criterion.code) {
            // A guided criterion on today's evidence stays: this click only opens its guide.
            if scope == Editing::Today && !criterion.guide.is_empty() {
                self.open_guide(&criterion.code, choice.gene_level.clone(), choice.situation.clone());
                self.evidence_mut(scope).insert(criterion.code.clone(), choice);
            }
            return;
        }
        if scope == Editing::Today && !criterion.guide.is_empty() {
            self.open_guide(&criterion.code, None, None);
            return;
        }
        // A record should start from the strength the 2015 paper used, not from
        // a later recommendation it predates.
        let strength = match scope {
            Editing::Today => criterion.strength,
            Editing::Record(_) => criterion.original_strength.unwrap_or(criterion.strength),
        };
        let (code, choice) = (criterion.code.clone(), Choice::new(strength));
        match scope {
            Editing::Today => self.count_today(code, choice),
            Editing::Record(_) => {
                self.evidence_mut(scope).insert(code, choice);
            }
        }
    }

    /// Take a criterion off the evidence being edited, including a guided one.
    pub(super) fn remove_criterion(&mut self, code: &str) {
        if self.raising.as_ref().is_some_and(|raise| raise.code == code) {
            self.raising = None;
        }
        self.evidence_mut(self.editing).remove(code);
        if self.guide_open.as_deref() == Some(code) {
            self.close_guide();
        }
    }

    /// Give the panel to a criterion's strength guide, starting from a saved answer when there is one.
    fn open_guide(&mut self, code: &str, gene_level: Option<String>, situation: Option<String>) {
        self.about = None;
        self.guide_open = Some(code.to_string());
        self.guide_gene_level = gene_level;
        self.guide_pick = situation;
    }

    /// Close every page that takes the right panel (a criterion's page, a guide, an open helper).
    /// Pages belong to the evidence on screen, so they close when that evidence changes.
    pub(super) fn close_pages(&mut self) {
        self.about = None;
        self.close_guide();
        self.close_helpers();
    }

    #[cfg(test)]
    pub(super) fn open_guide_for_test(&mut self, code: &str) {
        self.open_guide(code, None, None);
    }

    /// Leave the strength guide without answering it: nothing changes in the evidence.
    pub(super) fn close_guide(&mut self) {
        self.guide_open = None;
        self.guide_gene_level = None;
        self.guide_pick = None;
    }

    /// The curator's answer to a strength guide, with the gene-level answer when
    /// the criterion asks one. `None` means the criterion does not apply.
    pub(super) fn answer_guide(&mut self, code: &str, strength: Option<Strength>, situation: String) {
        self.guide_open = None;
        self.guide_pick = None;
        let gene_level = self.guide_gene_level.take();
        match strength {
            Some(strength) => {
                self.count_today(code.to_string(), Choice::guided(strength, situation, gene_level));
            }
            None => {
                self.today.remove(code);
            }
        };
    }

    pub(super) fn next_strength(&mut self, criterion: &Criterion, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.current().get(&criterion.code).map(|choice| choice.strength) else { return };
        if self.editing == Editing::Today && !criterion.guide.is_empty() {
            let choice = self.today.get(&criterion.code);
            let (level, situation) =
                (choice.and_then(|c| c.gene_level.clone()), choice.and_then(|c| c.situation.clone()));
            self.open_guide(&criterion.code, level, situation);
            return;
        }
        let options = self.strength_options(criterion);
        let at = options.iter().position(|s| *s == current).unwrap_or(0);
        self.choose_strength(criterion, options[(at + 1) % options.len()], window, cx);
    }

    /// The strengths the criterion may take in the evidence being edited. Today's
    /// limits apply to today's evidence; a record may use what the lab used.
    pub(super) fn strength_options(&self, criterion: &Criterion) -> Vec<Strength> {
        match self.editing {
            Editing::Today => criterion.allowed_strengths(),
            Editing::Record(_) => criterion.recorded_strengths(),
        }
    }

    /// Set a strength chosen by hand. Raising today's strength above the default
    /// needs a written reason first, so that case opens the reason box instead.
    pub(super) fn choose_strength(
        &mut self,
        criterion: &Criterion,
        next: Strength,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editing == Editing::Today && self.rules.magnitude(next) > self.rules.magnitude(criterion.strength) {
            let earlier = self.today.get(&criterion.code).and_then(|c| c.reason.clone()).unwrap_or_default();
            self.boxes.reason.update(cx, |input, cx| input.set_value(earlier, window, cx));
            self.raising = Some(Raise { code: criterion.code.clone(), to: next, note: false });
            return;
        }
        // A manual change no longer matches a guide answer or an earlier reason, so those are dropped.
        let scope = self.editing;
        self.evidence_mut(scope).insert(criterion.code.clone(), Choice::new(next));
    }

    /// Save the raised strength with the reason that was typed. Does nothing
    /// while the box is empty: the button is disabled then too.
    pub(super) fn confirm_raise(&mut self, cx: &mut Context<Self>) {
        let typed = self.boxes.reason.read(cx).value();
        let reason: String = typed.trim().chars().take(MAX_REASON_CHARS).collect();
        if reason.is_empty() {
            return;
        }
        match self.raising.take() {
            Some(Raise { code, note: true, .. }) => {
                if let Some(choice) = self.today.get_mut(&code) {
                    choice.reason = Some(reason);
                }
            }
            Some(raise) => {
                self.count_today(raise.code, Choice::with_reason(raise.to, reason));
            }
            None => {}
        }
    }

    /// Open the prompt to note what the curator looked at for an applied criterion
    /// (a source and its date, for example). The current note is shown to edit.
    pub(super) fn start_note(&mut self, code: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(choice) = self.today.get(code) else { return };
        let (to, earlier) = (choice.strength, choice.reason.clone().unwrap_or_default());
        self.boxes.reason.update(cx, |input, cx| input.set_value(earlier, window, cx));
        self.raising = Some(Raise { code: code.to_string(), to, note: true });
    }

    /// Switch between the light and dark theme, and remember the choice for the
    /// next time the program starts.
    /// Use the light or dark theme, and remember the choice on this machine.
    pub(super) fn set_theme(&mut self, mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) {
        if (mode == ThemeMode::Dark) == cx.theme().is_dark() {
            return;
        }
        Theme::change(mode, Some(window), cx);
        crate::settings::save_theme(mode);
        cx.notify();
    }
}
