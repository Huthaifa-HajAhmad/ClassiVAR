//! The population frequency helper: BA1 (Ghosh 2018), BS1 (Whiffin 2017) and
//! PM2 (ClinGen 2020) from the numbers the curator reads in gnomAD.

use super::*;
use classivar_core::variant::VariantIdentity;
use classivar_core::{DiseaseArchitecture, EvidenceReading, Inheritance, PopulationFacts, PopulationFinding};

/// The boxes of the helper, in order (see `PopulationState`).
pub(in crate::view) const POPULATION_BOXES: [(&str, &str); 10] = [
    ("Highest continental allele frequency", "e.g. 0.0004"),
    ("Alleles at the site in that group", "e.g. 30000"),
    ("Filtering allele frequency (FAF95)", "e.g. 0.0002"),
    ("Prevalence: 1 in", "e.g. 500"),
    ("Penetrance (0 to 1)", "e.g. 0.5"),
    ("Largest share of cases from one variant (0 to 1)", "e.g. 0.02"),
    ("Share of cases from this gene (0 to 1, recessive)", "e.g. 0.09"),
    ("Gene's own BA1 limit (optional)", "e.g. 0.01"),
    ("Gene's own BS1 limit (optional)", "e.g. 0.0001"),
    ("Gene's own PM2 limit (optional, recessive)", "e.g. 0.00002"),
];

/// The boxes that hold a frequency or a share, so a value from 0 to 1.
const FRACTION_BOXES: [usize; 8] = [0, 2, 4, 5, 6, 7, 8, 9];

/// Turn what the boxes hold into facts. A box that does not hold a number is an
/// error with a reason; an empty box is simply unknown.
pub(super) fn population_facts(
    numbers: &[Option<Result<f64, ()>>; 10],
    absent: bool,
    inheritance: Inheritance,
) -> Result<PopulationFacts, String> {
    if let Some(at) = numbers.iter().position(|n| matches!(n, Some(Err(())))) {
        return Err(format!(
            "\"{}\" is not a number. Use digits and a point, for example 0.0004.",
            POPULATION_BOXES[at].0
        ));
    }
    let n = |i: usize| numbers[i].and_then(Result::ok);
    // Frequencies and shares are fractions; a number outside 0 to 1 is a typing
    // mistake, not an empty box. (Prevalence and the allele count have their own rules below.)
    if let Some(at) = FRACTION_BOXES.into_iter().find(|&i| n(i).is_some_and(|v| !(0.0..=1.0).contains(&v))) {
        return Err(format!("\"{}\" is a fraction between 0 and 1, for example 0.0004.", POPULATION_BOXES[at].0));
    }
    let alleles = match n(1) {
        Some(count) if count >= 0.0 && count.fract() == 0.0 => Some(count as u64),
        Some(_) => return Err("The allele number is a whole count, for example 30000.".into()),
        None => None,
    };
    let prevalence = match n(3) {
        Some(one_in) if one_in >= 1.0 => Some(1.0 / one_in),
        Some(_) => return Err("Prevalence is \"1 in N\": type N, at least 1 (for example 500).".into()),
        None => None,
    };
    let recessive = inheritance == Inheritance::AutosomalRecessive;
    let genetic = if recessive { n(6) } else { Some(1.0) };
    let architecture = match (prevalence, n(4), n(5), genetic) {
        (Some(prevalence), Some(penetrance), Some(allelic_contribution), Some(genetic_contribution)) => {
            Some(DiseaseArchitecture { prevalence, penetrance, allelic_contribution, genetic_contribution })
        }
        _ => None,
    };
    Ok(PopulationFacts {
        frequency: n(0),
        alleles,
        filtering_frequency: n(2),
        absent,
        inheritance: (inheritance != Inheritance::Unknown).then_some(inheritance),
        architecture,
        gene_ba1: n(7),
        gene_bs1: n(8),
        gene_pm2: n(9),
    })
}

