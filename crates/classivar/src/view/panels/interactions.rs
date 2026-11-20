//! How one criterion works with the others, shown under "More": the criteria it
//! cannot be counted with, and the pairs whose points are limited together. Every
//! line comes from the rule file's own tables, with its source, so the screen
//! adds no clinical text of its own.
//!
//! The lines are plain data built by `interactions`, so tests need no window.

use classivar_core::{Criterion, RuleSet};

use super::*;

/// One fact about a criterion and another one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Interaction {
    other: String,
    /// `None`: the two are not counted together. `Some(points)`: together they
    /// may add up to at most this many points.
    limit_points: Option<i32>,
    reason: String,
    /// The citation of the rule's source (its id when the source is unknown).
    citation: String,
}

impl Interaction {
    fn sentence(&self, code: &str) -> String {
        let rule = match self.limit_points {
            None => format!("{code} and {} are not counted together.", self.other),
            Some(points) => format!("{code} and {} together add at most {points} points.", self.other),
        };
        format!("{rule} {} ({})", self.reason, self.citation)
    }
}

/// The pairs and limits the rule file holds for this criterion, in file order
/// (pairs first, then limits). The other criterion is named whichever side it is on.
fn interactions(c: &Criterion, rules: &RuleSet) -> Vec<Interaction> {
    let other_of = |a: &str, b: &str| -> Option<String> {
        if a == c.code {
            Some(b.to_string())
        } else if b == c.code {
            Some(a.to_string())
        } else {
            None
        }
    };
    let citation = |id: &str| rules.source(id).map_or_else(|| id.to_string(), |source| source.citation.clone());
    let pairs = rules.exclusive_pairs.iter().filter_map(|pair| {
        let other = other_of(&pair.a, &pair.b)?;
        Some(Interaction { other, limit_points: None, reason: pair.reason.clone(), citation: citation(&pair.source) })
    });
    let caps = rules.combination_caps.iter().filter_map(|cap| {
        let other = other_of(&cap.a, &cap.b)?;
        Some(Interaction {
            other,
            limit_points: Some(cap.max_points),
            reason: cap.reason.clone(),
            citation: citation(&cap.source),
        })
    });
    pairs.chain(caps).collect()
}

impl Workbench {
    /// The lines under "More", or nothing when the criterion has no pair and no limit.
    pub(super) fn interactions_panel(&self, c: &Criterion, cx: &mut Context<Self>) -> Option<Div> {
        let lines = interactions(c, self.rules);
        if lines.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        let mut panel = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().font_weight(FontWeight::BOLD).child("Used with other criteria"));
        for line in &lines {
            panel = panel.child(div().text_color(muted).child(format!("• {}", line.sentence(&c.code))));
        }
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::RuleSet;

    use super::interactions;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn of(code: &str) -> Vec<super::Interaction> {
        interactions(rules().criterion(code).expect("criterion exists"), rules())
    }

    #[test]
    fn a_pair_is_listed_from_both_sides() {
        let from_pvs1 = of("PVS1");
        assert!(from_pvs1.iter().any(|i| i.other == "PM4" && i.limit_points.is_none()), "{from_pvs1:?}");
        let from_pm4 = of("PM4");
        assert!(from_pm4.iter().any(|i| i.other == "PVS1" && i.limit_points.is_none()), "{from_pm4:?}");
    }

    #[test]
    fn a_criterion_in_two_pairs_lists_both() {
        let others: Vec<String> = of("PM2").into_iter().map(|i| i.other).collect();
        assert!(others.contains(&"BA1".to_string()) && others.contains(&"BS1".to_string()), "{others:?}");
    }

    #[test]
    fn a_combined_limit_says_how_many_points_and_who_said_so() {
        let pp1 = of("PP1");
        let with_pp4 = pp1.iter().find(|i| i.other == "PP4").expect("PP1 and PP4 are limited together");
        assert_eq!(with_pp4.limit_points, Some(5), "ClinGen 2024: five points in all");
        let sentence = with_pp4.sentence("PP1");
        assert!(sentence.starts_with("PP1 and PP4 together add at most 5 points."), "{sentence}");
        assert!(sentence.contains("ClinGen"), "the source is named: {sentence}");
    }

    #[test]
    fn a_pair_reads_as_not_counted_together_with_the_file_s_own_reason() {
        let pvs1 = of("PVS1");
        let sentence = pvs1.iter().find(|i| i.other == "PM4").expect("pair").sentence("PVS1");
        assert!(sentence.starts_with("PVS1 and PM4 are not counted together."), "{sentence}");
        assert!(sentence.contains("counts the same change twice"), "{sentence}");
        assert!(sentence.contains("Abou Tayoun"), "{sentence}");
    }

    #[test]
    fn every_listed_other_criterion_exists_and_no_criterion_lists_itself() {
        for c in &rules().criteria {
            for line in interactions(c, rules()) {
                assert!(rules().criterion(&line.other).is_some(), "{} lists {}", c.code, line.other);
                assert_ne!(line.other, c.code);
            }
        }
    }

    #[test]
    fn a_criterion_with_no_pair_and_no_limit_lists_nothing() {
        let lonely = rules().criteria.iter().find(|c| interactions(c, rules()).is_empty());
        assert!(lonely.is_some(), "some criterion stands alone");
    }
}
