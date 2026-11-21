//! What the screen knows about the case's variant and condition: the last
//! check against the ClinGen Allele Registry, the identity the case is about,
//! the condition, and a ClinVar import in progress.

use classivar_core::variant::{Build, VariantIdentity};
use classivar_core::{Condition, Inheritance};
use classivar_sources::allele_registry;
use classivar_sources::clinvar::ClinVarVariation;

use super::*;

pub(super) struct VariantState {
    /// The box text the last check was for, so the screen can say when it changed since.
    pub(super) checked_text: Option<String>,
    /// The build the curator chose for a position that names none. Never guessed.
    pub(super) build: Option<Build>,
    /// Why the box text cannot be checked as it is, in words.
    pub(super) problem: Option<String>,
    pub(super) lookup: Lookup<allele_registry::Lookup>,
    /// The identity this case is about. Saved with the case.
    pub(super) identity: Option<VariantIdentity>,
    /// The condition this case classifies the variant for. Saved with the case.
    pub(super) condition: Option<Condition>,
    pub(super) condition_input: Entity<InputState>,
    /// The condition box of the earlier record being edited.
    pub(super) record_condition_input: Entity<InputState>,
    pub(super) clinvar: Lookup<Option<ClinVarVariation>>,
    /// Counts checks, so a slow old answer cannot replace a newer one.
    pub(super) lookup_id: u64,
    pub(super) _tasks: Vec<Task<()>>,
}

impl VariantState {
    pub(super) fn new(subscriptions: &mut Vec<Subscription>, window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        let condition_input = cx.new(|cx| InputState::new(window, cx).placeholder("Condition, e.g. cystic fibrosis"));
        subscriptions.push(super::variant::watch_condition_box(&condition_input, window, cx));
        let record_condition_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Condition the lab named, if any"));
        subscriptions.push(super::variant::watch_record_condition_box(&record_condition_input, window, cx));
        Self {
            checked_text: None,
            build: None,
            problem: None,
            lookup: Lookup::Idle,
            identity: None,
            condition: None,
            condition_input,
            record_condition_input,
            clinvar: Lookup::Idle,
            lookup_id: 0,
            _tasks: Vec::new(),
        }
    }

    /// The inheritance the case names, or Unknown.
    pub(super) fn inheritance(&self) -> Inheritance {
        self.condition.as_ref().map_or(Inheritance::Unknown, |c| c.inheritance)
    }
}
