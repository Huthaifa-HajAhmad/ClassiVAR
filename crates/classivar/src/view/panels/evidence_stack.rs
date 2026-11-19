//! "How the points add up": each counted criterion is one step that starts where
//! the one before it ended, on the same point axis as the points scale. The last
//! step ends at the total, so the learner can see why the class is the class.
//!
//! The layout rules (order and running total) live in plain functions that need
//! no window, so tests can check them. The bars are drawn by `point_axis`.

use classivar_core::{Assessment, Direction, Item, Strength};

use super::point_axis::{Bar, axis, points_text, widened};
use super::*;

/// One counted criterion on the running total.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Step {
    code: String,
    direction: Direction,
    strength: Strength,
    /// Signed: positive for pathogenic evidence, negative for benign.
    points: i32,
    /// The running total before this step.
    from: i32,
    /// The running total after this step.
    to: i32,
}

impl Step {
    /// "+4" or "−4". Never "0": a counted criterion is worth at least one point.
    fn change(&self) -> String {
        let sign = if self.points > 0 { "+" } else { "−" };
        format!("{sign}{}", self.points.unsigned_abs())
    }
}

/// The counted criteria as steps: pathogenic evidence first, the strongest
/// first, then benign evidence the same way. Criteria that are listed but do
/// not count add nothing and are left out.
fn steps(items: &[Item]) -> Vec<Step> {
    let mut counted: Vec<&Item> = items.iter().filter(|item| item.counted).collect();
    // The code breaks ties, so the same evidence always draws the same picture.
    counted.sort_by_key(|item| (item.direction == Direction::Benign, -item.points.abs(), item.code.clone()));
    let mut total = 0;
    counted
        .into_iter()
        .map(|item| {
            let from = total;
            total += item.points;
            Step {
                code: item.code.clone(),
                direction: item.direction,
                strength: item.strength,
                points: item.points,
                from,
                to: total,
            }
        })
        .collect()
}

/// All steps in one sentence, for the line under the picture: the same facts as
/// the bars, in words.
fn summary(steps: &[Step]) -> String {
    let total = steps.last().map_or(0, |step| step.to);
    let parts: Vec<_> =
        steps.iter().map(|step| format!("{} {} → {}", step.code, step.change(), points_text(step.to))).collect();
    format!("{}. Total {} point{}.", parts.join(", "), points_text(total), plural(total.abs()))
}

/// The steps as bars on the shared axis: filled, because the evidence is applied.
fn bars(steps: &[Step]) -> Vec<Bar> {
    steps
        .iter()
        .map(|step| Bar {
            label: format!("{} {}", step.code, step.change()),
            note: step.strength.label().to_string(),
            direction: step.direction,
            from: step.from,
            to: step.to,
            end: format!("= {}", points_text(step.to)),
            hollow: false,
            neutral: false,
        })
        .collect()
}

impl Workbench {
    /// The picture of how the counted criteria add up to the total. It is shown
    /// only where the points scale is, so it never suggests a class the result
    /// withholds.
    pub(super) fn evidence_stack(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let steps = steps(&result.items);
        let bars = bars(&steps);
        let scale = widened(&scale_for(self.rules), &bars);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(axis("stack-scroll", &scale, &bars, border, muted))
            .child(caption(muted).child(summary(&steps)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, RuleSet, Strength};

    use super::{bars, scale_for, steps, summary, widened};

    fn assessed(applied: &[Applied]) -> classivar_core::Assessment {
        RuleSet::builtin().expect("the built-in rules are valid").assess(applied).expect("valid evidence")
    }

    fn mixed() -> classivar_core::Assessment {
        assessed(&[
            Applied::new("BP7", Strength::Supporting),
            Applied::new("PM2", Strength::Supporting),
            Applied::new("PS1", Strength::Strong),
        ])
    }

    #[test]
    fn pathogenic_steps_come_first_strongest_first_then_benign() {
        let result = mixed();
        let codes: Vec<_> = steps(&result.items).into_iter().map(|s| s.code).collect();
        assert_eq!(codes, ["PS1", "PM2", "BP7"]);
    }

    #[test]
    fn each_step_starts_where_the_last_ended_and_the_last_ends_at_the_total() {
        let result = mixed();
        let steps = steps(&result.items);
        assert_eq!(steps[0].from, 0);
        for pair in steps.windows(2) {
            assert_eq!(pair[0].to, pair[1].from);
        }
        assert_eq!(steps.last().unwrap().to, result.total_points);
        for step in &steps {
            assert_eq!(step.to - step.from, step.points);
        }
    }

    #[test]
    fn a_criterion_that_does_not_count_is_left_out() {
        // PP5 is discouraged: a new assessment lists it but adds no points.
        let result = assessed(&[Applied::new("PM2", Strength::Supporting), Applied::new("PP5", Strength::Supporting)]);
        assert!(result.items.iter().any(|i| i.code == "PP5" && !i.counted));
        let codes: Vec<_> = steps(&result.items).into_iter().map(|s| s.code).collect();
        assert_eq!(codes, ["PM2"]);
    }

    #[test]
    fn the_axis_keeps_the_scale_when_every_step_fits() {
        let rules = RuleSet::builtin().unwrap();
        let scale = scale_for(rules);
        let wide = widened(&scale, &bars(&steps(&mixed().items)));
        assert_eq!(wide, scale);
    }

    #[test]
    fn the_axis_grows_to_hold_every_step_and_the_end_zones_grow_with_it() {
        let rules = RuleSet::builtin().unwrap();
        let scale = scale_for(rules);
        let high = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS2", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
            Applied::new("PS4", Strength::Strong),
            Applied::new("PM1", Strength::Moderate),
        ]);
        assert!(high.total_points > scale.max, "the example must go past the top of the scale");
        let wide = widened(&scale, &bars(&steps(&high.items)));
        assert_eq!((wide.min, wide.max), (scale.min, high.total_points));
        assert_eq!(wide.zones.last().unwrap().2, high.total_points);
        assert_eq!(wide.zones.first(), scale.zones.first(), "the bottom is unchanged");
        // The same, downwards: benign evidence below the scale's bottom at -8.
        let low = assessed(&[
            Applied::new("BS1", Strength::Strong),
            Applied::new("BS2", Strength::Strong),
            Applied::new("BS3", Strength::Strong),
        ]);
        assert!(low.total_points < scale.min, "the example must go past the bottom of the scale");
        let wide = widened(&scale, &bars(&steps(&low.items)));
        assert_eq!((wide.min, wide.max), (low.total_points, scale.max));
        assert_eq!(wide.zones.first().unwrap().1, low.total_points);
    }

    #[test]
    fn the_sentence_names_every_step_and_the_total() {
        let text = summary(&steps(&mixed().items));
        assert_eq!(text, "PS1 +4 → 4, PM2 +1 → 5, BP7 −1 → 4. Total 4 points.");
    }

    #[test]
    fn a_benign_total_is_written_with_a_real_minus_sign() {
        let result = assessed(&[Applied::new("BP7", Strength::Supporting), Applied::new("BS1", Strength::Strong)]);
        assert_eq!(summary(&steps(&result.items)), "BS1 −4 → −4, BP7 −1 → −5. Total −5 points.");
    }

    #[test]
    fn one_point_is_not_pluralised() {
        let result = assessed(&[Applied::new("PM2", Strength::Supporting)]);
        assert_eq!(summary(&steps(&result.items)), "PM2 +1 → 1. Total 1 point.");
    }
}
