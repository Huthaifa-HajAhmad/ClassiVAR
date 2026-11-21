//! The ACMG/AMP 2015 combining table (Richards et al., Table 5) as data, plus
//! three additions, each marked with `added_by`:
//! - Two Very Strong criteria give Pathogenic. In 2015 PVS1 was the only Very
//!   Strong criterion, so the table had no row for two; later ClinGen
//!   recommendations let PM3, PS2, PM6 and PS3 reach Very Strong. On the points
//!   scale (Tavtigian 2020) two are 16 points, Pathogenic from 10, more than the
//!   12 of "Very Strong + Strong". All 58 ClinGen expert-panel classifications
//!   with two Very Strong criteria in the benchmark snapshot are Pathogenic.
//! - ClinGen's PM2 recommendation of 4 September 2020 lowered PM2 to Supporting
//!   and, so that PVS1 + PM2 stays Likely pathogenic, added "one Very Strong +
//!   one Supporting = Likely pathogenic".
//! - Pejaver 2022 (Discussion) introduced BP4 at Moderate, which the 2015 table
//!   has no row for, and allowed it to invoke the "likely benign" rules alone or
//!   with one Strong benign criterion. This file applies that to any Moderate
//!   benign evidence: Richards 2015 combines by strength, not by code, BS3 at
//!   Moderate (Brnich 2019) is the other Moderate benign criterion, and on the
//!   points scale one Moderate (-2) equals the two Supporting of an existing
//!   row. One Moderate benign criterion gives Likely benign.
//!
//! Each row names the evidence it needs. Keeping the rows as a list, instead of
//! one long condition, lets the screen show which row the evidence meets. The
//! rows hold the same rules the engine used before; a test compares the two over
//! every combination of counts.

use crate::assess::Item;
use crate::model::{Class, Direction, Strength};

/// "At least this many criteria of this strength."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    pub strength: Strength,
    pub at_least: usize,
}

/// One row of the table: evidence on one side that gives a class.
#[derive(Debug, PartialEq, Eq)]
pub struct TableRow {
    pub side: Direction,
    pub class: Class,
    /// Every need must be met for the row to apply.
    pub needs: &'static [Need],
    /// Who added the row, for the rows that are not in the 2015 paper.
    pub added_by: Option<&'static str>,
}

impl TableRow {
    /// The row in words, for example "≥1 Very strong + ≥2 Moderate". "≥" means "at least".
    pub fn text(&self) -> String {
        let parts: Vec<_> =
            self.needs.iter().map(|need| format!("≥{} {}", need.at_least, need.strength.label())).collect();
        parts.join(" + ")
    }

    /// What the counts still lack for this row: for each need that is not met,
    /// how many more criteria of that strength are required. Empty means the row is met.
    pub fn shortfall(&self, counts: &Counts) -> Vec<Need> {
        self.needs
            .iter()
            .filter_map(|need| {
                let have = counts.of(self.side, need.strength);
                (have < need.at_least).then(|| Need { strength: need.strength, at_least: need.at_least - have })
            })
            .collect()
    }
}

/// How many counted criteria of each strength there are on each side. The table
/// has no rows for other strengths, such as Moderate benign evidence.
#[derive(Debug, Default, Clone, Copy)]
pub struct Counts {
    pub very_strong: usize,
    pub strong: usize,
    pub moderate: usize,
    pub supporting: usize,
    pub stand_alone_benign: usize,
    pub strong_benign: usize,
    pub moderate_benign: usize,
    pub supporting_benign: usize,
}

impl Counts {
    pub fn from_items(items: &[Item]) -> Self {
        let mut counts = Self::default();
        for item in items.iter().filter(|item| item.counted) {
            match (item.direction, item.strength) {
                (Direction::Pathogenic, Strength::VeryStrong) => counts.very_strong += 1,
                (Direction::Pathogenic, Strength::Strong) => counts.strong += 1,
                (Direction::Pathogenic, Strength::Moderate) => counts.moderate += 1,
                (Direction::Pathogenic, Strength::Supporting) => counts.supporting += 1,
                (Direction::Benign, Strength::StandAlone) => counts.stand_alone_benign += 1,
                (Direction::Benign, Strength::Strong) => counts.strong_benign += 1,
                (Direction::Benign, Strength::Moderate) => counts.moderate_benign += 1,
                (Direction::Benign, Strength::Supporting) => counts.supporting_benign += 1,
                _ => {}
            }
        }
        counts
    }

    fn of(&self, side: Direction, strength: Strength) -> usize {
        match (side, strength) {
            (Direction::Pathogenic, Strength::VeryStrong) => self.very_strong,
            (Direction::Pathogenic, Strength::Strong) => self.strong,
            (Direction::Pathogenic, Strength::Moderate) => self.moderate,
            (Direction::Pathogenic, Strength::Supporting) => self.supporting,
            (Direction::Benign, Strength::StandAlone) => self.stand_alone_benign,
            (Direction::Benign, Strength::Strong) => self.strong_benign,
            (Direction::Benign, Strength::Moderate) => self.moderate_benign,
            (Direction::Benign, Strength::Supporting) => self.supporting_benign,
            _ => 0,
        }
    }
}

