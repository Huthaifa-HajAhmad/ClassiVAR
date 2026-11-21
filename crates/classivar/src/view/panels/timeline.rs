//! "Records against today, in points": each earlier record's own evidence as a
//! bar from zero to its total on the shared point axis, oldest first, then today.
//! It shows how the same variant moved over the years, which is the question
//! ClassiVAR was built to answer.
//!
//! A record is drawn from what it recorded, scored the way it was made. The class
//! a lab reported is written beside the bar and never drawn: only points are
//! drawn, so a lab's class is never mistaken for one ClassiVAR worked out. A
//! record whose own class the engine withholds, that has no counted evidence or
//! that could not be scored has no honest total to show, so it is left out and
//! the sentence under the picture says so. The rows are plain data built by
//! `timeline`, so tests need no window.

use classivar_core::{Direction, Reevaluation, Rescored};

use super::point_axis::{
    Bar, class_strip, fluid_bar, fraction, limit_lines, points_text, tick_row, widened, zone_labels,
};
use super::*;
use crate::look::figure;
use crate::view::describe::Scale;

/// The bars to draw, and the same facts in words.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Timeline {
    bars: Vec<Bar>,
    /// For the line under the picture.
    summary: String,
}

/// Why a record has no bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Skip {
    NoEvidence,
    /// It has evidence, but the engine shows no class for it (for example a contradiction).
    Withheld,
    Unscored,
}

/// How many records were left out, for each reason.
#[derive(Debug, Default)]
struct Skipped {
    no_evidence: usize,
    withheld: usize,
    unscored: usize,
}

impl Skipped {
    fn add(&mut self, skip: Skip) {
        match skip {
            Skip::NoEvidence => self.no_evidence += 1,
            Skip::Withheld => self.withheld += 1,
            Skip::Unscored => self.unscored += 1,
        }
    }

    /// For example " Not drawn: 1 record with no counted evidence, 2 records whose class is withheld."
    /// Empty when every record was drawn.
    fn sentence(&self) -> String {
        let count =
            |n: usize, what: &str| (n > 0).then(|| format!("{n} record{} {what}", if n == 1 { "" } else { "s" }));
        let parts: Vec<String> = [
            count(self.no_evidence, "with no counted evidence"),
            count(self.withheld, "whose class is withheld"),
            count(self.unscored, "that could not be scored"),
        ]
        .into_iter()
        .flatten()
        .collect();
        if parts.is_empty() { String::new() } else { format!(" Not drawn: {}.", parts.join(", ")) }
    }
}

/// A bar from zero to `total`, filled. Red above zero, blue below, and grey at
/// zero, where the points favour neither side.
fn bar(label: String, note: String, total: i32) -> Bar {
    let direction = if total < 0 { Direction::Benign } else { Direction::Pathogenic };
    Bar {
        label,
        note,
        direction,
        from: 0,
        to: total,
        end: format!("= {}", points_text(total)),
        hollow: false,
        neutral: total == 0,
    }
}

/// The records oldest first. The order they were entered in breaks a tie in year
/// (a stable sort keeps it), the same rule the engine uses for the latest record.
fn oldest_first(re: &Reevaluation) -> Vec<&Rescored> {
    let mut records: Vec<&Rescored> = re.records.iter().collect();
    records.sort_by_key(|record| record.year);
    records
}

/// A record's total points, or the reason it has none to draw. Only a record
/// with a class the engine shows is drawn: a total beside a withheld class would
/// teach that the points alone decide.
fn total_of(record: &Rescored) -> Result<i32, Skip> {
    match &record.assessment {
        Err(_) => Err(Skip::Unscored),
        Ok(assessment) if assessment.class().is_some() => Ok(assessment.total_points),
        Ok(assessment) if assessment.has_evidence => Err(Skip::Withheld),
        Ok(_) => Err(Skip::NoEvidence),
    }
}

/// The year, which is what the bar is ordered by. When another drawn record has
/// the same year, the start of the lab's name is added so the bars can be told apart.
fn bar_label(record: &Rescored, shares_year: bool) -> String {
    const LAB_CHARS: usize = 8;
    if !shares_year {
        return record.year.to_string();
    }
    let lab: String = record.label.chars().take(LAB_CHARS).collect();
    let cut = if record.label.chars().count() > LAB_CHARS { "…" } else { "" };
    format!("{} · {lab}{cut}", record.year)
}

