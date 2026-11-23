//! A criterion's strength guide as a page (PVS1: Abou Tayoun 2018). Step 1 asks
//! about the gene when the rules have that question; step 2 asks which situation
//! fits the variant. Choosing an answer applies nothing: the bar at the foot names
//! the outcome (strength and points, or "does not apply"), and Apply records it.

use classivar_core::rules::StrengthGuide;

use super::focus::{Page, choice_card, criterion_heading, criterion_kicker, page_part, reading, tagged};
use super::point_axis::signed;
use super::*;

/// What the answers so far give. `None`: not decided yet. `Some(None)`: the criterion
/// does not apply. `Some(Some(strength))`: it applies at that strength.
pub(super) fn guide_outcome(c: &Criterion, gene_level: Option<&str>, pick: Option<&str>) -> Option<Option<Strength>> {
    if !c.gene_level.is_empty() {
        let level = c.gene_level_option(gene_level?)?;
        if level.steps.is_none() {
            return Some(None);
        }
    }
    Some(c.strength_for(pick?, gene_level))
}

/// The words on the Apply button: "Apply PVS1 · Strong (+4 points)", or that it does not apply.
pub(super) fn apply_words(code: &str, outcome: Option<Strength>, points: i32) -> String {
    match outcome {
        Some(strength) => format!("Apply {code} · {} ({} points)", strength.label(), signed(points)),
        None => format!("Record that {code} does not apply"),
    }
}

impl Workbench {
    /// The points a strength adds, negative for a benign criterion.
    pub(super) fn signed_points(&self, c: &Criterion, strength: Strength) -> i32 {
        let size = self.rules.magnitude(strength);
        if c.direction == Direction::Benign { -size } else { size }
    }

    pub(super) fn guide_page(&self, c: &'static Criterion, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let asks_gene = !c.gene_level.is_empty();
        let back = self.back_button(
            (format!("guide-back-{}", c.code), format!("guide-back-{}", c.code)),
            format!("Close the {} strength guide", c.code),
            |this| this.close_guide(),
            cx,
        );
        let intro = if asks_gene {
            format!(
                "Two questions set the strength of {}: one about the gene, then one about the variant. Nothing is applied until you press Apply.",
                c.code
            )
        } else {
            format!("One question sets the strength of {}. Nothing is applied until you press Apply.", c.code)
        };
        let mut body = div().flex().flex_col().gap_6();
        if asks_gene {
            body = body.child(self.gene_step(c, cx));
        }
        body = body.child(self.situation_step(c, asks_gene, cx)).children(self.guide_sources(c, cx));
        let page = Page {
            id: format!("guide-page-{}", c.code),
            back,
            kicker: criterion_kicker(c, Some("Strength guide")),
            title: criterion_heading(c),
            intro: Some(intro),
            body,
            footer: Some(self.guide_footer(c, cx)),
        };
        self.focus_page(width, page, cx)
    }

