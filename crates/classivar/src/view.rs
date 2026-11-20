//! The main screen. Two paths:
//! - New variant: tick the criteria you apply; see points, class and why.
//! - Reported variant: enter what earlier records used, apply today's evidence,
//!   and see what changed and whether the class holds.
//!
//! The curator decides every criterion; the screen only adds up and explains.

use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use classivar_core::{
    Applied, Assessment, CaseFile, Category, Class, Consistency, Criterion, Direction, GeneDecision, MAX_LAB_CHARS,
    MAX_REASON_CHARS, MAX_RECORDS, MAX_VARIANT_CHARS, Reason, Reevaluation, RuleSet, SavedRecord, Strength, Submission,
    Verdict, Warning, rules::ReviewStatus,
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
    button::{Button, ButtonCustomVariant, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::design::{
    CLASSES, caption, class_fill, class_ink, confirm_fill, confirm_ink, direction_colour, direction_ink, scrim,
    warning_fill, warning_ink,
};
use crate::status::{Status, Tone};

mod actions;
mod case_io;
mod confirm;
mod describe;
mod evidence;
mod example;
mod file_io;
mod gene;
#[cfg(test)]
mod headless;
mod helper_state;

use helper_state::{AssayRoute, Helpers, PhenotypeGenes, new_box};
mod panels;
mod records;
mod watchers;
mod widgets;

use describe::{
    CARD_LINES, describe_clingen, describe_g2p, describe_panelapp, discouraged_row_note, plural, rules_about_text,
    rules_tag, scale_for, tiers_are_ranged,
};
use evidence::{carry_forward, evidence_of, open_notes, to_applied};
use file_io::{default_save_folder, suggested_file_name, write_case};
use watchers::{repaint_on_change, watch_gene_box, watch_keys, watch_record_name};
use widgets::{class_badge, notice};

/// Width of one point in the evidence lanes.
const POINT_WIDTH: f32 = 36.0;
/// Width of one point on the points scale. Chosen so the whole scale fits the
/// result pane at the 960 px minimum window width.
const SCALE_UNIT: f32 = 20.0;

/// The largest a case file may be before it is refused without reading it.
const MAX_CASE_BYTES: u64 = 4 * 1024 * 1024;
/// What a record is called when its name box is empty.
const UNNAMED_RECORD: &str = "Unnamed record";

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
        Self { strength, situation: None, reason: Some(bounded_reason(reason)) }
    }
}

/// A reason cut to the length the engine accepts, with a marker when something
/// was left out. A reason longer than the limit makes the whole evidence
/// unscorable, and the helpers build their reasons out of rule-file text,
/// which has no length limit of its own.
fn bounded_reason(reason: String) -> String {
    if reason.chars().count() <= MAX_REASON_CHARS {
        return reason;
    }
    let kept: String = reason.chars().take(MAX_REASON_CHARS - 3).collect();
    format!("{kept}...")
}

/// A strength the curator is raising above its default, and has not yet explained.
struct Raise {
    code: String,
    to: Strength,
}

type Evidence = BTreeMap<String, Choice>;

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

