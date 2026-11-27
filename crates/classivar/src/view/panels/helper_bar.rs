//! The evidence helpers: what each one is called, which criteria it sets, and how
//! it opens and draws. Each helper reads its criteria from the core crate's shared
//! `HELPER_CRITERIA` table (references like BA1 exceptions name their own code),
//! shows as a card in the tab of the criteria it sets, and opens on its own page
//! over the list (`helper_cards.rs`).

use classivar_core::rules::helper_criteria;

use super::*;

/// The criteria an evidence helper owns, read from the core crate's shared table.
fn owned_codes(key: &'static str) -> &'static [&'static str] {
    helper_criteria(key).unwrap_or_else(|| panic!("helper '{key}' is missing from HELPER_CRITERIA"))
}

/// One helper: how it is named on screen and how it opens, closes and draws.
pub(super) struct Entry {
    /// Stem of the ids: `{key}-toggle` opens it, `{key}-close` closes it.
    pub key: &'static str,
    /// The name on its card, for example "Population frequency".
    pub label: &'static str,
    /// The title of its page, with the criteria it sets.
    pub title: &'static str,
    /// Used in the accessible names: "Open the {what}".
    pub what: &'static str,
    /// One sentence on what it reads and what it suggests.
    pub about: &'static str,
    /// The criteria it fills in. They decide the tabs its card shows in.
    pub codes: &'static [&'static str],
    pub open: bool,
    pub toggle: fn(&mut Workbench),
    pub panel: fn(&Workbench, &mut Context<Workbench>) -> AnyElement,
}

/// The words of one helper: key, card label, page title, accessible name and its sentence.
type Words = (&'static str, &'static str, &'static str, &'static str, &'static str);

fn entry(
    words: Words,
    codes: &'static [&'static str],
    open: bool,
    toggle: fn(&mut Workbench),
    panel: fn(&Workbench, &mut Context<Workbench>) -> AnyElement,
) -> Entry {
    let (key, label, title, what, about) = words;
    Entry { key, label, title, what, about, codes, open, toggle, panel }
}

/// An evidence helper whose criteria come from `classivar_core::rules::HELPER_CRITERIA`.
fn helper_entry(
    words: Words,
    open: bool,
    toggle: fn(&mut Workbench),
    panel: fn(&Workbench, &mut Context<Workbench>) -> AnyElement,
) -> Entry {
    entry(words, owned_codes(words.0), open, toggle, panel)
}

impl Workbench {
    pub(super) fn toggle_predictor(&mut self) {
        self.helpers.predictor.open = !self.helpers.predictor.open;
        if !self.helpers.predictor.open {
            self.helpers.predictor.eligible = false;
        }
    }

    /// The helpers that read a measurement: population frequency, predictor score and functional assay.
    fn measurement_entries(&self) -> [(bool, Entry); 3] {
        let (rules, helpers, today) = (self.rules, &self.helpers, self.editing == Editing::Today);
        [
            (
                today && rules.population.is_some(),
                helper_entry(
                    (
                        "population",
                        "Population frequency",
                        "Population frequency → BA1, BS1 or PM2",
                        "population frequency helper",
                        "Compares how common the variant is in population data with the limits in the rules, and suggests BA1, BS1 or PM2.",
                    ),
                    helpers.population.open,
                    |this| this.helpers.population.open = !this.helpers.population.open,
                    Self::population_helper,
                ),
            ),
            (
                today && !rules.predictors.is_empty(),
                helper_entry(
                    (
                        "predictor",
                        "Predictor score",
                        "Predictor score → PP3 or BP4",
                        "predictor score helper",
                        "Reads a computational predictor's score against its calibrated limits, and suggests PP3 or BP4 at a strength.",
                    ),
                    helpers.predictor.open,
                    Self::toggle_predictor,
                    Self::predictor_helper,
                ),
            ),
            (
                today && rules.functional.is_some(),
                helper_entry(
                    (
                        "functional",
                        "Functional assay",
                        "Functional assay → PS3 or BS3",
                        "functional evidence helper",
                        "Reads a lab assay's controls or its OddsPath value, and suggests PS3 (damaging) or BS3 (no damage) at a strength.",
                    ),
                    helpers.functional.open,
                    |this| this.helpers.functional.open = !this.helpers.functional.open,
                    Self::functional_helper,
                ),
            ),
        ]
    }