/// Is the checked variant one of ClinGen's BA1 exceptions? Matched by the
/// Allele Registry id, so a differently written variant still matches.
pub(super) fn on_ba1_exception_list(rules: &RuleSet, identity: Option<&VariantIdentity>) -> bool {
    let Some(id) = identity.and_then(|i| i.allele_id.as_deref()) else { return false };
    rules.ba1_exceptions.iter().any(|row| row.allele_id.eq_ignore_ascii_case(id))
}

/// The reason saved with an applied population criterion.
fn population_reason(finding: &PopulationFinding, source: &str) -> String {
    format!("Population frequency: {} ({source})", finding.why)
}

impl Workbench {
    pub(in crate::view::panels) fn population_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let panel = self.helper_start(
            ("population", "Population frequency → BA1, BS1 or PM2"),
            "population frequency helper",
            |this| this.helpers.population.open = !this.helpers.population.open,
            cx,
        );
        let Some(table) = self.rules.population.as_ref() else { return panel.into_any_element() };
        let muted = cx.theme().muted_foreground;
        let state = &self.helpers.population;
        let numbers: [Option<Result<f64, ()>>; 10] = std::array::from_fn(|i| self.typed_number(&state.inputs[i], cx));
        let inheritance = self.variant.inheritance();
        let exempt = on_ba1_exception_list(self.rules, self.variant.identity.as_ref());
        let outcome = population_facts(&numbers, state.absent, inheritance)
            .map(|facts| self.rules.read_population(&facts, exempt).unwrap_or_default());
        let inheritance_line = match inheritance {
            Inheritance::Unknown => {
                "Inheritance: not set. Set it under Condition in the case form; BS1 and recessive PM2 need it."
                    .to_string()
            }
            known => format!("Inheritance (from the case): {}.", known.label()),
        };
        panel
            .children(table.notes.iter().map(|note| caption(muted).child(note.clone())))
            .child(div().text_sm().child(inheritance_line))
            .child(self.population_absent_button(cx))
            .child(self.population_boxes())
            .child(match outcome {
                Ok(findings) => self.population_findings(findings, cx).into_any_element(),
                Err(problem) => notice(problem, warning_fill(), warning_ink()).into_any_element(),
            })
            .into_any_element()
    }

    fn population_absent_button(&self, cx: &mut Context<Self>) -> Button {
        let absent = self.helpers.population.absent;
        Button::new("population-absent")
            .label("Absent from the population data (allele count 0, site well covered)")
            .accessibility_label("The variant is absent from the population data")
            .small()
            .selected(absent)
            .toggled(absent)
            .on_click(cx.listener(|this, _, _, cx| {
                this.helpers.population.absent = !this.helpers.population.absent;
                cx.notify();
            }))
    }

    fn population_boxes(&self) -> Div {
        let mut grid = div().flex().flex_wrap().gap_2();
        for ((label, _), input) in POPULATION_BOXES.iter().zip(&self.helpers.population.inputs) {
            grid = grid.child(
                div()
                    .w(px(220.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().child(*label))
                    .child(Input::new(input).aria_label(*label)),
            );
        }
        grid
    }

    /// One line per criterion: why it does or does not apply, and an Apply button when it does.
    fn population_findings(&self, findings: Vec<PopulationFinding>, cx: &mut Context<Self>) -> Div {
        let mut list = div().flex().flex_col().gap_1();
        for finding in findings {
            let line = div().text_sm().child(format!("{}: {}", finding.code, finding.why));
            let Some(reading) = finding.reading.clone() else {
                list = list.child(line);
                continue;
            };
            let source = self.population_source(finding.code);
            let reason = population_reason(&finding, &source);
            list = list.child(line).child(self.population_apply(reading, reason, cx));
        }
        list
    }

    fn population_source(&self, code: &str) -> String {
        let Some(table) = &self.rules.population else { return String::new() };
        let id = match code {
            "BA1" => &table.ba1_source,
            "BS1" => &table.bs1_source,
            _ => &table.pm2_source,
        };
        self.rules.source(id).map_or(id.clone(), |s| s.citation.chars().take(60).collect())
    }

    fn population_apply(&self, reading: EvidenceReading, reason: String, cx: &mut Context<Self>) -> Button {
        let label = format!("Apply {} {}", reading.code, reading.strength.label());
        Button::new(SharedString::from(format!("population-apply-{}", reading.code)))
            .label(label.clone())
            .accessibility_label(format!("{label} to today's evidence"))
            .small()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.apply_reading(&reading, reason.clone());
                cx.notify();
            }))
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{Inheritance, RuleSet, Strength};

    use super::{on_ba1_exception_list, population_facts};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn typed(values: [&str; 10]) -> [Option<Result<f64, ()>>; 10] {
        values.map(crate::view::helper_state::parse_number)
    }

    #[test]
    fn the_boxes_become_facts_and_prevalence_is_one_in_n() {
        let numbers = typed(["0.0001", "120000", "0.00005", "500", "0.5", "0.02", "", "", "", ""]);
        let facts = population_facts(&numbers, false, Inheritance::AutosomalDominant).unwrap();
        let architecture = facts.architecture.expect("dominant needs no genetic contribution");
        assert!((architecture.prevalence - 0.002).abs() < 1e-12);
        assert_eq!(facts.alleles, Some(120_000));
        let findings = rules().read_population(&facts, false).unwrap();
        let bs1 = findings.iter().find(|f| f.code == "BS1").unwrap();
        assert_eq!(bs1.reading.as_ref().map(|r| r.strength), Some(Strength::Strong), "{}", bs1.why);
    }

    #[test]
    fn a_recessive_disease_needs_the_genes_share_and_bad_entries_are_named() {
        let numbers = typed(["0.001", "", "0.0005", "10000", "0.5", "0.57", "", "", "", ""]);
        let facts = population_facts(&numbers, false, Inheritance::AutosomalRecessive).unwrap();
        assert_eq!(facts.architecture, None, "the recessive formula needs the gene's share");
        let bad = typed(["lots", "", "", "", "", "", "", "", "", ""]);
        assert!(population_facts(&bad, false, Inheritance::Unknown).unwrap_err().contains("Highest continental"));
        let fraction = typed(["", "12.5", "", "", "", "", "", "", "", ""]);
        assert!(population_facts(&fraction, false, Inheritance::Unknown).unwrap_err().contains("whole count"));
        let prevalence = typed(["", "", "", "0.5", "", "", "", "", "", ""]);
        assert!(population_facts(&prevalence, false, Inheritance::Unknown).unwrap_err().contains("1 in N"));
    }

    #[test]
    fn a_frequency_or_share_outside_zero_to_one_is_named_not_read_as_empty() {
        let percent = typed(["5", "", "", "", "", "", "", "", "", ""]);
        let error = population_facts(&percent, false, Inheritance::Unknown).unwrap_err();
        assert!(error.contains("Highest continental") && error.contains("between 0 and 1"), "{error}");
        let penetrance = typed(["", "", "", "", "50", "", "", "", "", ""]);
        assert!(population_facts(&penetrance, false, Inheritance::Unknown).unwrap_err().contains("Penetrance"));
        let negative = typed(["", "", "-0.1", "", "", "", "", "", "", ""]);
        assert!(population_facts(&negative, false, Inheritance::Unknown).unwrap_err().contains("FAF95"));
        let edge = typed(["1", "", "0", "", "1", "1", "", "", "", ""]);
        assert!(population_facts(&edge, false, Inheritance::Unknown).is_ok(), "0 and 1 are valid fractions");
    }

    #[test]
    fn a_variant_on_the_ba1_exception_list_is_recognised_by_its_registry_id() {
        let mut identity = classivar_core::variant::VariantIdentity {
            allele_id: Some("CA113795".into()),
            grch38: None,
            grch37: None,
            mane_select: None,
            coding: Vec::new(),
            protein: None,
            gene: Some("HFE".into()),
            clinvar_variation: Some(9),
            dbsnp: None,
            checked_with: "test".into(),
            checked_on: "2026-11-21".into(),
        };
        assert!(on_ba1_exception_list(rules(), Some(&identity)), "HFE p.Cys282Tyr is on the 2018 list");
        identity.allele_id = Some("CA003404".into());
        assert!(!on_ba1_exception_list(rules(), Some(&identity)));
        assert!(!on_ba1_exception_list(rules(), None));
    }
}
