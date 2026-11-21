//! The strength ladder of one criterion, shown under "More": every strength the
//! criterion can have, with its points, and which step is the default, which one
//! the 2015 paper used (when a later recommendation changed it), which steps this
//! program does not allow, and which one is applied now. It follows the evidence
//! being edited: today's evidence is held to today's limits and defaults, an
//! earlier record is not.
//!
//! The steps are plain data built by `ladder`, so tests need no window. Every
//! fact on a step is also written as a word on the step, and `sentence` says the
//! same in one line, so nothing depends on colour or position alone.

use classivar_core::{Criterion, Direction, RuleSet, Strength};

use super::*;

/// One step of the ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Rung {
    strength: Strength,
    /// Signed: negative for benign evidence.
    points: i32,
    /// False for a discouraged criterion in a new assessment: it adds no points there.
    counted: bool,
    /// The strength the rule file gives when nothing else is chosen.
    default: bool,
    /// The strength the 2015 paper gave, when a later recommendation changed it.
    paper: bool,
    /// Above the limit this program sets for the criterion.
    above_limit: bool,
    /// Applied on the screen now.
    applied: bool,
    /// How many strength-guide answers set exactly this strength.
    guide_answers: usize,
}

impl Rung {
    /// The points line of the step, or "not counted" when the criterion adds none.
    fn points_text(&self) -> String {
        if self.counted { format!("{} pts", points_with_sign(self.points)) } else { "not counted".to_string() }
    }

    /// The words written on the step, in a fixed order.
    fn tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        if self.default {
            tags.push("default".to_string());
        }
        if self.paper {
            tags.push("2015 paper".to_string());
        }
        if self.above_limit {
            tags.push("above limit".to_string());
        }
        if self.guide_answers > 0 {
            tags.push(format!("{} guide answer{}", self.guide_answers, plural(self.guide_answers as i32)));
        }
        if self.applied {
            tags.push("applied".to_string());
        }
        tags
    }
}

/// The steps from weakest to strongest, for the evidence being edited. For
/// today's evidence the steps above this program's limit are kept and marked, so
/// the learner sees what the published scale has and where this program stops. An
/// earlier record is not bound by today's limit or today's default: it starts at
/// the strength the 2015 paper used and may hold any strength a lab could use,
/// which is what its own code and strength buttons do.
fn ladder(c: &Criterion, rules: &RuleSet, scope: Editing, applied: Option<Strength>) -> Vec<Rung> {
    let today = scope == Editing::Today;
    let allowed = if today { c.allowed_strengths() } else { c.recorded_strengths() };
    let starting = if today { c.strength } else { c.original_strength.unwrap_or(c.strength) };
    let sign = if c.direction == Direction::Benign { -1 } else { 1 };
    c.recorded_strengths()
        .into_iter()
        .map(|strength| Rung {
            strength,
            points: sign * rules.magnitude(strength),
            counted: !(today && c.discouraged),
            default: strength == starting,
            paper: today && c.original_strength == Some(strength),
            above_limit: !allowed.contains(&strength),
            applied: applied == Some(strength),
            guide_answers: c.guide.iter().filter(|answer| answer.strength == Some(strength)).count(),
        })
        .collect()
}

/// The ladder in one sentence, for the line under it.
fn sentence(c: &Criterion, rungs: &[Rung], scope: Editing) -> String {
    let today = scope == Editing::Today;
    let mut parts = Vec::new();
    if let Some(default) = rungs.iter().find(|rung| rung.default) {
        let place = if today { "" } else { " in a record" };
        parts.push(format!("{} starts at {}{place}.", c.code, default.strength.label()));
    }
    if let Some(paper) = rungs.iter().find(|rung| rung.paper) {
        parts.push(format!("The 2015 paper used {}.", paper.strength.label()));
    }
    match c.max_strength {
        Some(limit) if today && rungs.iter().any(|rung| rung.above_limit) => {
            parts.push(format!("These rules stop at {}.", limit.label()));
        }
        Some(_) if !today => parts.push("Today's limit does not apply to a record.".to_string()),
        _ => {}
    }
    if rungs.iter().any(|rung| !rung.counted) {
        parts.push("It is not counted in a new assessment.".to_string());
    }
    if !c.guide.is_empty() {
        parts.push("A guide answer sets the strength.".to_string());
    }
    match rungs.iter().find(|rung| rung.applied) {
        Some(applied) => parts.push(format!("Applied: {}.", applied.strength.label())),
        None => parts.push("Not applied.".to_string()),
    }
    parts.join(" ")
}

