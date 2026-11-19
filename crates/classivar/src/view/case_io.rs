//! Saving, opening and clearing a case.

use super::unsaved::banner_status;
use super::*;

/// The evidence of a saved block, as the screen holds it. A reason is cut to
/// the length the engine accepts, so an opened case can always be scored.
fn saved_evidence(list: &[Applied]) -> Evidence {
    list.iter()
        .map(|a| {
            (
                a.code.clone(),
                Choice {
                    strength: a.strength,
                    situation: a.situation.clone(),
                    reason: a.reason.clone().map(bounded_reason),
                },
            )
        })
        .collect()
}

impl Workbench {
    /// Everything the curator entered, ready to be written to a file. Results are
    /// not saved: they are worked out again when the case is opened.
    pub(super) fn case_file(&self, cx: &App) -> CaseFile {
        let saved_record = |r: &Record| SavedRecord {
            label: r.label.clone(),
            year: r.year,
            class: r.class,
            reported_tier: r.tier.clone(),
            criteria: to_applied(&r.evidence),
        };
        CaseFile {
            format: classivar_core::CASE_FORMAT,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            saved_on: classivar_sources::utc_today(),
            rule_set: self.rules.reference(),
            variant: self.variant_name(cx).unwrap_or_default(),
            // The gene the case was checked against, not whatever is being typed.
            gene: self.case_gene.clone(),
            gene_decision: self.gene_link,
            today: to_applied(&self.today),
            records: self.records.iter().map(saved_record).collect(),
        }
    }

    /// Ask where to save, then write the case file there.
    pub(super) fn save_case(&mut self, cx: &mut Context<Self>) {
        // A case file names a variant and a gene; the example is a demo, so it
        // is never written. Checking a gene on the example already asks first.
        if self.example {
            self.file_status = Some(Status::attention(
                "The fictional example is not saved. Clear it (checking a gene asks first) and build a real case, then Save.",
            ));
            cx.notify();
            return;
        }
        let case = self.case_file(cx);
        let saved = case.clone();
        let folder = default_save_folder();
        let chosen = cx.prompt_for_new_path(&folder, Some(&suggested_file_name(&case.variant)));
        self._file_task = Some(cx.spawn(async move |this, cx| {
            let message = match chosen.await {
                Ok(Ok(Some(path))) => Some(match write_case(&path, &case.to_json()) {
                    Ok(()) => Status::done(format!("Saved to {}.", path.display())),
                    Err(error) => Status::attention(format!("Could not save to {}: {error}", path.display())),
                }),
                // The curator closed the dialog on purpose; nothing to report.
                Ok(Ok(None)) => None,
                Ok(Err(error)) => Some(Status::attention(format!("Could not open the save dialog: {error}"))),
                Err(_) => Some(Status::attention("The save dialog closed without an answer.")),
            };
            this.update(cx, |this, cx| this.finish_save(message, saved, cx)).ok();
        }));
    }

