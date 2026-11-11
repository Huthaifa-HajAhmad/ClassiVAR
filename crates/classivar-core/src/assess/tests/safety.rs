//! Conflicts, caps, warnings, guides and strength limits.

use super::*;

#[test]
fn ba1_has_no_other_strength_and_stays_benign() {
    let ba1 = rules().criterion("BA1").unwrap();
    assert_eq!(ba1.allowed_strengths(), vec![StandAlone]);
    assert_eq!(ba1.recorded_strengths(), vec![StandAlone]);
    // A record cannot be given a weaker BA1 either.
    assert!(rules().assess_as(&[Applied::new("BA1", Supporting)], Counting::AsRecorded).is_err());
}

#[test]
fn a_guided_criterion_needs_a_guide_answer() {
    // The engine, not only the screen, refuses a new PVS1 at a strength the
    // tree did not give.
    let bare = [Applied::new("PVS1", VeryStrong)];
    assert_eq!(rules().assess(&bare), Err(AssessError::GuideAnswerRequired { code: "PVS1".into() }));
    let pvs1 = rules().criterion("PVS1").unwrap();
    let start_loss =
        pvs1.guide.iter().find(|g| g.situation.starts_with("Start-loss") && g.strength == Some(Supporting)).unwrap();
    assert!(rules().assess(&[Applied::guided("PVS1", Supporting, start_loss.id.clone())]).is_ok());
}

#[test]
fn pvs1_and_pp3_together_are_flagged_as_double_counting() {
    let a = apply(&[("PVS1", VeryStrong), ("PP3", Strong)]);
    assert!(a.warnings.iter().any(|w| matches!(w, Warning::DoubleCounted { a, b, .. } if a == "PVS1" && b == "PP3")));
}

#[test]
fn ps2_and_pm6_together_are_flagged_as_double_counting() {
    let a = apply(&[("PS2", VeryStrong), ("PM6", Strong)]);
    assert!(a.warnings.iter().any(|w| matches!(w, Warning::DoubleCounted { a, b, .. } if a == "PS2" && b == "PM6")));
}

#[test]
fn pp1_and_pp4_together_are_capped_at_five_points() {
    // ClinGen 2024: Strong + Strong (8) is not an allowed pair for these two.
    let a = apply(&[("PP1", Strong), ("PP4", Strong)]);
    assert_eq!(a.total_points, 8);
    assert_eq!(a.combination_overuse.len(), 1);
    assert_eq!(a.class(), None);
    assert_eq!(a.combination_overuse[0].max_points, 5);

    // Strong + Supporting is the strongest allowed pair.
    let a = apply(&[("PP1", Strong), ("PP4", Supporting)]);
    assert!(a.combination_overuse.is_empty());
    assert_eq!(a.class(), Some(Class::Vus));
}

#[test]
fn pm1_and_pp3_together_are_capped_at_strong() {
    // Pejaver 2022: the sum of PP3 and PM1 may not go above Strong (4 points).
    let a = apply(&[("PM1", Moderate), ("PP3", Strong)]);
    assert_eq!(a.total_points, 6);
    assert_eq!(a.combination_overuse.len(), 1);
    assert_eq!(a.class(), None);

    // Moderate + Moderate is exactly Strong, which is allowed.
    let a = apply(&[("PM1", Moderate), ("PP3", Moderate)]);
    assert!(a.combination_overuse.is_empty());
    assert_eq!(a.class(), Some(Class::Vus));
}

#[test]
fn a_start_loss_at_the_guide_strength_does_not_become_likely_pathogenic() {
    // PVS1_Supporting for a start-loss + PM2_Supporting: 2 points, a VUS. At the old
    // Very Strong default the same variant scored 9 points, Likely pathogenic.
    let a = apply(&[("PVS1", Supporting), ("PM2", Supporting)]);
    assert_eq!(a.total_points, 2);
    assert_eq!(a.class(), Some(Class::Vus));
}

