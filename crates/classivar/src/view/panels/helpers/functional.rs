//! The functional assay helper: validation controls or an OddsPath value become PS3 or BS3.

use super::*;
use classivar_core::{AssayValidation, EvidenceReading, FunctionalRules};

/// What the boxes of the controls route hold: the two counts of validation controls.
pub(super) struct TypedControls {
    pub(super) pathogenic: Option<Result<u32, ()>>,
    pub(super) benign: Option<Result<u32, ()>>,
}

/// What the validation of an assay means without statistics (Brnich 2019, step 3).
/// `experimental` is the curator's confirmation that the assay has normal and
/// abnormal experimental controls and replicates.
pub(super) fn controls_outcome(
    rules: &RuleSet,
    table: &FunctionalRules,
    damaging: bool,
    experimental: bool,
    typed: TypedControls,
) -> (String, Option<EvidenceReading>) {
    if !experimental {
        return (
            "Confirm the experimental controls and replicates first. Without them, Brnich 2019 says not to apply functional evidence."
                .to_string(),
            None,
        );
    }
    let (pathogenic, benign) = match (typed.pathogenic, typed.benign) {
        (Some(Ok(p)), Some(Ok(b))) => (p, b),
        (Some(Err(())), _) | (_, Some(Err(()))) => {
            return ("Use whole numbers of controls, for example 6 and 5.".to_string(), None);
        }
        _ => {
            return (
                "Type how many pathogenic and how many benign validation controls the assay used (0 if none)."
                    .to_string(),
                None,
            );
        }
    };
    let assay =
        AssayValidation { experimental_controls: true, pathogenic_controls: pathogenic, benign_controls: benign };
    match rules.read_functional_controls(damaging, assay) {
        Some(reading) => (
            format!(
                "{pathogenic} pathogenic and {benign} benign validation controls, no statistical analysis: {} for {}. Moderate needs at least {} validation controls including both kinds; this route goes no higher.",
                reading.strength.label(),
                reading.code,
                table.controls_moderate_min
            ),
            Some(reading),
        ),
        None => ("The rules have no functional evidence table.".to_string(), None),
    }
}

/// What an OddsPath value means (Brnich 2019). `typed` is what the box holds (see `parse_number`).
fn oddspath_outcome(
    rules: &RuleSet,
    table: &FunctionalRules,
    typed: Option<Result<f64, ()>>,
) -> (String, Option<EvidenceReading>) {
    let value = match typed {
        None => return ("Type the OddsPath value from the assay's statistical analysis.".to_string(), None),
        Some(Err(())) => return ("That is not a number. Use digits and a point, for example 5.2.".to_string(), None),
        Some(Ok(value)) => value,
    };
    match rules.read_oddspath(value) {
        Some(reading) => {
            (format!("OddsPath {value}: {} for {}.", reading.strength.label(), reading.code), Some(reading))
        }
        None => (
            format!(
                "OddsPath {value} is not a valid value, or lies in the range with no evidence (between {} and {}).",
                table.benign.first().map_or(0.0, |c| c.at),
                table.pathogenic.first().map_or(0.0, |c| c.at)
            ),
            None,
        ),
    }
}

