//! PP1 and PP4 points and the strength choices of ClinGen 2024.

use super::*;

/// ClinGen 2024, Table 2, copied here by hand so the rule file is checked against
/// an independent copy: (diagnostic yield in percent, points).
const TABLE_2: [(f64, f64); 23] = [
    (19.1, 1.0),
    (25.4, 1.5),
    (33.0, 2.0),
    (41.5, 2.5),
    (50.6, 3.0),
    (59.6, 3.5),
    (68.0, 4.0),
    (75.4, 4.5),
    (81.6, 5.0),
    (86.4, 5.5),
    (90.2, 6.0),
    (93.0, 6.5),
    (95.0, 7.0),
    (96.5, 7.5),
    (97.5, 8.0),
    (98.3, 8.5),
    (98.8, 9.0),
    (99.2, 9.5),
    (99.4, 10.0),
    (99.6, 10.5),
    (99.7, 11.0),
    (99.8, 11.5),
    (99.9, 12.0),
];

#[test]
fn pp4_points_follow_every_row_of_table_2_and_round_down_between_rows() {
    let rules = RuleSet::builtin().unwrap();
    let mut below = None;
    for (at, points) in TABLE_2 {
        assert_eq!(rules.pp4_points(at, 1), Some(points), "at {at}%");
        // Just under a row, the row below applies (nothing under the first row).
        assert_eq!(rules.pp4_points(at - 0.01, 1), below, "just under {at}%");
        below = Some(points);
    }
    assert_eq!(rules.pp4_points(0.0, 1), None);
    assert_eq!(rules.pp4_points(100.0, 1), Some(12.0));
    // Not a percentage.
    assert_eq!(rules.pp4_points(100.1, 1), None);
    assert_eq!(rules.pp4_points(-5.0, 1), None);
    assert_eq!(rules.pp4_points(f64::NAN, 1), None);
    assert_eq!(rules.pp4_points(f64::INFINITY, 1), None);
}

#[test]
fn variants_sharing_an_allele_divide_the_yield_not_the_points() {
    let rules = RuleSet::builtin().unwrap();
    // The paper's example (Figure 2B and Section 3.3): a 95.8% yield is 7 points.
    // Shared by two variants, each has 47.9%, which rounds down to the 41.5% row:
    // 2.5 points each. Dividing the 7 points would wrongly give 3.5.
    assert_eq!(rules.pp4_points(95.8, 1), Some(7.0));
    assert_eq!(rules.pp4_points(95.8, 2), Some(2.5));
    // A yield that falls below the table once shared earns nothing.
    assert_eq!(rules.pp4_points(30.0, 2), None);
    // No variants is not a case.
    assert_eq!(rules.pp4_points(95.8, 0), None);
}

#[test]
fn pp1_points_follow_table_3() {
    let rules = RuleSet::builtin().unwrap();
    assert_eq!(rules.pp1_points("dominant", 0), Some(0.0));
    assert_eq!(rules.pp1_points("dominant", 3), Some(3.0));
    assert_eq!(rules.pp1_points("dominant", 5), Some(5.0));
    assert_eq!(rules.pp1_points("x-linked-male", 4), Some(4.0));
    assert_eq!(rules.pp1_points("recessive-affected", 1), Some(2.0));
    assert_eq!(rules.pp1_points("recessive-affected", 5), Some(10.0));
    // 0.4 is not exact in binary; the result must still read as the table does.
    assert_eq!(rules.pp1_points("recessive-unaffected", 3), Some(1.2));
    assert_eq!(rules.pp1_points("recessive-unaffected", 5), Some(2.0));
    // The paper says to keep adding 0.4 for each one above five.
    assert_eq!(rules.pp1_points("recessive-unaffected", 7), Some(2.8));
    assert_eq!(rules.pp1_points("not-a-kind", 3), None);
}

/// The PP1 and PP4 strengths of each choice, in the order the rule file lists them.
type Choices = Vec<(Option<Strength>, Option<Strength>)>;

fn locus_with(pp1: f64, pp4: f64, pp1_allowed: bool) -> (f64, bool, Choices) {
    let r = RuleSet::builtin().unwrap().read_locus(pp1, pp4, pp1_allowed).unwrap();
    (r.total, r.capped, r.choices.iter().map(|c| (c.pp1, c.pp4)).collect())
}

