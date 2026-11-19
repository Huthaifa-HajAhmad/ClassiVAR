//! The result at the top of the right pane: the class, the points and the scale.

use super::*;
use crate::view::describe::Scale;

impl Workbench {
    /// Who this result is about, the class, the points and where they sit.
    pub(super) fn hero(&self, result: &Assessment, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        // The gene the evidence was checked against, not text still being typed.
        let gene = self.case_gene.clone();
        let scope = (self.mode == Mode::ReportedVariant).then(|| self.editing_label());
        let title = match variant {
            Some(name) => div().text_xl().font_weight(FontWeight::BOLD).child(name.to_string()),
            None => div().text_xl().text_color(muted).child("Unnamed variant"),
        };
        let hero = div()
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
            None => hero_empty(hero, result, muted),
        }
        .into_any_element()
    }

    /// A class is shown: the badge with its review status, the points, the scale, and how the 2015 table compares.
    fn hero_class(&self, hero: Div, result: &Assessment, class: Class, cx: &mut Context<Self>) -> Div {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let dark = cx.theme().is_dark();
        let sign = if result.total_points > 0 { "+" } else { "" };
        let classic = classic_comparison(result);
        // Stand-alone evidence fixes the class; the points total alone would not.
        let stand_alone = result
            .items
            .iter()
            .any(|i| i.counted && i.direction == Direction::Benign && i.strength == Strength::StandAlone);
        hero.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_5()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .px_4()
                        .py_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(border)
                        .bg(class_fill(class, dark))
                        .text_color(class_ink(class, dark))
                        .child(div().text_2xl().font_weight(FontWeight::BOLD).child(class.label()))
                        .child(div().text_xs().child(rules_tag(self.rules))),
                )
                .child(div().text_2xl().font_weight(FontWeight::BOLD).child(format!(
                    "{sign}{} point{}",
                    result.total_points,
                    plural(result.total_points.abs())
                )))
                .when_some(result.vus_tier.clone(), |row, tier| row.child(div().text_lg().child(tier))),
        )
        .child(self.points_scale(result.total_points, class, cx))
        .child(div().text_sm().text_color(muted).child(classic))
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

    pub(super) fn points_scale(&self, total: i32, active: Class, cx: &mut Context<Self>) -> AnyElement {
        let foreground = cx.theme().foreground;
        let scale = scale_for(self.rules);
        let x_of = |points: i32| (points - scale.min) as f32 * SCALE_UNIT;
        let marker_x = x_of(total.clamp(scale.min, scale.max - 1)) + SCALE_UNIT / 2.0 - 7.0;
        let width = (scale.max - scale.min) as f32 * SCALE_UNIT;
        div()
            .id("scale-scroll")
            .overflow_x_scroll()
            .child(
                div()
                    .relative()
                    .w(px(width))
                    .h(px(74.))
                    .child(div().absolute().top(px(0.)).left(px(marker_x)).text_color(foreground).child("▼"))
                    .child(scale_bar(&scale, active, cx.theme().is_dark(), foreground))
                    .child(scale_ticks(&scale, width, cx.theme().muted_foreground)),
            )
            .into_any_element()
    }
}

/// The five class zones side by side, the active class outlined.
fn scale_bar(scale: &Scale, active: Class, dark: bool, foreground: Hsla) -> Div {
    let mut bar = div().flex().absolute().top(px(18.)).left(px(0.));
    for (class, from, to) in &scale.zones {
        let is_active = *class == active;
        let fill = class_fill(*class, dark);
        bar = bar.child(
            div()
                .w(px((to - from) as f32 * SCALE_UNIT))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .bg(fill)
                .text_color(class_ink(*class, dark))
                .text_xs()
                .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .border_2()
                .border_color(if is_active { foreground } else { fill.into() })
                .child(class.short()),
        );
    }
    bar
}

/// The point value where each zone after the first starts.
fn scale_ticks(scale: &Scale, width: f32, muted: Hsla) -> Div {
    let mut ticks = div().absolute().top(px(54.)).left(px(0.)).w(px(width));
    for edge in scale.zones.iter().skip(1).map(|(_, from, _)| *from) {
        ticks = ticks.child(
            div()
                .absolute()
                .left(px((edge - scale.min) as f32 * SCALE_UNIT - 10.))
                .w(px(20.))
                .flex()
                .justify_center()
                .text_xs()
                .text_color(muted)
                .child(edge.to_string()),
        );
    }
    ticks
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
        "Apply criteria on the left. A class appears once at least one criterion counts."
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
