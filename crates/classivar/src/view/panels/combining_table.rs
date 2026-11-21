//! "What the 2015 table says": every row of the ACMG/AMP 2015 combining table
//! (with ClinGen's 2020 addition), grouped by class, with the rows the evidence
//! meets marked. It shows the second way to reach a class, next to the points.
//!
//! The sentence that says which rows are met is a plain function with tests;
//! the drawing below it only lays it out.

use classivar_core::combining_table::Counts;
use classivar_core::{Assessment, Class, Direction, TABLE_ROWS, TableRow, combining_table};

use super::*;
use crate::look::figure;

/// The classes the table has rows for, in the order they are drawn.
const GROUPS: [Class; 4] = [Class::Pathogenic, Class::LikelyPathogenic, Class::Benign, Class::LikelyBenign];

/// One met row in words, for example "≥2 Strong (Pathogenic)".
fn met_text(row: &TableRow) -> String {
    let addition = if row.clingen_2020_addition { ", ClinGen 2020 addition" } else { "" };
    format!("{} ({}{addition})", row.text(), row.class.label())
}

/// Whether to draw the table. It shows wherever a class is shown. It also shows
/// when the only reason the class is withheld is that rows are met on both sides:
/// there the table is the explanation ("this needs expert judgment"), and its
/// sentence names no class. It stays hidden when no evidence counts, when two
/// criteria contradict each other (the table has no answer then), and when a
/// combined limit is exceeded (the table does not explain that).
pub(super) fn is_shown(result: &Assessment) -> bool {
    result.class().is_some()
        || (result.both_sides_met && result.conflicts.is_empty() && result.combination_overuse.is_empty())
}

/// Which rows are met and what the table gives, in one sentence. It is the text
/// version of the picture. When the class is withheld it says so instead of
/// naming what the table gives.
fn summary(result: &Assessment) -> String {
    const NO_ROW: &str = "The 2015 table has no row for Moderate or Very strong benign evidence";
    if result.table_rows.is_empty() {
        return match result.classic_class {
            Some(class) => format!("No row is met, so the 2015 table gives {}.", class.label()),
            None => format!("{NO_ROW}, so it gives no class."),
        };
    }
    // Rows are listed strongest first, so the first met row on a side gives that side's class.
    // A weaker row on the same side (Likely pathogenic next to Pathogenic) adds nothing to
    // the sentence; the picture still marks it.
    let (pathogenic_class, benign_class) = combining_table::sides(&result.table_rows);
    let deciding = |row: &&&TableRow| match row.side {
        Direction::Pathogenic => Some(row.class) == pathogenic_class,
        Direction::Benign => Some(row.class) == benign_class,
    };
    let met: Vec<_> = result.table_rows.iter().filter(deciding).map(|row| met_text(row)).collect();
    // Some rows are met, but a strength in use has no row, so the table cannot give one class.
    let Some(class) = result.classic_class else {
        return format!("{NO_ROW}, so it cannot combine the two sides. Met: {}.", met.join("; "));
    };
    let both_sides = pathogenic_class.is_some() && benign_class.is_some();
    let note = if both_sides { " Rows are met on both sides, which the 2015 guidance calls uncertain." } else { "" };
    let ending = match result.class() {
        Some(_) => format!("The 2015 table gives {}.", class.label()),
        None => "ClassiVAR shows no class here: this needs expert judgment.".to_string(),
    };
    format!("Met: {}.{note} {ending}", met.join("; "))
}

/// The most criteria a row may still lack and be worth a hint. Further than that
/// the row is not a realistic next step, and a hint on every row would be noise.
const HINT_MAX_MISSING: usize = 2;

