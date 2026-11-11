//! The functional assay helper: validation controls or an OddsPath value become PS3 or BS3.

use super::*;
use classivar_core::{EvidenceReading, FunctionalRules};

/// What the number of validation controls means (Brnich 2019, no statistical analysis).
/// `typed` is what the box holds (see `parse_count`).
fn controls_outcome(
    rules: &RuleSet,
    table: &FunctionalRules,
    damaging: bool,
    typed: Option<Result<u32, ()>>,
) -> (String, Option<EvidenceReading>) {
    let controls = match typed {
        None => {
            return (
                "Type how many validation controls the assay used (pathogenic and benign variants together)."
                    .to_string(),
                None,
            );
        }
        Some(Err(())) => return ("Use a whole number of controls, for example 11.".to_string(), None),
        Some(Ok(controls)) => controls,
    };
    match rules.read_functional_controls(damaging, controls) {
        Some(reading) => (
            format!(
                "{controls} controls, no statistical analysis: {} for {}. {} or more controls reach Moderate; fewer reach Supporting; this route goes no higher.",
                reading.strength.label(),
                reading.code,
                table.controls_moderate_min
            ),
            Some(reading),
        ),
        None if controls < table.controls_supporting_min => {
            ("An assay with no validation controls gives no evidence (Brnich 2019).".to_string(), None)
        }
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
        let panel = self.helper_start(
            ("functional", "Functional assay → PS3 or BS3"),
            "functional evidence helper",
            |this| this.helpers.functional.open = !this.helpers.functional.open,
            cx,
        );
        let Some(table) = self.rules.functional.as_ref() else {
            return panel.into_any_element();
        };
        let state = &self.helpers.functional;
        let (inputs, (message, reading)) = match state.route {
            AssayRoute::Controls => (
                self.functional_controls_inputs(cx),
                controls_outcome(self.rules, table, state.damaging, self.typed_count(&state.controls_input, cx)),
            ),
            AssayRoute::OddsPath => (
                self.functional_oddspath_input(cx),
                oddspath_outcome(self.rules, table, self.typed_number(&state.oddspath_input, cx)),
            ),
        };
        let citation = self.rules.source(&table.source).map(|s| s.citation.clone());
        panel
            .child(caption(muted).child(
                "First do the paper's four steps yourself: define the disease mechanism, judge the class of assay, judge this assay, then apply it to the variant.",
            ))
            .child(self.functional_route_buttons(cx))
            .child(inputs)
            .child(div().text_sm().child(message))
            .when_some(reading, |d, reading| d.child(self.functional_apply(reading, cx)))
            .when_some(citation, |d, citation| d.child(caption(muted).child(format!("Source: {citation}"))))
            .into_any_element()
    }

    /// A choice between the two ways to read an assay.
    fn functional_route_buttons(&self, cx: &mut Context<Self>) -> Div {
        let route_button = |id: &'static str, label: &'static str, route: AssayRoute, cx: &mut Context<Self>| {
            let chosen = self.helpers.functional.route == route;
            Button::new(id).label(label).small().selected(chosen).toggled(chosen).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.helpers.functional.route = route;
                    cx.notify();
                },
            ))
        };
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(caption(cx.theme().muted_foreground).child("What is known"))
            .child(route_button("functional-route-controls", "Number of controls", AssayRoute::Controls, cx))
            .child(route_button("functional-route-oddspath", "OddsPath value", AssayRoute::OddsPath, cx))
    }

    /// The damage / no damage choice and the box for the number of controls.
    fn functional_controls_inputs(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let direction_button = |id: &'static str, label: &'static str, damaging: bool, cx: &mut Context<Self>| {
            let chosen = self.helpers.functional.damaging == damaging;
            Button::new(id).label(label).small().selected(chosen).toggled(chosen).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.helpers.functional.damaging = damaging;
                    cx.notify();
                },
            ))
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(caption(muted).child("The assay showed"))
                    .child(direction_button("functional-damage", "Damage (PS3)", true, cx))
                    .child(direction_button("functional-no-damage", "No damage (BS3)", false, cx)),
            )
            .child(
                div().flex().items_center().gap_2().child(caption(muted).child("Validation controls")).child(
                    div().w(px(120.)).child(
                        Input::new(&self.helpers.functional.controls_input).aria_label("Validation control count"),
                    ),
                ),
            )
    }

    fn functional_oddspath_input(&self, cx: &mut Context<Self>) -> Div {
        div().flex().items_center().gap_2().child(caption(cx.theme().muted_foreground).child("OddsPath")).child(
            div()
                .w(px(120.))
                .child(Input::new(&self.helpers.functional.oddspath_input).aria_label("OddsPath value from the assay")),
        )
    }

    /// The Apply button, with a caution when the reading is Very strong PS3.
    fn functional_apply(&self, reading: EvidenceReading, cx: &mut Context<Self>) -> Div {
        let state = &self.helpers.functional;
        let label = format!("Apply {} {}", reading.code, reading.strength.label());
        let reason = match state.route {
            AssayRoute::Controls => format!(
                "Functional assay with {} validation controls, no statistical analysis (Brnich 2019)",
                state.controls_input.read(cx).value().trim()
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

    use super::{controls_outcome, oddspath_outcome};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    #[test]
    fn controls_give_supporting_or_moderate_and_never_more() {
        let table = rules().functional.as_ref().unwrap();
        let read = |damaging, controls| controls_outcome(rules(), table, damaging, Some(Ok(controls)));
        assert_eq!(read(true, 0).1, None);
        assert!(read(true, 0).0.contains("no validation controls"));
        let (_, reading) = read(true, table.controls_moderate_min);
        assert_eq!((reading.as_ref().unwrap().code, reading.unwrap().strength), ("PS3", Strength::Moderate));
        let (_, reading) = read(false, table.controls_supporting_min);
        assert_eq!((reading.as_ref().unwrap().code, reading.unwrap().strength), ("BS3", Strength::Supporting));
    }

    #[test]
    fn an_empty_or_bad_controls_box_asks_instead_of_reading() {
        let table = rules().functional.as_ref().unwrap();
        assert!(controls_outcome(rules(), table, true, None).0.starts_with("Type how many"));
        assert!(controls_outcome(rules(), table, true, Some(Err(()))).0.contains("whole number"));
        assert_eq!(controls_outcome(rules(), table, true, Some(Err(()))).1, None);
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
