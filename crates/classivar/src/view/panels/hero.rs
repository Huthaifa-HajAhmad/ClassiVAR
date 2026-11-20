//! The result at the top of the explanation pane: the class, the points and the evidence track.

use super::*;

impl Workbench {
    /// Who this result is about, the class, the points and where they sit.
    pub(super) fn hero(&self, result: &Assessment, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        // The gene the evidence was checked against, not text still being typed.
        let gene = self.case_gene.clone();
        let scope = (self.mode == Mode::ReportedVariant).then(|| self.editing_label());
        let title = match variant {
            Some(name) => div().text_xl().font_weight(FontWeight::BOLD).child(name.to_string()),
            None if self.example => div().text_xl().font_weight(FontWeight::SEMIBOLD).child("Fictional case"),
            None => div().text_lg().text_color(muted).child("New assessment"),
        };
        let hero = div()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .flex()
            .flex_col()
            .gap_3()
            .when_some(scope, |d, scope| {
                d.child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("Result for {scope}")))
            })
            .child(title)
            .when(!gene.is_empty(), |d| d.child(div().text_sm().text_color(muted).child(gene)));
        match result.class() {
            Some(class) => self.hero_class(hero, result, class, cx),
            None if !result.conflicts.is_empty() => hero_conflicts(hero, result, muted),
            None if result.both_sides_met => hero_both_sides(hero, result),
            None if !result.combination_overuse.is_empty() => hero_overuse(hero, result),
            None => {
                let card = hero_empty(hero, result, muted);
                if result.items.is_empty() { card.child(self.category_picker(cx)) } else { card }
            }
        }
        .into_any_element()
    }

    fn category_picker(&self, cx: &mut Context<Self>) -> Div {
        let mut choices = div().flex().flex_wrap().gap_2();
        for category in Category::ALL {
            if self.rules.criteria.iter().any(|criterion| criterion.category == category) {
                choices = choices.child(
                    Button::new(SharedString::from(format!("browse-{}", category.label())))
                        .outline()
                        .label(category.label())
                        .selected(self.category == Some(category))
                        .toggled(self.category == Some(category))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.category = Some(category);
                            this.reading = false;
                            this.criteria_scroll.set_offset(point(px(0.), px(0.)));
                            cx.notify();
                        })),
                );
            }
        }
        choices
    }

    /// A class is shown: the class banner with its review status, the evidence
    /// track, the selected criterion, and how the 2015 table compares.
    fn hero_class(&self, hero: Div, result: &Assessment, class: Class, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let classic = classic_comparison(result);
        // Stand-alone evidence fixes the class; the points total alone would not.
        let stand_alone = result
            .items
            .iter()
            .any(|i| i.counted && i.direction == Direction::Benign && i.strength == Strength::StandAlone);
        hero.child(self.class_banner(result, class, cx))
            .child(self.evidence_track(result, class, cx))
            .child(self.inspector(result, cx))
            .child(div().text_sm().text_color(muted).child(classic))
            .when(stand_alone, |d| {
                d.child(caption(muted).child(
                    "Stand-alone benign evidence (BA1) fixes the class at Benign, whatever else is applied. The points total is shown for transparency.",
                ))
            })
            .when(class == Class::Vus, |d| d.child(self.vus_note(result.total_points, result.vus_tier.as_deref(), cx)))
    }

    /// The class in large type on its own colour, with the rule version and review status, and the total.
    fn class_banner(&self, result: &Assessment, class: Class, cx: &mut Context<Self>) -> AnyElement {
        let dark = cx.theme().is_dark();
        let sign = if result.total_points > 0 { "+" } else { "" };
        let total = format!("{sign}{}", super::point_axis::points_text(result.total_points));
        div()
            .flex()
            .items_center()
            .gap_5()
            .px_5()
            .py_4()
            .rounded_xl()
            .bg(class_fill(class, dark))
            .text_color(class_ink(class, dark))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(34.))
                            .line_height(px(40.))
                            .font_weight(FontWeight::BOLD)
                            .child(class.label()),
                    )
                    .when_some(result.vus_tier.clone(), |d, tier| d.child(div().text_base().child(tier)))
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(rules_tag(self.rules))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(div().text_size(px(40.)).line_height(px(44.)).font_weight(FontWeight::BOLD).child(total))
                    .child(div().text_xs().child(format!("point{}", plural(result.total_points.abs())))),
            )
            .with_animation(
                SharedString::from(format!("banner-{class:?}")),
                Animation::new(std::time::Duration::from_millis(320)).with_easing(ease_out_quint()),
                |el, t| el.opacity(0.35 + 0.65 * t),
            )
            .into_any_element()
    }

    /// Where in the VUS range the points are. The range comes from the rule set.
    pub(super) fn vus_note(&self, total: i32, tier: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let low = self.rules.class_min_points.vus;
        let high = self.rules.class_min_points.likely_pathogenic - 1;
        let tiers = self.rules.vus_tiers.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ");
        let tier_note = if let Some(tier) = tier {
            format!("The loaded rule file gives official point ranges for the sub-tiers; this variant falls in {tier}.")
        } else if self.rules.vus_tiers.is_empty() {
            "The loaded rule file defines no VUS sub-tiers.".to_string()
        } else {
            format!(
                "Sub-tiers ({tiers}) have no official point ranges yet, so ClassiVAR does not name one for new evidence."
            )
        };
        div()
            .text_sm()
            .text_color(muted)
            .child(format!(
                "VUS covers {low} to {high} points. This variant is at {total}: {} above the bottom and {} below Likely pathogenic. {tier_note}",
                total - low,
                self.rules.class_min_points.likely_pathogenic - total,
            ))
            .into_any_element()
    }
}