fn points_in_words(points: i32) -> String {
    format!("{} point{}", points_text(points), plural(points.abs()))
}

/// The records oldest first, then today. Today is always read from the
/// reevaluation, never from the record being edited, so the "Today" bar is the
/// same total as the history table's. Nothing is drawn (None) unless today's
/// class is shown, like the points scale.
fn timeline(re: &Reevaluation) -> Option<Timeline> {
    let today = re.today.as_ref().ok().filter(|today| today.class().is_some())?;
    let mut bars = Vec::new();
    let mut parts = Vec::new();
    let mut skipped = Skipped::default();
    let mut drawn = Vec::new();
    for record in oldest_first(re) {
        match total_of(record) {
            Ok(points) => drawn.push((record, points)),
            Err(skip) => skipped.add(skip),
        }
    }
    for (record, points) in &drawn {
        let reported = record.reported_class.short();
        let shares_year = drawn.iter().filter(|(other, _)| other.year == record.year).count() > 1;
        bars.push(bar(bar_label(record, shares_year), format!("reported {reported}"), *points));
        parts.push(format!("{} {} reported {reported} ({})", record.year, record.label, points_in_words(*points)));
    }
    bars.push(bar("Today".to_string(), format!("now {}", today.points_class.short()), today.total_points));
    parts.push(format!("today {}, {}", points_in_words(today.total_points), today.points_class.short()));
    let summary = format!("{}.{}", parts.join("; "), skipped.sentence());
    Some(Timeline { bars, summary })
}

/// Width of the year column and of the total column, and the height of one row.
const LABEL_WIDTH: f32 = 112.;
const END_WIDTH: f32 = 48.;
const ROW: f32 = 40.;

impl Workbench {
    /// The picture, or nothing when today's class is not shown.
    pub(super) fn timeline(&self, re: &Reevaluation, cx: &mut Context<Self>) -> AnyElement {
        let Some(Timeline { bars, summary }) = timeline(re) else { return div().into_any_element() };
        let theme = cx.theme();
        let (ink, muted, border, paper) = (theme.foreground, theme.muted_foreground, theme.border, theme.muted);
        let scale = widened(&scale_for(self.rules), &bars);
        let today = re.today.as_ref().ok().and_then(|today| today.class());
        let last = bars.len().saturating_sub(1);
        let mut rows = div().flex().flex_col();
        if let Some(class) = today {
            let head = div().pl(px(LABEL_WIDTH)).pr(px(END_WIDTH)).pb_1().flex().flex_col().gap_1();
            rows = rows.child(head.child(zone_labels(&scale, class, (ink, muted), None)).child(class_strip(
                &scale,
                class,
                (theme.popover, ink),
                6.,
            )));
        }
        for (i, bar) in bars.iter().enumerate() {
            rows = rows.child(record_row(bar, i == last, &scale, (ink, muted, border, paper)));
        }
        rows = rows.child(div().pl(px(LABEL_WIDTH)).pr(px(END_WIDTH)).child(tick_row(&scale, muted)));
        crate::look::card(cx)
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(rows)
            .child(caption(muted).child(summary))
            .into_any_element()
    }
}

