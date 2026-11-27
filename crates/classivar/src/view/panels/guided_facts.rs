//! Optional variant-context fact questions and the sourced hints their answers add
//! to guided review steps. Facts and hints are words only: they never apply, exclude
//! or score criteria, and every step stays open.

use std::collections::BTreeMap;

use classivar_core::rules::{Fact, Hint, ReviewStep};

use super::focus::{reading, tagged};
use super::guided_page::step_name;
use super::*;

/// Capitalize a fact answer for display without altering clinical symbols:
/// "in-frame indel" -> "In-frame indel", "canonical splice (±1,2)" -> "Canonical splice (±1,2)".
pub(super) fn answer_label(answer: &str) -> String {
    let mut chars = answer.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// One-line summary of answered facts, or a short prompt when none are answered yet.
pub(super) fn facts_summary(facts: &[Fact], answers: &BTreeMap<String, String>) -> String {
    let parts: Vec<String> = facts
        .iter()
        .filter_map(|fact| {
            let chosen = answers.get(&fact.id).filter(|a| fact.lists(a))?;
            Some(format!("{}: {}", step_name(&fact.id), answer_label(chosen)))
        })
        .collect();
    if parts.is_empty() {
        "Optional: answers add sourced hints and never score evidence.".to_string()
    } else {
        parts.join(" · ")
    }
}

impl Workbench {
    /// The collapsible optional variant-context card above the open step.
    pub(super) fn guided_facts_card(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.rules.facts.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        let summary = facts_summary(&self.rules.facts, &self.review.facts);
        let any = self.rules.facts.iter().any(|f| self.review.facts.get(&f.id).is_some_and(|a| f.lists(a)));
        let label = match (self.facts_open, any) {
            (true, _) => "Hide",
            (false, true) => "Edit",
            (false, false) => "Answer",
        };
        let toggle = Button::new("guided-facts-toggle")
            .label(label)
            .accessibility_label(if self.facts_open {
                "Hide optional variant context questions"
            } else {
                "Answer optional variant context questions"
            })
            .small()
            .outline()
            .on_click(cx.listener(|this, _, _, cx| {
                this.facts_open = !this.facts_open;
                cx.notify();
            }));
        let head = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(crate::look::eyebrow("Variant context (optional)", cx))
                    .child(caption(muted).whitespace_normal().child(summary)),
            )
            .child(tagged("guided-facts-toggle".into(), toggle));
        let mut card = crate::look::card(cx).flex().flex_col().gap_3().child(head);
        if self.facts_open {
            card =
                card.child(caption(muted).whitespace_normal().child(
                    "Optional. Answers add hints to the steps below; they never apply, exclude or score criteria.",
                ));
            for fact in &self.rules.facts {
                card = card.child(self.fact_question(fact, cx));
            }
        }
        Some(card)
    }

    fn fact_question(&self, fact: &'static Fact, cx: &mut Context<Self>) -> Div {
        let chosen = self.review.facts.get(&fact.id).map(String::as_str);
        let mut chips = div().flex().flex_wrap().gap_1p5();
        for (i, answer) in fact.answers.iter().enumerate() {
            let active = chosen == Some(answer.as_str());
            let id = format!("guided-fact-{}-{i}", fact.id);
            let fact_id = fact.id.clone();
            let value = answer.clone();
            let label = answer_label(answer);
            let button = crate::look::segment(id.clone(), label.clone(), active, cx)
                .border_1()
                .border_color(if active { crate::look::edge(cx) } else { cx.theme().border })
                .accessibility_label(format!("{}: {label}", fact.question))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = (!active).then(|| value.clone());
                    this.set_fact(&fact_id, next);
                    cx.notify();
                }));
            chips = chips.child(tagged(id, button));
        }
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(fact.question.clone()))
            .child(chips)
    }

    /// Matching hints for the open step, shown under the step's question.
    pub(super) fn step_hints(&self, step: &'static ReviewStep, cx: &App) -> Option<Div> {
        let hints = step.matching_hints(&self.review.facts);
        if hints.is_empty() {
            return None;
        }
        let selector = format!("guided-hints-{}", step.id);
        let items = hints.into_iter().map(|hint| self.hint_item(hint, cx));
        Some(
            crate::look::card(cx)
                .debug_selector(move || selector)
                .border_1()
                .border_color(cx.theme().border)
                .flex()
                .flex_col()
                .gap_2()
                .child(crate::look::eyebrow("Hints · guidance only (does not score)", cx))
                .children(items),
        )
    }

    fn hint_item(&self, hint: &Hint, cx: &App) -> Div {
        let muted = cx.theme().muted_foreground;
        let source = self
            .rules
            .source(&hint.source)
            .map(|s| s.citation.split('.').next().unwrap_or(&s.citation).trim())
            .unwrap_or(&hint.source);
        let note = format!("{}: {} · {}", step_name(&hint.fact), answer_label(&hint.answer), source);
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(reading(hint.text.clone()))
            .child(caption(muted).whitespace_normal().child(note))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use classivar_core::RuleSet;

    use super::{answer_label, facts_summary};

    #[test]
    fn answer_labels_and_summary_read_cleanly_and_ignore_unlisted_answers() {
        assert_eq!(answer_label("in-frame indel"), "In-frame indel");
        assert_eq!(answer_label("unknown"), "Unknown");
        let rules = RuleSet::builtin().unwrap();
        let mut answers = BTreeMap::new();
        assert!(facts_summary(&rules.facts, &answers).contains("never score evidence"));
        answers.insert("consequence".into(), "missense".into());
        answers.insert("splicing".into(), "unlisted answer".into());
        assert_eq!(facts_summary(&rules.facts, &answers), "Consequence: Missense");
        answers.insert("splicing".into(), "predicted splicing impact".into());
        assert_eq!(
            facts_summary(&rules.facts, &answers),
            "Consequence: Missense · Splicing: Predicted splicing impact"
        );
    }
}