#[test]
fn pvs1_and_pm4_are_not_used_together() {
    let a = apply(&[("PVS1", Strong), ("PM4", Moderate)]);
    assert_eq!(a.class(), None);
    assert!(a.conflicts.iter().any(|c| c.a == "PVS1" && c.b == "PM4"));
}

#[test]
fn weak_evidence_cannot_be_raised_to_a_confident_class() {
    let r = rules();
    for code in ["PP2", "PP4", "BP1", "BP4", "BP7"] {
        let one = [Applied::new(code, VeryStrong)];
        assert_eq!(
            r.assess(&one),
            Err(AssessError::StrengthNotAllowed { code: code.into(), strength: VeryStrong }),
            "{code}"
        );
    }
}

#[test]
fn contradicting_criteria_withhold_the_class() {
    // PS1 + PM5 would add to 6 points (Likely pathogenic) although PS1 alone is only 4.
    let a = apply(&[("PS1", Strong), ("PM5", Moderate)]);
    assert_eq!(a.conflicts.len(), 1);
    assert_eq!((a.conflicts[0].a.as_str(), a.conflicts[0].b.as_str()), ("PS1", "PM5"));
    assert_eq!(a.class(), None);

    // BA1 + PM2 must not be turned into "Likely benign" by adding the points.
    let a = apply(&[("BA1", StandAlone), ("PM2", Moderate)]);
    assert_eq!(a.class(), None);

    // PP3 + BP4: computational evidence cannot point both ways.
    assert_eq!(apply(&[("PP3", Supporting), ("BP4", Supporting)]).class(), None);

    // Removing one of the pair gives the class back.
    assert_eq!(apply(&[("PS1", Strong)]).class(), Some(Class::Vus));
}

#[test]
fn contradictory_evidence_gets_no_class_from_the_2015_table_either() {
    let a = apply(&[("PS1", Strong), ("PM5", Moderate)]);
    assert_eq!(a.classic_class, None);
    assert_eq!(a.frameworks_agree(), None);
}

#[test]
fn an_old_record_may_use_a_strength_that_is_capped_today() {
    let pp3 = applied(&[("PP3", VeryStrong)]);
    assert!(rules().assess(&pp3).is_err());
    let old = rules().assess_as(&pp3, Counting::AsRecorded).unwrap();
    assert_eq!(old.total_points, 8);
}

#[test]
fn pp4_and_ps4_together_are_flagged_as_double_counting() {
    let a = apply(&[("PP4", Strong), ("PS4", Strong)]);
    assert!(a.warnings.iter().any(|w| matches!(w, Warning::DoubleCounted { a, b, .. } if a == "PP4" && b == "PS4")));
    assert!(!apply(&[("PP4", Strong)]).warnings.iter().any(|w| matches!(w, Warning::DoubleCounted { .. })));
}

#[test]
fn a_guide_answer_is_not_reported_as_a_manual_strength_change() {
    // The PVS1 Supporting answer explains the strength; it must not look
    // like the curator raised or lowered the criterion by hand.
    let pvs1 = rules().criterion("PVS1").unwrap();
    let situation = pvs1.situation_for(Supporting).unwrap().to_string();
    let a = rules().assess(&[Applied::guided("PVS1", Supporting, situation)]).unwrap();
    assert!(a.warnings.iter().all(|w| !matches!(w, Warning::StrengthChanged { .. })), "{a:?}");
    assert_eq!(a.total_points, 1);
}

