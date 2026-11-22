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
    Applied, Assessment, CaseFile, Category, Class, Condition, Consistency, Criterion, Direction, GeneDecision,
    MAX_LAB_CHARS, MAX_REASON_CHARS, MAX_RECORDS, MAX_VARIANT_CHARS, Reason, RecordSource, Reevaluation, RuleSet,
    SavedRecord, Strength, Submission, Verdict, Warning, rules::ReviewStatus,
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
mod clinvar_records;
mod confirm;
mod describe;
mod evidence;
mod file_io;
mod gene;
#[cfg(test)]
mod headless;
#[cfg(test)]
mod headless_case;
#[cfg(test)]
mod headless_keys;
mod helper_state;
#[cfg(test)]
mod real_case;

use helper_state::{AssayRoute, Helpers, PhenotypeGenes, new_box};
mod panels;
mod records;
mod rules_window;
mod types;
mod unsaved;
mod variant;
mod variant_state;
mod watchers;
mod widgets;

use types::*;

use variant_state::VariantState;

use describe::{
    CARD_LINES, describe_clingen, describe_g2p, describe_panelapp, discouraged_row_note, plural, rules_about_text,
    rules_tag, scale_for, tiers_are_ranged,
};
use evidence::{carry_forward, open_notes, to_applied};
use file_io::{default_save_folder, suggested_file_name, write_case};
use watchers::{repaint_on_change, watch_close, watch_gene_box, watch_keys, watch_record_name};
use widgets::{class_badge, notice};

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
    /// The text boxes of the case form and the reason prompt.
    boxes: Boxes,
    /// Scroll and focus handles the keyboard paths use.
    handles: Handles,
    /// What the picture remembers between draws.
    stage: StageMemory,
    reading: bool,
    baseline: Option<usize>,
    comparison: bool,
    inspected: Option<String>,
    /// The criterion under the pointer on the dial, the column or the track, for its hover card.
    /// Dropped when the picture, the tab or the mode changes.
    hovered: Option<String>,
    preview: bool,
    /// The picture that explains the class: a dial, a column or a track.
    picture: crate::settings::Picture,
    /// A strength being raised by hand, waiting for its reason.
    raising: Option<Raise>,
    /// The evidence helpers: which are open, and what is typed in them.
    helpers: Helpers,
    gene: GeneChecks,
    /// The variant check, the identity and the condition of the case.
    variant: VariantState,
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
    /// The gene-level answer chosen in the open guide, before its situation is chosen.
    guide_gene_level: Option<String>,
    gene_link: GeneDecision,
    /// Confirm modal state: `None` when no question is waiting.
    confirm: Option<Confirm>,
    /// The result of the last save or open, shown until the curator dismisses it.
    file_status: Option<Status>,
    /// The case as last saved or opened, to tell whether it has changed since (see `unsaved.rs`).
    saved_case: Option<CaseFile>,
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
        let boxes = Boxes::new(&mut subscriptions, window, cx);
        subscriptions.push(watch_keys(cx));
        watch_close(window, cx);
        let helpers = Helpers::new(rules, &mut subscriptions, window, cx);
        let variant = VariantState::new(&mut subscriptions, window, cx);
        Self {
            rules,
            mode: Mode::NewVariant,
            today: Evidence::new(),
            records: Vec::new(),
            editing: Editing::Today,
            category: None,
            show_helpers: false,
            welcome: true,
            boxes,
            handles: Handles::new(cx),
            stage: StageMemory::new(),
            reading: false,
            baseline: None,
            comparison: false,
            inspected: None,
            hovered: None,
            preview: false,
            picture: crate::settings::load_picture(),
            raising: None,
            helpers,
            gene: GeneChecks::idle(),
            variant,
            gene_error: None,
            case_gene: String::new(),
            gene_cards_open: false,
            expanded_cards: HashSet::new(),
            open_details: HashSet::new(),
            guide_open: None,
            guide_gene_level: None,
            gene_link: GeneDecision::NotDecided,
            confirm: None,
            file_status: None,
            saved_case: None,
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
        let applied = Applied::new("PM2", Strength::Supporting).with_reason(reason);
        let rules = RuleSet::builtin().expect("the built-in rules are valid");
        assert!(rules.assess(&[applied]).is_ok());
    }

    #[test]
    fn a_reason_within_the_limit_is_kept_whole() {
        let reason = "a founder population".to_string();
        assert_eq!(Choice::with_reason(Strength::Supporting, reason.clone()).reason, Some(reason));
    }
}