/// Criteria that contradict each other: no class until one of each pair is removed.
fn hero_conflicts(hero: Div, result: &Assessment, muted: Hsla) -> Div {
    hero.child(notice(
        "Class withheld: some criteria contradict each other. Remove one of each pair to see a class.",
        warning_fill(),
        warning_ink(),
    ))
    .children(
        result
            .conflicts
            .iter()
            .map(|c| div().text_sm().child(format!("• {} and {} cannot both apply. {}", c.a, c.b, c.reason))),
    )
    .child(caption(muted).child(format!("Points so far: {} (not a classification).", result.total_points)))
}

/// Evidence meets a pathogenic and a benign rule of the 2015 table: the points would hide the conflict.
fn hero_both_sides(hero: Div, result: &Assessment) -> Div {
    let side = |direction: Direction| {
        result
            .items
            .iter()
            .filter(|item| item.counted && item.direction == direction)
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let has_ba1 = result.items.iter().any(|item| item.counted && item.code == "BA1");
    hero.child(notice(
        "Needs expert judgment: this evidence meets a pathogenic rule and a benign rule at the same time.",
        warning_fill(),
        warning_ink(),
    ))
    .child(div().text_sm().child(format!(
        "Pathogenic side: {}. Benign side: {}. The 2015 guidance calls this uncertain (VUS). The points alone would add up to {} and say {}, which hides the conflict, so ClassiVAR shows no class. Check that each criterion really applies.",
        side(Direction::Pathogenic),
        side(Direction::Benign),
        result.total_points,
        result.points_class.label()
    )))
    .when(has_ba1, |card| {
        card.child(div().text_xs().child(
            "BA1 is stand-alone, but the pathogenic evidence here reaches a pathogenic rule on its own. A well-known pathogenic variant can be common (founder effect, reduced penetrance), which is why ClinGen keeps a BA1 exception list. Check that list before calling this Benign.",
        ))
    })
}

/// Two criteria together go over a published combined limit.
fn hero_overuse(hero: Div, result: &Assessment) -> Div {
    hero.child(notice(
        "Needs expert judgment: two criteria together exceed a published combined limit.",
        warning_fill(),
        warning_ink(),
    ))
    .children(result.combination_overuse.iter().map(|over| {
        div().text_sm().child(format!(
            "{} + {} are {} points together; the limit is {}. {} Lower one of them (or remove one) to get a class.",
            over.a, over.b, over.points, over.max_points, over.reason
        ))
    }))
}

/// Nothing counts yet, so there is no class to show.
fn hero_empty(hero: Div, result: &Assessment, muted: Hsla) -> Div {
    let small = if result.items.is_empty() {
        "Choose an evidence category to browse its criteria. A class appears once you apply evidence that counts."
    } else {
        "Only criteria that no longer count are applied. Apply at least one that counts."
    };
    hero.child(div().text_2xl().font_weight(FontWeight::BOLD).text_color(muted).child("No class yet"))
        .child(div().text_sm().text_color(muted).child(small))
}

/// How the 2015 combining table (with ClinGen's 2020 addition) compares with the points.
fn classic_comparison(result: &Assessment) -> String {
    match (result.classic_class, result.frameworks_agree()) {
        (Some(c), Some(true)) => format!("The 2015 table plus ClinGen's 2020 addition also gives {}.", c.label()),
        (Some(c), _) => format!(
            "The 2015 table plus ClinGen's 2020 addition gives {}. It differs from the points; the two methods are not identical.",
            c.label()
        ),
        (None, _) => "The 2015 table does not cover the strengths used.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{Applied, RuleSet, Strength};

    use super::classic_comparison;

    #[test]
    fn the_comparison_says_when_the_two_methods_agree_differ_or_do_not_apply() {
        let rules = RuleSet::builtin().unwrap();
        let agree = rules
            .assess(&[Applied::new("PM2", Strength::Supporting), Applied::new("PP3", Strength::Supporting)])
            .unwrap();
        assert!(classic_comparison(&agree).contains("also gives"), "{}", classic_comparison(&agree));
        // PVS1 alone is 8 points (Likely pathogenic) while the 2015 table calls it uncertain.
        let situation = rules.criterion("PVS1").unwrap().situation_for(Strength::VeryStrong).unwrap().to_string();
        let differ = rules.assess(&[Applied::guided("PVS1", Strength::VeryStrong, situation)]).unwrap();
        assert!(classic_comparison(&differ).contains("It differs from the points"), "{}", classic_comparison(&differ));
        let uncovered = rules.assess(&[Applied::new("BP4", Strength::Moderate)]).unwrap();
        assert!(classic_comparison(&uncovered).contains("does not cover the strengths used"));
    }
}