/// One record (or today, which is drawn a little heavier): its year and what
/// it reported, its bar from zero, and its total.
fn record_row(bar: &Bar, now: bool, scale: &Scale, (ink, muted, border, paper): (Hsla, Hsla, Hsla, Hsla)) -> Div {
    let label = div()
        .w(px(LABEL_WIDTH))
        .flex_none()
        .flex()
        .flex_col()
        .child(figure(12.).text_color(ink).child(bar.label.clone()))
        .child(caption(muted).child(bar.note.clone()));
    let track = div()
        .relative()
        .flex_1()
        .h_full()
        .flex()
        .items_center()
        .children(limit_lines(scale, border))
        .child(div().absolute().top_0().h_full().left(relative(fraction(0, scale))).w(px(1.)).bg(muted))
        .child(fluid_bar(bar, scale, (paper, muted), if now { 12. } else { 8. }));
    let end = figure(12.).w(px(END_WIDTH)).flex_none().flex().justify_end().child(bar.end.clone());
    div()
        .flex()
        .items_center()
        .h(px(ROW))
        .when(now, |row| row.border_t_1().border_color(border))
        .child(label)
        .child(track)
        .child(end)
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, Class, Direction, RuleSet, Strength, Submission};

    use super::timeline;

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn record(label: &str, year: u16, class: Class, list: &[(&str, Strength)]) -> Submission {
        Submission {
            label: label.into(),
            year,
            reported_class: class,
            reported_tier: None,
            rule_set: None,
            criteria: list.iter().map(|(code, strength)| Applied::new(*code, *strength)).collect(),
        }
    }

    fn today() -> Vec<Applied> {
        vec![Applied::new("PS1", Strength::Strong), Applied::new("PM2", Strength::Supporting)]
    }

    #[test]
    fn records_are_drawn_oldest_first_and_today_last() {
        let records = [
            record("Lab B", 2023, Class::Vus, &[("PM2", Strength::Supporting)]),
            record("Lab A", 2019, Class::Vus, &[("PM2", Strength::Supporting)]),
        ];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).expect("today has a class");
        let labels: Vec<_> = drawn.bars.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["2019", "2023", "Today"]);
        assert!(!drawn.summary.contains("Not drawn"));
    }

    #[test]
    fn a_tie_in_year_keeps_the_order_the_records_were_entered() {
        let records = [
            record("First", 2020, Class::Vus, &[("PM2", Strength::Supporting)]),
            record("Second", 2020, Class::LikelyPathogenic, &[("PS1", Strength::Strong), ("PM2", Strength::Moderate)]),
        ];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).expect("today has a class");
        assert_eq!(drawn.bars[0].note, "reported VUS");
        assert_eq!(drawn.bars[1].note, "reported LP");
    }

    #[test]
    fn records_from_the_same_year_are_told_apart_by_their_lab_and_others_keep_just_the_year() {
        let records = [
            record("Lab Alpha Laboratories", 2020, Class::Vus, &[("PM2", Strength::Supporting)]),
            record("Lab B", 2020, Class::Vus, &[("PM2", Strength::Supporting)]),
            record("Lab C", 2023, Class::Vus, &[("PM2", Strength::Supporting)]),
        ];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).expect("today has a class");
        let labels: Vec<_> = drawn.bars.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["2020 · Lab Alph…", "2020 · Lab B", "2023", "Today"]);
    }

    #[test]
    fn each_bar_runs_from_zero_to_its_own_total_and_says_what_the_lab_reported() {
        let records = [record("Lab A", 2019, Class::Vus, &[("PS1", Strength::Strong)])];
        let re = rules().reevaluate(&records, &today());
        let today_result = re.today.as_ref().unwrap();
        let drawn = timeline(&re).unwrap();
        let (old, now) = (&drawn.bars[0], &drawn.bars[1]);
        assert_eq!((old.from, old.to, old.end.as_str()), (0, 4, "= 4"));
        assert_eq!(old.note, "reported VUS", "the lab's class is written, not drawn");
        assert_eq!((now.from, now.to), (0, today_result.total_points));
        assert_eq!(now.note, format!("now {}", today_result.points_class.short()));
        assert!(drawn.bars.iter().all(|b| !b.hollow), "recorded evidence is applied, so it is filled");
    }

    #[test]
    fn a_benign_total_is_blue_and_written_with_a_real_minus_sign() {
        let records =
            [record("Lab A", 2019, Class::LikelyBenign, &[("BS1", Strength::Strong), ("BP7", Strength::Supporting)])];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).expect("today has a class");
        assert_eq!(drawn.bars[0].direction, Direction::Benign);
        assert_eq!(drawn.bars[0].end, "= −5");
        assert!(!drawn.bars[0].neutral);
        assert_eq!(drawn.bars[1].direction, Direction::Pathogenic);
    }

    #[test]
    fn a_total_of_zero_is_neither_red_nor_blue() {
        // One point for and one against: a shown class (VUS), and a total that favours neither side.
        let records =
            [record("Lab A", 2019, Class::Vus, &[("PM2", Strength::Supporting), ("BP7", Strength::Supporting)])];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).expect("today has a class");
        assert_eq!((drawn.bars[0].to, drawn.bars[0].end.as_str()), (0, "= 0"));
        assert!(drawn.bars[0].neutral, "zero points must not be painted as pathogenic");
        assert!(!drawn.bars[1].neutral, "today has points on one side");
    }

    #[test]
    fn a_record_with_no_counted_evidence_is_left_out_and_counted() {
        let records = [
            record("Empty", 2018, Class::Vus, &[]),
            record("Lab A", 2019, Class::Vus, &[("PM2", Strength::Supporting)]),
        ];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).unwrap();
        assert_eq!(drawn.bars.len(), 2, "one record and today");
        assert!(drawn.summary.ends_with(" Not drawn: 1 record with no counted evidence."), "{}", drawn.summary);
    }

    #[test]
    fn a_record_whose_class_is_withheld_is_not_drawn_and_the_sentence_says_why() {
        // Two criteria that cannot both apply: the engine shows no class, so no total is drawn.
        let records = [record("Lab A", 2019, Class::Vus, &[("PS3", Strength::Strong), ("BS3", Strength::Strong)])];
        let re = rules().reevaluate(&records, &today());
        assert!(re.records[0].assessment.as_ref().is_ok_and(|a| a.has_evidence && a.class().is_none()));
        let drawn = timeline(&re).unwrap();
        assert_eq!(drawn.bars.len(), 1, "only today");
        assert!(drawn.summary.ends_with(" Not drawn: 1 record whose class is withheld."), "{}", drawn.summary);
        assert!(!drawn.summary.contains("Lab A"), "{}", drawn.summary);
    }

    #[test]
    fn a_record_that_cannot_be_scored_is_named_as_such_not_as_empty() {
        let records = [record("Odd", 2018, Class::Vus, &[("NOT-A-CODE", Strength::Supporting)])];
        let re = rules().reevaluate(&records, &today());
        assert!(re.records[0].assessment.is_err());
        let drawn = timeline(&re).unwrap();
        assert!(drawn.summary.ends_with(" Not drawn: 1 record that could not be scored."), "{}", drawn.summary);
    }

    #[test]
    fn several_reasons_and_plurals_are_written_out() {
        let records = [
            record("A", 2016, Class::Vus, &[]),
            record("B", 2017, Class::Vus, &[]),
            record("C", 2018, Class::Vus, &[("PS3", Strength::Strong), ("BS3", Strength::Strong)]),
        ];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).unwrap();
        assert!(
            drawn
                .summary
                .ends_with(" Not drawn: 2 records with no counted evidence, 1 record whose class is withheld."),
            "{}",
            drawn.summary
        );
    }

    #[test]
    fn the_sentence_names_every_drawn_record_and_today() {
        let records = [record("Lab A", 2019, Class::Vus, &[("PM2", Strength::Supporting)])];
        let re = rules().reevaluate(&records, &today());
        let text = timeline(&re).unwrap().summary;
        assert_eq!(text, "2019 Lab A reported VUS (1 point); today 5 points, VUS.");
    }

    #[test]
    fn nothing_is_drawn_while_today_has_no_class() {
        let records = [record("Lab A", 2019, Class::Vus, &[("PM2", Strength::Supporting)])];
        let re = rules().reevaluate(&records, &[]);
        assert!(timeline(&re).is_none(), "no evidence today, so no class to compare with");
    }

    #[test]
    fn today_is_the_reevaluations_today_not_a_record_being_edited() {
        // The history table's Today column and this picture must give one total.
        // The function takes only the reevaluation, so a record on screen cannot change it.
        let records = [record("Lab A", 2019, Class::Vus, &[("PM2", Strength::Supporting)])];
        let re = rules().reevaluate(&records, &today());
        let drawn = timeline(&re).unwrap();
        assert_eq!(drawn.bars.last().unwrap().to, re.today.as_ref().unwrap().total_points);
        assert_ne!(drawn.bars[0].to, drawn.bars[1].to, "the record and today differ in this example");
    }
}
