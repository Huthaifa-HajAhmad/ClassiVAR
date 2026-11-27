//! The de novo patients helper: one total offered as PS2 or as PM6.

use super::*;
use classivar_core::{DeNovoPhenotype, DeNovoReading, DeNovoRules, EvidenceReading};

/// The typed counts of one table row: patients whose parents were confirmed, and not confirmed.
type TypedPair = [Option<Result<u32, ()>>; 2];

/// What the patient counts mean: a sentence for the curator, and the buttons that
/// can apply the total (as PS2, and as PM6 when this program allows it).
struct DeNovoOutcome {
    message: String,
    /// (button id, evidence); the same reason is saved for each.
    apply: Vec<(&'static str, EvidenceReading)>,
    reason: String,
}

/// Reads the typed counts (one pair for each row of the table; a row worth zero points is not read).
fn de_novo_outcome(
    rules: &RuleSet,
    table: &DeNovoRules,
    typed: &[TypedPair],
    recessive_without_second_variant: bool,
) -> DeNovoOutcome {
    let (counts, parts, unusable) = count_patients(table, typed);
    let none = |message: String| DeNovoOutcome { message, apply: Vec::new(), reason: String::new() };
    if unusable {
        return none("Use whole numbers of patients, for example 2.".to_string());
    }
    let Some(reading) = rules.read_de_novo(&counts, recessive_without_second_variant) else {
        return none("The rules have no de novo table that fits.".to_string());
    };
    if parts.is_empty() {
        return none("Type how many patients you found.".to_string());
    }
    let Some(evidence) = &reading.evidence else {
        return none(if reading.lowered_for_recessive {
            format!(
                "{} points: Supporting, lowered one level for the recessive condition, leaves no evidence.",
                reading.points
            )
        } else {
            format!(
                "{} points: below the lowest level ({}), so no evidence.",
                reading.points,
                table.thresholds.first().map_or(0.0, |c| c.at)
            )
        });
    };
    let message = total_message(&reading, evidence);
    let reason = format!(
        "De novo: {} = {} points{} ({})",
        parts.join(" + "),
        reading.points,
        if reading.lowered_for_recessive { ", lowered one level for a recessive condition" } else { "" },
        table.source
    );
    let apply = [("de-novo-apply", reading.evidence), ("de-novo-apply-pm6", reading.pm6)]
        .into_iter()
        .filter_map(|(id, evidence)| evidence.map(|evidence| (id, evidence)))
        .collect();
    DeNovoOutcome { message, apply, reason }
}

/// The patient counts of each row as numbers, what each box added in words (for the saved
/// reason), and whether a box holds something that is not a count.
fn count_patients(table: &DeNovoRules, typed: &[TypedPair]) -> (Vec<(u32, u32)>, Vec<String>, bool) {
    let mut counts = Vec::new();
    let mut parts = Vec::new();
    let mut unusable = false;
    for (row, pair) in table.phenotypes.iter().zip(typed) {
        if row.confirmed == 0.0 && row.unconfirmed == 0.0 {
            counts.push((0, 0));
            continue;
        }
        let mut numbers = [0u32; 2];
        for (slot, entry) in numbers.iter_mut().zip(pair) {
            match entry {
                None => {}
                Some(Ok(count)) => *slot = *count,
                Some(Err(())) => unusable = true,
            }
        }
        let [confirmed, unconfirmed] = numbers;
        for (count, kind) in [(confirmed, "parents confirmed"), (unconfirmed, "parents not confirmed")] {
            if count > 0 {
                parts.push(format!("{count} x {} ({kind})", row.id));
            }
        }
        counts.push((confirmed, unconfirmed));
    }
    (counts, parts, unusable)
}
/// The sentence for a total that reaches a level: the strength for PS2 or PM6, and the two
/// ways the result may differ from the table (the recessive rule, and a PM6 limit in a gene-specific rule file).
fn total_message(reading: &DeNovoReading, evidence: &EvidenceReading) -> String {
    let lowered = if reading.lowered_for_recessive { " Lowered one level for the recessive condition." } else { "" };
    let pm6_note = match &reading.pm6 {
        Some(pm6) if pm6.strength != evidence.strength => {
            format!(" These rules stop PM6 at {}; use PS2 for {}.", pm6.strength.label(), evidence.strength.label())
        }
        _ => String::new(),
    };
    format!(
        "{} points: {} for PS2, or for PM6 (Table 2 allows either code, not both).{lowered}{pm6_note}",
        reading.points,
        evidence.strength.label()
    )
}
fn points_text(points: f64) -> String {
    format!("{points} {}", if points == 1.0 { "point" } else { "points" })
}

impl Workbench {
    pub(in crate::view::panels) fn de_novo_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let panel = self.helper_start();
        let Some(table) = self.rules.de_novo.as_ref() else {
            return panel.into_any_element();
        };
        let typed: Vec<TypedPair> = self
            .helpers
            .de_novo
            .inputs
            .iter()
            .map(|pair| [self.typed_count(&pair[0], cx), self.typed_count(&pair[1], cx)])
            .collect();
        let outcome = de_novo_outcome(self.rules, table, &typed, self.helpers.de_novo.recessive);
        let citation = self.rules.source(&table.source).map(|s| s.citation.clone());
        let result = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(super::super::focus::reading(outcome.message.clone()))
            .when(!outcome.apply.is_empty(), |d| d.child(self.de_novo_apply(&outcome, cx)));
        panel
            .gap_6()
            .child(reading_note(
                "How many patients had this variant new (de novo)? Sort them by how well the phenotype fits the gene, and by whether the parental relationships were confirmed.",
                muted,
            ))
            .child(
                part("Your patients", cx)
                    .gap_4()
                    .children(table.phenotypes.iter().enumerate().map(|(i, row)| self.de_novo_row(i, row, muted)))
                    .child(self.de_novo_recessive_choice(table, cx)),
            )
            .child(result_card(result, cx))
            .children(notes_part(table.notes.iter().cloned(), cx))
            .children(source_part(citation, cx))
            .into_any_element()
    }

