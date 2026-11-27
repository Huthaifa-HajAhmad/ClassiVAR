//! The state of the evidence helpers (which are open, what is typed, what is
//! chosen), what they read from their boxes, and what they apply to the evidence.
//!
//! Each helper has its own small state type. A new helper adds one type here,
//! one field to `Helpers`, and its own drawing file; `Workbench` does not change.

use super::*;

/// How the functional-evidence helper reads an assay.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AssayRoute {
    /// Only the number of validation controls is known.
    Controls,
    /// A statistical analysis gave an OddsPath value.
    OddsPath,
}

/// What the curator said about how many genes can cause the phenotype. PP1 counts
/// only when several can: with a single gene the paper allows no segregation evidence.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PhenotypeGenes {
    Unanswered,
    One,
    Several,
}

/// The predictor helper (PP3 and BP4).
pub(super) struct PredictorState {
    pub(super) open: bool,
    /// Which tool of `rules.predictors` the helper reads.
    pub(super) tool: usize,
    /// True when the curator confirmed the selected tool's extra condition.
    pub(super) eligible: bool,
    pub(super) input: Entity<InputState>,
}

/// The functional-evidence helper (PS3 and BS3).
pub(super) struct FunctionalState {
    pub(super) open: bool,
    pub(super) route: AssayRoute,
    /// For the controls route: did the assay show damage (PS3) or none (BS3)?
    pub(super) damaging: bool,
    /// The curator confirmed normal and abnormal experimental controls and replicates.
    pub(super) experimental: bool,
    pub(super) pathogenic_input: Entity<InputState>,
    pub(super) benign_input: Entity<InputState>,
    pub(super) oddspath_input: Entity<InputState>,
}

impl FunctionalState {
    fn new(subscriptions: &mut Vec<Subscription>, window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        Self {
            open: false,
            route: AssayRoute::Controls,
            damaging: true,
            experimental: false,
            pathogenic_input: new_box("e.g. 6", subscriptions, window, cx),
            benign_input: new_box("e.g. 5", subscriptions, window, cx),
            oddspath_input: new_box("OddsPath, e.g. 5.2", subscriptions, window, cx),
        }
    }
}

/// The co-segregation and phenotype helper (PP1 and PP4).
pub(super) struct LocusState {
    pub(super) open: bool,
    /// Which kind of `rules.locus_evidence.segregation` the PP1 count belongs to.
    pub(super) kind: usize,
    pub(super) genes: PhenotypeGenes,
    pub(super) yield_input: Entity<InputState>,
    pub(super) segregations_input: Entity<InputState>,
    /// How many variants share the allele (the paper divides the yield among them).
    pub(super) variants_input: Entity<InputState>,
}

/// The non-segregation helper (BS4): the situation chosen.
pub(super) struct SegregationState {
    pub(super) open: bool,
    pub(super) situation: usize,
}

/// The PM3 patients helper: one box for each row of the rule file's PM3 table.
pub(super) struct Pm3State {
    pub(super) open: bool,
    pub(super) eligible: bool,
    pub(super) inputs: Vec<Entity<InputState>>,
}

/// The de novo helper (PS2 or PM6): the recessive answer, and two patient-count
/// boxes (parents confirmed, not confirmed) for each row of the de novo table.
pub(super) struct DeNovoState {
    pub(super) open: bool,
    pub(super) recessive: bool,
    pub(super) inputs: Vec<[Entity<InputState>; 2]>,
}

/// The population frequency helper (BA1, BS1, PM2): the "absent" answer and
/// one box for each number (see `POPULATION_BOXES`).
pub(super) struct PopulationState {
    pub(super) open: bool,
    pub(super) absent: bool,
    pub(super) inputs: Vec<Entity<InputState>>,
}

impl PopulationState {
    fn new(subscriptions: &mut Vec<Subscription>, window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        let inputs = super::panels::POPULATION_BOXES
            .iter()
            .map(|(_, example)| new_box(example.to_string(), subscriptions, window, cx))
            .collect();
        Self { open: false, absent: false, inputs }
    }
}

pub(super) struct Helpers {
    pub(super) predictor: PredictorState,
    pub(super) functional: FunctionalState,
    pub(super) locus: LocusState,
    pub(super) segregation: SegregationState,
    pub(super) pm3: Pm3State,
    pub(super) ba1_open: bool,
    pub(super) de_novo: DeNovoState,
    pub(super) population: PopulationState,
}

/// A text box that repaints the screen when it changes. Its subscription is kept in `subscriptions`.
pub(super) fn new_box(
    placeholder: impl Into<SharedString>,
    subscriptions: &mut Vec<Subscription>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) -> Entity<InputState> {
    let placeholder = placeholder.into();
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    subscriptions.push(repaint_on_change(&input, window, cx));
    input
}

fn clear_box(input: &Entity<InputState>, window: &mut Window, cx: &mut Context<Workbench>) {
    input.update(cx, |input, cx| input.set_value("", window, cx));
}

