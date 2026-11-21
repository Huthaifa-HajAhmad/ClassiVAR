//! The result at the top of the explanation pane: the class, the points and the evidence track.

use super::*;

impl Workbench {
    /// Who this result is about, the class, the points and where they sit.
    pub(super) fn hero(&self, result: &Assessment, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        // The gene the evidence was checked against, not text still being typed.
        let gene = self.case_gene.clone();
        let title = match variant {
            Some(name) => name.to_string(),
            None if self.example => "Fictional case".to_string(),
            None => "New assessment".to_string(),
        };
        // With the rail folded away, the case name moves here as one small line. A
        // withheld result has no readout, so it says here whose evidence it is.
        let rail = self.stage.rail_shown.get();
        let header = div()
            .flex()
            .flex_wrap()
            .items_baseline()
            .gap_3()
            .when(!rail, |d| d.child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(title)))
            .when(!rail && !gene.is_empty(), |d| d.child(div().text_sm().text_color(muted).child(gene)))
            .when(result.class().is_none(), |d| d.child(crate::look::eyebrow(self.scope_words(), cx)));
        let hero = div().flex().flex_col().gap_4().child(header);
        match result.class() {
            Some(class) => self.hero_class(hero, result, class, cx),
            None if !result.conflicts.is_empty() => hero_conflicts(hero, result, muted),
            None if result.both_sides_met => hero_both_sides(hero, result),
            None if !result.combination_overuse.is_empty() => hero_overuse(hero, result),
            None if result.items.is_empty() => {
                let room = (self.stage.width.get(), self.stage.height.get());
                let dial = div().flex().justify_center().child(self.dial(result, None, room, false, cx));
                hero_empty(hero.child(dial), result, muted).child(self.category_picker(cx))
            }
            None => hero_empty(hero, result, muted),
        }
        .into_any_element()
    }

    fn category_picker(&self, cx: &mut Context<Self>) -> Div {
        let mut choices = div().flex().flex_wrap().justify_center().gap_2();
        for category in Category::ALL {
            if self.rules.criteria.iter().any(|criterion| criterion.category == category) {
                choices = choices.child(
                    Button::new(SharedString::from(format!("browse-{}", category.label())))
                        .outline()
                        .rounded(px(999.))
                        .label(category.label())
                        .selected(self.category == Some(category))
                        .toggled(self.category == Some(category))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.category = Some(category);
                            this.reading = false;
                            this.handles.criteria_scroll.set_offset(point(px(0.), px(0.)));
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
        // Stand-alone evidence fixes the class; the points total alone would not.
        let stand_alone = result
            .items
            .iter()
            .any(|i| i.counted && i.direction == Direction::Benign && i.strength == Strength::StandAlone);
        hero.child(self.signature(result, class, cx))
            .child(self.inspector(result, cx))
            .child(self.insights(result, class, cx))
            .when(stand_alone, |d| {
                d.child(caption(muted).child(
                    "Stand-alone benign evidence (BA1) fixes the class at Benign, whatever else is applied. The points total is shown for transparency.",
                ))
            })
            .when(class == Class::Vus, |d| d.child(self.vus_note(result.total_points, result.vus_tier.as_deref(), cx)))
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
    hero.child(
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .child(crate::look::display().text_color(muted).child("No class yet"))
            .child(div().text_sm().text_center().text_color(muted).child(small)),
    )
}

/// How the 2015 combining table (with its later additions) compares with the points.
pub(super) fn classic_comparison(result: &Assessment) -> String {
    match (result.classic_class, result.frameworks_agree()) {
        (Some(c), Some(true)) => format!("The 2015 table (with its later additions) also gives {}.", c.label()),
        (Some(c), _) => format!(
            "The 2015 table (with its later additions) gives {}. It differs from the points; the two methods are not identical.",
            c.label()
        ),
        (None, _) => "The 2015 table gives no class while the evidence contradicts itself.".to_string(),
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
        let differ = rules
            .assess(&[Applied::guided("PVS1", Strength::VeryStrong, situation).with_gene_level("established")])
            .unwrap();
        assert!(classic_comparison(&differ).contains("It differs from the points"), "{}", classic_comparison(&differ));
        let contradiction = rules
            .assess(&[Applied::new("PP3", Strength::Supporting), Applied::new("BP4", Strength::Supporting)])
            .unwrap();
        assert!(
            classic_comparison(&contradiction).contains("contradicts itself"),
            "{}",
            classic_comparison(&contradiction)
        );
    }
}