    /// Step 1: is loss of function how the gene causes the disease? (Abou Tayoun 2018, Table 1)
    /// Once answered, it folds to the chosen answer with Change, so step 2 comes into view.
    fn gene_step(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Div {
        let mut step = page_part("Step 1 · The gene", cx)
            .child(reading("Is loss of function how this gene causes the disease?").font_weight(FontWeight::SEMIBOLD));
        let answered = self.guide_gene_level.as_deref();
        for (i, level) in c.gene_level.iter().enumerate() {
            let chosen = answered == Some(level.id.as_str());
            if answered.is_some() && !chosen {
                continue;
            }
            let gives = match level.steps {
                None => format!("{} does not apply", c.code),
                Some(0) => "Full strength".to_string(),
                Some(n) => format!("{n} level{} lower", if n == 1 { "" } else { "s" }),
            };
            let id = level.id.clone();
            let card = choice_card(
                format!("gene-level-{}-{i}", c.code),
                (level.short.clone(), level.situation.clone(), gives),
                chosen,
                cx,
            )
            .accessibility_label(format!("{} gene level: {}", c.code, level.short))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.guide_gene_level = Some(id.clone());
                cx.notify();
            }));
            step = step.child(tagged(format!("gene-level-option-{}-{i}", c.code), card));
        }
        if answered.is_some() {
            let change = Button::new(SharedString::from(format!("gene-level-change-{}", c.code)))
                .label("Change the gene's answer")
                .accessibility_label(format!("Change the {} gene-level answer", c.code))
                .small()
                .ghost()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.guide_gene_level = None;
                    cx.notify();
                }));
            step = step.child(div().flex().child(change));
        }
        step
    }

    /// Step 2 (or the only step): which situation fits the variant.
    fn situation_step(&self, c: &'static Criterion, asks_gene: bool, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let label = if asks_gene { "Step 2 · The variant" } else { "The variant" };
        let step =
            page_part(label, cx).child(reading("Which of these fits the variant?").font_weight(FontWeight::SEMIBOLD));
        let level = self.guide_gene_level.as_deref().and_then(|id| c.gene_level_option(id));
        if asks_gene && level.is_none() {
            return step.child(
                reading("Answer step 1 first: the gene's answer sets how far each strength here is lowered.")
                    .text_color(muted),
            );
        }
        if level.is_some_and(|level| level.steps.is_none()) {
            return step.child(
                reading(format!(
                    "{} does not apply for this gene, whatever the variant. Press the button below to record that.",
                    c.code
                ))
                .text_color(muted),
            );
        }
        let cards: Vec<Div> =
            c.guide.iter().enumerate().map(|(i, option)| self.situation_card(c, i, option, cx)).collect();
        step.children(cards)
    }

    /// One situation as a card, with the strength it gives after the gene-level answer.
    fn situation_card(&self, c: &'static Criterion, i: usize, option: &StrengthGuide, cx: &mut Context<Self>) -> Div {
        let strength = c.strength_for(&option.id, self.guide_gene_level.as_deref());
        let gives = strength
            .map_or("Does not apply".to_string(), |s| format!("{} · {}", s.label(), signed(self.signed_points(c, s))));
        let chosen = self.guide_pick.as_deref() == Some(option.id.as_str());
        let id = option.id.clone();
        let card = choice_card(
            format!("guide-{}-{i}", c.code),
            (option.short.clone(), option.situation.clone(), gives),
            chosen,
            cx,
        )
        .accessibility_label(format!(
            "{} {}: {}",
            c.code,
            strength.map_or("does not apply", Strength::label),
            option.short
        ))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.guide_pick = Some(id.clone());
            cx.notify();
        }));
        tagged(format!("guide-option-{}-{i}", c.code), card)
    }

    /// Where the guide's questions come from.
    fn guide_sources(&self, c: &Criterion, cx: &App) -> Option<Div> {
        let mut ids: Vec<&str> =
            [c.gene_level_source.as_deref(), c.guide_source.as_deref()].into_iter().flatten().collect();
        ids.dedup();
        let citations: Vec<String> =
            ids.iter().filter_map(|id| self.rules.source(id)).map(|s| s.citation.clone()).collect();
        (!citations.is_empty()).then(|| {
            let muted = cx.theme().muted_foreground;
            page_part("Source", cx).children(citations.into_iter().map(move |text| reading(text).text_color(muted)))
        })
    }

    /// The bar at the foot: Cancel, and Apply once the answers decide the outcome.
    fn guide_footer(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let outcome = guide_outcome(c, self.guide_gene_level.as_deref(), self.guide_pick.as_deref());
        let label = match outcome {
            Some(result) => apply_words(&c.code, result, result.map_or(0, |s| self.signed_points(c, s))),
            None => format!("Apply {}", c.code),
        };
        let (code, situation) = (c.code.clone(), self.guide_pick.clone().unwrap_or_default());
        let apply = Button::new(SharedString::from(format!("guide-apply-{}", c.code)))
            .label(label.clone())
            .accessibility_label(label)
            .primary()
            .disabled(outcome.is_none())
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(result) = outcome {
                    this.answer_guide(&code, result, situation.clone());
                }
                cx.notify();
            }));
        let cancel = Button::new(SharedString::from(format!("guide-cancel-{}", c.code)))
            .label("Cancel")
            .accessibility_label(format!("Close the {} strength guide without answering", c.code))
            .ghost()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_guide();
                cx.notify();
            }));
        let waiting = match outcome {
            None => "Choose an answer to see what it gives.",
            Some(_) => "Check the answer, then apply it.",
        };
        div()
            .child(caption(muted).flex_1().min_w_0().child(waiting))
            .child(cancel)
            .child(tagged(format!("guide-apply-{}", c.code), apply))
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_words, guide_outcome};
    use classivar_core::{RuleSet, Strength};

    fn pvs1() -> &'static classivar_core::rules::Criterion {
        RuleSet::builtin().expect("the built-in rules are valid").criterion("PVS1").expect("PVS1 exists")
    }

    #[test]
    fn nothing_is_decided_until_both_steps_are_answered() {
        assert_eq!(guide_outcome(pvs1(), None, Some("nmd")), None, "the gene step comes first");
        assert_eq!(guide_outcome(pvs1(), Some("established"), None), None);
        assert_eq!(guide_outcome(pvs1(), Some("established"), Some("nmd")), Some(Some(Strength::VeryStrong)));
        assert_eq!(guide_outcome(pvs1(), Some("lower_one"), Some("nmd")), Some(Some(Strength::Strong)));
    }

    #[test]
    fn no_loss_of_function_mechanism_decides_at_once_that_it_does_not_apply() {
        assert_eq!(guide_outcome(pvs1(), Some("no_lof_mechanism"), None), Some(None));
    }

    #[test]
    fn the_button_names_the_strength_and_signed_points() {
        assert_eq!(apply_words("PVS1", Some(Strength::Strong), 4), "Apply PVS1 · Strong (+4 points)");
        assert_eq!(apply_words("BS4", Some(Strength::Strong), -4), "Apply BS4 · Strong (−4 points)");
        assert_eq!(apply_words("PVS1", None, 0), "Record that PVS1 does not apply");
    }
}