fn locus(pp1: f64, pp4: f64) -> (f64, bool, Choices) {
    locus_with(pp1, pp4, true)
}

#[test]
fn locus_points_give_the_strength_choices_of_table_4() {
    use Strength::*;
    // 0 to 0.9 points: not applicable.
    assert_eq!(locus(0.0, 0.0), (0.0, false, vec![]));
    assert_eq!(locus(0.4, 0.5), (0.9, false, vec![]));
    // 1 to 1.9: one criterion at Supporting.
    let one = vec![(Some(Supporting), None), (None, Some(Supporting))];
    assert_eq!(locus(1.0, 0.0), (1.0, false, one.clone()));
    assert_eq!(locus(0.5, 1.4), (1.9, false, one));
    // 2 to 2.9.
    let two = vec![(Some(Moderate), None), (None, Some(Moderate)), (Some(Supporting), Some(Supporting))];
    assert_eq!(locus(1.0, 1.0), (2.0, false, two.clone()));
    assert_eq!(locus(0.0, 2.9), (2.9, false, two));
    // 3 to 3.9.
    let three = vec![(Some(Supporting), Some(Moderate)), (Some(Moderate), Some(Supporting))];
    assert_eq!(locus(1.0, 2.0), (3.0, false, three.clone()));
    assert_eq!(locus(0.0, 3.5), (3.5, false, three));
    // 4 to 4.9.
    let four = vec![(Some(Strong), None), (None, Some(Strong)), (Some(Moderate), Some(Moderate))];
    assert_eq!(locus(4.0, 0.0), (4.0, false, four.clone()));
    assert_eq!(locus(2.0, 2.5), (4.5, false, four));
    // 5 or more: the largest allowed pair is Strong with Supporting, and the sum stops at 5.
    let five = vec![(Some(Strong), Some(Supporting)), (Some(Supporting), Some(Strong))];
    assert_eq!(locus(3.0, 2.0), (5.0, false, five.clone()));
    assert_eq!(locus(5.0, 12.0), (5.0, true, five));
}

#[test]
fn the_reading_keeps_the_sum_before_the_cap() {
    let reading = RuleSet::builtin().unwrap().read_locus(5.0, 12.0, true).unwrap();
    assert_eq!((reading.sum, reading.total, reading.capped), (17.0, 5.0, true));
}

#[test]
fn when_only_one_gene_causes_the_phenotype_only_pp4_choices_are_listed() {
    use Strength::*;
    // Table 4 says "maximum allowable", so a lower band's PP4-only choice is allowed
    // when the band the total reaches has none (bands 3 and 5 have only pairs).
    assert_eq!(locus_with(0.0, 0.5, false), (0.5, false, vec![]));
    assert_eq!(locus_with(0.0, 1.5, false), (1.5, false, vec![(None, Some(Supporting))]));
    assert_eq!(locus_with(0.0, 2.0, false), (2.0, false, vec![(None, Some(Moderate))]));
    assert_eq!(locus_with(0.0, 3.5, false), (3.5, false, vec![(None, Some(Moderate))]));
    assert_eq!(locus_with(0.0, 4.5, false), (4.5, false, vec![(None, Some(Strong))]));
    // The most PP4 can reach alone is Strong, even at the 5 point limit.
    assert_eq!(locus_with(0.0, 12.0, false), (5.0, true, vec![(None, Some(Strong))]));
}

#[test]
fn a_point_value_that_is_not_a_number_or_is_negative_gives_no_reading() {
    let rules = RuleSet::builtin().unwrap();
    for (pp1, pp4) in [(f64::NAN, 1.0), (1.0, f64::NAN), (f64::INFINITY, 0.0), (0.0, f64::INFINITY), (-1.0, 3.0)] {
        assert_eq!(rules.read_locus(pp1, pp4, true), None, "{pp1}, {pp4}");
    }
}

#[test]
fn locus_points_that_add_to_a_band_edge_are_not_lost_to_rounding_noise() {
    // 0.1 + 0.7 is 0.7999999999999999 in binary arithmetic, and plus 0.2 it
    // lands just under 1.0. The sum is rounded before the band is chosen.
    let (total, _, choices) = locus(0.1 + 0.7, 0.2);
    assert_eq!(total, 1.0);
    assert_eq!(choices.len(), 2);
}

