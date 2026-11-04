//! The main screen. Two paths:
//! - New variant: tick the criteria you apply; see points, class and why.
//! - Reported variant: enter what earlier records used, apply today's evidence,
//!   and see what changed and whether the class holds.
//!
//! The curator decides every criterion; the screen only adds up and explains.

use std::collections::{BTreeMap, HashSet};

use classivar_core::{
    Applied, Assessment, CaseFile, Category, Class, Consistency, Criterion, Direction, GeneDecision, Reason,
    Reevaluation, RuleSet, SavedRecord, Strength, Submission, Verdict, Warning, rules::ReviewStatus,
};
use std::path::PathBuf;

use classivar_sources::{
    Fetched, SourceError,
    clingen::{self, ClinGenGene},
    g2p::{self, G2pGene},
    panelapp::{self, Confidence, Instance, PanelAppGene},
};
use gpui_kit::component::{
    ActiveTheme, Disableable, Selectable, Sizable, Theme, ThemeMode,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

mod panels;

/// Width of one point in the evidence lanes.
const POINT_WIDTH: f32 = 36.0;
/// Width of one point on the points scale. Chosen so the whole scale fits the
/// result pane at the 960 px minimum window width.
const SCALE_UNIT: f32 = 20.0;

/// The largest a case file may be before it is refused without reading it.
const MAX_CASE_BYTES: u64 = 4 * 1024 * 1024;
/// The most earlier records one case can hold. The history table gets unreadable
/// long before this; the cap also keeps a damaged file from filling the screen.
const MAX_RECORDS: usize = 30;
/// The longest lab name the box accepts (in characters).
const MAX_LAB_NAME: usize = 60;
/// What a record is called when its name box is empty.
const UNNAMED_RECORD: &str = "Unnamed record";

// The fills are chosen so that the ink on them (white or near-black) reaches the
// WCAG AA contrast of 4.5:1. A test below checks every pair.
fn pathogenic_colour() -> Rgba {
    rgb(0xc62828)
}

fn benign_colour() -> Rgba {
    rgb(0x1f5fb5)
}

fn direction_colour(direction: Direction) -> Rgba {
    match direction {
        Direction::Pathogenic => pathogenic_colour(),
        Direction::Benign => benign_colour(),
    }
}

fn class_fill(class: Class, dark: bool) -> Rgba {
    if dark {
        return match class {
            Class::Pathogenic => pathogenic_colour(),
            Class::LikelyPathogenic => rgb(0x5f1b1b),
            Class::Vus => rgb(0x33332f),
            Class::LikelyBenign => rgb(0x173a5e),
            Class::Benign => benign_colour(),
        };
    }
    match class {
        Class::Pathogenic => pathogenic_colour(),
        Class::LikelyPathogenic => rgb(0xf6c9c8),
        Class::Vus => rgb(0xe6e5e0),
        Class::LikelyBenign => rgb(0xcde2fb),
        Class::Benign => benign_colour(),
    }
}

fn class_ink(class: Class, dark: bool) -> Rgba {
    if dark {
        return match class {
            Class::Pathogenic | Class::Benign => rgb(0xffffff),
            Class::LikelyPathogenic => rgb(0xffd9d5),
            Class::Vus => rgb(0xe8e7e1),
            Class::LikelyBenign => rgb(0xd6e8ff),
        };
    }
    if matches!(class, Class::Pathogenic | Class::Benign) { rgb(0xffffff) } else { rgb(0x0b0b0b) }
}

const CLASSES: [Class; 5] =
    [Class::Benign, Class::LikelyBenign, Class::Vus, Class::LikelyPathogenic, Class::Pathogenic];

fn plural(n: i32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// The points scale: its range and the zones, taken from the rule set so the
/// scale and the class badge cannot disagree.
#[derive(Debug, PartialEq, Eq)]
struct Scale {
    min: i32,
    max: i32,
    /// (class, first point, first point of the next zone)
    zones: Vec<(Class, i32, i32)>,
}

fn scale_for(rules: &RuleSet) -> Scale {
    let t = &rules.class_min_points;
    // Two points of room beyond the first and last class limit, for the Benign and Pathogenic zones.
    let (min, max) = (t.likely_benign - 2, t.pathogenic + 2);
    Scale {
        min,
        max,
        zones: vec![
            (Class::Benign, min, t.likely_benign),
            (Class::LikelyBenign, t.likely_benign, t.vus),
            (Class::Vus, t.vus, t.likely_pathogenic),
            (Class::LikelyPathogenic, t.likely_pathogenic, t.pathogenic),
            (Class::Pathogenic, t.pathogenic, max),
        ],
    }
}

/// How many detail lines a source card shows before "Show all".
const CARD_LINES: usize = 3;

fn describe_panelapp(found: &Fetched<PanelAppGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.entries.is_empty() {
        return (
            format!("{} is on no panel. That does not prove it is unrelated to disease.", gene.symbol),
            Vec::new(),
        );
    }
    let headline = format!(
        "{} is on {} panel{}: {} green, {} amber, {} red.",
        gene.symbol,
        gene.entries.len(),
        plural(gene.entries.len() as i32),
        gene.count_at(Confidence::Green),
        gene.count_at(Confidence::Amber),
        gene.count_at(Confidence::Red),
    );
    let lines = gene
        .entries
        .iter()
        .map(|e| {
            format!("• {} (v{}): {} · {}", e.panel_name, e.panel_version, e.confidence.label(), e.mode_of_inheritance)
        })
        .collect();
    (headline, lines)
}

fn describe_g2p(found: &Fetched<G2pGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.records.is_empty() {
        return (format!("G2P has no record for {}.", gene.symbol), Vec::new());
    }
    let headline = format!("{} gene-disease record{}.", gene.records.len(), plural(gene.records.len() as i32));
    let lines = gene
        .records
        .iter()
        .map(|r| {
            format!("• {}: {} · {} · {}", r.disease, r.confidence, r.genotype.replace('_', " "), r.molecular_mechanism)
        })
        .collect();
    (headline, lines)
}

fn describe_clingen(found: &Fetched<ClinGenGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.curations.is_empty() {
        return (format!("ClinGen has not rated {}.", gene.symbol), Vec::new());
    }
    let headline = format!("{} gene-disease rating{}.", gene.curations.len(), plural(gene.curations.len() as i32));
    let lines = gene
        .curations
        .iter()
        .map(|c| {
            format!(
                "• {} ({}): {} · {} · {}",
                c.disease, c.inheritance, c.classification, c.classified_on, c.expert_panel
            )
        })
        .collect();
    (headline, lines)
}

/// One criterion as the curator applied it: the strength, and the guide answer
/// that set it when a guide was used. The answer is saved with the case so the
/// reason for a strength is not lost.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice {
    strength: Strength,
    situation: Option<String>,
    /// Why the curator chose this strength (typed by them, or filled in by a helper).
    reason: Option<String>,
}

impl Choice {
    fn new(strength: Strength) -> Self {
        Self { strength, situation: None, reason: None }
    }

    fn guided(strength: Strength, situation: String) -> Self {
        Self { strength, situation: Some(situation), reason: None }
    }

    fn with_reason(strength: Strength, reason: String) -> Self {
        Self { strength, situation: None, reason: Some(reason) }
    }
}

/// A strength the curator is raising above its default, and has not yet explained.
struct Raise {
    code: String,
    to: Strength,
}

/// The longest reason the box accepts (in characters). The case file allows more.
const MAX_REASON_TYPED: usize = 300;

type Evidence = BTreeMap<String, Choice>;

fn to_applied(evidence: &Evidence) -> Vec<Applied> {
    evidence
        .iter()
        .map(|(code, choice)| Applied {
            code: code.clone(),
            strength: choice.strength,
            situation: choice.situation.clone(),
            reason: choice.reason.clone(),
        })
        .collect()
}

/// Bring saved evidence onto today's rules: lower a strength above today's cap,
/// drop a criterion that no longer exists, and fill in a guide answer that
/// matches the strength. Returns the choices and a plain-language list of what
/// changed, so the screen can say it rather than repair anything silently.
fn normalize_today(rules: &RuleSet, list: &[Applied]) -> (Vec<(String, Choice)>, Vec<String>) {
    let mut changed = Vec::new();
    let mut out = Vec::new();
    for applied in list {
        let Some(criterion) = rules.criterion(&applied.code) else {
            changed.push(format!("{} (criterion no longer exists, dropped)", applied.code));
            continue;
        };
        let allowed = criterion.allowed_strengths();
        let strength = if allowed.contains(&applied.strength) {
            applied.strength
        } else {
            let highest = *allowed.last().unwrap_or(&criterion.strength);
            changed.push(format!("{} to {}", applied.code, highest.label()));
            highest
        };
        let situation = if criterion.guide.is_empty() {
            None
        } else {
            let saved = applied
                .situation
                .as_deref()
                .filter(|s| criterion.guide.iter().any(|g| g.id == *s && g.strength == Some(strength)));
            saved.map(str::to_string).or_else(|| criterion.situation_for(strength).map(str::to_string))
        };
        // A reason explains one strength, so it is dropped when the strength was lowered.
        let reason = applied.reason.clone().filter(|_| strength == applied.strength);
        out.push((applied.code.clone(), Choice { strength, situation, reason }));
    }
    (out, changed)
}

/// The colours of every warning and notice. The kit's own warning pair is hard
/// to read (about 1.8:1 in the light theme), so the app uses its own pair, which
/// reads the same in both themes. A test checks the contrast.
fn warning_fill() -> Rgba {
    rgb(0xffe9a8)
}

fn warning_ink() -> Rgba {
    rgb(0x2b2100)
}

/// A short banner text on the warning colours.
fn notice(text: impl Into<SharedString>, background: impl Into<Hsla>, foreground: impl Into<Hsla>) -> Div {
    div().px_2().py_1().rounded_sm().bg(background.into()).text_color(foreground.into()).text_sm().child(text.into())
}

/// What the curator should read before trusting a class: the review status and rule version.
fn rules_tag(rules: &RuleSet) -> String {
    let meta = &rules.meta;
    match meta.status {
        ReviewStatus::Unverified => format!("UNVERIFIED rules v{}", meta.version),
        ReviewStatus::Reviewed => format!(
            "rules v{}, reviewed by {} on {}",
            meta.version,
            meta.reviewed_by.as_deref().unwrap_or("?"),
            meta.reviewed_on.as_deref().unwrap_or("?")
        ),
    }
}

/// A class with its review status and rule version, so a screenshot of the
/// class alone still shows how far it can be trusted.
fn class_badge(class: Class, rules: &RuleSet, prefix: &str, border: Hsla, dark: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(border)
        .bg(class_fill(class, dark))
        .text_color(class_ink(class, dark))
        .child(div().font_weight(FontWeight::BOLD).child(format!("{prefix}{}", class.label())))
        .child(div().text_xs().child(rules_tag(rules)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    NewVariant,
    ReportedVariant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Editing {
    Today,
    Record(usize),
}

/// What an earlier record (for example a lab in ClinVar) said and used.
struct Record {
    label: String,
    year: u16,
    /// None until the curator says what the record reported.
    class: Option<Class>,
    /// The VUS sub-tier the record named, if any (only meaningful when `class` is a VUS).
    tier: Option<String>,
    evidence: Evidence,
}

/// How the functional-evidence helper reads an assay.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AssayRoute {
    /// Only the number of validation controls is known.
    Controls,
    /// A statistical analysis gave an OddsPath value.
    OddsPath,
}

/// A step that throws work away. It waits for a click on "Yes" first.
#[derive(Clone, PartialEq, Eq)]
enum Confirm {
    Clear(Editing),
    StartFromLatest,
    LoadExample,
    /// Check another gene, which starts a new case.
    NewGene(String),
    /// Remove an earlier record and its evidence.
    RemoveRecord(usize, String),
    /// Open a case file over the work on screen.
    OpenCase(Box<CaseFile>),
}

/// Where one lookup stands.
enum Lookup<T> {
    Idle,
    Loading,
    Done(Fetched<T>),
    /// The lookup did not work. This is never shown as "nothing found".
    Failed(String),
}

/// What the four gene sources said about the gene the curator asked about.
struct GeneChecks {
    england: Lookup<PanelAppGene>,
    australia: Lookup<PanelAppGene>,
    g2p: Lookup<G2pGene>,
    clingen: Lookup<ClinGenGene>,
}

impl GeneChecks {
    fn idle() -> Self {
        Self { england: Lookup::Idle, australia: Lookup::Idle, g2p: Lookup::Idle, clingen: Lookup::Idle }
    }
}

/// A file name for the save dialog, made from the variant name. Anything that is
/// not a letter, digit, dash or dot becomes an underscore, so the name is safe on every system.
fn suggested_file_name(variant: &str) -> String {
    let stem: String = variant
        .trim()
        .chars()
        .take(40)
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect();
    let stem = if stem.is_empty() { "case".to_string() } else { stem };
    // Windows refuses CON.classivar.json even with an extension, so a reserved
    // device name gets a prefix rather than a raw OS error at save time.
    let reserved = |name: &str| {
        let base = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && base.as_bytes()[3].is_ascii_digit()
                && base.as_bytes()[3] != b'0')
    };
    let stem = if reserved(&stem) { format!("_{stem}") } else { stem };
    format!("{stem}.classivar.json")
}

/// Where the save dialog starts: the user's Documents folder. Saving into the
/// program's working directory (often the repository) is how case files end up
/// committed by accident.
fn default_save_folder() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from);
    if let Some(home) = home {
        let documents = home.join("Documents");
        return if documents.is_dir() { documents } else { home };
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// The fictional variant from the design sketch: (label, year, class, criteria).
type ExampleRecord = (&'static str, u16, Class, &'static [(&'static str, Strength)]);

const EXAMPLE_RECORDS: [ExampleRecord; 2] = [
    ("Lab A (example)", 2019, Class::Vus, &[("PM2", Strength::Moderate), ("PP3", Strength::Supporting)]),
    (
        "Lab B (example)",
        2022,
        Class::Vus,
        &[("PM2", Strength::Moderate), ("PP3", Strength::Supporting), ("PP4", Strength::Supporting)],
    ),
];

const EXAMPLE_TODAY: &[(&str, Strength)] = &[
    ("PM2", Strength::Supporting),
    ("PP3", Strength::Supporting),
    ("PP4", Strength::Supporting),
    ("PM5", Strength::Moderate),
];

fn evidence_of(list: &[(&str, Strength)]) -> Evidence {
    list.iter().map(|(code, strength)| (code.to_string(), Choice::new(*strength))).collect()
}

pub struct Workbench {
    rules: &'static RuleSet,
    mode: Mode,
    /// The curator's own evidence, today.
    today: Evidence,
    records: Vec<Record>,
    /// Which evidence set the criteria list is editing.
    editing: Editing,
    /// When on, every criterion shows its plain-language rule and source.
    learning: bool,
    variant_input: Entity<InputState>,
    gene_input: Entity<InputState>,
    /// The name box for the earlier record being edited (the lab or source).
    record_name_input: Entity<InputState>,
    /// The score box of the predictor helper (PP3 and BP4).
    predictor_input: Entity<InputState>,
    /// Whether the predictor helper is open.
    predictor_open: bool,
    /// Which tool of `rules.predictors` the helper reads.
    predictor_tool: usize,
    /// Scroll state of the criteria list and of the result pane, so the keyboard can scroll them.
    criteria_scroll: ScrollHandle,
    result_scroll: ScrollHandle,
    /// A strength being raised by hand, waiting for its reason.
    raising: Option<Raise>,
    /// The box where the reason for a raised strength is typed.
    reason_input: Entity<InputState>,
    /// The functional-evidence helper (PS3 and BS3): open or closed.
    functional_open: bool,
    functional_route: AssayRoute,
    /// For the controls route: did the assay show damage (PS3) or none (BS3)?
    functional_damaging: bool,
    controls_input: Entity<InputState>,
    oddspath_input: Entity<InputState>,
    gene: GeneChecks,
    /// Why the gene box was not checked (empty or not a symbol), shown under the row.
    gene_error: Option<String>,
    /// The gene this case is about: the last one checked.
    case_gene: String,
    /// Whether the four source cards are shown. They are tall, so the curator can hide them.
    gene_cards_open: bool,
    /// Cards that show all their lines instead of the first few.
    expanded_cards: HashSet<&'static str>,
    /// Criteria whose full note and source are open.
    open_details: HashSet<String>,
    /// The criterion whose strength guide is waiting for an answer.
    guide_open: Option<String>,
    gene_link: GeneDecision,
    confirm: Option<Confirm>,
    /// The result of the last save or open, shown until the curator dismisses it.
    file_status: Option<String>,
    /// The next number for a new record, so removing one cannot create two "Record 2"s.
    next_record: u32,
    /// Counts lookups, so a slow old answer cannot replace a newer one.
    lookup_id: u64,
    _lookup_tasks: Vec<Task<()>>,
    _file_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Workbench {
    pub fn new(rules: &'static RuleSet, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let variant_input = cx.new(|cx| InputState::new(window, cx).placeholder("Variant, e.g. NM_007294.4:c.5266dup"));
        let gene_input = cx.new(|cx| InputState::new(window, cx).placeholder("Gene symbol, e.g. BRCA1"));
        let on_gene_input = cx.subscribe_in(&gene_input, window, |this, _, event: &InputEvent, window, cx| {
            match event {
                InputEvent::PressEnter { .. } => this.check_gene(window, cx),
                // The gene box also drives the fill-in suggestions under it.
                InputEvent::Change => cx.notify(),
                InputEvent::Focus | InputEvent::Blur => {}
            }
        });
        // The result title and the notes read the variant box, so typing there
        // must repaint the workbench, not only the input.
        let on_variant_input = cx.subscribe_in(&variant_input, window, |_this, _, event: &InputEvent, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let record_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Lab or source, e.g. Lab A (ClinVar)"));
        // Typing in the name box renames the record that is open. A programmatic
        // change gives the same text, so it is harmless.
        let on_record_name =
            cx.subscribe_in(&record_name_input, window, |this, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Editing::Record(i) = this.editing else { return };
                let typed = input.read(cx).value();
                let name: String = typed.trim().chars().take(MAX_LAB_NAME).collect();
                if let Some(record) = this.records.get_mut(i) {
                    record.label = if name.is_empty() { UNNAMED_RECORD.to_string() } else { name };
                    cx.notify();
                }
            });
        let predictor_input = cx.new(|cx| InputState::new(window, cx).placeholder("Score, e.g. 0.85"));
        let on_predictor_input =
            cx.subscribe_in(&predictor_input, window, |_this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            });
        // Escape is watched for the whole window: no element has to hold keyboard
        // focus (after a click on a plain button none may), and a focused text box
        // must not hide it.
        let on_escape = cx.observe_keystrokes(|this, event, window, cx| match event.keystroke.key.as_str() {
            "escape" => this.escape(window, cx),
            key @ ("pageup" | "pagedown" | "home" | "end") => this.scroll_with_key(key, window, cx),
            _ => {}
        });
        let reason_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Why is this stronger than the default?"));
        let on_reason_input = cx.subscribe_in(&reason_input, window, |_this, _, event: &InputEvent, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let controls_input = cx.new(|cx| InputState::new(window, cx).placeholder("Controls, e.g. 11"));
        let oddspath_input = cx.new(|cx| InputState::new(window, cx).placeholder("OddsPath, e.g. 5.2"));
        let on_controls_input =
            cx.subscribe_in(&controls_input, window, |_this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            });
        let on_oddspath_input =
            cx.subscribe_in(&oddspath_input, window, |_this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            });
        Self {
            rules,
            mode: Mode::NewVariant,
            today: Evidence::new(),
            records: Vec::new(),
            editing: Editing::Today,
            learning: true,
            variant_input,
            gene_input,
            record_name_input,
            predictor_input,
            predictor_open: false,
            predictor_tool: 0,
            criteria_scroll: ScrollHandle::new(),
            result_scroll: ScrollHandle::new(),
            raising: None,
            reason_input,
            functional_open: false,
            functional_route: AssayRoute::Controls,
            functional_damaging: true,
            controls_input,
            oddspath_input,
            gene: GeneChecks::idle(),
            gene_error: None,
            case_gene: String::new(),
            gene_cards_open: false,
            expanded_cards: HashSet::new(),
            open_details: HashSet::new(),
            guide_open: None,
            gene_link: GeneDecision::NotDecided,
            confirm: None,
            file_status: None,
            next_record: 1,
            lookup_id: 0,
            _lookup_tasks: Vec::new(),
            _file_task: None,
            _subscriptions: vec![
                on_gene_input,
                on_variant_input,
                on_record_name,
                on_predictor_input,
                on_controls_input,
                on_oddspath_input,
                on_reason_input,
                on_escape,
            ],
        }
    }

    /// The variant name the curator typed, if any.
    fn variant_name(&self, cx: &App) -> Option<String> {
        let typed = self.variant_input.read(cx).value();
        let typed = typed.trim();
        (!typed.is_empty()).then(|| typed.to_string())
    }

    fn has_case_data(&self, cx: &App) -> bool {
        !self.today.is_empty()
            || !self.records.is_empty()
            || self.variant_name(cx).is_some()
            || !self.gene_input.read(cx).value().trim().is_empty()
    }

    /// Ask a question, unless one is already waiting for an answer. A second
    /// question must never replace the first: the user may have read it already
    /// and could confirm something else by mistake.
    fn ask(&mut self, confirm: Confirm) {
        if self.confirm.is_none() {
            self.confirm = Some(confirm);
        }
    }

    /// Scroll the pane under the mouse pointer (the criteria list when the
    /// pointer is over neither pane) with Page Up, Page Down, Home and End.
    /// Home and End belong to a text box while one has focus.
    fn scroll_with_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        // The confirmation window covers the screen: keys must not scroll what is behind it.
        if self.confirm.is_some() {
            return;
        }
        if matches!(key, "home" | "end") {
            let inputs = [
                &self.variant_input,
                &self.gene_input,
                &self.record_name_input,
                &self.predictor_input,
                &self.controls_input,
                &self.oddspath_input,
                &self.reason_input,
            ];
            if inputs.iter().any(|input| input.read(cx).focus_handle(cx).is_focused(window)) {
                return;
            }
        }
        let handle = if self.result_scroll.bounds().contains(&window.mouse_position()) {
            &self.result_scroll
        } else {
            &self.criteria_scroll
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

    /// What Escape does: answer "Cancel" to a question, otherwise close the open
    /// strength guide.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm.is_some() {
            self.cancel_confirm(window, cx);
        } else if self.guide_open.take().is_some() || self.raising.take().is_some() {
            cx.notify();
        }
    }

    /// Ask all four gene sources about the gene in the text box. A different gene
    /// from the one this case is about starts a new case, so the curator is asked first.
    fn check_gene(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.gene_input.read(cx).value().to_string();
        let symbol = typed.trim().to_ascii_uppercase();
        if !crate::genes::is_gene_symbol(&symbol) {
            self.gene_error = Some(
                "Type a gene symbol before checking: letters, digits or hyphens, up to 20 characters.".to_string(),
            );
            cx.notify();
            return;
        }
        self.gene_error = None;
        if !self.case_gene.is_empty() && symbol != self.case_gene && self.has_case_data(cx) {
            self.ask(Confirm::NewGene(symbol));
            cx.notify();
            return;
        }
        self.run_gene_check(typed, window, cx);
    }

    /// Start the four lookups. Each network call runs in the background and fills
    /// in its own card when it arrives, so one slow or failing source does not
    /// hold up the others.
    fn run_gene_check(&mut self, typed: String, _window: &mut Window, cx: &mut Context<Self>) {
        self.gene_error = None;
        let symbol = typed.trim().to_ascii_uppercase();
        if symbol != self.case_gene {
            // The decision about the old gene says nothing about this one.
            self.gene_link = GeneDecision::NotDecided;
            self.case_gene = symbol;
        }
        self.lookup_id += 1;
        let id = self.lookup_id;
        self.gene = GeneChecks {
            england: Lookup::Loading,
            australia: Lookup::Loading,
            g2p: Lookup::Loading,
            clingen: Lookup::Loading,
        };

        let (a, b, c, d) = (typed.clone(), typed.clone(), typed.clone(), typed);
        self._lookup_tasks = vec![
            Self::start_lookup(id, cx, move || panelapp::lookup_gene(Instance::England, &a), |g| &mut g.england),
            Self::start_lookup(id, cx, move || panelapp::lookup_gene(Instance::Australia, &b), |g| &mut g.australia),
            Self::start_lookup(id, cx, move || g2p::lookup_gene(&c), |g| &mut g.g2p),
            Self::start_lookup(id, cx, move || clingen::lookup_gene(&d), |g| &mut g.clingen),
        ];
        cx.notify();
    }

    /// Run one lookup in the background, then store its answer in `slot`.
    /// An answer that arrives after a newer check was started is dropped.
    fn start_lookup<T: Send + 'static>(
        id: u64,
        cx: &mut Context<Self>,
        work: impl FnOnce() -> Result<Fetched<T>, SourceError> + Send + 'static,
        slot: fn(&mut GeneChecks) -> &mut Lookup<T>,
    ) -> Task<()> {
        let pending = cx.background_spawn(async move { work() });
        cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |this, cx| {
                if this.lookup_id != id {
                    return;
                }
                *slot(&mut this.gene) = match result {
                    Ok(found) => Lookup::Done(found),
                    Err(error) => Lookup::Failed(error.to_string()),
                };
                cx.notify();
            })
            .ok();
        })
    }

    fn evidence(&self, scope: Editing) -> &Evidence {
        match scope {
            Editing::Today => &self.today,
            Editing::Record(i) => &self.records[i].evidence,
        }
    }

    fn evidence_mut(&mut self, scope: Editing) -> &mut Evidence {
        match scope {
            Editing::Today => &mut self.today,
            Editing::Record(i) => &mut self.records[i].evidence,
        }
    }

    fn current(&self) -> &Evidence {
        self.evidence(self.editing)
    }

    fn scope_label(&self, scope: Editing) -> String {
        match scope {
            Editing::Today => "Today (you)".to_string(),
            Editing::Record(i) => format!("{} · {}", self.records[i].label, self.records[i].year),
        }
    }

    fn editing_label(&self) -> String {
        self.scope_label(self.editing)
    }

    /// Apply, open the guide for, or remove a criterion. A guided criterion on
    /// today's evidence never disappears from this button: the first click opens
    /// the guide, a second closes it, and removal has its own button.
    fn toggle(&mut self, criterion: &Criterion) {
        // Clicking the code ends any open reason prompt for it, so the prompt cannot outlive the criterion.
        if self.raising.as_ref().is_some_and(|raise| raise.code == criterion.code) {
            self.raising = None;
        }
        if self.guide_open.as_deref() == Some(criterion.code.as_str()) {
            self.guide_open = None;
            return;
        }
        let scope = self.editing;
        let applied = self.evidence_mut(scope).remove(&criterion.code);
        if applied.is_some() {
            if scope == Editing::Today && !criterion.guide.is_empty() {
                if let Some(choice) = applied {
                    self.evidence_mut(scope).insert(criterion.code.clone(), choice);
                }
                self.guide_open = Some(criterion.code.clone());
            }
            return;
        }
        if scope == Editing::Today && !criterion.guide.is_empty() {
            self.guide_open = Some(criterion.code.clone());
            return;
        }
        // A record should start from the strength the 2015 paper used, not from
        // a later recommendation it predates.
        let strength = match scope {
            Editing::Today => criterion.strength,
            Editing::Record(_) => criterion.original_strength.unwrap_or(criterion.strength),
        };
        self.evidence_mut(scope).insert(criterion.code.clone(), Choice::new(strength));
    }

    /// Take a criterion off the evidence being edited, including a guided one.
    fn remove_criterion(&mut self, code: &str) {
        if self.raising.as_ref().is_some_and(|raise| raise.code == code) {
            self.raising = None;
        }
        self.evidence_mut(self.editing).remove(code);
        if self.guide_open.as_deref() == Some(code) {
            self.guide_open = None;
        }
    }

    /// The curator's answer to a strength guide. `None` means the criterion does not apply.
    fn answer_guide(&mut self, code: &str, strength: Option<Strength>, situation: String) {
        self.guide_open = None;
        match strength {
            Some(strength) => {
                self.today.insert(code.to_string(), Choice::guided(strength, situation));
            }
            None => {
                self.today.remove(code);
            }
        };
    }

    fn next_strength(&mut self, criterion: &Criterion, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.current().get(&criterion.code).map(|choice| choice.strength) else { return };
        if self.editing == Editing::Today && !criterion.guide.is_empty() {
            self.guide_open = Some(criterion.code.clone());
            return;
        }
        // Today's limits apply to today's evidence; a record may use what the lab used.
        let options = match self.editing {
            Editing::Today => criterion.allowed_strengths(),
            Editing::Record(_) => criterion.recorded_strengths(),
        };
        let at = options.iter().position(|s| *s == current).unwrap_or(0);
        let next = options[(at + 1) % options.len()];
        // Raising today's strength above the default needs a written reason first.
        if self.editing == Editing::Today && self.rules.magnitude(next) > self.rules.magnitude(criterion.strength) {
            let earlier = self.today.get(&criterion.code).and_then(|c| c.reason.clone()).unwrap_or_default();
            self.reason_input.update(cx, |input, cx| input.set_value(earlier, window, cx));
            self.raising = Some(Raise { code: criterion.code.clone(), to: next });
            return;
        }
        // A manual change no longer matches a guide answer or an earlier reason, so those are dropped.
        let scope = self.editing;
        self.evidence_mut(scope).insert(criterion.code.clone(), Choice::new(next));
    }

    /// Save the raised strength with the reason that was typed. Does nothing
    /// while the box is empty: the button is disabled then too.
    fn confirm_raise(&mut self, cx: &mut Context<Self>) {
        let typed = self.reason_input.read(cx).value();
        let reason: String = typed.trim().chars().take(MAX_REASON_TYPED).collect();
        if reason.is_empty() {
            return;
        }
        if let Some(raise) = self.raising.take() {
            self.today.insert(raise.code, Choice::with_reason(raise.to, reason));
        }
    }

    /// The score typed in the predictor helper: `None` when the box is empty,
    /// `Some(Err(()))` when it is not a finite number.
    fn typed_score(&self, cx: &App) -> Option<Result<f64, ()>> {
        let typed = self.predictor_input.read(cx).value();
        let typed = typed.trim();
        (!typed.is_empty()).then(|| typed.parse::<f64>().ok().filter(|score| score.is_finite()).ok_or(()))
    }

    /// What the functional helper's box holds: nothing, a bad entry, or a number.
    fn typed_number(&self, input: &Entity<InputState>, cx: &App) -> Option<Result<f64, ()>> {
        let typed = input.read(cx).value();
        let typed = typed.trim();
        (!typed.is_empty()).then(|| typed.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or(()))
    }

    /// Put the strength a score or an assay result gives onto today's evidence.
    /// The reason records where the number came from, so it is saved with the case.
    fn apply_reading(&mut self, reading: &classivar_core::EvidenceReading, reason: String) {
        self.today.insert(reading.code.to_string(), Choice::with_reason(reading.strength, reason));
    }

    /// Forget the score and close the helper. It belongs to the variant on screen.
    fn reset_helpers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.raising = None;
        self.predictor_open = false;
        self.predictor_tool = 0;
        self.predictor_input.update(cx, |input, cx| input.set_value("", window, cx));
        self.functional_open = false;
        self.functional_route = AssayRoute::Controls;
        self.functional_damaging = true;
        self.controls_input.update(cx, |input, cx| input.set_value("", window, cx));
        self.oddspath_input.update(cx, |input, cx| input.set_value("", window, cx));
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::NewVariant {
            self.editing = Editing::Today;
        }
    }

    /// Switch between the light and dark theme, and remember the choice for the
    /// next time the program starts.
    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next = if cx.theme().is_dark() { ThemeMode::Light } else { ThemeMode::Dark };
        Theme::change(next, Some(window), cx);
        crate::settings::save_theme(next);
        cx.notify();
    }

    /// Open an earlier record for editing and show its name in the name box.
    fn show_record(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(record) = self.records.get(index) else { return };
        let name = record.label.clone();
        self.editing = Editing::Record(index);
        self.record_name_input.update(cx, |input, cx| input.set_value(name, window, cx));
    }

    fn add_record(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.records.len() >= MAX_RECORDS {
            self.file_status = Some(format!("A case holds at most {MAX_RECORDS} earlier records."));
            return;
        }
        let label = format!("Record {}", self.next_record);
        self.next_record += 1;
        self.records.push(Record {
            label,
            year: classivar_sources::current_year(),
            class: None,
            tier: None,
            evidence: Evidence::new(),
        });
        self.show_record(self.records.len() - 1, window, cx);
    }

    fn remove_record(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.records.len() {
            return;
        }
        self.records.remove(index);
        // Stay next to the record that took its place, rather than jumping away.
        if self.records.is_empty() {
            self.editing = Editing::Today;
        } else {
            self.show_record(index.min(self.records.len() - 1), window, cx);
        }
        self.confirm = None;
    }

    /// The next "Record N" number, continuing after the highest one in `records`.
    fn next_record_number(records: &[Record]) -> u32 {
        records
            .iter()
            .filter_map(|record| record.label.strip_prefix("Record ").and_then(|number| number.parse::<u32>().ok()))
            .max()
            .unwrap_or(0)
            + 1
    }

    /// The fictional variant from the design sketch, so the history view can be tried at once.
    /// Does nothing if the rules no longer accept its criteria (a test also guards this).
    fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let accepted =
            EXAMPLE_RECORDS.iter().all(|(_, _, _, list)| self.rules.assess(&to_applied(&evidence_of(list))).is_ok())
                && self.rules.assess(&to_applied(&evidence_of(EXAMPLE_TODAY))).is_ok();
        if !accepted {
            self.file_status = Some(
                "The example does not fit the current rule file, so it was not loaded. Load the default rules to try it."
                    .to_string(),
            );
            return;
        }
        self.records = EXAMPLE_RECORDS
            .iter()
            .map(|(label, year, class, list)| Record {
                label: label.to_string(),
                year: *year,
                class: Some(*class),
                tier: None,
                evidence: evidence_of(list),
            })
            .collect();
        self.today = evidence_of(EXAMPLE_TODAY);
        self.editing = Editing::Today;
        // The example is fictional: it must not inherit the gene or variant that
        // happened to be on screen, or the class would be attributed to them.
        self.case_gene.clear();
        self.gene_link = GeneDecision::NotDecided;
        self.gene_error = None;
        self.gene = GeneChecks::idle();
        self.lookup_id += 1;
        self._lookup_tasks.clear();
        self.variant_input.update(cx, |input, cx| input.set_value("", window, cx));
        self.gene_input.update(cx, |input, cx| input.set_value("", window, cx));
        self.guide_open = None;
        self.confirm = None;
        self.reset_helpers(window, cx);
    }

    /// Copy the latest record's evidence into today's evidence, to adjust from there.
    fn start_from_latest(&mut self) {
        let latest = self.records.iter().enumerate().max_by_key(|(i, r)| (r.year, *i)).map(|(_, r)| r.evidence.clone());
        let Some(evidence) = latest else { return };
        let mut lowered = Vec::new();
        let mut guided = Vec::new();
        let mut dropped = Vec::new();
        let mut today = Evidence::new();
        for (code, choice) in evidence {
            let Some(criterion) = self.rules.criterion(&code) else {
                dropped.push(format!("{code} (criterion no longer exists)"));
                continue;
            };
            let allowed = criterion.allowed_strengths();
            let strength = if allowed.contains(&choice.strength) {
                choice.strength
            } else {
                let highest = *allowed.last().unwrap_or(&criterion.strength);
                lowered.push(format!("{code} to {}", highest.label()));
                highest
            };
            let situation = if criterion.guide.is_empty() {
                None
            } else {
                guided.push(code.clone());
                let saved = choice
                    .situation
                    .as_deref()
                    .filter(|s| criterion.guide.iter().any(|g| g.id == *s && g.strength == Some(strength)));
                match saved.map(str::to_string).or_else(|| criterion.situation_for(strength).map(str::to_string)) {
                    Some(situation) => Some(situation),
                    None => {
                        dropped.push(format!("{code} (no guide answer for {})", strength.label()));
                        continue;
                    }
                }
            };
            let reason = choice.reason.clone().filter(|_| strength == choice.strength);
            today.insert(code, Choice { strength, situation, reason });
        }
        self.today = today;
        let mut notes = Vec::new();
        if !lowered.is_empty() {
            notes.push(format!("Lowered to today's limit: {}.", lowered.join(", ")));
        }
        if !dropped.is_empty() {
            notes.push(format!("Not carried over: {}.", dropped.join(", ")));
        }
        if !guided.is_empty() {
            notes.push(format!(
                "Check the strength of {}: click its strength button to answer the guide again.",
                guided.join(", ")
            ));
        }
        if !notes.is_empty() {
            self.file_status = Some(notes.join(" "));
        }
    }

    /// Forget the whole case: evidence, records, the gene decision and the variant name.
    fn clear_case(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.today.clear();
        self.records.clear();
        self.editing = Editing::Today;
        self.gene_link = GeneDecision::NotDecided;
        self.gene_error = None;
        self.guide_open = None;
        self.confirm = None;
        self.open_details.clear();
        self.next_record = 1;
        self.variant_input.update(cx, |input, cx| input.set_value("", window, cx));
        self.reset_helpers(window, cx);
    }

    // ---------- saving and opening a case ----------

    /// Everything the curator entered, ready to be written to a file. Results are
    /// not saved: they are worked out again when the case is opened.
    fn case_file(&self, cx: &App) -> CaseFile {
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
    fn save_case(&mut self, cx: &mut Context<Self>) {
        let case = self.case_file(cx);
        let folder = default_save_folder();
        let chosen = cx.prompt_for_new_path(&folder, Some(&suggested_file_name(&case.variant)));
        self._file_task = Some(cx.spawn(async move |this, cx| {
            let message = match chosen.await {
                Ok(Ok(Some(path))) => Some(match std::fs::write(&path, case.to_json()) {
                    Ok(()) => format!("Saved to {}.", path.display()),
                    Err(error) => format!("Could not save to {}: {error}", path.display()),
                }),
                // The curator closed the dialog on purpose; nothing to report.
                Ok(Ok(None)) => None,
                Ok(Err(error)) => Some(format!("Could not open the save dialog: {error}")),
                Err(_) => Some("The save dialog closed without an answer.".to_string()),
            };
            this.update(cx, |this, cx| {
                if message.is_some() {
                    this.file_status = message;
                }
                cx.notify();
            })
            .ok();
        }));
    }

    /// Ask which file to open, read and check it, then show it (after a question if work is on screen).
    fn open_case(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                    let message = format!("Could not open the file dialog: {error}");
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
                    if case.records.len() > MAX_RECORDS {
                        return Err(format!(
                            "it holds {} earlier records and this program opens at most {MAX_RECORDS}",
                            case.records.len()
                        ));
                    }
                    Ok(case)
                })
                .await;
            this.update_in(cx, |this, window, cx| match loaded {
                Err(error) => this.set_file_status(format!("Could not open {shown_path}: {error}"), cx),
                Ok(case) if this.has_case_data(cx) => {
                    this.ask(Confirm::OpenCase(Box::new(case)));
                    cx.notify();
                }
                Ok(case) => this.apply_case(case, window, cx),
            })
            .ok();
        }));
    }

    fn set_file_status(&mut self, message: String, cx: &mut Context<Self>) {
        self.file_status = Some(message);
        cx.notify();
    }

    /// Replace what is on screen with a case that was already checked by `CaseFile::from_json`.
    fn apply_case(&mut self, case: CaseFile, window: &mut Window, cx: &mut Context<Self>) {
        let evidence = |list: &[Applied]| -> Evidence {
            list.iter()
                .map(|a| {
                    (
                        a.code.clone(),
                        Choice { strength: a.strength, situation: a.situation.clone(), reason: a.reason.clone() },
                    )
                })
                .collect()
        };
        let (today, adjusted) = normalize_today(self.rules, &case.today);
        self.today = today.into_iter().collect();
        self.records = case
            .records
            .iter()
            .map(|r| Record {
                label: r.label.clone(),
                year: r.year,
                class: r.class,
                tier: r.reported_tier.clone(),
                evidence: evidence(&r.criteria),
            })
            .collect();
        self.editing = Editing::Today;
        self.mode = if self.records.is_empty() { Mode::NewVariant } else { Mode::ReportedVariant };
        self.gene_link = case.gene_decision;
        self.case_gene = case.gene.clone();
        self.gene_error = None;
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

        let mut notes = vec!["Opened.".to_string()];
        if case.saved_with_other_rules(self.rules) {
            let now = self.rules.reference();
            notes.push(format!(
                "It was saved with rules {} v{} and the program now uses v{}, so classes may differ from what you saw.",
                case.rule_set.id, case.rule_set.version, now.version
            ));
        }
        if !adjusted.is_empty() {
            notes.push(format!("Adjusted to today's limits: {}.", adjusted.join(", ")));
        }
        notes.push("Check the gene again to refresh the gene sources.".to_string());
        self.set_file_status(notes.join(" "), cx);
    }

    fn file_status_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let message = self.file_status.as_ref()?;
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .px_4()
                .py_1()
                .child(notice(message.clone(), warning_fill(), warning_ink()))
                .child(Button::new("file-status-dismiss").label("Dismiss").small().outline().on_click(cx.listener(
                    |this, _, _, cx| {
                        this.file_status = None;
                        cx.notify();
                    },
                )))
                .into_any_element(),
        )
    }

    fn apply_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(confirm) = self.confirm.take() else { return };
        match confirm {
            // An index, not a name: a record may have been removed while the
            // question waited, so never index without checking.
            Confirm::Clear(Editing::Today) => self.today.clear(),
            Confirm::Clear(Editing::Record(i)) => {
                if let Some(record) = self.records.get_mut(i) {
                    record.evidence.clear();
                }
            }
            Confirm::StartFromLatest => self.start_from_latest(),
            Confirm::LoadExample => self.load_example(window, cx),
            Confirm::OpenCase(case) => self.apply_case(*case, window, cx),
            Confirm::RemoveRecord(index, _) => self.remove_record(index, window, cx),
            Confirm::NewGene(symbol) => {
                self.clear_case(window, cx);
                self.run_gene_check(symbol, window, cx);
            }
        }
        cx.notify();
    }

    fn cancel_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.confirm, Some(Confirm::NewGene(_))) {
            // Put the old gene back in the box, so the box and the case agree.
            let old = self.case_gene.clone();
            self.gene_input.update(cx, |input, cx| input.set_value(old, window, cx));
        }
        self.confirm = None;
        cx.notify();
    }

    // ---------- confirmation window ----------

    /// The question as a window over the screen. The dim layer takes every click,
    /// so nothing behind it can be used until the curator answers.
    fn confirm_modal(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let confirm = self.confirm.as_ref()?;
        let (background, foreground, border) = (cx.theme().background, cx.theme().foreground, cx.theme().border);
        let question = match confirm {
            Confirm::Clear(scope) => {
                format!("Clear every criterion from {}? This cannot be undone.", self.scope_label(*scope))
            }
            Confirm::StartFromLatest => {
                "Replace today's evidence with a copy of the latest record's evidence? Today's criteria will be lost."
                    .to_string()
            }
            Confirm::LoadExample => {
                "Load the fictional example? It replaces today's evidence and all records.".to_string()
            }
            Confirm::OpenCase(case) => format!(
                "Open the case for {}? It replaces today's evidence, the records and the gene decision on screen.",
                if case.variant.is_empty() { "an unnamed variant" } else { case.variant.as_str() }
            ),
            Confirm::NewGene(symbol) => format!(
                "You are switching from {} to {}. The evidence, records, gene decision and variant name belong to {}. Start a new case for {}? This clears them.",
                self.case_gene, symbol, self.case_gene, symbol
            ),
            Confirm::RemoveRecord(_, label) => {
                format!("Remove {label} and every criterion it reported? This cannot be undone.")
            }
        };
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
                .bg(hsla(0., 0., 0., 0.45))
                .child(card)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose: `use super::*` would also bring in GPUI's own
    // `#[test]` macro (through `gpui_kit::*`), which replaces Rust's.
    use classivar_core::{Applied, Class, Direction, RuleSet, Strength};
    use gpui_kit::{Rgba, rgb};

    use super::{
        CLASSES, EXAMPLE_RECORDS, EXAMPLE_TODAY, class_fill, class_ink, direction_colour, evidence_of, normalize_today,
        scale_for, suggested_file_name, to_applied, warning_fill, warning_ink,
    };

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    /// WCAG relative luminance of a colour.
    fn luminance(c: Rgba) -> f32 {
        let linear = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }

    fn contrast(a: Rgba, b: Rgba) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn class_colours_reach_wcag_aa_contrast_for_their_text() {
        for dark in [false, true] {
            for class in CLASSES {
                let ratio = contrast(class_fill(class, dark), class_ink(class, dark));
                assert!(ratio >= 4.5, "{class:?} (dark={dark}): {ratio:.2}:1");
            }
        }
        for direction in [Direction::Pathogenic, Direction::Benign] {
            let ratio = contrast(direction_colour(direction), rgb(0xffffff));
            assert!(ratio >= 4.5, "{direction:?} chip text: {ratio:.2}:1");
        }
        let ratio = contrast(warning_fill(), warning_ink());
        assert!(ratio >= 4.5, "warning text: {ratio:.2}:1");
    }

    #[test]
    fn the_scale_follows_the_rule_set() {
        let scale = scale_for(rules());
        assert_eq!((scale.min, scale.max), (-8, 12));
        let expected = vec![
            (Class::Benign, -8, -6),
            (Class::LikelyBenign, -6, 0),
            (Class::Vus, 0, 6),
            (Class::LikelyPathogenic, 6, 10),
            (Class::Pathogenic, 10, 12),
        ];
        assert_eq!(scale.zones, expected);
        // Every point from the bottom to the top of the scale lies in the zone of the class the rules give it.
        for points in scale.min..scale.max {
            let zone = scale.zones.iter().find(|(_, from, to)| (*from..*to).contains(&points)).unwrap();
            assert_eq!(zone.0, rules().class_for_points(points), "{points} points");
        }
    }

    #[test]
    fn the_scale_moves_when_the_rule_set_changes() {
        let text =
            include_str!("../../../rules/acmg-amp-points.toml").replacen("pathogenic = 10", "pathogenic = 12", 1);
        let changed = RuleSet::parse(&text).unwrap();
        assert_eq!(scale_for(&changed).max, 14);
        assert!(scale_for(&changed).zones.contains(&(Class::LikelyPathogenic, 6, 12)));
    }

    #[test]
    fn old_evidence_is_brought_onto_todays_rules_not_rejected() {
        let rules = rules();
        let (today, changed) = normalize_today(
            rules,
            &[Applied::new("PP3", Strength::VeryStrong), Applied::new("PVS1", Strength::VeryStrong)],
        );
        let today: std::collections::BTreeMap<_, _> = today.into_iter().collect();
        assert_eq!(today["PP3"].strength, Strength::Strong, "today's cap lowers the saved strength");
        assert!(today["PVS1"].situation.is_some(), "the guide answer is reconstructed");
        assert_eq!(changed.len(), 1);
        assert!(changed[0].starts_with("PP3 to Strong"));
    }

    #[test]
    fn an_unknown_criterion_is_dropped_not_carried_into_an_unscorable_state() {
        let (today, changed) = normalize_today(rules(), &[Applied::new("ZZ9", Strength::Strong)]);
        assert!(today.is_empty());
        assert!(changed[0].contains("ZZ9"));
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(suggested_file_name("NM_007294.4:c.5266dup"), "NM_007294.4_c.5266dup.classivar.json");
        assert_eq!(suggested_file_name("  "), "case.classivar.json");
        assert_eq!(suggested_file_name("a/b\\c:d*e"), "a_b_c_d_e.classivar.json");
        assert!(suggested_file_name(&"x".repeat(200)).len() < 60);
        // Windows device names cannot be used as file names, even with an extension.
        assert_eq!(suggested_file_name("CON"), "_CON.classivar.json");
        assert_eq!(suggested_file_name("com1"), "_com1.classivar.json");
        assert_eq!(suggested_file_name("COM10"), "COM10.classivar.json");
    }

    #[test]
    fn the_fictional_example_only_uses_criteria_the_rules_accept() {
        for (_, _, _, list) in EXAMPLE_RECORDS {
            assert!(rules().assess(&to_applied(&evidence_of(list))).is_ok());
        }
        assert!(rules().assess(&to_applied(&evidence_of(EXAMPLE_TODAY))).is_ok());
    }
}
