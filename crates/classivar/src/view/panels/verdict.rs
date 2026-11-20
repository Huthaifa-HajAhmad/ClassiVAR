//! The verdict on a reported variant and the reasons behind it.

use super::*;
use classivar_core::Rescored;

impl Workbench {
    pub(super) fn verdict_card(
        &self,
        re: Option<&Reevaluation>,
        incomplete: &[String],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let card = div().flex().flex_col().gap_2().p_4().rounded_lg().border_1().border_color(border);
        let warn = |text: String| notice(text, warning_fill(), warning_ink());

        let Some(re) = re else {
            return card
                .child(warn(format!(
                    "Choose the class each record reported to get a verdict. Missing for: {}.",
                    incomplete.join(", ")
                )))
                .when_some(self.records.iter().position(|r| r.class.is_none()), |card, index| {
                    card.child(Button::new("edit-incomplete-report").small().label("Edit unfinished report").on_click(
                        cx.listener(move |this, _, window, cx| {
                            this.reading = false;
                            this.show_record(index, window, cx);
                            cx.notify();
                        }),
                    ))
                })
                .into_any_element();
        };
        let today = match &re.today {
            Ok(today) => today,
            Err(error) => {
                return card
                    .child(warn(format!("Today's evidence cannot be scored: {error}. Remove that criterion.")))
                    .into_any_element();
            }
        };
        let latest = re.latest_index.map(|i| &re.records[i]);
        let (Some(verdict), Some(latest), Some(today_class)) = (re.verdict, latest, today.class()) else {
            let hint = self.no_verdict_hint(today);
            return card.child(div().text_sm().text_color(muted).child(hint)).into_any_element();
        };

        let (symbol, title) = verdict_title(verdict);
        let detail = verdict_detail(verdict, latest, today, today_class);
        card.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_4()
                .child(div().text_xl().font_weight(FontWeight::BOLD).child(format!("{symbol}  {title}")))
                .child(class_badge(today_class, self.rules, "Today: ", border, cx.theme().is_dark())),
        )
        .child(div().text_sm().child(detail))
        .children(self.reported_tier_note(latest))
        .child(caption(muted).child("This is a candidate for you to review, not a decision. Check each change below."))
        .into_any_element()
    }

    /// Why there is no verdict even though every record has a class.
    fn no_verdict_hint(&self, today: &Assessment) -> &'static str {
        if self.records.is_empty() {
            "Add the earlier records, then apply your own evidence to get a verdict."
        } else if !today.conflicts.is_empty() {
            "Your evidence contradicts itself, so there is no class to compare. Remove one of the pair."
        } else if today.both_sides_met {
            "Your evidence meets a pathogenic rule and a benign rule at the same time. It needs expert judgment, so there is no class to compare."
        } else if !today.combination_overuse.is_empty() {
            "Two of your criteria together exceed a published combined limit. Lower or remove one of them; there is no class to compare."
        } else {
            "Apply your evidence, or start from the latest record's evidence, to get a verdict."
        }
    }

    /// What the VUS tier the latest record named means, with its source.
    fn reported_tier_note(&self, latest: &Rescored) -> Option<Div> {
        let tier =
            latest.reported_tier.as_deref().and_then(|name| self.rules.vus_tiers.iter().find(|t| t.name == name))?;
        let citation = self.rules.source(&tier.source).map_or(tier.source.as_str(), |s| s.citation.as_str());
        let ranges_note = if tiers_are_ranged(self.rules) {
            "The loaded rule file gives official point ranges to the tiers."
        } else {
            "ClassiVAR cannot say which tier today's evidence falls in: official point ranges for the tiers are not published yet."
        };
        Some(div().text_sm().child(format!("{} means: {} ({citation}) {ranges_note}", tier.name, tier.description)))
    }

    pub(super) fn reasons_list(&self, re: &Reevaluation, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        if re.reasons.is_empty() {
            return list
                .child(div().text_color(muted).child("No differences from the latest record."))
                .into_any_element();
        }
        let citation_of = |id: &str| self.rules.source(id).map_or(id.to_string(), |s| s.citation.clone());
        for reason in &re.reasons {
            let text = match reason {
                Reason::Added { code, strength } => {
                    format!("{code} added at {}: new evidence since the latest record.", strength.label())
                }
                Reason::Removed { code } => format!("{code} is no longer applied."),
                Reason::StrengthChanged { code, from, to } => {
                    format!("{code} changed from {} to {}.", from.label(), to.label())
                }
                Reason::RuleChanged { code, from, to, source } => format!(
                    "{code}: the record used {} (the strength in the 2015 paper). The current rule is {}. Source: {}",
                    from.label(),
                    to.label(),
                    citation_of(source)
                ),
                Reason::RuleSetChanged { from, to } => {
                    format!(
                        "The record used rules {} v{}. Today's rules are {} v{}.",
                        from.id, from.version, to.id, to.version
                    )
                }
                Reason::TierChanged { from, to } => format!("The record named {from}. Today's evidence falls in {to}."),
            };
            list = list.child(div().flex().gap_2().child("•").child(div().flex_1().min_w_0().child(text)));
        }
        list.into_any_element()
    }
}

/// The symbol and the headline of a verdict.
fn verdict_title(verdict: Verdict) -> (&'static str, &'static str) {
    match verdict {
        Verdict::Holds => ("=", "Candidate: the classification still holds"),
        Verdict::UpgradeCandidate => ("↑", "Upgrade candidate"),
        Verdict::DowngradeCandidate => ("↓", "Downgrade candidate"),
        Verdict::FrameworkDiffers => ("≠", "Same evidence, different class"),
        Verdict::RecordInconsistent => ("?", "The latest record does not fit its own evidence"),
    }
}

/// The sentence under the headline: what the latest record said and what today's evidence gives.
fn verdict_detail(verdict: Verdict, latest: &Rescored, today: &Assessment, today_class: Class) -> String {
    let latest_summary = format!(
        "The latest record ({}, {}) reported {}{}.",
        latest.label,
        latest.year,
        latest.reported_class.label(),
        latest.reported_tier.as_deref().map(|t| format!(" ({t})")).unwrap_or_default()
    );
    match verdict {
        Verdict::Holds | Verdict::UpgradeCandidate | Verdict::DowngradeCandidate => {
            format!("{latest_summary} Today's evidence gives {} points: {}.", today.total_points, today_class.label())
        }
        Verdict::FrameworkDiffers => format!(
            "{latest_summary} No evidence changed, but today's evidence scores {} ({} points). The record may have used the 2015 table, a method without points, or a criterion that no longer counts. This is not a reason to upgrade or downgrade.",
            today_class.label(),
            today.total_points
        ),
        Verdict::RecordInconsistent => {
            let own = latest.assessment.as_ref().ok();
            format!(
                "{latest_summary} Its own evidence scores {} by points and {} by the 2015 table, so a comparison would mean little. No upgrade or downgrade is suggested.",
                own.map_or("?", |a| a.points_class.label()),
                own.and_then(|a| a.classic_class).map_or("not defined", Class::label),
            )
        }
    }
}
