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
                    gene_level: a.gene_level.clone(),
                    reason: a.reason.clone().map(bounded_reason),
                },
            )
        })
        .collect()
}

/// Read and check a case file from disk before opening it.
pub(crate) fn read_case_file(path: &std::path::Path, rules: &RuleSet) -> Result<CaseFile, String> {
    let size = std::fs::metadata(path).map(|meta| meta.len()).map_err(|error| error.to_string())?;
    if size > MAX_CASE_BYTES {
        return Err(format!("the file is larger than {} MB and cannot be a case file", MAX_CASE_BYTES / (1024 * 1024)));
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    CaseFile::from_json(&text, rules).map_err(|error| error.to_string())
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
            condition: r.condition.clone(),
            source: r.source.clone(),
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
            identity: self.variant.identity.clone(),
            condition: self.variant.condition.clone(),
        }
    }

    /// Ask where to save, then write the case file there.
    pub(super) fn save_case(&mut self, cx: &mut Context<Self>) {
        let case = self.case_file(cx);
        // Never write a file the app would refuse to open again.
        if let Err(error) = case.check(self.rules) {
            self.set_file_status(Status::attention(format!("Not saved: {error}.")), cx);
            return;
        }
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
            let loaded = cx.background_spawn(async move { read_case_file(&path, rules) }).await;
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
    pub(crate) fn apply_case(&mut self, case: CaseFile, window: &mut Window, cx: &mut Context<Self>) {
        self.category = None;
        self.baseline = None;
        self.reading = false;
        self.comparison = false;
        self.inspected = None;
        self.preview = false;
        self.welcome = false;
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
                condition: r.condition.clone(),
                source: r.source.clone(),
            })
            .collect();
        self.editing = Editing::Today;
        self.mode = if self.records.is_empty() { Mode::NewVariant } else { Mode::ReportedVariant };
        self.gene_link = case.gene_decision;
        self.case_gene = case.gene.clone();
        self.pending_gene_box = None;
        self.typing_condition = false;
        self.gene_cards_open = false;
        self.gene_error = None;
        // A question or an open guide belongs to the case that was on screen.
        self.confirm = None;
        self.close_pages();
        self.next_record = Self::next_record_number(&self.records);
        // Gene answers from the network are not saved; the curator checks the gene again.
        self.gene = GeneChecks::idle();
        self.lookup_id += 1;
        self._lookup_tasks.clear();
        self.boxes.variant.update(cx, |input, cx| input.set_value(case.variant.clone(), window, cx));
        self.boxes.gene.update(cx, |input, cx| input.set_value(case.gene.clone(), window, cx));
        self.reset_helpers(window, cx);
        self.restore_variant(&case, window, cx);

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
                .px_4()
                .pb_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .pl_4()
                        .pr_1()
                        .py_1()
                        .rounded(px(crate::look::CARD_RADIUS))
                        .bg(fill)
                        .text_color(ink)
                        .child(div().flex_1().min_w_0().py_1().text_sm().child(status.text.clone()))
                        .child(
                            Button::new("file-status-dismiss")
                                .icon(Icon::new(IconName::Close))
                                .label("Dismiss")
                                .small()
                                .ghost()
                                .text_color(ink)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.file_status = None;
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
        )
    }

    /// Forget the whole case: evidence, records, the gene decision and the variant name.
    pub(super) fn clear_case(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.category = None;
        self.baseline = None;
        self.reading = false;
        self.comparison = false;
        self.inspected = None;
        self.preview = false;
        self.today.clear();
        self.records.clear();
        self.editing = Editing::Today;
        self.pending_gene_box = None;
        self.typing_condition = false;
        self.gene_cards_open = false;
        self.gene_link = GeneDecision::NotDecided;
        self.gene_error = None;
        self.file_status = None;
        self.forget_saved();
        self.confirm = None;
        self.close_pages();
        self.next_record = 1;
        self.boxes.variant.update(cx, |input, cx| input.set_value("", window, cx));
        self.reset_helpers(window, cx);
        self.reset_variant(window, cx);
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

    use super::{MAX_CASE_BYTES, open_status, read_case_file};
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
            identity: None,
            condition: None,
        }
    }

    #[test]
    fn a_case_file_that_cannot_be_read_is_refused_with_a_reason() {
        let rules = RuleSet::builtin().unwrap();
        let path = |tag: &str| {
            std::env::temp_dir().join(format!("classivar-case-read-{}-{tag}.classivar.json", std::process::id()))
        };
        let good_text = case(rules).to_json();
        let too_big = vec![b' '; usize::try_from(MAX_CASE_BYTES).unwrap() + 1];
        let files: [(&str, Vec<u8>); 4] = [
            ("good", good_text.clone().into_bytes()),
            ("truncated", good_text.as_bytes()[..good_text.len() / 2].to_vec()),
            ("not-utf8", vec![0xff, 0xfe, 0x00]),
            ("too-big", too_big),
        ];
        for (tag, bytes) in &files {
            std::fs::write(path(tag), bytes).expect("the temp folder is writable");
        }

        assert!(read_case_file(&path("good"), rules).is_ok());
        for tag in ["truncated", "not-utf8"] {
            let error = read_case_file(&path(tag), rules).unwrap_err();
            assert!(!error.is_empty(), "{tag} is refused with a reason");
        }
        let error = read_case_file(&path("too-big"), rules).unwrap_err();
        assert!(error.contains("larger than 4 MB"), "{error}");
        let error = read_case_file(&path("missing"), rules).unwrap_err();
        assert!(!error.is_empty(), "a missing file is refused with a reason");

        for (tag, _) in &files {
            std::fs::remove_file(path(tag)).ok();
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