/// For a row the evidence is part of the way to, what it still needs, in words
/// ("needs 1 more Strong"). It names strengths, not criteria: the curator decides
/// which criteria really apply. None for a met row (it is marked instead), for a
/// row the evidence has not started on, for a row that is too far away, and for a
/// row whose class the evidence already has: adding to it would change nothing.
fn hint(row: &TableRow, counts: &Counts, given: &[Class]) -> Option<String> {
    if given.contains(&row.class) {
        return None;
    }
    let short = row.shortfall(counts);
    let missing: usize = short.iter().map(|need| need.at_least).sum();
    let needed: usize = row.needs.iter().map(|need| need.at_least).sum();
    if missing == 0 || missing == needed || missing > HINT_MAX_MISSING {
        return None;
    }
    let parts: Vec<_> = short.iter().map(|need| format!("{} more {}", need.at_least, need.strength.label())).collect();
    Some(format!("needs {}", parts.join(" + ")))
}

/// For each need of a row, how many criteria the evidence holds towards it and
/// how many it asks for: one dot per criterion, filled when held. Extra
/// criteria never overflow a need.
fn pips(row: &TableRow, counts: &Counts) -> Vec<(usize, usize)> {
    let short = row.shortfall(counts);
    row.needs
        .iter()
        .map(|need| {
            let missing = short.iter().find(|s| s.strength == need.strength).map_or(0, |s| s.at_least);
            (need.at_least - missing, need.at_least)
        })
        .collect()
}

/// Rows the evidence meets in one class group, and how many rows the group has.
fn tally(class: Class, met: &[&TableRow]) -> (usize, usize) {
    let rows = TABLE_ROWS.iter().filter(|row| row.class == class).count();
    (met.iter().filter(|row| row.class == class).count(), rows)
}

/// One row of the table: a check when the evidence meets it, its text, a note
/// for the ClinGen row and what a part-way row still needs, and one dot per
/// criterion it asks for, filled where the evidence holds one.
fn table_line(row: &TableRow, met: bool, hint: Option<String>, counts: &Counts, cx: &App) -> Div {
    let theme = cx.theme();
    let (ink, muted) = (theme.foreground, theme.muted_foreground);
    let colour = direction_colour(row.side);
    let mark =
        div().size(px(18.)).flex_none().flex().items_center().justify_center().rounded_full().when(met, |mark| {
            mark.bg(colour).text_color(direction_ink()).text_xs().font_weight(FontWeight::BOLD).child("✓")
        });
    let notes: Vec<String> =
        row.clingen_2020_addition.then(|| "added by ClinGen in 2020".to_string()).into_iter().chain(hint).collect();
    let words = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(
            figure(12.)
                .text_color(if met { ink } else { muted })
                .when(!met, |text| text.font_weight(FontWeight::NORMAL))
                .child(if met { format!("{} · met", row.text()) } else { row.text() }),
        )
        .when(!notes.is_empty(), |words| words.child(caption(muted).child(notes.join(" · "))));
    let mut dots = div().flex_none().flex().items_center().gap(px(8.));
    for (held, asked) in pips(row, counts) {
        let group = (0..asked).map(|i| {
            let dot = div().size(px(7.)).rounded_full();
            if i < held { dot.bg(colour) } else { dot.border_1().border_color(muted.opacity(0.6)) }
        });
        dots = dots.child(div().flex().gap(px(3.)).children(group));
    }
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_2()
        .py(px(6.))
        .rounded(px(7.))
        .when(met, |line| line.bg(theme.popover))
        .child(mark)
        .child(words)
        .child(dots)
}