    /// One patient group: its label and the two count boxes. A group worth zero points has no boxes.
    fn de_novo_row(&self, i: usize, row: &DeNovoPhenotype, muted: Hsla) -> Div {
        if row.confirmed == 0.0 && row.unconfirmed == 0.0 {
            return div().child(caption(muted).child(format!("{}: 0 points. Do not count these patients.", row.label)));
        }
        let boxes = &self.helpers.de_novo.inputs[i];
        let limit = row.max_points.map(|max| format!("This row adds at most {max} points in total."));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.label.clone()))
            .child(fields([
                field(
                    format!("Parents confirmed · {} each", points_text(row.confirmed)),
                    Input::new(&boxes[0]).aria_label(format!("De novo patients, {} (parents confirmed)", row.id)),
                    muted,
                ),
                field(
                    format!("Not confirmed · {} each", points_text(row.unconfirmed)),
                    Input::new(&boxes[1]).aria_label(format!("De novo patients, {} (parents not confirmed)", row.id)),
                    muted,
                ),
            ]))
            .children(limit.map(|text| caption(muted).child(text)))
    }

    fn de_novo_recessive_choice(&self, table: &DeNovoRules, cx: &mut Context<Self>) -> Div {
        let chosen = self.helpers.de_novo.recessive;
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap_1()
            .child(
                tick("de-novo-recessive", "Recessive condition, no second variant found", chosen, cx)
                    .accessibility_label("Recessive condition, no second variant found")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.helpers.de_novo.recessive = !this.helpers.de_novo.recessive;
                        cx.notify();
                    })),
            )
            .child(caption(cx.theme().muted_foreground).whitespace_normal().child(table.recessive_text.clone()))
    }

    /// One Apply button for each code the total can be applied as.
    fn de_novo_apply(&self, outcome: &DeNovoOutcome, cx: &mut Context<Self>) -> Div {
        let mut buttons = div().flex().flex_wrap().gap_2();
        for (id, evidence) in &outcome.apply {
            let (evidence, reason) = (evidence.clone(), outcome.reason.clone());
            buttons = buttons.child(self.reading_apply(*id, evidence, reason, cx));
        }
        buttons
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{RuleSet, Strength};

    use super::{DeNovoOutcome, TypedPair, de_novo_outcome};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn outcome(first_row: TypedPair, recessive: bool) -> DeNovoOutcome {
        let table = rules().de_novo.as_ref().unwrap();
        let mut typed: Vec<TypedPair> = vec![[None, None]; table.phenotypes.len()];
        typed[0] = first_row;
        de_novo_outcome(rules(), table, &typed, recessive)
    }

    #[test]
    fn an_empty_form_asks_and_a_bad_entry_stops_the_reading() {
        assert!(outcome([None, None], false).message.starts_with("Type how many"));
        let bad = outcome([Some(Err(())), None], false);
        assert!(bad.message.starts_with("Use whole numbers"));
        assert!(bad.apply.is_empty());
    }

    #[test]
    fn confirmed_patients_in_the_most_specific_row_reach_a_level_offered_as_ps2_and_pm6() {
        let result = outcome([Some(Ok(2)), None], false);
        let codes: Vec<_> = result.apply.iter().map(|(_, e)| e.code).collect();
        assert_eq!(codes, vec!["PS2", "PM6"]);
        assert!(result.reason.starts_with("De novo: 2 x "), "{}", result.reason);
        assert!(result.message.contains("Table 2 allows either code, not both"), "{}", result.message);
    }

    #[test]
    fn a_very_strong_total_is_offered_at_very_strong_for_both_codes() {
        // Four points reach Very strong; SVI Table 2 lists PS2_VeryStrong or PM6_VeryStrong.
        let result = outcome([Some(Ok(4)), None], false);
        let ps2 = &result.apply[0].1;
        let pm6 = &result.apply[1].1;
        assert_eq!(ps2.strength, Strength::VeryStrong);
        assert_eq!(pm6.strength, Strength::VeryStrong);
        assert!(!result.message.contains("stop PM6"), "{}", result.message);
    }

    #[test]
    fn the_recessive_rule_lowers_one_level_and_the_reason_says_so() {
        let result = outcome([Some(Ok(2)), None], true);
        assert!(result.message.contains("Lowered one level for the recessive condition."), "{}", result.message);
        assert!(result.reason.contains("lowered one level for a recessive condition"), "{}", result.reason);
    }
}