impl Helpers {
    /// Every helper closed and empty. The boxes for the rule file's PM3 and de
    /// novo rows are made here, so the number of boxes follows the loaded rules.
    pub(super) fn new(
        rules: &RuleSet,
        subscriptions: &mut Vec<Subscription>,
        window: &mut Window,
        cx: &mut Context<Workbench>,
    ) -> Self {
        let population = PopulationState::new(subscriptions, window, cx);
        let functional = FunctionalState::new(subscriptions, window, cx);
        let mut boxed = |placeholder: &str, window: &mut Window, cx: &mut Context<Workbench>| {
            new_box(placeholder.to_string(), subscriptions, window, cx)
        };
        let predictor =
            PredictorState { open: false, tool: 0, eligible: false, input: boxed("Score, e.g. 0.85", window, cx) };
        let locus = LocusState {
            open: false,
            kind: 0,
            genes: PhenotypeGenes::Unanswered,
            yield_input: boxed("Diagnostic yield %, e.g. 60", window, cx),
            segregations_input: boxed("Informative family members, e.g. 3", window, cx),
            variants_input: boxed("Variants on the allele, e.g. 1", window, cx),
        };
        let pm3_inputs: Vec<_> = rules
            .pm3
            .iter()
            .flat_map(|table| &table.rows)
            .map(|row| boxed(&format!("PM3 patients: {}", row.id), window, cx))
            .collect();
        let de_novo_inputs: Vec<_> = rules
            .de_novo
            .iter()
            .flat_map(|table| &table.phenotypes)
            .map(|_| [boxed("Patients, e.g. 1", window, cx), boxed("Patients, e.g. 1", window, cx)])
            .collect();
        Self {
            predictor,
            functional,
            locus,
            segregation: SegregationState { open: false, situation: 0 },
            pm3: Pm3State { open: false, eligible: false, inputs: pm3_inputs },
            ba1_open: false,
            de_novo: DeNovoState { open: false, recessive: false, inputs: de_novo_inputs },
            population,
        }
    }

    /// Every text box of every helper.
    pub(super) fn text_boxes(&self) -> impl Iterator<Item = &Entity<InputState>> {
        [
            &self.predictor.input,
            &self.functional.pathogenic_input,
            &self.functional.benign_input,
            &self.functional.oddspath_input,
            &self.locus.yield_input,
            &self.locus.segregations_input,
            &self.locus.variants_input,
        ]
        .into_iter()
        .chain(&self.pm3.inputs)
        .chain(self.de_novo.inputs.iter().flatten())
        .chain(&self.population.inputs)
    }

    /// Close every helper and forget what was typed. A helper belongs to the variant on screen.
    pub(super) fn reset(&mut self, window: &mut Window, cx: &mut Context<Workbench>) {
        self.predictor.open = false;
        self.predictor.tool = 0;
        self.predictor.eligible = false;
        self.functional.open = false;
        self.functional.route = AssayRoute::Controls;
        self.functional.damaging = true;
        self.functional.experimental = false;
        self.locus.open = false;
        self.locus.kind = 0;
        self.locus.genes = PhenotypeGenes::Unanswered;
        self.segregation = SegregationState { open: false, situation: 0 };
        self.pm3.open = false;
        self.pm3.eligible = false;
        self.ba1_open = false;
        self.de_novo.open = false;
        self.de_novo.recessive = false;
        self.population.open = false;
        self.population.absent = false;
        for input in self.text_boxes() {
            clear_box(input, window, cx);
        }
    }
}

/// What a number box holds: `None` when it is empty, `Some(Err(()))` when the
/// entry is not a finite number, otherwise the number.
pub(super) fn parse_number(text: &str) -> Option<Result<f64, ()>> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or(()))
}

/// A count of people: `None` when the box is empty, `Some(Err(()))` when the entry
/// is not a whole number that fits a count.
pub(super) fn parse_count(text: &str) -> Option<Result<u32, ()>> {
    match parse_number(text)? {
        Ok(n) if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) => Some(Ok(n as u32)),
        _ => Some(Err(())),
    }
}

impl Workbench {
    /// What a helper's number box holds, see `parse_number`.
    pub(super) fn typed_number(&self, input: &Entity<InputState>, cx: &App) -> Option<Result<f64, ()>> {
        parse_number(&input.read(cx).value())
    }

    /// What a helper's count box holds, see `parse_count`.
    pub(super) fn typed_count(&self, input: &Entity<InputState>, cx: &App) -> Option<Result<u32, ()>> {
        parse_count(&input.read(cx).value())
    }

    /// Put the strength a score or an assay result gives onto today's evidence.
    /// The reason records where the number came from, so it is saved with the case.
    pub(super) fn apply_reading(&mut self, reading: &classivar_core::EvidenceReading, reason: String) {
        self.count_today(reading.code.to_string(), Choice::with_reason(reading.strength, reason));
    }

    /// Put the PP1 and PP4 strengths of one paper-allowed choice onto today's
    /// evidence. A criterion the choice leaves out is not touched. The reason
    /// records the points behind the strengths, so it is saved with the case.
    pub(super) fn apply_locus_choice(&mut self, choice: classivar_core::LocusChoice, reason: String) {
        for (code, strength) in [("PP1", choice.pp1), ("PP4", choice.pp4)] {
            if let Some(strength) = strength {
                self.count_today(code.to_string(), Choice::with_reason(strength, reason.clone()));
            }
        }
    }

    /// Forget what the helpers hold, and any strength waiting for its reason.
    pub(super) fn reset_helpers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.raising = None;
        self.kept.clear();
        self.helpers.reset(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_count, parse_number};

    #[test]
    fn an_empty_box_is_nothing_and_a_bad_entry_is_an_error() {
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("   "), None);
        assert_eq!(parse_number(" 0.85 "), Some(Ok(0.85)));
        assert_eq!(parse_number("abc"), Some(Err(())));
        assert_eq!(parse_number("NaN"), Some(Err(())));
        assert_eq!(parse_number("inf"), Some(Err(())));
    }

    #[test]
    fn a_count_is_a_whole_number_that_is_not_negative() {
        assert_eq!(parse_count(""), None);
        assert_eq!(parse_count("3"), Some(Ok(3)));
        assert_eq!(parse_count("0"), Some(Ok(0)));
        assert_eq!(parse_count("2.5"), Some(Err(())));
        assert_eq!(parse_count("-1"), Some(Err(())));
        assert_eq!(parse_count("99999999999"), Some(Err(())));
        assert_eq!(parse_count("x"), Some(Err(())));
    }
}
