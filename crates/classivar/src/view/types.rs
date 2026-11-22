//! The plain types the workbench is built from: one applied criterion, the
//! evidence of a record, the screen's mode, a question waiting for an answer,
//! and where a lookup stands.

use super::*;

/// The largest a case file may be before it is refused without reading it.
pub(super) const MAX_CASE_BYTES: u64 = 4 * 1024 * 1024;
/// What a record is called when its name box is empty.
pub(super) const UNNAMED_RECORD: &str = "Unnamed record";

/// One criterion as the curator applied it: the strength, and the guide answer
/// that set it when a guide was used. The answer is saved with the case so the
/// reason for a strength is not lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Choice {
    pub(super) strength: Strength,
    pub(super) situation: Option<String>,
    /// The gene-level answer that lowered the guide's strength (PVS1), when asked.
    pub(super) gene_level: Option<String>,
    /// Why the curator chose this strength (typed by them, or filled in by a helper).
    pub(super) reason: Option<String>,
}

impl Choice {
    pub(super) fn new(strength: Strength) -> Self {
        Self { strength, situation: None, gene_level: None, reason: None }
    }

    pub(super) fn guided(strength: Strength, situation: String, gene_level: Option<String>) -> Self {
        Self { strength, situation: Some(situation), gene_level, reason: None }
    }

    pub(super) fn with_reason(strength: Strength, reason: String) -> Self {
        Self { strength, situation: None, gene_level: None, reason: Some(bounded_reason(reason)) }
    }
}

/// A reason cut to the length the engine accepts, with a marker when something
/// was left out. A reason longer than the limit makes the whole evidence
/// unscorable, and the helpers build their reasons out of rule-file text,
/// which has no length limit of its own.
pub(super) fn bounded_reason(reason: String) -> String {
    if reason.chars().count() <= MAX_REASON_CHARS {
        return reason;
    }
    let kept: String = reason.chars().take(MAX_REASON_CHARS - 3).collect();
    format!("{kept}...")
}

/// A strength the curator is raising above its default, and has not yet explained.
pub(super) struct Raise {
    pub(super) code: String,
    pub(super) to: Strength,
    /// True for a note on the evidence behind a criterion already applied: the
    /// strength and any guide answers stay as they are, only the text is saved.
    pub(super) note: bool,
}

pub(super) type Evidence = BTreeMap<String, Choice>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    NewVariant,
    ReportedVariant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Editing {
    Today,
    Record(usize),
}

/// What an earlier record (for example a lab in ClinVar) said and used.
pub(super) struct Record {
    pub(super) label: String,
    pub(super) year: u16,
    /// None until the curator says what the record reported.
    pub(super) class: Option<Class>,
    /// The VUS sub-tier the record named, if any (only meaningful when `class` is a VUS).
    pub(super) tier: Option<String>,
    pub(super) evidence: Evidence,
    /// The condition the record classified the variant for, if known.
    pub(super) condition: Option<Condition>,
    /// Where the record came from, when it was imported from ClinVar.
    pub(super) source: Option<RecordSource>,
}

/// A step that throws work away. It waits for a click on "Yes" first.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum Confirm {
    Clear(Editing),
    /// Forget the whole case: variant, gene, condition, records and evidence.
    ClearCase,
    StartFromLatest,
    /// Check another gene, which starts a new case.
    NewGene(String),
    /// Remove an earlier record and its evidence.
    RemoveRecord(usize, String),
    /// Open a case file over the work on screen.
    OpenCase(Box<CaseFile>),
    /// Close the window although the case changed since it was saved.
    CloseWindow,
}

/// Where one lookup stands.
pub(super) enum Lookup<T> {
    Idle,
    Loading,
    Done(Fetched<T>),
    /// The lookup did not work. This is never shown as "nothing found".
    Failed(String),
}

/// What the four gene sources said about the gene the curator asked about.
pub(super) struct GeneChecks {
    pub(super) england: Lookup<PanelAppGene>,
    pub(super) australia: Lookup<PanelAppGene>,
    pub(super) g2p: Lookup<G2pGene>,
    pub(super) clingen: Lookup<ClinGenGene>,
}

impl GeneChecks {
    pub(super) fn idle() -> Self {
        Self { england: Lookup::Idle, australia: Lookup::Idle, g2p: Lookup::Idle, clingen: Lookup::Idle }
    }
}

/// The text boxes of the case form and the reason prompt.
pub(super) struct Boxes {
    /// The result title and the notes read the variant box, so typing there
    /// repaints the workbench, not only the input.
    pub(super) variant: Entity<InputState>,
    pub(super) gene: Entity<InputState>,
    /// The name box for the earlier record being edited (the lab or source).
    pub(super) record_name: Entity<InputState>,
    /// The box where the reason for a raised strength is typed.
    pub(super) reason: Entity<InputState>,
}

impl Boxes {
    pub(super) fn new(subscriptions: &mut Vec<Subscription>, window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        let variant = new_box("HGVS, VCF or rsID", subscriptions, window, cx);
        let reason = new_box("Why is this stronger than the default?", subscriptions, window, cx);
        let gene = cx.new(|cx| InputState::new(window, cx).placeholder("e.g. BRCA1"));
        subscriptions.push(watch_gene_box(&gene, window, cx));
        let record_name = cx.new(|cx| InputState::new(window, cx).placeholder("Lab or source, e.g. Lab A (ClinVar)"));
        subscriptions.push(watch_record_name(&record_name, window, cx));
        Self { variant, gene, record_name, reason }
    }
}

/// Scroll and focus handles the keyboard paths use.
pub(super) struct Handles {
    /// Scroll state of the criteria list and of the result pane, so the keyboard can scroll them.
    pub(super) criteria_scroll: ScrollHandle,
    pub(super) result_scroll: ScrollHandle,
    /// The confirmation window holds the keyboard focus while it is open, so a
    /// stray Tab or Enter cannot reach the controls behind the question.
    pub(super) confirm_focus: FocusHandle,
    /// Holds the focus when no control does, so Tab always has a place to start from.
    pub(super) page_focus: FocusHandle,
}

impl Handles {
    pub(super) fn new(cx: &mut Context<Workbench>) -> Self {
        Self {
            criteria_scroll: ScrollHandle::new(),
            result_scroll: ScrollHandle::new(),
            confirm_focus: cx.focus_handle(),
            page_focus: cx.focus_handle(),
        }
    }
}

/// What the picture remembers between draws.
pub(super) struct StageMemory {
    /// The total the track marker last slid from and to, so a new total animates from where the marker was.
    pub(super) needle: Cell<(i32, i32)>,
    /// Where the dial was last painted, so a click can find the slice under the pointer.
    pub(super) dial_frame: Rc<Cell<Option<Point<Pixels>>>>,
    /// The width of the middle panel at the last layout, so the dial can size itself.
    pub(super) width: Cell<f32>,
    /// The height the dial may use at the last layout, so the class below it stays in view.
    pub(super) height: Cell<f32>,
    /// Whether the window is wide enough for the reports rail at the left.
    pub(super) rail_shown: Cell<bool>,
    /// The rail's width at the last layout.
    pub(super) rail_width: Cell<f32>,
}

impl StageMemory {
    pub(super) fn new() -> Self {
        Self {
            needle: Cell::new((0, 0)),
            dial_frame: Rc::new(Cell::new(None)),
            width: Cell::new(640.),
            height: Cell::new(330.),
            rail_shown: Cell::new(false),
            rail_width: Cell::new(236.),
        }
    }
}
