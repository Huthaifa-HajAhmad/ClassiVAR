//! The PM3 patients helper.

use super::*;
use classivar_core::{EvidenceReading, Pm3Rules};

/// What the patient counts mean for PM3: a sentence for the curator, and the
/// evidence with the reason to save when it can be applied.
struct Pm3Outcome {
    message: String,
    apply: Option<(EvidenceReading, String)>,
}

/// Reads the typed counts (one for each row of the table, see `parse_count`). Nothing is
/// read until the curator has confirmed the source's conditions.
fn pm3_outcome(rules: &RuleSet, table: &Pm3Rules, typed: &[Option<Result<u32, ()>>], eligible: bool) -> Pm3Outcome {
    let mut counts = Vec::new();
    let mut parts = Vec::new();
    let mut invalid = false;
    for (row, entry) in table.rows.iter().zip(typed) {
        let count = match entry {
            None => 0,
            Some(Ok(count)) => *count,
            Some(Err(())) => {
                invalid = true;
                0
            }
        };
        counts.push(count);
        if count > 0 {
            parts.push(format!("{count} x {}", row.id));
        }
    }
    if invalid {
        return Pm3Outcome { message: "Use whole numbers of patients, for example 2.".to_string(), apply: None };
    }
    if !eligible {
        return Pm3Outcome { message: "Confirm the conditions above before applying PM3.".to_string(), apply: None };
    }
    let Some(reading) = rules.read_pm3(&counts) else {
        return Pm3Outcome {
            message: "No patient total reached a PM3 level, so there is no evidence to apply.".to_string(),
            apply: None,
        };
    };
    let level = reading
        .evidence
        .as_ref()
        .map(|e| format!("PM3 {}", e.strength.label()))
        .unwrap_or_else(|| "below Supporting; no evidence".to_string());
    let message = format!("{} patient points: {level}.", reading.points);
    let reason = format!(
        "PM3: {} = {} patient points after row limits; conditions checked ({})",
        parts.join(" + "),
        reading.points,
        table.source
    );
    Pm3Outcome { message, apply: reading.evidence.map(|evidence| (evidence, reason)) }
}

impl Workbench {
    pub(in crate::view::panels) fn pm3_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let panel = self.helper_start();
        let Some(table) = self.rules.pm3.as_ref() else {
            return panel.into_any_element();
        };
        let muted = cx.theme().muted_foreground;
        let typed: Vec<_> = self.helpers.pm3.inputs.iter().map(|input| self.typed_count(input, cx)).collect();
        let outcome = pm3_outcome(self.rules, table, &typed, self.helpers.pm3.eligible);
        let citation = self.rules.source(&table.source).map(|s| s.citation.clone());
        let result = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(super::super::focus::reading(outcome.message))
            .when_some(outcome.apply, |d, (evidence, reason)| d.child(self.pm3_apply(evidence, reason, cx)));
        panel
            .gap_6()
            .children(notes_part(table.notes.iter().cloned(), cx))
            .child(
                part("Your patients", cx)
                    .gap_3()
                    .child(self.pm3_eligibility(cx))
                    .child(fields(self.pm3_rows(table, muted))),
            )
            .child(result_card(result, cx))
            .children(source_part(citation, cx))
            .into_any_element()
    }

    /// A count box for each patient group of the table.
    fn pm3_rows(&self, table: &Pm3Rules, muted: Hsla) -> Vec<Div> {
        table
            .rows
            .iter()
            .zip(&self.helpers.pm3.inputs)
            .map(|(row, input)| {
                let limit = row.max_points.map(|n| format!("; row limit {n}")).unwrap_or_default();
                field(
                    format!("{} ({} per patient{})", row.label, row.points, limit),
                    Input::new(input).aria_label(format!("PM3 count: {}", row.label)),
                    muted,
                )
            })
            .collect()
    }

    fn pm3_eligibility(&self, cx: &mut Context<Self>) -> Div {
        let confirmed = self.helpers.pm3.eligible;
        div().flex().child(
            tick("pm3-eligible", "I checked the PM3 conditions", confirmed, cx)
                .accessibility_label("I checked the PM3 conditions")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.helpers.pm3.eligible = !this.helpers.pm3.eligible;
                    cx.notify();
                })),
        )
    }

    fn pm3_apply(&self, evidence: EvidenceReading, reason: String, cx: &mut Context<Self>) -> Div {
        let label = format!("Apply PM3 {}", evidence.strength.label());
        div().child(
            Button::new("pm3-apply")
                .label(label.clone())
                .accessibility_label(format!("{label} to today's evidence"))
                .small()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.apply_reading(&evidence, reason.clone());
                    cx.notify();
                })),
        )
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;

    use super::pm3_outcome;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn outcome(typed: &[Option<Result<u32, ()>>], eligible: bool) -> super::Pm3Outcome {
        pm3_outcome(rules(), rules().pm3.as_ref().unwrap(), typed, eligible)
    }

    #[test]
    fn nothing_is_read_until_the_conditions_are_confirmed() {
        let rows = rules().pm3.as_ref().unwrap().rows.len();
        let typed = vec![Some(Ok(2)); rows];
        let result = outcome(&typed, false);
        assert!(result.message.starts_with("Confirm the conditions"));
        assert!(result.apply.is_none());
    }

    #[test]
    fn a_bad_entry_stops_the_reading_even_when_confirmed() {
        let rows = rules().pm3.as_ref().unwrap().rows.len();
        let mut typed = vec![None; rows];
        typed[0] = Some(Err(()));
        let result = outcome(&typed, true);
        assert!(result.message.starts_with("Use whole numbers"));
        assert!(result.apply.is_none());
    }

    #[test]
    fn enough_patients_give_evidence_and_a_reason_that_names_the_rows() {
        let table = rules().pm3.as_ref().unwrap();
        let typed: Vec<_> = (0..table.rows.len()).map(|_| Some(Ok(20))).collect();
        let result = outcome(&typed, true);
        let (evidence, reason) = result.apply.expect("many patients reach a level");
        assert_eq!(evidence.code, "PM3");
        assert!(reason.contains(&format!("20 x {}", table.rows[0].id)), "{reason}");
        assert!(reason.contains(&table.source), "{reason}");
    }

    #[test]
    fn no_patients_give_no_evidence() {
        let rows = rules().pm3.as_ref().unwrap().rows.len();
        let result = outcome(&vec![None; rows], true);
        assert!(result.apply.is_none());
    }
}
