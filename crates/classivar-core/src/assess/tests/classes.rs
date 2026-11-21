//! Points, classes, the 2015 table and the "next class" hints.

use super::*;

#[test]
fn builtin_rules_are_valid_and_complete() {
    let r = rules();
    assert_eq!(r.criteria.len(), 28);
    assert_eq!(r.criteria.iter().filter(|c| c.direction == Direction::Pathogenic).count(), 16);
    assert_eq!(r.criteria.iter().filter(|c| c.direction == Direction::Benign).count(), 12);
    assert!(r.criteria.iter().all(|c| r.source(&c.source).is_some()));
}

#[test]
fn the_sketch_example_is_a_vus_one_point_short_of_likely_pathogenic() {
    // PM2 supporting (1) + PP3 (1) + PP4 (1) + PM5 moderate (2) = 5
    let a = apply(&[("PM2", Supporting), ("PP3", Supporting), ("PP4", Supporting), ("PM5", Moderate)]);
    assert_eq!(a.total_points, 5);
    assert_eq!(a.points_class, Class::Vus);
    assert_eq!(a.class(), Some(Class::Vus));
    assert_eq!(a.to_upgrade, Some(NextClass { class: Class::LikelyPathogenic, points_needed: 1 }));
    assert_eq!(a.to_downgrade, Some(NextClass { class: Class::LikelyBenign, points_needed: 6 }));
    assert_eq!(a.vus_tier, None, "no official tier ranges exist yet");
}

#[test]
fn class_edges() {
    let r = rules();
    assert_eq!(r.class_for_points(10), Class::Pathogenic);
    assert_eq!(r.class_for_points(9), Class::LikelyPathogenic);
    assert_eq!(r.class_for_points(6), Class::LikelyPathogenic);
    assert_eq!(r.class_for_points(5), Class::Vus);
    assert_eq!(r.class_for_points(0), Class::Vus);
    assert_eq!(r.class_for_points(-1), Class::LikelyBenign);
    assert_eq!(r.class_for_points(-6), Class::LikelyBenign);
    assert_eq!(r.class_for_points(-7), Class::Benign);
}

#[test]
fn no_evidence_is_not_a_classification() {
    let a = apply(&[]);
    assert!(!a.has_evidence);
    assert_eq!(a.total_points, 0);
    assert_eq!(a.class(), None, "zero points with no criteria must not look like a VUS");
    assert_eq!(a.classic_class, None);
    assert_eq!(a.frameworks_agree(), None);
}

#[test]
fn classic_table_pathogenic_and_likely_pathogenic() {
    // PVS1 + PS3 -> Pathogenic
    assert_eq!(apply(&[("PVS1", VeryStrong), ("PS3", Strong)]).classic_class, Some(Class::Pathogenic));
    // Two strong -> Pathogenic
    assert_eq!(apply(&[("PS1", Strong), ("PS3", Strong)]).classic_class, Some(Class::Pathogenic));
    // PVS1 + PM2 at moderate -> Likely pathogenic
    assert_eq!(apply(&[("PVS1", VeryStrong), ("PM2", Moderate)]).classic_class, Some(Class::LikelyPathogenic));
    // Three moderate -> Likely pathogenic
    assert_eq!(
        apply(&[("PM1", Moderate), ("PM3", Moderate), ("PM4", Moderate)]).classic_class,
        Some(Class::LikelyPathogenic)
    );
    // One strong + one supporting is not enough
    assert_eq!(apply(&[("PS3", Strong), ("PP3", Supporting)]).classic_class, Some(Class::Vus));
}

#[test]
fn pvs1_with_pm2_at_supporting_stays_likely_pathogenic_in_both_methods() {
    // The ClinGen 2020 addition: one Very Strong + one Supporting = Likely pathogenic.
    let a = apply(&[("PVS1", VeryStrong), ("PM2", Supporting)]);
    assert_eq!(a.total_points, 9);
    assert_eq!(a.points_class, Class::LikelyPathogenic);
    assert_eq!(a.classic_class, Some(Class::LikelyPathogenic));
    assert_eq!(a.frameworks_agree(), Some(true));
}

