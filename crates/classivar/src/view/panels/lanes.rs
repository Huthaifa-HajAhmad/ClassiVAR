//! What the result is made of, what would change it, and the notes.

use super::*;
use classivar_core::Item;

impl Workbench {
    pub(super) fn evidence_lanes(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let mut lanes = div().flex().flex_col();
        for category in Category::ALL {
            let items: Vec<_> = result
                .items
                .iter()
                .filter(|i| self.rules.criterion(&i.code).is_some_and(|c| c.category == category))
                .collect();
            let mut track = div().flex().flex_wrap().items_center().gap_1().min_h(px(30.)).flex_1();
            if items.is_empty() {
                track = track.child(caption(muted).child("none"));
            }
            for item in items {
                track = lane_item(track, item, muted);
            }
            lanes = lanes.child(
                div()
                    .flex()
                    .items_center()
                    .py_1()
                    .border_t_1()
                    .border_color(border)
                    .child(div().w(px(120.)).flex_none().text_xs().text_color(muted).child(category.label()))
                    .child(track),
            );
        }
        lanes.into_any_element()
    }

    /// A chip for one criterion cell of the history table.
    pub(super) fn evidence_chip(
        &self,
        code: &str,
        strength: Option<Strength>,
        changed: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (muted, foreground) = (cx.theme().muted_foreground, cx.theme().foreground);
        let cell = div().flex().items_center().gap_1().p_1().rounded_sm();
        let Some(strength) = strength else {
            return cell.text_color(muted).child("–").into_any_element();
        };
        let direction = self.rules.criterion(code).map_or(Direction::Pathogenic, |c| c.direction);
        let points = self.rules.magnitude(strength);
        let sign = if direction == Direction::Pathogenic { "+" } else { "−" };
        cell.when(changed, |d| d.border_2().border_color(foreground))
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

    pub(super) fn what_changes(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        if let Some(next) = result.to_upgrade {
            list = list.child(format!(
                "Up: {} more point{} of pathogenic evidence {} {}.",
                next.points_needed,
                plural(next.points_needed),
                if next.points_needed == 1 { "reaches" } else { "reach" },
                next.class.label()
            ));
        }
        if let Some(next) = result.to_downgrade {
            list = list.child(format!(
                "Down: {} more point{} of benign evidence {} {}.",
                next.points_needed,
                plural(next.points_needed),
                if next.points_needed == 1 { "reaches" } else { "reach" },
                next.class.label()
            ));
        }
        list.child(caption(muted).child("The evidence that could move it is in the lanes marked none."))
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

/// One criterion in its lane: a bar as wide as its points, then its sign and strength.
/// A criterion that is listed but not counted is only named.
fn lane_item(track: Div, item: &Item, muted: Hsla) -> Div {
    if !item.counted {
        return track.child(caption(muted).pr_2().child(format!(
            "{} {}: listed, not counted",
            item.code,
            item.strength.label()
        )));
    }
    let width = (item.points.unsigned_abs() as f32 * POINT_WIDTH).max(POINT_WIDTH);
    let sign = if item.points > 0 { "+" } else { "−" };
    track
        .child(
            div()
                .w(px(width))
                .h(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_r_md()
                .bg(direction_colour(item.direction))
                .text_color(direction_ink())
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .child(item.code.clone()),
        )
        .child(caption(muted).pr_2().child(format!("{sign}{} {}", item.points.unsigned_abs(), item.strength.label())))
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