const fn need(strength: Strength, at_least: usize) -> Need {
    Need { strength, at_least }
}

const fn row(side: Direction, class: Class, needs: &'static [Need]) -> TableRow {
    TableRow { side, class, needs, added_by: None }
}

/// The table, strongest class first on each side. `sides` relies on that order.
pub static ROWS: &[TableRow] = {
    use Class::{Benign, LikelyBenign, LikelyPathogenic, Pathogenic};
    use Direction::{Benign as Against, Pathogenic as For};
    use Strength::{Moderate as M, Strong as S, Supporting as P, VeryStrong as VS};
    &[
        TableRow { side: For, class: Pathogenic, needs: &[need(VS, 2)], added_by: Some("Tavtigian 2020 points") },
        row(For, Pathogenic, &[need(VS, 1), need(S, 1)]),
        row(For, Pathogenic, &[need(VS, 1), need(M, 2)]),
        row(For, Pathogenic, &[need(VS, 1), need(M, 1), need(P, 1)]),
        row(For, Pathogenic, &[need(VS, 1), need(P, 2)]),
        row(For, Pathogenic, &[need(S, 2)]),
        row(For, Pathogenic, &[need(S, 1), need(M, 3)]),
        row(For, Pathogenic, &[need(S, 1), need(M, 2), need(P, 2)]),
        row(For, Pathogenic, &[need(S, 1), need(M, 1), need(P, 4)]),
        row(For, LikelyPathogenic, &[need(VS, 1), need(M, 1)]),
        TableRow {
            side: For,
            class: LikelyPathogenic,
            needs: &[need(VS, 1), need(P, 1)],
            added_by: Some("ClinGen 2020"),
        },
        row(For, LikelyPathogenic, &[need(S, 1), need(M, 1)]),
        row(For, LikelyPathogenic, &[need(S, 1), need(P, 2)]),
        row(For, LikelyPathogenic, &[need(M, 3)]),
        row(For, LikelyPathogenic, &[need(M, 2), need(P, 2)]),
        row(For, LikelyPathogenic, &[need(M, 1), need(P, 4)]),
        row(Against, Benign, &[need(Strength::StandAlone, 1)]),
        row(Against, Benign, &[need(S, 2)]),
        row(Against, LikelyBenign, &[need(S, 1), need(P, 1)]),
        row(Against, LikelyBenign, &[need(P, 2)]),
        TableRow { side: Against, class: LikelyBenign, needs: &[need(M, 1)], added_by: Some("Pejaver 2022") },
    ]
};

/// The rows these counts meet, in table order.
pub fn met_by(counts: &Counts) -> Vec<&'static TableRow> {
    ROWS.iter().filter(|row| row.shortfall(counts).is_empty()).collect()
}

/// The rows the counted criteria meet.
pub fn met_rows(items: &[Item]) -> Vec<&'static TableRow> {
    met_by(&Counts::from_items(items))
}