impl Workbench {
    /// The ladder with its sentence, for the "More" panel of a criterion.
    pub(super) fn strength_ladder(&self, c: &Criterion, applied: Option<Strength>, cx: &mut Context<Self>) -> Div {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let rungs = ladder(c, self.rules, self.editing, applied);
        let sentence = sentence(c, &rungs, self.editing);
        let mut steps = div().flex().flex_wrap().gap_1();
        for rung in &rungs {
            let colour = direction_colour(c.direction);
            let mut cell = div()
                .w(px(80.))
                .text_xs()
                .px_1p5()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(border)
                .flex()
                .flex_col()
                .when(rung.applied, |cell| cell.border_2().border_color(colour))
                .when(rung.above_limit, |cell| cell.opacity(0.55))
                .child(div().font_weight(FontWeight::BOLD).child(rung.strength.label()))
                .child(div().child(rung.points_text()));
            for tag in rung.tags() {
                cell = cell.child(caption(muted).child(tag));
            }
            steps = steps.child(cell);
        }
        div().flex().flex_col().gap_1().child(steps).child(sentence)
    }
}

/// "+4" or "−4" with a real minus sign, like the totals elsewhere on the screen.
fn points_with_sign(points: i32) -> String {
    if points < 0 { format!("−{}", points.unsigned_abs()) } else { format!("+{points}") }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{RuleSet, Strength};

    use super::{ladder, points_with_sign, sentence};
    use crate::view::Editing;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn rungs_in(code: &str, scope: Editing, applied: Option<Strength>) -> (Vec<super::Rung>, String) {
        let c = rules().criterion(code).expect("the criterion exists");
        let rungs = ladder(c, rules(), scope, applied);
        let text = sentence(c, &rungs, scope);
        (rungs, text)
    }

    fn rungs_of(code: &str, applied: Option<Strength>) -> (Vec<super::Rung>, String) {
        rungs_in(code, Editing::Today, applied)
    }

    #[test]
    fn a_pathogenic_criterion_has_four_steps_with_the_points_of_the_rules() {
        let (rungs, _) = rungs_of("PS1", None);
        let steps: Vec<_> = rungs.iter().map(|r| (r.strength, r.points)).collect();
        assert_eq!(
            steps,
            [(Strength::Supporting, 1), (Strength::Moderate, 2), (Strength::Strong, 4), (Strength::VeryStrong, 8)]
        );
        assert_eq!(rungs.iter().filter(|r| r.default).count(), 1, "exactly one default");
    }

    #[test]
    fn benign_points_are_negative_and_have_no_very_strong_step() {
        let (rungs, _) = rungs_of("BS1", None);
        assert!(rungs.iter().all(|r| r.points < 0));
        assert!(rungs.iter().all(|r| r.strength != Strength::VeryStrong));
        assert_eq!(points_with_sign(-4), "−4", "a real minus sign");
        assert_eq!(points_with_sign(4), "+4");
    }

    #[test]
    fn stand_alone_is_the_only_step_of_ba1() {
        let (rungs, text) = rungs_of("BA1", Some(Strength::StandAlone));
        assert_eq!(rungs.len(), 1);
        assert!(rungs[0].default && rungs[0].applied && !rungs[0].above_limit);
        assert!(text.ends_with("Applied: Stand-alone."), "{text}");
    }

    #[test]
    fn the_steps_above_the_rules_limit_are_kept_and_marked() {
        let c = rules().criterion("PM5").expect("PM5 exists");
        let limit = c.max_strength.expect("PM5 has a limit");
        let (rungs, text) = rungs_of("PM5", None);
        for rung in &rungs {
            assert_eq!(rung.above_limit, !c.allowed_strengths().contains(&rung.strength));
        }
        assert!(rungs.iter().any(|r| r.above_limit), "the limit must cut something off");
        assert!(text.contains(&format!("These rules stop at {}.", limit.label())), "{text}");
    }

    #[test]
    fn a_criterion_without_a_limit_says_nothing_about_one() {
        assert!(rules().criterion("PS2").unwrap().max_strength.is_none(), "PS2 must have no program limit");
        let (rungs, text) = rungs_of("PS2", None);
        assert!(rungs.iter().all(|r| !r.above_limit));
        assert!(!text.contains("limit"), "{text}");
    }

    #[test]
    fn the_paper_strength_is_marked_only_where_a_later_rule_changed_it() {
        let changed = rules().criteria.iter().find(|c| c.original_strength.is_some()).expect("one criterion changed");
        let (rungs, text) = rungs_of(&changed.code, None);
        let paper = changed.original_strength.unwrap();
        assert!(rungs.iter().any(|r| r.paper && r.strength == paper));
        assert!(text.contains(&format!("The 2015 paper used {}.", paper.label())), "{text}");
        let (rungs, text) = rungs_of("PS1", None);
        assert!(rungs.iter().all(|r| !r.paper));
        assert!(!text.contains("2015 paper"), "{text}");
    }

    #[test]
    fn a_guided_criterion_counts_the_answers_that_set_each_step() {
        let c = rules().criterion("PVS1").expect("PVS1 exists");
        let (rungs, text) = rungs_of("PVS1", None);
        for rung in &rungs {
            let expected = c.guide.iter().filter(|a| a.strength == Some(rung.strength)).count();
            assert_eq!(rung.guide_answers, expected);
        }
        assert!(rungs.iter().map(|r| r.guide_answers).sum::<usize>() > 0);
        assert!(text.contains("A guide answer sets the strength."), "{text}");
    }

    #[test]
    fn the_applied_step_is_marked_and_the_sentence_says_so() {
        let (rungs, text) = rungs_of("PS1", Some(Strength::Moderate));
        let applied: Vec<_> = rungs.iter().filter(|r| r.applied).map(|r| r.strength).collect();
        assert_eq!(applied, [Strength::Moderate]);
        assert!(text.ends_with("Applied: Moderate."), "{text}");
        let (_, text) = rungs_of("PS1", None);
        assert!(text.ends_with("Not applied."), "{text}");
    }

    #[test]
    fn every_fact_on_a_step_is_also_a_word() {
        let (rungs, _) = rungs_of("PS1", Some(Strength::Strong));
        let strong = rungs.iter().find(|r| r.strength == Strength::Strong).unwrap();
        assert_eq!(strong.tags(), ["default", "applied"]);
    }

    #[test]
    fn a_record_starts_at_the_strength_of_the_2015_paper_like_its_code_button_does() {
        // Clicking a criterion's code in a record applies the paper's strength, not today's.
        let c = rules().criteria.iter().find(|c| c.original_strength.is_some()).expect("one criterion changed");
        let paper = c.original_strength.unwrap();
        assert_ne!(paper, c.strength, "the example must differ from today's strength");
        let (today, _) = rungs_in(&c.code, Editing::Today, None);
        let (record, text) = rungs_in(&c.code, Editing::Record(0), None);
        assert_eq!(today.iter().find(|r| r.default).unwrap().strength, c.strength);
        assert_eq!(record.iter().find(|r| r.default).unwrap().strength, paper);
        assert!(text.contains(&format!("{} starts at {} in a record.", c.code, paper.label())), "{text}");
        assert!(record.iter().all(|r| !r.paper), "in a record the paper's strength is the default itself");
    }

    #[test]
    fn a_record_is_not_bound_by_todays_limit() {
        // The strength button of a record cycles through every strength a lab could use.
        let (rungs, text) = rungs_in("PM5", Editing::Record(0), None);
        assert!(rungs.iter().all(|r| !r.above_limit), "no step is above a limit in a record");
        assert!(!text.contains("These rules stop at"), "{text}");
        assert!(text.contains("Today's limit does not apply to a record."), "{text}");
    }

    #[test]
    fn a_discouraged_criterion_is_not_counted_today_but_is_counted_in_a_record() {
        let c = rules().criteria.iter().find(|c| c.discouraged).expect("PP5 or BP6 is discouraged");
        let (today, text) = rungs_in(&c.code, Editing::Today, None);
        assert!(today.iter().all(|r| !r.counted));
        assert!(today.iter().all(|r| r.points_text() == "not counted"));
        assert!(text.contains("It is not counted in a new assessment."), "{text}");
        let (record, text) = rungs_in(&c.code, Editing::Record(0), None);
        assert!(record.iter().all(|r| r.counted));
        assert!(!text.contains("not counted"), "{text}");
    }
}