#[test]
fn no_locus_choice_promises_more_than_the_points_earned_or_the_pair_limit() {
    let rules = RuleSet::builtin().unwrap();
    let locus = rules.locus_evidence.as_ref().unwrap();
    let cap = rules.combination_caps.iter().find(|c| c.a == "PP1" && c.b == "PP4").unwrap();
    for band in &locus.bands {
        for choice in &band.options {
            let worth: i32 = [choice.pp1, choice.pp4].into_iter().flatten().map(|s| rules.magnitude(s)).sum();
            assert!(f64::from(worth) <= band.min_points, "{choice:?} in the band from {}", band.min_points);
            assert!(worth <= cap.max_points, "{choice:?} goes over the PP1+PP4 limit");
        }
    }
    assert_eq!(f64::from(cap.max_points), locus.cap_points);
}

#[test]
fn locus_bands_that_would_overstate_the_evidence_are_refused() {
    let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadLocusEvidence(_)));
    let band_one = r#"{ min_points = 1.0, options = [{ pp1 = "supporting" }, { pp4 = "supporting" }] }"#;
    // A Moderate choice in the band that starts at one point.
    assert!(is_bad(edited(
        band_one,
        r#"{ min_points = 1.0, options = [{ pp1 = "moderate" }, { pp4 = "supporting" }] }"#
    )));
    // A choice that names neither criterion.
    assert!(is_bad(edited(band_one, r#"{ min_points = 1.0, options = [{ }, { pp4 = "supporting" }] }"#)));
    // A band with no choice.
    assert!(is_bad(edited(band_one, "{ min_points = 1.0, options = [] }")));
    // A choice with a strength PP1 does not allow.
    assert!(is_bad(edited(
        r#"{ pp1 = "strong", pp4 = "supporting" }"#,
        r#"{ pp1 = "stand_alone", pp4 = "supporting" }"#
    )));
    // No bands at all.
    assert!(is_bad(without_lines("{ min_points")));
    // A band that starts above the limit for the pair can never be reached.
    assert!(is_bad(edited("cap_points = 5.0", "cap_points = 3.0")));
    // Bands must rise.
    assert!(is_bad(edited("{ min_points = 3.0,", "{ min_points = 1.0,")));
}

#[test]
fn locus_table_content_that_would_overstate_the_evidence_is_refused() {
    let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadLocusEvidence(_)));
    // Yield rows must rise and stay percentages.
    assert!(is_bad(edited("{ at = 25.4, points = 1.5 }", "{ at = 10.0, points = 1.5 }")));
    assert!(is_bad(edited("{ at = 99.9, points = 12.0 }", "{ at = 120.0, points = 12.0 }")));
    // Co-segregation kinds need positive points, unique ids, and there must be some.
    assert!(is_bad(edited("points_each = 0.4", "points_each = 0.0")));
    assert!(is_bad(edited(r#"id = "x-linked-male""#, r#"id = "dominant""#)));
    assert!(is_bad(edited(r#"{ id = "dominant", label"#, r#"{ id = "", label"#)));
    assert!(is_bad(without_lines("{ id = ")));
    // The texts shown with the helper must not be empty.
    assert!(is_bad(with_line_replaced("homogeneity_text", r#"homogeneity_text = """#)));
    assert!(is_bad(edited("yield_text_minimum = 20.0", "yield_text_minimum = 0.0")));
    // The table must cite a source the file lists.
    assert!(matches!(
        edited("source = \"clingen-pp1-pp4-2024\"\ncap_points", "source = \"nowhere\"\ncap_points"),
        Err(RuleError::UnknownSource { .. })
    ));
}

#[test]
fn the_locus_table_must_agree_with_the_rest_of_the_rules() {
    let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadLocusEvidence(_)));
    // The class is withheld by the PP1/PP4 combination cap, so it must equal the helper's limit.
    assert!(is_bad(edited("max_points = 5", "max_points = 6")));
    assert!(is_bad(edited("a = \"PP1\"\nb = \"PP4\"", "a = \"PP1\"\nb = \"PM1\"")));
    // Table 4 is written in the standard points.
    assert!(is_bad(edited("strong = 4\nvery_strong = 8", "strong = 5\nvery_strong = 8")));
}
