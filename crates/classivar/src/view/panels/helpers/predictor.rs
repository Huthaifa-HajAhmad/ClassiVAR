//! The predictor score helper: a tool's score becomes PP3 or BP4 at the published strength.

use super::*;
use classivar_core::{EvidenceReading, Predictor};

/// What a typed score means for a tool: a sentence for the curator, and the
/// evidence it gives when it gives any. `typed` is what the box holds (see `parse_number`).
fn predictor_outcome(
    rules: &RuleSet,
    tool: &Predictor,
    typed: Option<Result<f64, ()>>,
) -> (String, Option<EvidenceReading>) {
    let score = match typed {
        None => return ("Type the score the tool gave.".to_string(), None),
        Some(Err(())) => {
            return ("That is not a number. Use digits and a point, for example 0.85.".to_string(), None);
        }
        Some(Ok(score)) => score,
    };
    if let (Some(min), Some(max)) = (tool.min_score, tool.max_score)
        && !(min..=max).contains(&score)
    {
        return (format!("That score is outside this tool's range ({min} to {max})."), None);
    }
    let Some(reading) = rules.read_predictor(&tool.id, score) else {
        return (format!("{} {score} lies between the limits: this tool gives neither PP3 nor BP4.", tool.name), None);
    };
    let mut text = match tool.calibrated {
        Some(calibrated) => format!(
            "{} {score}: the paper's table rates the evidence {}, and advises applying {} at {}.",
            tool.name,
            calibrated.label(),
            reading.code,
            reading.published.label()
        ),
        None => {
            format!("{} {score}: the paper rates this {} for {}.", tool.name, reading.published.label(), reading.code)
        }
    };
    if reading.published != reading.strength {
        text.push_str(&format!(
            " These rules stop {} at {}, so it applies {}.",
            reading.code,
            reading.strength.label(),
            reading.strength.label()
        ));
    }
    (text, Some(reading))
}

/// The reason saved with the strength: the tool, the score, the paper and anything the curator confirmed.
fn predictor_reason(tool: &Predictor, reading: &EvidenceReading, score: &str) -> String {
    let confirmation = tool.eligibility.as_deref().map(|text| format!("; confirmed {text}")).unwrap_or_default();
    let (name, code, strength) = (&tool.name, reading.code, reading.strength.label());
    match tool.calibrated {
        Some(calibrated) => format!(
            "{name} score {score}: {strength} for {code} (paper evidence: {}{confirmation}; {})",
            calibrated.label(),
            tool.reference
        ),
        None if reading.published != reading.strength => format!(
            "{name} score {score}: {strength} for {code} (the paper rates this {}; these rules cap {code} at {strength}; {}{confirmation})",
            reading.published.label(),
            tool.reference
        ),
        None => {
            format!("{name} score {score}: {} for {code} ({}{confirmation})", reading.published.label(), tool.reference)
        }
    }
}