    /// Ask which file to open, read and check it, then show it (after a question if work is on screen).
    pub(super) fn open_case(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rules = self.rules;
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open a ClassiVAR case".into()),
        });
        self._file_task = Some(cx.spawn_in(window, async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    let message = Status::attention(format!("Could not open the file dialog: {error}"));
                    this.update(cx, |this, cx| this.set_file_status(message, cx)).ok();
                    return;
                }
                Err(_) => None,
            };
            let Some(path) = path else { return };
            let shown_path = path.display().to_string();
            // Reading and re-scoring a case can be slow, so it runs off the UI
            // thread. The size check stops a huge file from being read at all.
            let loaded = cx
                .background_spawn(async move {
                    let size = std::fs::metadata(&path).map(|meta| meta.len()).map_err(|error| error.to_string())?;
                    if size > MAX_CASE_BYTES {
                        return Err(format!(
                            "the file is larger than {} MB and cannot be a case file",
                            MAX_CASE_BYTES / (1024 * 1024)
                        ));
                    }
                    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                    let case = CaseFile::from_json(&text, rules).map_err(|error| error.to_string())?;
                    Ok(case)
                })
                .await;
            this.update_in(cx, |this, window, cx| match loaded {
                Err(error) => {
                    this.set_file_status(Status::attention(format!("Could not open {shown_path}: {error}")), cx)
                }
                Ok(case) if this.has_case_data(cx) => {
                    this.ask(Confirm::OpenCase(Box::new(case)));
                    cx.notify();
                }
                Ok(case) => this.apply_case(case, window, cx),
            })
            .ok();
        }));
    }

    pub(super) fn set_file_status(&mut self, message: Status, cx: &mut Context<Self>) {
        self.file_status = Some(message);
        cx.notify();
    }

    /// Replace what is on screen with a case that was already checked by `CaseFile::from_json`.
    pub(super) fn apply_case(&mut self, case: CaseFile, window: &mut Window, cx: &mut Context<Self>) {
        let (today, changes) = carry_forward(self.rules, &case.today);
        let adjusted = open_notes(&changes);
        self.today = today.into_iter().collect();
        self.records = case
            .records
            .iter()
            .map(|r| Record {
                label: r.label.clone(),
                year: r.year,
                class: r.class,
                tier: r.reported_tier.clone(),
                evidence: saved_evidence(&r.criteria),
            })
            .collect();
        self.editing = Editing::Today;
        self.mode = if self.records.is_empty() { Mode::NewVariant } else { Mode::ReportedVariant };
        self.gene_link = case.gene_decision;
        self.case_gene = case.gene.clone();
        self.gene_error = None;
        self.example = false;
        // A question or an open guide belongs to the case that was on screen.
        self.confirm = None;
        self.guide_open = None;
        self.open_details.clear();
        self.next_record = Self::next_record_number(&self.records);
        // Gene answers from the network are not saved; the curator checks the gene again.
        self.gene = GeneChecks::idle();
        self.lookup_id += 1;
        self._lookup_tasks.clear();
        self.variant_input.update(cx, |input, cx| input.set_value(case.variant.clone(), window, cx));
        self.gene_input.update(cx, |input, cx| input.set_value(case.gene.clone(), window, cx));
        self.reset_helpers(window, cx);

        // What is on screen now is the baseline: the file holds the case before it was adjusted.
        let shown = self.case_file(cx);
        self.remember_saved(shown);
        let status = open_status(self.rules, &case, &adjusted);
        self.set_file_status(status, cx);
    }

    /// The message about the last save or open. `unsaved` says whether the case has changed since.
    pub(super) fn file_status_banner(&self, unsaved: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let status = banner_status(self.file_status.as_ref()?, unsaved);
        let (fill, ink) = match status.tone {
            Tone::Done => (confirm_fill(), confirm_ink()),
            Tone::Attention => (warning_fill(), warning_ink()),
        };
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .px_4()
                .py_1()
                .child(notice(status.text.clone(), fill, ink))
                .child(Button::new("file-status-dismiss").label("Dismiss").small().outline().on_click(cx.listener(
                    |this, _, _, cx| {
                        this.file_status = None;
                        cx.notify();
                    },
                )))
                .into_any_element(),
        )
    }

    /// Forget the whole case: evidence, records, the gene decision and the variant name.
    pub(super) fn clear_case(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.today.clear();
        self.records.clear();
        self.editing = Editing::Today;
        self.gene_link = GeneDecision::NotDecided;
        self.gene_error = None;
        self.example = false;
        self.file_status = None;
        self.forget_saved();
        self.guide_open = None;
        self.confirm = None;
        self.open_details.clear();
        self.next_record = 1;
        self.variant_input.update(cx, |input, cx| input.set_value("", window, cx));
        self.reset_helpers(window, cx);
    }
}

/// What to tell the curator after a case was opened. Only a case that needed a
/// change, or was saved under other rules, asks for attention.
fn open_status(rules: &RuleSet, case: &CaseFile, adjusted: &[String]) -> Status {
    let mut notes = vec!["Opened.".to_string()];
    let other_rules = case.saved_with_other_rules(rules);
    if other_rules {
        notes.push(format!(
            "It was saved with rules {} v{} and the program now uses v{}, so classes may differ from what you saw.",
            case.rule_set.id,
            case.rule_set.version,
            rules.reference().version
        ));
    }
    if !adjusted.is_empty() {
        notes.push(format!("Adjusted to today's limits: {}.", adjusted.join(", ")));
    }
    notes.push("Check the gene again to refresh the gene sources.".to_string());
    let text = notes.join(" ");
    if other_rules || !adjusted.is_empty() { Status::attention(text) } else { Status::done(text) }
}

#[cfg(test)]
mod tests {
    use classivar_core::{CASE_FORMAT, CaseFile, GeneDecision, RuleSet};

    use super::open_status;
    use crate::status::Tone;

    fn case(rules: &RuleSet) -> CaseFile {
        CaseFile {
            format: CASE_FORMAT,
            app_version: "0.1.0".into(),
            saved_on: "2026-11-11".into(),
            rule_set: rules.reference(),
            variant: String::new(),
            gene: String::new(),
            gene_decision: GeneDecision::NotDecided,
            today: vec![],
            records: vec![],
        }
    }

    #[test]
    fn an_unchanged_case_opens_with_a_plain_done_message() {
        let rules = RuleSet::builtin().unwrap();
        let status = open_status(rules, &case(rules), &[]);
        assert_eq!(status.tone, Tone::Done);
        assert!(status.text.starts_with("Opened."), "{}", status.text);
    }

    #[test]
    fn a_case_from_other_rules_or_with_adjustments_asks_for_attention() {
        let rules = RuleSet::builtin().unwrap();
        let mut older = case(rules);
        older.rule_set.version = "0.1.0".into();
        let status = open_status(rules, &older, &[]);
        assert_eq!(status.tone, Tone::Attention);
        assert!(status.text.contains("saved with rules"), "{}", status.text);
        let adjusted = open_status(rules, &case(rules), &["PP3 lowered to Strong.".to_string()]);
        assert_eq!(adjusted.tone, Tone::Attention);
        assert!(adjusted.text.contains("Adjusted to today's limits: PP3 lowered to Strong."), "{}", adjusted.text);
    }
}