pub struct Workbench {
    rules: &'static RuleSet,
    mode: Mode,
    /// The curator's own evidence, today.
    today: Evidence,
    records: Vec<Record>,
    /// Which evidence set the criteria list is editing.
    editing: Editing,
    /// When on, every criterion shows its plain-language rule and source.
    category: Option<Category>,
    show_helpers: bool,
    welcome: bool,
    learning: bool,
    variant_input: Entity<InputState>,
    gene_input: Entity<InputState>,
    /// The name box for the earlier record being edited (the lab or source).
    record_name_input: Entity<InputState>,
    /// Scroll state of the criteria list and of the result pane, so the keyboard can scroll them.
    criteria_scroll: ScrollHandle,
    reading: bool,
    baseline: Option<usize>,
    comparison: bool,
    inspected: Option<String>,
    preview: bool,
    /// The total the track marker last slid from and to, so a new total animates from where the marker was.
    needle: Cell<(i32, i32)>,
    result_scroll: ScrollHandle,
    /// The picture that explains the class: a dial, a column or a track.
    picture: crate::settings::Picture,
    /// Where the dial was last painted, so a click can find the slice under the pointer.
    dial_frame: Rc<Cell<Option<Point<Pixels>>>>,
    /// The width of the middle panel at the last layout, so the dial can size itself.
    stage_width: Cell<f32>,
    /// The height the dial may use at the last layout, so the class below it stays in view.
    stage_height: Cell<f32>,
    /// Whether the window is wide enough for the reports rail at the left.
    rail_shown: Cell<bool>,
    /// A strength being raised by hand, waiting for its reason.
    raising: Option<Raise>,
    /// The box where the reason for a raised strength is typed.
    reason_input: Entity<InputState>,
    /// The evidence helpers: which are open, and what is typed in them.
    helpers: Helpers,
    gene: GeneChecks,
    /// Why the gene box was not checked (empty or not a symbol), shown under the row.
    gene_error: Option<String>,
    /// The gene this case is about: the last one checked.
    case_gene: String,
    /// True while the fictional example is on screen. A gene checked on it
    /// would start a real case, so the curator is asked first (see `check_gene`).
    example: bool,
    /// Whether the four source cards are shown. They are tall, so the curator can hide them.
    gene_cards_open: bool,
    /// Cards that show all their lines instead of the first few.
    expanded_cards: HashSet<&'static str>,
    /// Criteria whose full note and source are open.
    open_details: HashSet<String>,
    /// The criterion whose strength guide is waiting for an answer.
    guide_open: Option<String>,
    gene_link: GeneDecision,
    /// Confirm modal state: `None` when no question is waiting.
    confirm: Option<Confirm>,
    /// The confirmation window holds the keyboard focus while it is open, so a
    /// stray Tab or Enter cannot reach the controls behind the question.
    confirm_focus: FocusHandle,
    /// The result of the last save or open, shown until the curator dismisses it.
    file_status: Option<Status>,
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
        let mut subscriptions = Vec::new();
        // The result title and the notes read the variant box, so typing there
        // must repaint the workbench, not only the input.
        let variant_input = new_box("Variant, e.g. NM_007294.4:c.5266dup", &mut subscriptions, window, cx);
        let reason_input = new_box("Why is this stronger than the default?", &mut subscriptions, window, cx);
        let gene_input = cx.new(|cx| InputState::new(window, cx).placeholder("Gene symbol, e.g. BRCA1"));
        subscriptions.push(watch_gene_box(&gene_input, window, cx));
        let record_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Lab or source, e.g. Lab A (ClinVar)"));
        subscriptions.push(watch_record_name(&record_name_input, window, cx));
        subscriptions.push(watch_keys(cx));
        let helpers = Helpers::new(rules, &mut subscriptions, window, cx);
        Self {
            rules,
            mode: Mode::NewVariant,
            today: Evidence::new(),
            records: Vec::new(),
            editing: Editing::Today,
            category: None,
            show_helpers: false,
            welcome: true,
            learning: false,
            variant_input,
            gene_input,
            record_name_input,
            criteria_scroll: ScrollHandle::new(),
            reading: false,
            baseline: None,
            comparison: false,
            inspected: None,
            preview: false,
            needle: Cell::new((0, 0)),
            result_scroll: ScrollHandle::new(),
            picture: crate::settings::load_picture(),
            dial_frame: Rc::new(Cell::new(None)),
            stage_width: Cell::new(640.),
            stage_height: Cell::new(330.),
            rail_shown: Cell::new(false),
            raising: None,
            reason_input,
            helpers,
            gene: GeneChecks::idle(),
            gene_error: None,
            case_gene: String::new(),
            example: false,
            gene_cards_open: false,
            expanded_cards: HashSet::new(),
            open_details: HashSet::new(),
            guide_open: None,
            gene_link: GeneDecision::NotDecided,
            confirm: None,
            confirm_focus: cx.focus_handle(),
            file_status: None,
            next_record: 1,
            lookup_id: 0,
            _lookup_tasks: Vec::new(),
            _file_task: None,
            _subscriptions: subscriptions,
        }
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose: `use super::*` would also bring in GPUI's own
    // `#[test]` macro (through `gpui_kit::*`), which replaces Rust's.
    use classivar_core::{Applied, RuleSet, Strength};

    use super::{Choice, MAX_REASON_CHARS};

    #[test]
    fn a_reason_the_helpers_build_is_always_short_enough_for_the_engine() {
        // The helpers build their reasons out of rule-file text, which has no
        // length limit of its own. A reason over the limit makes the whole
        // evidence unscorable, so it is cut here.
        let choice = Choice::with_reason(Strength::Supporting, "x".repeat(2000));
        let reason = choice.reason.expect("the choice carries a reason");
        assert!(reason.chars().count() <= MAX_REASON_CHARS, "{}", reason.chars().count());
        assert!(reason.ends_with("..."), "the cut is marked");
        // The engine must accept what the screen builds.
        let applied =
            Applied { code: "PM2".into(), strength: Strength::Supporting, situation: None, reason: Some(reason) };
        let rules = RuleSet::builtin().expect("the built-in rules are valid");
        assert!(rules.assess(&[applied]).is_ok());
    }

    #[test]
    fn a_reason_within_the_limit_is_kept_whole() {
        let reason = "a founder population".to_string();
        assert_eq!(Choice::with_reason(Strength::Supporting, reason.clone()).reason, Some(reason));
    }
}
