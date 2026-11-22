//! Checking the variant, choosing the condition, and importing ClinVar records.

use classivar_core::variant::{self, Build, Notation, VariantIdentity};
use classivar_core::{Condition, Inheritance, MAX_CONDITION_CHARS};
use classivar_sources::allele_registry::{self, Lookup as Found};
use classivar_sources::clingen::Curation;
use classivar_sources::clinvar;

use super::clinvar_records::{decision_for_validity, import_plan, import_summary};
use super::*;

/// Did this ClinGen curation supply the case's current condition? The same disease (its
/// MONDO id, or its name when ClinGen gave no id) with the same inheritance. The
/// inheritance counts because one disease can have a recessive and a dominant curation.
pub(super) fn curation_is_chosen(condition: Option<&Condition>, curation: &Curation) -> bool {
    let Some(condition) = condition else { return false };
    let same_disease = if curation.disease_id.is_empty() {
        let name: String = curation.disease.chars().take(MAX_CONDITION_CHARS).collect();
        condition.mondo.is_none() && condition.name == name
    } else {
        condition.mondo.as_deref() == Some(curation.disease_id.as_str())
    };
    same_disease && condition.inheritance == Inheritance::from_clingen(&curation.inheritance)
}

/// Typing a condition by hand keeps its MONDO id only while the name is the one it came with.
pub(super) fn watch_condition_box(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) -> Subscription {
    cx.subscribe_in(input, window, |this, input, event: &InputEvent, _window, cx| {
        if !matches!(event, InputEvent::Change) {
            return;
        }
        let name: String = input.read(cx).value().trim().chars().take(MAX_CONDITION_CHARS).collect();
        let inheritance = this.variant.inheritance();
        this.variant.condition = match this.variant.condition.take() {
            _ if name.is_empty() && inheritance == Inheritance::Unknown => None,
            Some(kept) if kept.name == name => Some(kept),
            _ => Some(Condition { name, mondo: None, inheritance }),
        };
        cx.notify();
    })
}

/// Typing in the record's condition box names the condition the open record classified for.
pub(super) fn watch_record_condition_box(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) -> Subscription {
    cx.subscribe_in(input, window, |this, input, event: &InputEvent, _window, cx| {
        let (InputEvent::Change, Editing::Record(i)) = (event, this.editing) else { return };
        let name: String = input.read(cx).value().trim().chars().take(MAX_CONDITION_CHARS).collect();
        if let Some(record) = this.records.get_mut(i) {
            record.condition = match record.condition.take() {
                _ if name.is_empty() => None,
                Some(kept) if kept.name == name => Some(kept),
                _ => Some(Condition { name, mondo: None, inheritance: Inheritance::Unknown }),
            };
            cx.notify();
        }
    })
}

impl Workbench {
    /// Check the variant box with the ClinGen Allele Registry. A description that
    /// cannot be read, or a position without a build, is answered here, in words.
    pub(super) fn check_variant(&mut self, cx: &mut Context<Self>) {
        let typed = self.variant_name(cx).unwrap_or_default();
        let query = variant::parse(&typed)
            .map_err(|problem| problem.message())
            .and_then(|notation| notation.query(self.variant.build).map_err(|p| p.message().to_string()));
        match query {
            Ok(query) => self.start_registry_lookup(typed, query, cx),
            Err(problem) => {
                self.variant.problem = Some(problem);
                self.variant.lookup = Lookup::Idle;
            }
        }
        cx.notify();
    }

    /// Does the box hold a position that needs the curator to choose a build?
    pub(super) fn variant_needs_build(&self, cx: &App) -> bool {
        self.variant_name(cx).and_then(|typed| variant::parse(&typed).ok()).is_some_and(|n: Notation| n.needs_build())
    }

    pub(super) fn choose_build(&mut self, build: Build, cx: &mut Context<Self>) {
        self.variant.build = Some(build);
        self.check_variant(cx);
    }