#[test]
fn the_assessment_names_the_rows_of_the_table_it_meets() {
    // PVS1 + PM2 Supporting meets one row: the ClinGen 2020 addition.
    let a = apply(&[("PVS1", VeryStrong), ("PM2", Supporting)]);
    assert_eq!(a.table_rows.len(), 1);
    assert_eq!(a.table_rows[0].added_by, Some("ClinGen 2020"));
    assert_eq!(a.table_rows[0].class, Class::LikelyPathogenic);
    // Two Strong criteria meet the "two Strong" row, and Pathogenic comes first.
    let a = apply(&[("PS1", Strong), ("PS3", Strong)]);
    assert_eq!(a.table_rows[0].class, Class::Pathogenic);
    assert_eq!(a.table_rows[0].text(), "≥2 Strong");
    // One Supporting criterion meets no row, so the table gives VUS.
    let a = apply(&[("PM2", Supporting)]);
    assert!(a.table_rows.is_empty());
    assert_eq!(a.classic_class, Some(Class::Vus));
}

#[test]
fn classic_table_benign_side() {
    assert_eq!(apply(&[("BA1", StandAlone)]).classic_class, Some(Class::Benign));
    assert_eq!(apply(&[("BS1", Strong), ("BS2", Strong)]).classic_class, Some(Class::Benign));
    assert_eq!(apply(&[("BS1", Strong), ("BP4", Supporting)]).classic_class, Some(Class::LikelyBenign));
    assert_eq!(apply(&[("BP4", Supporting), ("BP7", Supporting)]).classic_class, Some(Class::LikelyBenign));
}

#[test]
fn moderate_benign_evidence_meets_the_likely_benign_row_pejaver_added() {
    // Pejaver 2022 (Discussion): BP4_Moderate alone, or with one Strong benign
    // criterion, may invoke the 2015 "likely benign" rules. Before this row the
    // table had no row for it and gave no class at all.
    let a = apply(&[("BP4", Moderate)]);
    assert_eq!(a.classic_class, Some(Class::LikelyBenign));
    assert_eq!(a.table_rows.len(), 1);
    assert_eq!(a.table_rows[0].added_by, Some("Pejaver 2022"));
    assert_eq!(apply(&[("BS1", Strong), ("BP4", Moderate)]).classic_class, Some(Class::LikelyBenign));
    // The points agree: -2 is Likely benign.
    assert_eq!(a.class(), Some(Class::LikelyBenign));
}

#[test]
fn moderate_benign_with_a_confident_pathogenic_side_now_needs_expert_judgment() {
    // Two Strong pathogenic criteria meet Pathogenic and BP4_Moderate now meets
    // Likely benign: both sides of the table are met while the points (6) would
    // say Likely pathogenic, so the class is withheld, as for any two-sided case.
    let a = apply(&[("PS1", Strong), ("PS3", Strong), ("BP4", Moderate)]);
    assert!(a.both_sides_met);
    assert_eq!(a.classic_class, Some(Class::Vus));
    assert_eq!(a.class(), None);
}

#[test]
fn both_sides_met_gives_vus_in_the_classic_table() {
    let a = apply(&[("PS1", Strong), ("PS3", Strong), ("BS1", Strong), ("BS2", Strong)]);
    assert_eq!(a.classic_class, Some(Class::Vus));
    assert!(a.warnings.contains(&Warning::ConflictingEvidence));
}

#[test]
fn both_sides_met_withholds_the_class_instead_of_adding_up() {
    // PS1 + PS3 meets Pathogenic (2 strong); BS1 + BS2 + BS4 meets Benign. The points
    // alone would say -4: a confident Likely benign for evidence the table calls uncertain.
    let a = apply(&[("PS1", Strong), ("PS3", Strong), ("BS1", Strong), ("BS2", Strong), ("BS4", Strong)]);
    assert_eq!(a.total_points, -4);
    assert_eq!(a.points_class, Class::LikelyBenign);
    assert!(a.both_sides_met);
    assert_eq!(a.class(), None);
    assert_eq!(a.to_upgrade, None);
    assert_eq!(a.to_downgrade, None);

    // Evidence on both sides that meets a rule on one side only still adds up.
    let a = apply(&[("PS3", Strong), ("BS1", Strong), ("BS2", Strong)]);
    assert!(!a.both_sides_met);
    assert_eq!(a.class(), Some(Class::LikelyBenign));
    assert_eq!(a.classic_class, Some(Class::Benign));
}