impl Workbench {
    /// Turns functional-assay facts into PS3 or BS3 at the strength Brnich 2019
    /// gives. Like the predictor helper, it only fills in the criterion.
    pub(in crate::view::panels) fn functional_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let panel = self.helper_start();
        let Some(table) = self.rules.functional.as_ref() else {
            return panel.into_any_element();
        };
        let state = &self.helpers.functional;
        let (inputs, (message, reading)) = match state.route {
            AssayRoute::Controls => {
                let typed = TypedControls {
                    pathogenic: self.typed_count(&state.pathogenic_input, cx),
                    benign: self.typed_count(&state.benign_input, cx),
                };
                (
                    self.functional_controls_inputs(cx),
                    controls_outcome(self.rules, table, state.damaging, state.experimental, typed),
                )
            }
            AssayRoute::OddsPath => (
                self.functional_oddspath_input(cx),
                oddspath_outcome(self.rules, table, self.typed_number(&state.oddspath_input, cx)),
            ),
        };
        let citation = self.rules.source(&table.source).map(|s| s.citation.clone());
        panel
            .gap_6()
            .child(reading_note(
                "First do the paper's four steps yourself: define the disease mechanism, judge the class of assay, judge this assay, then apply it to the variant.",
                muted,
            ))
            .child(part("The assay", cx).gap_3().child(self.functional_route_buttons(cx)).child(inputs))
            .child(result_card(
                div().flex().flex_col().gap_2().child(super::super::focus::reading(message)).when_some(reading, |d, reading| {
                    d.child(self.functional_apply(reading, cx))
                }),
                cx,
            ))
            .children(source_part(citation, cx))
            .into_any_element()
    }

    /// A choice between the two ways to read an assay.
    fn functional_route_buttons(&self, cx: &mut Context<Self>) -> Div {
        let answers = [
            ("functional-route-controls", "Number of controls", AssayRoute::Controls),
            ("functional-route-oddspath", "OddsPath value", AssayRoute::OddsPath),
        ]
        .into_iter()
        .map(|(id, label, route)| {
            answer(id, label, self.helpers.functional.route == route, cx).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.helpers.functional.route = route;
                    cx.notify();
                },
            ))
        })
        .collect();
        question("What is known about the assay?", answers, cx)
    }

    /// The damage / no damage choice, the controls confirmation and the boxes for the number of controls.
    fn functional_controls_inputs(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let answers = [("functional-damage", "Damage (PS3)", true), ("functional-no-damage", "No damage (BS3)", false)]
            .into_iter()
            .map(|(id, label, damaging)| {
                answer(id, label, self.helpers.functional.damaging == damaging, cx).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.helpers.functional.damaging = damaging;
                        cx.notify();
                    },
                ))
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(question("What did the assay show?", answers, cx))
            .child(div().flex().child(self.functional_experimental_button(cx)))
            .child(fields([
                field(
                    "Pathogenic validation controls",
                    Input::new(&self.helpers.functional.pathogenic_input)
                        .aria_label("Pathogenic validation control count"),
                    muted,
                ),
                field(
                    "Benign validation controls",
                    Input::new(&self.helpers.functional.benign_input).aria_label("Benign validation control count"),
                    muted,
                ),
            ]))
    }

    /// The curator's confirmation that the assay has experimental controls and replicates.
    fn functional_experimental_button(&self, cx: &mut Context<Self>) -> Button {
        let confirmed = self.helpers.functional.experimental;
        tick("functional-experimental", "It has normal and abnormal controls, and replicates", confirmed, cx)
            .accessibility_label("Confirm the assay has normal and abnormal experimental controls and replicates")
            .on_click(cx.listener(|this, _, _, cx| {
                this.helpers.functional.experimental = !this.helpers.functional.experimental;
                cx.notify();
            }))
    }

    fn functional_oddspath_input(&self, cx: &mut Context<Self>) -> Div {
        fields([field(
            "OddsPath from the assay",
            Input::new(&self.helpers.functional.oddspath_input).aria_label("OddsPath value from the assay"),
            cx.theme().muted_foreground,
        )])
    }

    /// The Apply button, with a caution when the reading is Very strong PS3.
    fn functional_apply(&self, reading: EvidenceReading, cx: &mut Context<Self>) -> Div {
        let state = &self.helpers.functional;
        let label = format!("Apply {} {}", reading.code, reading.strength.label());
        let reason = match state.route {
            AssayRoute::Controls => format!(
                "Functional assay with experimental controls and replicates; {} pathogenic and {} benign validation controls; no statistical analysis (Brnich 2019)",
                state.pathogenic_input.read(cx).value().trim(),
                state.benign_input.read(cx).value().trim()
            ),
            AssayRoute::OddsPath => format!("OddsPath {} (Brnich 2019)", state.oddspath_input.read(cx).value().trim()),
        };
        let caution = (reading.code == "PS3" && reading.strength == Strength::VeryStrong).then(|| {
            caption(cx.theme().muted_foreground).child(
                "Even Very strong functional evidence is not enough alone for a Pathogenic class (Brnich 2019). The program does not check this for you.",
            )
        });
        div().flex().flex_col().gap_2().children(caution).child(
            div().child(
                Button::new("functional-apply")
                    .label(label.clone())
                    .accessibility_label(format!("{label} to today's evidence"))
                    .small()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.apply_reading(&reading, reason.clone());
                        cx.notify();
                    })),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{RuleSet, Strength};

    use super::{TypedControls, controls_outcome, oddspath_outcome};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn controls(
        damaging: bool,
        experimental: bool,
        pathogenic: Option<Result<u32, ()>>,
        benign: Option<Result<u32, ()>>,
    ) -> (String, Option<classivar_core::EvidenceReading>) {
        let table = rules().functional.as_ref().unwrap();
        controls_outcome(rules(), table, damaging, experimental, TypedControls { pathogenic, benign })
    }

    #[test]
    fn controls_give_supporting_or_moderate_and_never_more() {
        let strength = |r: (String, Option<classivar_core::EvidenceReading>)| r.1.map(|r| (r.code, r.strength));
        assert_eq!(strength(controls(true, true, Some(Ok(0)), Some(Ok(0)))), Some(("PS3", Strength::Supporting)));
        assert_eq!(strength(controls(true, true, Some(Ok(6)), Some(Ok(5)))), Some(("PS3", Strength::Moderate)));
        assert_eq!(strength(controls(false, true, Some(Ok(11)), Some(Ok(0)))), Some(("BS3", Strength::Supporting)));
        let (text, _) = controls(true, true, Some(Ok(6)), Some(Ok(5)));
        assert!(text.contains("6 pathogenic and 5 benign"), "{text}");
    }

    #[test]
    fn without_experimental_controls_nothing_is_offered() {
        let (text, reading) = controls(true, false, Some(Ok(20)), Some(Ok(20)));
        assert_eq!(reading, None);
        assert!(text.contains("not to apply functional evidence"), "{text}");
    }

    #[test]
    fn an_empty_or_bad_controls_box_asks_instead_of_reading() {
        assert!(controls(true, true, None, Some(Ok(3))).0.starts_with("Type how many"));
        assert!(controls(true, true, Some(Err(())), Some(Ok(3))).0.contains("whole numbers"));
        assert_eq!(controls(true, true, Some(Err(())), Some(Ok(3))).1, None);
    }

    #[test]
    fn oddspath_follows_the_limits_and_names_the_empty_middle() {
        let table = rules().functional.as_ref().unwrap();
        let (_, reading) = oddspath_outcome(rules(), table, Some(Ok(100.0)));
        assert_eq!(reading.unwrap().code, "PS3");
        let (text, reading) = oddspath_outcome(rules(), table, Some(Ok(1.0)));
        assert_eq!(reading, None);
        assert!(text.contains("no evidence"), "{text}");
        assert!(oddspath_outcome(rules(), table, None).0.starts_with("Type the OddsPath"));
        assert!(oddspath_outcome(rules(), table, Some(Err(()))).0.starts_with("That is not a number"));
    }
}