impl Workbench {
    /// The 2015 combining table with the rows the evidence meets marked. Shown
    /// only where the class is shown, like the points scale.
    pub(super) fn combining_table(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let counts = Counts::from_items(&result.items);
        // The classes the evidence already has. A row that would give one of them
        // again is worth no hint: adding to it would change nothing.
        let given: Vec<Class> = result.table_rows.iter().map(|row| row.class).collect();
        let groups = GROUPS.into_iter().map(|class| {
            let rows = TABLE_ROWS
                .iter()
                .filter(|row| row.class == class)
                .map(|row| table_line(row, result.table_rows.contains(&row), hint(row, &counts, &given), &counts, cx));
            crate::look::card(cx)
                .flex_1()
                .min_w(px(300.))
                .flex()
                .flex_col()
                .gap_1()
                .child(group_head(class, tally(class, &result.table_rows), cx))
                .children(rows)
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_sm().child(summary(result)))
            .child(div().flex().flex_wrap().gap_3().children(groups))
            .child(caption(muted).child(
                "≥ means \"at least\". Each row lists what the evidence on one side must hold; each dot is one criterion it asks for, filled where your evidence holds one. A row that is part of the way met says what it still needs.",
            ))
            .into_any_element()
    }
}

/// The class a group gives, on its own colour, and how many of its rows are met.
fn group_head(class: Class, (met, rows): (usize, usize), cx: &App) -> Div {
    let dark = cx.theme().is_dark();
    let count = if met == 0 { format!("{rows} rows") } else { format!("{met} of {rows} met") };
    div()
        .flex()
        .items_center()
        .justify_between()
        .pb_1()
        .child(
            div()
                .px_2()
                .py(px(2.))
                .rounded_full()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .bg(class_fill(class, dark))
                .text_color(class_ink(class, dark))
                .child(class.label()),
        )
        .child(crate::look::eyebrow(count, cx))
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, Class, RuleSet, Strength};

    use classivar_core::TABLE_ROWS;
    use classivar_core::combining_table::{self, Counts};

    use super::{hint, is_shown, pips, summary};