    fn start_registry_lookup(&mut self, typed: String, query: variant::RegistryQuery, cx: &mut Context<Self>) {
        self.variant.problem = None;
        self.variant.checked_text = Some(typed);
        self.variant.lookup = Lookup::Loading;
        self.variant.lookup_id += 1;
        let id = self.variant.lookup_id;
        let pending = cx.background_spawn(async move { allele_registry::lookup(&query) });
        self.variant._tasks = vec![cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |this, cx| {
                if this.variant.lookup_id == id {
                    this.finish_registry_lookup(result);
                    cx.notify();
                }
            })
            .ok();
        })];
    }

    pub(super) fn finish_registry_lookup(&mut self, result: Result<Fetched<Found>, SourceError>) {
        self.variant.lookup = match result {
            Ok(fetched) => {
                // One allele is the answer; several (a multi-allelic rsID) wait for the curator's choice.
                if let Found::Found(list) = &fetched.value
                    && let [only] = list.as_slice()
                {
                    self.variant.identity = Some(only.clone());
                }
                Lookup::Done(fetched)
            }
            Err(error) => Lookup::Failed(error.to_string()),
        };
    }

    /// Take one of several alleles an id named as the case's variant.
    pub(super) fn choose_identity(&mut self, identity: VariantIdentity, cx: &mut Context<Self>) {
        self.variant.identity = Some(identity);
        cx.notify();
    }

    /// Use a ClinGen gene-disease curation as the case's condition, and suggest
    /// the gene decision its rating implies. The banner says what was set.
    pub(super) fn use_curation(&mut self, curation: &Curation, window: &mut Window, cx: &mut Context<Self>) {
        let condition = Condition {
            name: curation.disease.chars().take(MAX_CONDITION_CHARS).collect(),
            mondo: (!curation.disease_id.is_empty()).then(|| curation.disease_id.clone()),
            inheritance: Inheritance::from_clingen(&curation.inheritance),
        };
        let name = condition.name.clone();
        self.variant.condition = Some(condition);
        self.variant.condition_input.update(cx, |input, cx| input.set_value(name.clone(), window, cx));
        let decision = decision_for_validity(&curation.classification);
        if decision != GeneDecision::NotDecided {
            self.gene_link = decision;
        }
        self.file_status = Some(Status::attention(format!(
            "Condition set to {name} ({}). ClinGen rates the link {}, so the gene answer is now \"{}\". Change it if you disagree.",
            Inheritance::from_clingen(&curation.inheritance).label(),
            curation.classification,
            match self.gene_link {
                GeneDecision::Established => "Yes",
                GeneDecision::NotEstablished => "Doubtful",
                GeneDecision::NotDecided => "Not decided",
            }
        )));
        cx.notify();
    }

    pub(super) fn set_inheritance(&mut self, inheritance: Inheritance, cx: &mut Context<Self>) {
        let name = self.variant.condition_input.read(cx).value().trim().to_string();
        let kept = self.variant.condition.take();
        self.variant.condition = Some(Condition {
            name: kept.as_ref().map_or(name, |c| c.name.clone()),
            mondo: kept.and_then(|c| c.mondo),
            inheritance,
        });
        cx.notify();
    }

    /// The ClinVar variation the case can import from: the checked identity's.
    pub(super) fn clinvar_variation_id(&self) -> Option<u64> {
        self.variant.identity.as_ref().and_then(|identity| identity.clinvar_variation)
    }

    /// Fetch the variation's submissions from ClinVar, then add them as records.
    pub(super) fn import_clinvar(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.clinvar_variation_id() else { return };
        self.variant.clinvar = Lookup::Loading;
        let pending = cx.background_spawn(async move { clinvar::lookup_variation(id) });
        self.variant._tasks.push(cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |this, cx| this.finish_clinvar_import(result, cx)).ok();
        }));
        cx.notify();
    }

    pub(super) fn finish_clinvar_import(
        &mut self,
        result: Result<Fetched<Option<clinvar::ClinVarVariation>>, SourceError>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(fetched) => {
                if let Some(variation) = &fetched.value {
                    let existing = self.records.iter().filter_map(|r| r.source.as_ref().map(|s| s.accession.clone()));
                    let room = MAX_RECORDS.saturating_sub(self.records.len());
                    let year = classivar_sources::current_year();
                    let plan = import_plan(variation, &existing.collect(), room, &fetched.fetched_on, year);
                    self.file_status = Some(Status::attention(import_summary(&plan, &variation.accession)));
                    self.records.extend(plan.records);
                    self.next_record = Self::next_record_number(&self.records);
                } else {
                    self.file_status = Some(Status::attention("ClinVar says it has no such variation."));
                }
                self.variant.clinvar = Lookup::Done(fetched);
            }
            Err(error) => self.variant.clinvar = Lookup::Failed(error.to_string()),
        }
        cx.notify();
    }

    /// Show the identity and the condition an opened case saved.
    pub(super) fn restore_variant(&mut self, case: &CaseFile, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_variant(window, cx);
        self.variant.identity = case.identity.clone();
        self.variant.checked_text = case.identity.as_ref().map(|_| case.variant.clone());
        self.variant.condition = case.condition.clone();
        let condition = case.condition.as_ref().map(|c| c.name.clone()).unwrap_or_default();
        self.variant.condition_input.update(cx, |input, cx| input.set_value(condition, window, cx));
    }

    /// Forget the variant checks and the condition (a new or cleared case).
    pub(super) fn reset_variant(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.variant.checked_text = None;
        self.variant.build = None;
        self.variant.problem = None;
        self.variant.lookup = Lookup::Idle;
        self.variant.identity = None;
        self.variant.condition = None;
        self.variant.clinvar = Lookup::Idle;
        self.variant.lookup_id += 1;
        self.variant._tasks.clear();
        self.variant.condition_input.update(cx, |input, cx| input.set_value("", window, cx));
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{Condition, Inheritance};
    use classivar_sources::clingen::Curation;

    use super::curation_is_chosen;

    fn curation(disease: &str, id: &str, inheritance: &str) -> Curation {
        Curation {
            disease: disease.into(),
            disease_id: id.into(),
            inheritance: inheritance.into(),
            classification: "Definitive".into(),
            report_url: String::new(),
            classified_on: "2024-01-01".into(),
            expert_panel: String::new(),
        }
    }

    fn set_by(curation: &Curation) -> Condition {
        Condition {
            name: curation.disease.clone(),
            mondo: (!curation.disease_id.is_empty()).then(|| curation.disease_id.clone()),
            inheritance: Inheritance::from_clingen(&curation.inheritance),
        }
    }

    #[test]
    fn only_the_curation_that_set_the_condition_shows_as_chosen() {
        let recessive = curation("RYR1-related myopathy", "MONDO:0100150", "AR");
        let dominant = curation("RYR1-related myopathy", "MONDO:0100150", "AD");
        let condition = set_by(&recessive);
        assert!(curation_is_chosen(Some(&condition), &recessive));
        assert!(!curation_is_chosen(Some(&condition), &dominant), "same disease id, other inheritance");
        assert!(!curation_is_chosen(None, &recessive), "no condition set yet");
    }

    #[test]
    fn a_curation_without_a_disease_id_is_matched_by_its_name() {
        let unnamed_id = curation("Some rare disorder", "", "AD");
        let condition = set_by(&unnamed_id);
        assert!(curation_is_chosen(Some(&condition), &unnamed_id), "the empty id must not hide the choice");
        assert!(!curation_is_chosen(Some(&condition), &curation("Another disorder", "", "AD")));
        let with_id = Condition { mondo: Some("MONDO:1".into()), ..condition };
        assert!(!curation_is_chosen(Some(&with_id), &unnamed_id), "a condition with an id is a different one");
    }
}
