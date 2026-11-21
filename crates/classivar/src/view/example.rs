//! The fictional example case.

use super::*;

/// The fictional variant from the design sketch: (label, year, class, criteria).
pub(super) type ExampleRecord = (&'static str, u16, Class, &'static [(&'static str, Strength)]);

pub(super) const EXAMPLE_RECORDS: [ExampleRecord; 2] = [
    ("Lab A (example)", 2019, Class::Vus, &[("PM2", Strength::Moderate), ("PP3", Strength::Supporting)]),
    (
        "Lab B (example)",
        2022,
        Class::Vus,
        &[("PM2", Strength::Moderate), ("PP3", Strength::Supporting), ("PP4", Strength::Supporting)],
    ),
];

pub(super) const EXAMPLE_TODAY: &[(&str, Strength)] = &[
    ("PM2", Strength::Supporting),
    ("PP3", Strength::Supporting),
    ("PP4", Strength::Supporting),
    ("PM5", Strength::Moderate),
];

impl Workbench {
    /// The fictional variant from the design sketch, so the history view can be tried at once.
    /// Does nothing if the rules no longer accept its criteria (a test also guards this).
    pub(super) fn load_example(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let accepted =
            EXAMPLE_RECORDS.iter().all(|(_, _, _, list)| self.rules.assess(&to_applied(&evidence_of(list))).is_ok())
                && self.rules.assess(&to_applied(&evidence_of(EXAMPLE_TODAY))).is_ok();
        if !accepted {
            self.file_status = Some(Status::attention(
                "The example does not fit the current rule file, so it was not loaded. Load the default rules to try it.",
            ));
            return;
        }
        self.welcome = false;
        self.records = EXAMPLE_RECORDS
            .iter()
            .map(|(label, year, class, list)| Record {
                label: label.to_string(),
                year: *year,
                class: Some(*class),
                tier: None,
                evidence: evidence_of(list),
                condition: None,
                source: None,
            })
            .collect();
        self.today = evidence_of(EXAMPLE_TODAY);
        self.editing = Editing::Today;
        // Flag the case as fictional: a checked gene starts a real case, so the
        // curator is asked first, and Save refuses until the example is cleared.
        self.category = None;
        self.baseline = None;
        self.reading = false;
        self.comparison = false;
        self.inspected = None;
        self.preview = false;
        self.example = true;
        self.forget_saved();
        self.file_status = Some(Status::attention(
            "Fictional case. Explore the evidence and compare its two reports. Clear the example before saving a real case.",
        ));
        self.forget_names(window, cx);
        self.guide_open = None;
        self.confirm = None;
        self.reset_helpers(window, cx);
        self.reset_variant(window, cx);
    }

    /// The example is fictional: it must not inherit the gene or variant that
    /// happened to be on screen, or the class would be attributed to them.
    fn forget_names(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.case_gene.clear();
        self.gene_link = GeneDecision::NotDecided;
        self.gene_error = None;
        self.gene = GeneChecks::idle();
        self.lookup_id += 1;
        self._lookup_tasks.clear();
        self.boxes.variant.update(cx, |input, cx| input.set_value("", window, cx));
        self.boxes.gene.update(cx, |input, cx| input.set_value("", window, cx));
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;

    use super::{EXAMPLE_RECORDS, EXAMPLE_TODAY};
    use crate::view::evidence::{evidence_of, to_applied};

    #[test]
    fn the_fictional_example_only_uses_criteria_the_rules_accept() {
        let rules = RuleSet::builtin().expect("the built-in rules are valid");
        for (_, _, _, list) in EXAMPLE_RECORDS {
            assert!(rules.assess(&to_applied(&evidence_of(list))).is_ok());
        }
        assert!(rules.assess(&to_applied(&evidence_of(EXAMPLE_TODAY))).is_ok());
    }
}