/// What the met rows give on each side: (pathogenic side, benign side).
pub fn sides(met: &[&TableRow]) -> (Option<Class>, Option<Class>) {
    let first = |side: Direction| met.iter().find(|row| row.side == side).map(|row| row.class);
    (first(Direction::Pathogenic), first(Direction::Benign))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The conditions the engine used before the table became data, copied as they were.
    fn old_sides(c: &Counts) -> (Option<Class>, Option<Class>) {
        let (pvs, ps, pm, pp) = (c.very_strong, c.strong, c.moderate, c.supporting);
        let (ba, bs, bm, bp) = (c.stand_alone_benign, c.strong_benign, c.moderate_benign, c.supporting_benign);
        // `pvs >= 2` is the Tavtigian 2020 points addition; the rest are the 2015 rules as they were.
        let pathogenic = pvs >= 2
            || (pvs >= 1 && (ps >= 1 || pm >= 2 || (pm >= 1 && pp >= 1) || pp >= 2))
            || ps >= 2
            || (ps >= 1 && (pm >= 3 || (pm >= 2 && pp >= 2) || (pm >= 1 && pp >= 4)));
        let likely_pathogenic = (pvs >= 1 && pm >= 1)
            || (pvs >= 1 && pp >= 1)
            || (ps >= 1 && pm >= 1)
            || (ps >= 1 && pp >= 2)
            || pm >= 3
            || (pm >= 2 && pp >= 2)
            || (pm >= 1 && pp >= 4);
        let benign = ba >= 1 || bs >= 2;
        // The last condition is the Pejaver 2022 addition; the rest are the 2015 rules as they were.
        let likely_benign = (bs >= 1 && bp >= 1) || bp >= 2 || bm >= 1;
        let pathogenic_class = match (pathogenic, likely_pathogenic) {
            (true, _) => Some(Class::Pathogenic),
            (false, true) => Some(Class::LikelyPathogenic),
            _ => None,
        };
        let benign_class = match (benign, likely_benign) {
            (true, _) => Some(Class::Benign),
            (false, true) => Some(Class::LikelyBenign),
            _ => None,
        };
        (pathogenic_class, benign_class)
    }

    #[test]
    fn the_rows_give_the_same_classes_as_the_old_conditions_for_every_count() {
        let mut checked = 0;
        for n in 0..5usize.pow(8) {
            let digit = |place: u32| (n / 5usize.pow(place)) % 5;
            let counts = Counts {
                very_strong: digit(0),
                strong: digit(1),
                moderate: digit(2),
                supporting: digit(3),
                stand_alone_benign: digit(4),
                strong_benign: digit(5),
                supporting_benign: digit(6),
                moderate_benign: digit(7),
            };
            assert_eq!(sides(&met_by(&counts)), old_sides(&counts), "{counts:?}");
            checked += 1;
        }
        assert_eq!(checked, 390_625);
    }

    #[test]
    fn each_side_lists_its_strongest_class_first() {
        for side in [Direction::Pathogenic, Direction::Benign] {
            let classes: Vec<_> = ROWS.iter().filter(|row| row.side == side).map(|row| row.class).collect();
            assert!(!classes.is_empty());
            let mut sorted = classes.clone();
            if side == Direction::Pathogenic {
                sorted.sort_by(|a, b| b.cmp(a));
            } else {
                sorted.sort();
            }
            assert_eq!(classes, sorted, "{side:?} rows must run from the strongest class to the weakest");
        }
    }

    #[test]
    fn rows_agree_with_their_side_and_are_all_different() {
        for row in ROWS {
            let expected = if row.class > Class::Vus { Direction::Pathogenic } else { Direction::Benign };
            assert_eq!(row.side, expected, "{}", row.text());
        }
        for (i, row) in ROWS.iter().enumerate() {
            assert!(ROWS[i + 1..].iter().all(|other| other != row), "{} appears twice", row.text());
        }
        let added: Vec<_> = ROWS.iter().filter_map(|row| row.added_by).collect();
        assert_eq!(added, ["Tavtigian 2020 points", "ClinGen 2020", "Pejaver 2022"]);
    }

    fn row_with(text: &str) -> &'static TableRow {
        ROWS.iter().find(|row| row.text() == text && row.added_by.is_none()).expect(text)
    }

    #[test]
    fn the_shortfall_says_how_many_more_of_each_strength_a_row_needs() {
        // One Very strong criterion and nothing else.
        let counts = Counts { very_strong: 1, ..Counts::default() };
        let short =
            |row: &TableRow| row.shortfall(&counts).iter().map(|n| (n.strength, n.at_least)).collect::<Vec<_>>();
        assert_eq!(short(row_with("≥1 Very strong + ≥1 Strong")), [(Strength::Strong, 1)]);
        assert_eq!(short(row_with("≥1 Very strong + ≥2 Moderate")), [(Strength::Moderate, 2)]);
        assert_eq!(short(row_with("≥2 Strong")), [(Strength::Strong, 2)]);
        assert!(row_with("≥1 Very strong + ≥1 Moderate").shortfall(&Counts { moderate: 1, ..counts }).is_empty());
    }

    #[test]
    fn adding_exactly_the_shortfall_meets_the_row_and_nothing_less_does() {
        for n in 0..3usize.pow(8) {
            let digit = |place: u32| (n / 3usize.pow(place)) % 3;
            let counts = Counts {
                very_strong: digit(0),
                strong: digit(1),
                moderate: digit(2),
                supporting: digit(3),
                stand_alone_benign: digit(4),
                strong_benign: digit(5),
                supporting_benign: digit(6),
                moderate_benign: digit(7),
            };
            for row in ROWS {
                let short = row.shortfall(&counts);
                assert_eq!(short.is_empty(), met_by(&counts).contains(&row), "{counts:?} {}", row.text());
                let mut more = counts;
                for need in &short {
                    match (row.side, need.strength) {
                        (Direction::Pathogenic, Strength::VeryStrong) => more.very_strong += need.at_least,
                        (Direction::Pathogenic, Strength::Strong) => more.strong += need.at_least,
                        (Direction::Pathogenic, Strength::Moderate) => more.moderate += need.at_least,
                        (Direction::Pathogenic, Strength::Supporting) => more.supporting += need.at_least,
                        (Direction::Benign, Strength::StandAlone) => more.stand_alone_benign += need.at_least,
                        (Direction::Benign, Strength::Strong) => more.strong_benign += need.at_least,
                        (Direction::Benign, Strength::Moderate) => more.moderate_benign += need.at_least,
                        (Direction::Benign, Strength::Supporting) => more.supporting_benign += need.at_least,
                        _ => unreachable!("the table has no such row"),
                    }
                }
                assert!(row.shortfall(&more).is_empty(), "{counts:?} {}", row.text());
            }
        }
    }

    #[test]
    fn a_row_reads_in_words() {
        let row =
            ROWS.iter().find(|row| row.added_by == Some("ClinGen 2020")).expect("the 2020 addition is in the table");
        assert_eq!(row.text(), "≥1 Very strong + ≥1 Supporting");
        assert_eq!(row.class, Class::LikelyPathogenic);
    }
}