#[test]
fn the_reason_for_a_raised_strength_travels_with_the_warning() {
    let list = [Applied::new("PM2", Moderate).with_reason("founder population")];
    let a = rules().assess(&list).unwrap();
    assert!(a.warnings.contains(&Warning::StrengthChanged {
        code: "PM2".into(),
        from: Supporting,
        to: Moderate,
        reason: Some("founder population".into())
    }));
    // Empty and over-long reasons are refused at the door.
    let empty = [Applied::new("PM2", Moderate).with_reason("  ")];
    assert_eq!(rules().assess(&empty), Err(AssessError::BadReason { code: "PM2".into() }));
    let long = [Applied::new("PM2", Moderate).with_reason("x".repeat(MAX_REASON_CHARS + 1))];
    assert_eq!(rules().assess(&long), Err(AssessError::BadReason { code: "PM2".into() }));
}

#[test]
fn discouraged_criteria_are_listed_but_not_counted_today() {
    let a = apply(&[("PP5", Supporting)]);
    assert!(!a.has_evidence);
    assert_eq!(a.total_points, 0);
    assert_eq!(a.class(), None);
    assert_eq!(a.items[0].points, 0);
    assert!(a.warnings.iter().any(|w| matches!(w, Warning::Discouraged { code, counted: false, .. } if code == "PP5")));

    // PS3 Strong + PP5 Strong would be 8 points; PP5 adds nothing.
    let a = apply(&[("PS3", Strong), ("PP5", Strong)]);
    assert_eq!(a.total_points, 4);
    assert_eq!(a.classic_class, Some(Class::Vus));
}

#[test]
fn discouraged_criteria_count_when_rescoring_an_old_record() {
    let a = rules().assess_as(&applied(&[("PS3", Strong), ("PP5", Strong)]), Counting::AsRecorded).unwrap();
    assert_eq!(a.total_points, 8);
    assert!(a.warnings.iter().any(|w| matches!(w, Warning::Discouraged { counted: true, .. })));
}

#[test]
fn changing_a_strength_is_reported() {
    let a = apply(&[("PM2", Moderate)]);
    assert_eq!(a.total_points, 2);
    assert!(a.warnings.contains(&Warning::StrengthChanged {
        code: "PM2".into(),
        from: Supporting,
        to: Moderate,
        reason: None
    }));
}

#[test]
fn bad_input_is_rejected() {
    let r = rules();
    let one = |code: &str, strength| vec![Applied::new(code, strength)];
    assert_eq!(r.assess(&one("XX9", Strong)), Err(AssessError::UnknownCriterion("XX9".into())));
    assert_eq!(
        r.assess(&one("PS1", StandAlone)),
        Err(AssessError::StrengthNotAllowed { code: "PS1".into(), strength: StandAlone })
    );
    let twice = vec![Applied::new("PS1", Strong), Applied::new("PS1", Strong)];
    assert_eq!(r.assess(&twice), Err(AssessError::DuplicateCriterion("PS1".into())));
}

#[test]
fn a_computational_criterion_cannot_be_raised_to_very_strong() {
    let r = rules();
    let pp3 = vec![Applied::new("PP3", VeryStrong)];
    assert_eq!(r.assess(&pp3), Err(AssessError::StrengthNotAllowed { code: "PP3".into(), strength: VeryStrong }));
    // Strong is still allowed (calibrated predictors can reach it).
    assert!(r.assess(&[Applied::new("PP3", Strong)]).is_ok());
}

#[test]
fn the_assessment_names_the_rules_that_produced_it() {
    let a = apply(&[("PS3", Strong)]);
    assert_eq!(a.rule_set, rules().reference());
}

#[test]
fn vus_tiers_are_named_only_when_every_tier_has_a_range() {
    let mut r = rules().clone();
    assert_eq!(r.vus_tier_for(3), None);
    for (tier, min) in r.vus_tiers.iter_mut().zip([4, 2, 0]) {
        tier.min_points = Some(min); // made-up numbers, for the test only
    }
    assert_eq!(r.vus_tier_for(5).as_deref(), Some("VUS-high"));
    assert_eq!(r.vus_tier_for(2).as_deref(), Some("VUS-mid"));
    assert_eq!(r.vus_tier_for(0).as_deref(), Some("VUS-low"));
}