#[test]
fn both_sides_met_is_reported_as_the_vus_both_methods_agree_on() {
    // PS3 + PS4 meets Pathogenic; BS1 + BS2 meets Benign. The points are 0 and
    // the 2015 table says VUS, so refusing to name a class would be wrong.
    let a = apply(&[("PS3", Strong), ("PS4", Strong), ("BS1", Strong), ("BS2", Strong)]);
    assert_eq!(a.total_points, 0);
    assert_eq!(a.points_class, Class::Vus);
    assert!(a.both_sides_met);
    assert_eq!(a.class(), Some(Class::Vus));
    assert_eq!(a.classic_class, Some(Class::Vus));
    assert_eq!(a.frameworks_agree(), Some(true));
}

#[test]
fn points_and_classic_table_can_disagree() {
    // PVS1 alone is 8 points (Likely pathogenic) but the 2015 table needs more.
    let a = apply(&[("PVS1", VeryStrong)]);
    assert_eq!(a.points_class, Class::LikelyPathogenic);
    assert_eq!(a.classic_class, Some(Class::Vus));
    assert_eq!(a.frameworks_agree(), Some(false));
}

#[test]
fn ba1_alone_is_benign_by_points_too() {
    let a = apply(&[("BA1", StandAlone)]);
    assert_eq!(a.total_points, -8);
    assert_eq!(a.class(), Some(Class::Benign));
    assert_eq!(a.frameworks_agree(), Some(true));
}

#[test]
fn stand_alone_benign_evidence_is_not_outvoted() {
    // BA1 is stand-alone: adding pathogenic evidence must not lift it to
    // Likely benign or VUS. -8 + 4 = -4 would otherwise be Likely benign.
    let a = apply(&[("BA1", StandAlone), ("PS3", Strong)]);
    assert_eq!(a.total_points, -4);
    assert_eq!(a.class(), Some(Class::Benign));
    assert_eq!(a.classic_class, Some(Class::Benign));
    assert_eq!(a.to_upgrade, None, "there is no higher class to reach from stand-alone benign");

    // BA1 against evidence that itself reaches Pathogenic (two strong) is a real
    // conflict, as with a known common pathogenic variant: it needs expert judgment.
    let a = apply(&[("BA1", StandAlone), ("PS3", Strong), ("PS4", Strong)]);
    assert_eq!(a.total_points, 0);
    assert!(a.both_sides_met);
    assert_eq!(a.class(), None);
}

#[test]
fn ba1_is_withheld_against_any_evidence_that_meets_a_pathogenic_rule_not_only_pathogenic() {
    // PVS1 + PM5 meets "Likely pathogenic" in the 2015 table, short of Pathogenic. With
    // BA1 both sides are met, and the points (-8 + 8 + 2 = 2) would hide that, so the
    // class is withheld. The rule is "a pathogenic rule is met", not "Pathogenic is met".
    let a = apply(&[("BA1", StandAlone), ("PVS1", VeryStrong), ("PM5", Moderate)]);
    assert_eq!(a.total_points, 2);
    assert_eq!(a.classic_class, Some(Class::Vus));
    assert!(a.both_sides_met);
    assert_eq!(a.class(), None);
    // Pathogenic evidence that meets no rule at all leaves BA1 alone: still Benign.
    let a = apply(&[("BA1", StandAlone), ("PM5", Moderate)]);
    assert!(!a.both_sides_met);
    assert_eq!(a.class(), Some(Class::Benign));
}
#[test]
fn a_withheld_class_has_no_next_class_hint() {
    let a = apply(&[("PS3", Strong), ("BS3", Strong)]);
    assert_eq!(a.class(), None);
    assert_eq!(a.to_upgrade, None);
    assert_eq!(a.to_downgrade, None);
    assert_eq!(a.vus_tier, None);
}