    /// Four Strong pathogenic criteria (16 points) against two Strong benign ones (−8):
    /// rows are met on both sides, and the points alone would say Likely pathogenic.
    fn both_sides_withheld() -> classivar_core::Assessment {
        assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS2", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
            Applied::new("PS4", Strength::Strong),
            Applied::new("BS1", Strength::Strong),
            Applied::new("BS2", Strength::Strong),
        ])
    }

    #[test]
    fn a_withheld_class_is_not_named_by_the_table_sentence() {
        let result = both_sides_withheld();
        assert!(result.class().is_none() && result.both_sides_met);
        assert_eq!(
            summary(&result),
            "Met: ≥2 Strong (Pathogenic); ≥2 Strong (Benign). Rows are met on both sides, which the 2015 guidance calls uncertain. ClassiVAR shows no class here: this needs expert judgment."
        );
    }

    #[test]
    fn a_strength_the_table_lacks_a_row_for_does_not_hide_the_rows_that_are_met() {
        // Benign evidence at Moderate has no row in the 2015 table, so the table cannot
        // combine the two sides. The two Strong pathogenic criteria still meet a row,
        // and the sentence must not say that nothing is met.
        let result = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS2", Strength::Strong),
            Applied::new("BP4", Strength::Moderate),
        ]);
        assert_eq!(result.classic_class, None);
        assert!(result.class().is_some() && !result.table_rows.is_empty());
        let sentence = summary(&result);
        assert!(sentence.contains("cannot combine the two sides"), "{sentence}");
        assert!(sentence.contains("≥2 Strong (Pathogenic)"), "{sentence}");
        assert!(!sentence.contains("gives no class"), "{sentence}");
    }

    #[test]
    fn with_no_row_met_and_a_strength_the_table_lacks_the_sentence_says_it_gives_no_class() {
        let result = assessed(&[Applied::new("BP4", Strength::Moderate)]);
        assert!(result.table_rows.is_empty());
        assert!(summary(&result).contains("gives no class"));
    }

    #[test]
    fn the_table_shows_with_a_class_and_when_both_sides_are_the_only_reason_for_withholding() {
        assert!(is_shown(&assessed(&[Applied::new("PM2", Strength::Supporting)])));
        assert!(is_shown(&both_sides_withheld()));
    }

    #[test]
    fn the_table_stays_hidden_for_a_contradiction_a_combined_limit_and_no_evidence() {
        assert!(!is_shown(&assessed(&[])), "no evidence");
        // PP3 and BP4 contradict each other, and both sides of the table are met as well.
        let contradiction = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS2", Strength::Strong),
            Applied::new("BS1", Strength::Strong),
            Applied::new("BS2", Strength::Strong),
            Applied::new("PP3", Strength::Supporting),
            Applied::new("BP4", Strength::Supporting),
        ]);
        assert!(!contradiction.conflicts.is_empty() && contradiction.both_sides_met);
        assert!(!is_shown(&contradiction));
    }

    fn row(text: &str) -> &'static classivar_core::TableRow {
        TABLE_ROWS.iter().find(|row| row.text() == text).expect(text)
    }

    #[test]
    fn the_dots_show_how_much_of_each_need_the_evidence_holds() {
        let counts = Counts { very_strong: 1, moderate: 3, ..Counts::default() };
        assert_eq!(
            pips(row("≥1 Very strong + ≥2 Moderate"), &counts),
            [(1, 1), (2, 2)],
            "more than needed fills, never overflows"
        );
        assert_eq!(pips(row("≥1 Very strong + ≥1 Strong"), &counts), [(1, 1), (0, 1)]);
        assert_eq!(pips(row("≥2 Strong"), &counts), [(0, 2)]);
        assert_eq!(pips(row("≥1 Stand-alone"), &counts), [(0, 1)], "pathogenic evidence fills no benign dot");
    }

    fn hint_for(row_text: &str, counts: &Counts, given: &[Class]) -> Option<String> {
        let row = TABLE_ROWS.iter().find(|row| row.text() == row_text).expect(row_text);
        hint(row, counts, given)
    }

    /// The classes these counts already give, the way the drawing works it out.
    fn given_for(counts: &Counts) -> Vec<Class> {
        combining_table::met_by(counts).iter().map(|row| row.class).collect()
    }

    #[test]
    fn a_row_that_is_part_way_met_says_what_is_missing() {
        let counts = Counts { very_strong: 1, ..Counts::default() };
        let given = given_for(&counts);
        assert!(given.is_empty(), "one Very strong criterion meets no row");
        assert_eq!(hint_for("≥1 Very strong + ≥1 Strong", &counts, &given).as_deref(), Some("needs 1 more Strong"));
        assert_eq!(hint_for("≥1 Very strong + ≥2 Moderate", &counts, &given).as_deref(), Some("needs 2 more Moderate"));
        assert_eq!(
            hint_for("≥1 Very strong + ≥1 Moderate + ≥1 Supporting", &counts, &given).as_deref(),
            Some("needs 1 more Moderate + 1 more Supporting")
        );
    }

    #[test]
    fn a_row_that_would_not_change_the_class_says_nothing() {
        // Two Strong and a Moderate already give Pathogenic (via "≥2 Strong") and
        // Likely pathogenic (via "≥1 Strong + ≥1 Moderate"). "≥1 Strong + ≥3 Moderate"
        // is two Moderate short of that same Pathogenic class, so a hint on it would
        // only name a move that changes nothing.
        let result = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
            Applied::new("PM1", Strength::Moderate),
        ]);
        let counts = Counts::from_items(&result.items);
        let given: Vec<Class> = result.table_rows.iter().map(|row| row.class).collect();
        assert_eq!(given, [Class::Pathogenic, Class::LikelyPathogenic]);
        assert_eq!(hint_for("≥1 Strong + ≥3 Moderate", &counts, &given), None);

        // Two Strong alone give Pathogenic, and "≥1 Strong + ≥1 Moderate" would give
        // Likely pathogenic, which the evidence does not have yet: that one is worth a hint.
        let counts = Counts { strong: 2, ..Counts::default() };
        assert_eq!(given_for(&counts), [Class::Pathogenic]);
        assert_eq!(
            hint_for("≥1 Strong + ≥1 Moderate", &counts, &[Class::Pathogenic]).as_deref(),
            Some("needs 1 more Moderate")
        );
    }

    #[test]
    fn a_met_row_a_far_row_and_an_untouched_row_say_nothing() {
        let counts = Counts { very_strong: 1, moderate: 1, ..Counts::default() };
        let given = given_for(&counts);
        assert_eq!(
            hint_for("≥1 Very strong + ≥1 Moderate", &counts, &given),
            None,
            "met rows are marked, not explained"
        );
        // "≥2 Strong" lacks two criteria and the evidence has no Strong: not close, not started.
        assert_eq!(hint_for("≥2 Strong", &counts, &given), None);
        // Three more are needed here, which is too far to be a useful hint.
        assert_eq!(hint_for("≥1 Strong + ≥2 Moderate + ≥2 Supporting", &counts, &given), None);
        // No evidence on the benign side: nothing to build on.
        assert_eq!(hint_for("≥1 Stand-alone", &counts, &given), None);
    }

    fn assessed(applied: &[Applied]) -> classivar_core::Assessment {
        RuleSet::builtin().expect("the built-in rules are valid").assess(applied).expect("valid evidence")
    }

    #[test]
    fn no_row_met_gives_vus_in_words() {
        let result = assessed(&[Applied::new("PM2", Strength::Supporting)]);
        assert_eq!(summary(&result), "No row is met, so the 2015 table gives Uncertain significance (VUS).");
    }

    #[test]
    fn only_the_rows_that_decide_the_class_are_named() {
        // Two Strong and a Moderate also meet "Strong + Moderate" (Likely pathogenic),
        // which Pathogenic already covers. The picture marks both; the sentence names the deciding one.
        let result = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
            Applied::new("PM1", Strength::Moderate),
        ]);
        assert!(result.table_rows.len() > 1);
        assert_eq!(summary(&result), "Met: ≥2 Strong (Pathogenic). The 2015 table gives Pathogenic.");
    }

    #[test]
    fn a_met_row_is_named_with_its_class() {
        let result = assessed(&[Applied::new("PS1", Strength::Strong), Applied::new("PS3", Strength::Strong)]);
        assert_eq!(summary(&result), "Met: ≥2 Strong (Pathogenic). The 2015 table gives Pathogenic.");
    }

    #[test]
    fn the_clingen_addition_is_marked_as_not_from_the_2015_paper() {
        let rules = RuleSet::builtin().unwrap();
        let situation = rules.criterion("PVS1").unwrap().situation_for(Strength::VeryStrong).unwrap().to_string();
        let result = assessed(&[
            Applied::guided("PVS1", Strength::VeryStrong, situation),
            Applied::new("PM2", Strength::Supporting),
        ]);
        assert_eq!(
            summary(&result),
            "Met: ≥1 Very strong + ≥1 Supporting (Likely pathogenic, ClinGen 2020 addition). The 2015 table gives Likely pathogenic."
        );
    }

    #[test]
    fn rows_on_both_sides_are_called_uncertain() {
        let result = assessed(&[
            Applied::new("PS1", Strength::Strong),
            Applied::new("PS3", Strength::Strong),
            Applied::new("BS1", Strength::Strong),
            Applied::new("BS2", Strength::Strong),
        ]);
        let text = summary(&result);
        assert!(text.contains("Rows are met on both sides, which the 2015 guidance calls uncertain."), "{text}");
        assert!(text.ends_with("The 2015 table gives Uncertain significance (VUS)."), "{text}");
    }

    #[test]
    fn a_strength_the_table_does_not_cover_says_so() {
        let result = assessed(&[Applied::new("BP4", Strength::Moderate)]);
        assert_eq!(
            summary(&result),
            "The 2015 table has no row for Moderate or Very strong benign evidence, so it gives no class."
        );
    }
}
