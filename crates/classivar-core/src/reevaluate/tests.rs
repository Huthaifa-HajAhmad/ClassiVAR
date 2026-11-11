use super::*;
use crate::model::Strength::*;

fn rules() -> &'static RuleSet {
    RuleSet::builtin().expect("the built-in rules are valid")
}

fn applied(list: &[(&str, Strength)]) -> Vec<Applied> {
    list.iter()
        .map(|(code, strength)| match rules().criterion(code).and_then(|c| c.situation_for(*strength)) {
            Some(situation) => Applied::guided(*code, *strength, situation),
            None => Applied::new(*code, *strength),
        })
        .collect()
}

fn record(label: &str, year: u16, class: Class, list: &[(&str, Strength)]) -> Submission {
    Submission {
        label: label.into(),
        year,
        reported_class: class,
        reported_tier: None,
        rule_set: None,
        criteria: applied(list),
    }
}

/// The fictional variant from the design sketch.
fn sketch_records() -> Vec<Submission> {
    vec![
        record("Lab A", 2019, Class::Vus, &[("PM2", Moderate), ("PP3", Supporting)]),
        record("Lab B", 2022, Class::Vus, &[("PM2", Moderate), ("PP3", Supporting), ("PP4", Supporting)]),
    ]
}

#[test]
fn the_sketch_example_holds_and_explains_the_pm2_rule_change() {
    let today = applied(&[("PM2", Supporting), ("PP3", Supporting), ("PP4", Supporting), ("PM5", Moderate)]);
    let r = rules().reevaluate(&sketch_records(), &today);

    assert_eq!(r.records[0].assessment.as_ref().unwrap().total_points, 3);
    assert_eq!(r.records[1].assessment.as_ref().unwrap().total_points, 4);
    assert_eq!(r.today.as_ref().unwrap().total_points, 5);
    assert_eq!(r.verdict, Some(Verdict::Holds));
    assert_eq!(
        r.reasons,
        vec![
            Reason::RuleChanged {
                code: "PM2".into(),
                from: Moderate,
                to: Supporting,
                source: "clingen-svi-pm2".into()
            },
            Reason::Added { code: "PM5".into(), strength: Moderate },
        ]
    );
    // Table rows follow the rule file order: PM2, PM5, PP3, PP4.
    let codes: Vec<&str> = r.rows.iter().map(|row| row.code.as_str()).collect();
    assert_eq!(codes, ["PM2", "PM5", "PP3", "PP4"]);
    assert_eq!(r.rows[0].cells, vec![Some(Moderate), Some(Moderate), Some(Supporting)]);
    assert_eq!(r.rows[1].cells, vec![None, None, Some(Moderate)]);
}

#[test]
fn new_strong_evidence_makes_an_upgrade_candidate() {
    let today =
        applied(&[("PM2", Supporting), ("PP3", Supporting), ("PP4", Supporting), ("PM5", Moderate), ("PS3", Strong)]);
    let r = rules().reevaluate(&sketch_records(), &today);
    assert_eq!(r.today.as_ref().unwrap().total_points, 9);
    assert_eq!(r.verdict, Some(Verdict::UpgradeCandidate));
    assert!(r.reasons.contains(&Reason::Added { code: "PS3".into(), strength: Strength::Strong }));
}

#[test]
fn dropping_evidence_from_a_likely_pathogenic_record_is_a_downgrade_candidate() {
    let records = vec![record("Lab A", 2020, Class::LikelyPathogenic, &[("PS3", Strong), ("PM2", Moderate)])];
    let today = applied(&[("PM2", Supporting)]);
    let r = rules().reevaluate(&records, &today);
    assert_eq!(r.verdict, Some(Verdict::DowngradeCandidate));
    assert!(r.reasons.contains(&Reason::Removed { code: "PS3".into() }));
}

#[test]
fn a_strength_changed_by_the_curator_is_not_called_a_rule_change() {
    let records = vec![record("Lab A", 2021, Class::Vus, &[("PP3", Supporting)])];
    let today = applied(&[("PP3", Moderate)]);
    let r = rules().reevaluate(&records, &today);
    assert_eq!(r.reasons, vec![Reason::StrengthChanged { code: "PP3".into(), from: Supporting, to: Moderate }]);
}

#[test]
fn no_verdict_without_records_or_without_todays_evidence() {
    assert_eq!(rules().reevaluate(&[], &applied(&[("PM2", Supporting)])).verdict, None);
    assert_eq!(rules().reevaluate(&sketch_records(), &[]).verdict, None);
}

#[test]
fn contradicting_evidence_today_gives_no_verdict() {
    let r = rules().reevaluate(&sketch_records(), &applied(&[("PS1", Strong), ("PM5", Moderate)]));
    assert_eq!(r.verdict, None);
}

#[test]
fn the_latest_record_is_the_one_with_the_latest_year() {
    // Entered out of order: the 2022 record is first.
    let records = vec![
        record("Lab B", 2022, Class::LikelyPathogenic, &[("PS3", Strong), ("PM2", Moderate)]),
        record("Lab A", 2019, Class::Vus, &[("PM2", Moderate)]),
    ];
    let today = applied(&[("PS3", Strong), ("PM2", Moderate)]);
    let r = rules().reevaluate(&records, &today);
    assert_eq!(r.verdict, Some(Verdict::Holds));
    assert!(r.reasons.is_empty());
}

