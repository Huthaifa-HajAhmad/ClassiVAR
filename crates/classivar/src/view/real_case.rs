//! A real, published case for the window tests, in place of a made-up one.
//!
//! RYR1 NM_000540.3:c.7354C>T (ClinGen allele CA024770, ClinVar variation 65979),
//! classified by two ClinGen expert panels. The criteria each panel marked as met
//! are copied from the ClinGen Evidence Repository (accessed 2026-11-22; ClinGen
//! content is CC0, see `crates/classivar-core/tests/data/README.md`):
//!
//! - Malignant Hyperthermia Susceptibility VCEP, published 2023-04-07, Likely
//!   Pathogenic, for MONDO:0007783.
//! - Congenital Myopathies VCEP, published 2026-03-27, Pathogenic, for MONDO:0100150.
//!
//! Today's evidence is the later panel's criteria, for the later panel's condition.
//! The earlier panel named a different condition, so the app must show its record but
//! never compare it with today's class. ERepo does not state an inheritance for either
//! condition, so both are `Unknown`.

use classivar_core::{Class, Condition, Inheritance, Strength};

use super::{Choice, Editing, Evidence, Record, Workbench};

pub(super) const VARIANT: &str = "NM_000540.3(RYR1):c.7354C>T";
pub(super) const GENE: &str = "RYR1";

/// (panel, year published, class, condition name and MONDO id, criteria met)
type PanelRecord = (&'static str, u16, Class, (&'static str, &'static str), &'static [(&'static str, Strength)]);

pub(super) const MALIGNANT_HYPERTHERMIA: (&str, &str) =
    ("malignant hyperthermia, susceptibility to, 1", "MONDO:0007783");
pub(super) const RYR1_MYOPATHY: (&str, &str) = ("RYR1-related myopathy", "MONDO:0100150");

pub(super) fn condition(of: (&str, &str)) -> Condition {
    Condition { name: of.0.into(), mondo: Some(of.1.into()), inheritance: Inheritance::Unknown }
}

pub(super) const RECORDS: [PanelRecord; 2] = [
    (
        "ClinGen Malignant Hyperthermia Susceptibility VCEP",
        2023,
        Class::LikelyPathogenic,
        MALIGNANT_HYPERTHERMIA,
        &[
            ("PP1", Strength::Moderate),
            ("PM1", Strength::Moderate),
            ("PS4", Strength::Moderate),
            ("PS3", Strength::Moderate),
        ],
    ),
    (
        "ClinGen Congenital Myopathies VCEP",
        2026,
        Class::Pathogenic,
        RYR1_MYOPATHY,
        &[
            ("PP3", Strength::Supporting),
            ("PS3", Strength::Moderate),
            ("PM2", Strength::Supporting),
            ("PS4", Strength::Strong),
            ("PP1", Strength::Strong),
        ],
    ),
];

pub(super) fn evidence_of(list: &[(&str, Strength)]) -> Evidence {
    list.iter().map(|(code, strength)| (code.to_string(), Choice::new(*strength))).collect()
}

impl Workbench {
    /// Put the published RYR1 case on screen: both panel records, and the later
    /// panel's criteria as today's evidence.
    pub(super) fn load_real_case(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.welcome = false;
        self.records = RECORDS
            .iter()
            .map(|(label, year, class, of, list)| Record {
                label: label.to_string(),
                year: *year,
                class: Some(*class),
                tier: None,
                evidence: evidence_of(list),
                condition: Some(condition(*of)),
                source: None,
            })
            .collect();
        self.today = evidence_of(RECORDS[1].4);
        self.variant.condition = Some(condition(RYR1_MYOPATHY));
        self.editing = Editing::Today;
        self.case_gene = GENE.to_string();
        self.boxes.variant.update(cx, |input, cx| input.set_value(VARIANT, window, cx));
        self.boxes.gene.update(cx, |input, cx| input.set_value(GENE, window, cx));
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;

    use super::{RECORDS, evidence_of};
    use crate::view::evidence::to_applied;

    #[test]
    fn the_published_case_only_uses_criteria_the_rules_accept() {
        let rules = RuleSet::builtin().expect("the built-in rules are valid");
        for (panel, _, class, _, list) in RECORDS {
            let result = rules.assess(&to_applied(&evidence_of(list))).expect(panel);
            // The loaded rules agree with each panel's class, so the pictures have a class to draw.
            assert_eq!(result.class(), Some(class), "{panel}");
        }
    }
}
