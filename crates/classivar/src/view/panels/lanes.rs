//! The history table's chips and the notes on a result.

use super::*;

impl Workbench {
    /// A chip for one criterion cell of the history table.
    pub(super) fn evidence_chip(
        &self,
        code: &str,
        strength: Option<Strength>,
        changed: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let cell = div().flex().items_center().gap_1().p_1().rounded_sm();
        let Some(strength) = strength else {
            return cell.text_color(muted).child("–").into_any_element();
        };
        let direction = self.rules.criterion(code).map_or(Direction::Pathogenic, |c| c.direction);
        let points = self.rules.magnitude(strength);
        let sign = if direction == Direction::Pathogenic { "+" } else { "−" };
        cell.when(changed, |d| d.border_2().border_color(crate::look::edge(cx)))
            .child(
                div()
                    .px_1()
                    .rounded_sm()
                    .bg(direction_colour(direction))
                    .text_color(direction_ink())
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .child(format!("{sign}{points}")),
            )
            .child(div().text_xs().child(strength.label()))
            .into_any_element()
    }

    /// Notes about the result. They are information, so they are quiet, not yellow.
    pub(super) fn notes(&self, result: &Assessment, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut texts: Vec<String> = Vec::new();
        if variant.is_none() {
            texts.push(
                "No variant is named. Name it at the top, so this result can be attributed to a variant.".to_string(),
            );
        }
        match self.gene_link {
            GeneDecision::Established => {}
            GeneDecision::NotDecided => texts.push(
                "You have not confirmed that this gene causes the disease. ACMG/AMP criteria assume it does.".to_string(),
            ),
            GeneDecision::NotEstablished => texts.push(
                "You marked the gene-disease link as doubtful. ACMG/AMP criteria do not apply without it; treat any class with great care.".to_string(),
            ),
        }
        texts.extend(result.warnings.iter().map(|w| warning_text(w, |s| self.rules.magnitude(s))));
        if texts.is_empty() {
            return div().text_sm().text_color(muted).child("No notes.").into_any_element();
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_sm()
            .children(texts.into_iter().map(|text| note_row(text, muted)))
            .into_any_element()
    }
}

/// One note: a small "!" mark and the text.
fn note_row(text: String, muted: Hsla) -> Div {
    div()
        .flex()
        .gap_2()
        .child(
            div()
                .w(px(18.))
                .h(px(18.))
                .flex_none()
                .rounded_full()
                .border_1()
                .border_color(muted)
                .text_xs()
                .text_color(muted)
                .flex()
                .items_center()
                .justify_center()
                .child("!"),
        )
        .child(div().flex_1().min_w_0().child(text))
}

/// The sentence for one warning of the engine. `magnitude` gives the points of a strength.
fn warning_text(warning: &Warning, magnitude: impl Fn(Strength) -> i32) -> String {
    match warning {
        Warning::ConflictingEvidence => {
            "Evidence points both ways. Check that each criterion really applies.".to_string()
        }
        Warning::Discouraged { code, note, counted } => format!(
            "{code} is no longer recommended by the experts. {}{}",
            if *counted {
                "It is counted here because this is an earlier record."
            } else {
                "It is listed but not counted."
            },
            note.as_deref().map(|n| format!(" {n}")).unwrap_or_default()
        ),
        Warning::StrengthChanged { code, from, to, reason } => {
            let why = match reason {
                Some(reason) => format!(" Reason: {reason}"),
                None if magnitude(*to) > magnitude(*from) => {
                    " No reason is recorded; click the strength to raise it again with one.".to_string()
                }
                None => String::new(),
            };
            format!("{code} strength changed from {} to {}.{why}", from.label(), to.label())
        }
        Warning::DoubleCounted { a, b, reason } => format!("{a} and {b} may count the same observation. {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{Strength, Warning};

    use super::warning_text;

    fn points(strength: Strength) -> i32 {
        match strength {
            Strength::Supporting => 1,
            Strength::Moderate => 2,
            Strength::Strong => 4,
            Strength::VeryStrong => 8,
            Strength::StandAlone => 8,
        }
    }

    #[test]
    fn a_raised_strength_without_a_reason_says_so_and_a_lowered_one_does_not() {
        let raised = Warning::StrengthChanged {
            code: "PM2".into(),
            from: Strength::Supporting,
            to: Strength::Moderate,
            reason: None,
        };
        let text = warning_text(&raised, points);
        assert!(text.contains("PM2 strength changed from Supporting to Moderate."), "{text}");
        assert!(text.contains("No reason is recorded"), "{text}");
        let lowered = Warning::StrengthChanged {
            code: "PP3".into(),
            from: Strength::Strong,
            to: Strength::Supporting,
            reason: None,
        };
        assert!(!warning_text(&lowered, points).contains("No reason"));
    }

    #[test]
    fn a_discouraged_criterion_says_whether_it_was_counted() {
        let counted = Warning::Discouraged { code: "PP5".into(), note: None, counted: true };
        let not_counted =
            Warning::Discouraged { code: "PP5".into(), note: Some("Use ClinVar evidence.".into()), counted: false };
        assert!(warning_text(&counted, points).contains("counted here because this is an earlier record"));
        let text = warning_text(&not_counted, points);
        assert!(text.contains("listed but not counted") && text.contains("Use ClinVar evidence."), "{text}");
    }
}