#[test]
fn a_record_whose_class_does_not_match_its_own_evidence_is_flagged_and_gets_no_upgrade_verdict() {
    let records = vec![record("Lab A", 2020, Class::Pathogenic, &[("PM2", Supporting)])];
    let r = rules().reevaluate(&records, &applied(&[("PM2", Supporting), ("PS3", Strong)]));
    assert_eq!(r.records[0].consistency(), Consistency::Differs);
    assert_eq!(r.verdict, Some(Verdict::RecordInconsistent));
}

#[test]
fn a_record_that_was_right_under_the_2015_table_is_not_flagged() {
    // PVS1 + PM2 at Moderate: 10 points (Pathogenic) but the table says Likely pathogenic.
    let records = vec![record("Lab A", 2019, Class::LikelyPathogenic, &[("PVS1", VeryStrong), ("PM2", Moderate)])];
    let r = rules().reevaluate(&records, &[]);
    assert_eq!(r.records[0].consistency(), Consistency::Matches);
}

#[test]
fn the_same_evidence_with_a_different_method_is_not_an_upgrade() {
    // The record reported LP (2015 table). Today's points give P for the very same evidence.
    let evidence = [("PVS1", VeryStrong), ("PM2", Moderate)];
    let records = vec![record("Lab A", 2019, Class::LikelyPathogenic, &evidence)];
    let r = rules().reevaluate(&records, &applied(&evidence));
    assert!(r.reasons.is_empty());
    assert_eq!(r.today.as_ref().unwrap().points_class, Class::Pathogenic);
    assert_eq!(r.verdict, Some(Verdict::FrameworkDiffers));
}

#[test]
fn a_discouraged_criterion_still_counts_in_an_old_record_but_not_today() {
    let records = vec![record("Lab A", 2017, Class::LikelyPathogenic, &[("PS3", Strong), ("PP5", Strong)])];
    let r = rules().reevaluate(&records, &applied(&[("PS3", Strong), ("PP5", Strong)]));
    assert_eq!(r.records[0].assessment.as_ref().unwrap().total_points, 8);
    assert_eq!(r.today.as_ref().unwrap().total_points, 4);
    // Same evidence, so no reasons; the drop comes from PP5 no longer counting.
    assert!(r.reasons.is_empty());
    assert_eq!(r.verdict, Some(Verdict::FrameworkDiffers));
}

#[test]
fn a_changed_rule_set_is_a_reason() {
    let mut old = record("Lab A", 2020, Class::Vus, &[("PM2", Supporting)]);
    old.rule_set = Some(RuleSetRef { id: "acmg-amp-points".into(), version: "0.0.1".into() });
    let r = rules().reevaluate(&[old], &applied(&[("PM2", Supporting)]));
    assert!(matches!(r.reasons.as_slice(), [Reason::RuleSetChanged { .. }]));
}

#[test]
fn a_changed_rule_set_alone_is_not_an_upgrade() {
    // Same evidence and a different class: only the rules differ, so say so.
    let mut old = record("Lab A", 2020, Class::LikelyPathogenic, &[("PVS1", VeryStrong), ("PM2", Moderate)]);
    old.rule_set = Some(RuleSetRef { id: "acmg-amp-points".into(), version: "0.0.1".into() });
    let r = rules().reevaluate(&[old], &applied(&[("PVS1", VeryStrong), ("PM2", Moderate)]));
    assert!(matches!(r.reasons.as_slice(), [Reason::RuleSetChanged { .. }]));
    assert_eq!(r.verdict, Some(Verdict::FrameworkDiffers));
}

#[test]
fn contradictory_evidence_in_a_record_is_not_checked() {
    let records = vec![record("Lab A", 2020, Class::LikelyPathogenic, &[("PS1", Strong), ("PM5", Moderate)])];
    let r = rules().reevaluate(&records, &[]);
    assert_eq!(r.records[0].consistency(), Consistency::NotChecked);
}

#[test]
fn a_changed_vus_tier_is_a_reason() {
    let mut r = rules().clone();
    for (tier, min) in r.vus_tiers.iter_mut().zip([4, 2, 0]) {
        tier.min_points = Some(min); // made-up numbers, for the test only
    }
    let mut old = record("Lab A", 2020, Class::Vus, &[("PP3", Supporting)]);
    old.reported_tier = Some("VUS-low".into());
    let result = r.reevaluate(&[old], &applied(&[("PP3", Supporting), ("PP4", Supporting), ("PM5", Moderate)]));
    assert!(result.reasons.contains(&Reason::TierChanged { from: "VUS-low".into(), to: "VUS-high".into() }));
}

#[test]
fn bad_evidence_in_one_record_does_not_hide_the_others() {
    let records = vec![
        record("Lab A", 2020, Class::Vus, &[("ZZ9", Supporting)]),
        record("Lab B", 2021, Class::Vus, &[("PM2", Supporting)]),
    ];
    let r = rules().reevaluate(&records, &applied(&[("PM2", Supporting)]));
    assert_eq!(r.records[0].assessment, Err(AssessError::UnknownCriterion("ZZ9".into())));
    assert!(r.records[1].assessment.is_ok());
    assert_eq!(r.records[0].consistency(), Consistency::NotChecked);
    assert_eq!(r.verdict, Some(Verdict::Holds));
}