impl Workbench {
    /// Turns a prediction tool's score into PP3 or BP4 with the strength the
    /// published calibration gives. It only fills in the criterion; the curator
    /// still presses Apply.
    pub(in crate::view::panels) fn predictor_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let panel = self.helper_start();
        let Some(tool) = self.rules.predictors.get(self.helpers.predictor.tool) else {
            return panel.into_any_element();
        };
        let (message, reading) =
            predictor_outcome(self.rules, tool, self.typed_number(&self.helpers.predictor.input, cx));
        let citation = self.rules.source(&tool.source).map(|s| s.citation.clone());
        let score = fields([field(
            format!("{} score", tool.name),
            Input::new(&self.helpers.predictor.input).aria_label("Predictor score"),
            muted,
        )]);
        let result = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(super::super::focus::reading(message))
            .when_some(reading, |d, reading| d.child(self.predictor_apply(tool, reading, cx)));
        let notes = tool.note.clone().into_iter().chain(std::iter::once(
            "Choose one tool before you see its result, and use it for the whole case. The tool, score and source are saved with the case as the reason for the strength.".to_string(),
        ));
        panel
            .gap_6()
            .child(
                part("The score", cx)
                    .gap_3()
                    .child(self.predictor_tools(cx))
                    .child(score)
                    .when_some(tool.eligibility.as_deref(), |d, text| d.child(self.predictor_eligibility(text, cx))),
            )
            .child(result_card(result, cx))
            .children(notes_part(notes, cx))
            .children(source_part(citation, cx))
            .into_any_element()
    }

    /// One button for each tool the rules calibrate. Choosing another tool withdraws the confirmation.
    fn predictor_tools(&self, cx: &mut Context<Self>) -> Div {
        let mut tools = Vec::new();
        for (i, tool) in self.rules.predictors.iter().enumerate() {
            let chosen = self.helpers.predictor.tool == i;
            tools.push(answer(format!("predictor-tool-{}", tool.id), tool.name.clone(), chosen, cx).on_click(
                cx.listener(move |this, _, _, cx| {
                    if this.helpers.predictor.tool != i {
                        this.helpers.predictor.tool = i;
                        this.helpers.predictor.eligible = false;
                    }
                    cx.notify();
                }),
            ));
        }
        question("Which tool?", tools, cx)
    }

    /// The extra condition a tool needs, and the button that confirms it was checked.
    fn predictor_eligibility(&self, eligibility: &str, cx: &mut Context<Self>) -> Div {
        let confirmed = self.helpers.predictor.eligible;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .items_start()
            .child(reading_note(format!("Before applying, confirm {eligibility}."), cx.theme().muted_foreground))
            .child(
                tick("predictor-eligible", "I checked this condition", confirmed, cx)
                    .accessibility_label(format!("Confirm {eligibility}"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.helpers.predictor.eligible = !this.helpers.predictor.eligible;
                        cx.notify();
                    })),
            )
    }

    /// The Apply button, or a reminder to confirm the tool's condition first.
    fn predictor_apply(&self, tool: &Predictor, reading: EvidenceReading, cx: &mut Context<Self>) -> Div {
        if tool.eligibility.is_some() && !self.helpers.predictor.eligible {
            return div()
                .child(caption(cx.theme().muted_foreground).child("Confirm the condition above before applying."));
        }
        let label = format!("Apply {} {}", reading.code, reading.strength.label());
        let score = self.helpers.predictor.input.read(cx).value().trim().to_string();
        let reason = predictor_reason(tool, &reading, &score);
        div().child(
            Button::new("predictor-apply")
                .label(label.clone())
                .accessibility_label(format!("{label} to today's evidence"))
                .small()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.apply_reading(&reading, reason.clone());
                    cx.notify();
                })),
        )
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;

    use super::{predictor_outcome, predictor_reason};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    #[test]
    fn the_sentence_follows_what_the_box_holds() {
        let revel = rules().predictor("revel").unwrap();
        assert_eq!(predictor_outcome(rules(), revel, None).0, "Type the score the tool gave.");
        assert!(predictor_outcome(rules(), revel, Some(Err(()))).0.starts_with("That is not a number"));
        let spliceai = rules().predictor("spliceai").unwrap();
        assert!(predictor_outcome(rules(), spliceai, Some(Ok(1.5))).0.contains("outside this tool's range"));
        let (text, reading) = predictor_outcome(rules(), revel, Some(Ok(0.5)));
        assert!(text.contains("lies between the limits"), "{text}");
        assert!(reading.is_none());
    }

    #[test]
    fn a_score_in_a_band_gives_the_reading_and_says_when_this_program_stops_lower() {
        let revel = rules().predictor("revel").unwrap();
        let (text, reading) = predictor_outcome(rules(), revel, Some(Ok(0.95)));
        assert_eq!(reading.unwrap().code, "PP3");
        assert!(text.contains("the paper rates this"), "{text}");
        let (text, reading) = predictor_outcome(rules(), revel, Some(Ok(0.003)));
        let reading = reading.unwrap();
        assert_eq!(reading.code, "BP4");
        assert!(text.contains("These rules stop BP4 at Moderate"), "{text}");
        let reason = predictor_reason(revel, &reading, "0.003");
        assert!(reason.contains("REVEL score 0.003"), "{reason}");
        assert!(reason.contains("these rules cap BP4 at Moderate"), "{reason}");
        assert!(reason.contains("Pejaver 2022, Table 2"), "{reason}");
    }

    #[test]
    fn a_calibrated_tool_names_the_paper_evidence_in_the_saved_reason() {
        let spliceai = rules().predictor("spliceai").unwrap();
        let (_, reading) = predictor_outcome(rules(), spliceai, Some(Ok(0.5)));
        let reason = predictor_reason(spliceai, &reading.unwrap(), "0.5");
        assert!(reason.contains("paper evidence: Moderate"), "{reason}");
        assert!(reason.contains("confirmed"), "the eligibility condition is saved: {reason}");
    }
}