    /// The helpers about families and patients, and the dated BA1 list (which also shows for earlier records).
    fn patient_entries(&self) -> [(bool, Entry); 5] {
        let (rules, helpers, today) = (self.rules, &self.helpers, self.editing == Editing::Today);
        [
            (
                !rules.ba1_exceptions.is_empty(),
                entry(
                    (
                        "ba1-reference",
                        "BA1 exceptions",
                        "BA1 exceptions (July 2018)",
                        "BA1 exceptions reference",
                        "The dated list of variants that are common in population data but are not benign, so BA1 must not be used for them.",
                    ),
                    &["BA1"],
                    helpers.ba1_open,
                    |this| this.helpers.ba1_open = !this.helpers.ba1_open,
                    Self::ba1_reference,
                ),
            ),
            (
                today && rules.locus_evidence.is_some(),
                helper_entry(
                    (
                        "locus",
                        "Family and phenotype",
                        "Family and phenotype → PP1 and PP4",
                        "co-segregation and phenotype helper",
                        "Adds up the points for how the variant follows the disease in families and how specific the patients' phenotype is, and lists the PP1 and PP4 strengths they allow.",
                    ),
                    helpers.locus.open,
                    |this| this.helpers.locus.open = !this.helpers.locus.open,
                    Self::locus_helper,
                ),
            ),
            (
                today && rules.non_segregation.is_some(),
                helper_entry(
                    (
                        "segregation",
                        "Non-segregation",
                        "Non-segregation → BS4",
                        "non-segregation helper",
                        "Asks how the variant fails to follow the disease in a family, and offers BS4 only where the published rule supports it.",
                    ),
                    helpers.segregation.open,
                    |this| this.helpers.segregation.open = !this.helpers.segregation.open,
                    Self::non_segregation_helper,
                ),
            ),
            (
                today && rules.pm3.is_some(),
                helper_entry(
                    (
                        "pm3",
                        "PM3 patients",
                        "Recessive patients → PM3",
                        "PM3 patients helper",
                        "For a recessive disease: adds up the points of patients who carry this variant with a second one, and suggests a PM3 strength.",
                    ),
                    helpers.pm3.open,
                    |this| this.helpers.pm3.open = !this.helpers.pm3.open,
                    Self::pm3_helper,
                ),
            ),
            (
                today && rules.de_novo.is_some(),
                helper_entry(
                    (
                        "de-novo",
                        "De novo patients",
                        "De novo patients → PS2 or PM6",
                        "de novo helper",
                        "Gives each patient in whom the variant arose new a number of points, and suggests PS2 (parents confirmed) or PM6 (not confirmed).",
                    ),
                    helpers.de_novo.open,
                    |this| this.helpers.de_novo.open = !this.helpers.de_novo.open,
                    Self::de_novo_helper,
                ),
            ),
        ]
    }

    /// Every helper, whether or not it applies to what is being edited.
    fn all_helper_entries(&self) -> Vec<(bool, Entry)> {
        self.measurement_entries().into_iter().chain(self.patient_entries()).collect()
    }

    /// Each helper's key and criteria list, for tests checking parity with `HELPER_CRITERIA`.
    #[cfg(test)]
    pub(in crate::view) fn helper_entry_codes(&self) -> Vec<(&'static str, &'static [&'static str])> {
        self.all_helper_entries().into_iter().map(|(_, entry)| (entry.key, entry.codes)).collect()
    }

    /// Every helper that applies to what is being edited, population first: population evidence is read first.
    pub(super) fn helper_entries(&self) -> Vec<Entry> {
        self.all_helper_entries().into_iter().filter_map(|(applies, entry)| applies.then_some(entry)).collect()
    }

    /// Open one helper on its page and close any other, so one page fills the panel.
    pub(in crate::view) fn open_helper(&mut self, key: &str) {
        for entry in self.helper_entries() {
            if entry.open != (entry.key == key) {
                (entry.toggle)(self);
            }
        }
    }

    /// Close every open helper, including those that do not apply to what is on screen now,
    /// so none stays open out of sight. Returns whether one was open.
    pub(in crate::view) fn close_helpers(&mut self) -> bool {
        let open: Vec<fn(&mut Workbench)> = self
            .all_helper_entries()
            .into_iter()
            .filter(|(_, entry)| entry.open)
            .map(|(_, entry)| entry.toggle)
            .collect();
        for toggle in &open {
            toggle(self);
        }
        !open.is_empty()
    }
}
