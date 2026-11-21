//! PS3 and BS3 assay limits (Brnich 2019).

use super::*;

fn oddspath(value: f64) -> Option<(&'static str, Strength)> {
    RuleSet::builtin().unwrap().read_oddspath(value).map(|r| (r.code, r.strength))
}

#[test]
fn oddspath_limits_follow_brnich_2019_and_are_crossed_strictly() {
    use Strength::*;
    // Pathogenic: above 2.1, 4.3, 18.7 and 350. A value on a limit does not cross it.
    assert_eq!(oddspath(2.1), None);
    assert_eq!(oddspath(2.11), Some(("PS3", Supporting)));
    assert_eq!(oddspath(4.3), Some(("PS3", Supporting)));
    assert_eq!(oddspath(4.31), Some(("PS3", Moderate)));
    assert_eq!(oddspath(18.7), Some(("PS3", Moderate)));
    assert_eq!(oddspath(18.71), Some(("PS3", Strong)));
    assert_eq!(oddspath(350.0), Some(("PS3", Strong)));
    assert_eq!(oddspath(350.1), Some(("PS3", VeryStrong)));
    // Benign: below 0.48, 0.23 and 0.053.
    assert_eq!(oddspath(0.48), None);
    assert_eq!(oddspath(0.47), Some(("BS3", Supporting)));
    assert_eq!(oddspath(0.23), Some(("BS3", Supporting)));
    assert_eq!(oddspath(0.22), Some(("BS3", Moderate)));
    assert_eq!(oddspath(0.053), Some(("BS3", Moderate)));
    assert_eq!(oddspath(0.05), Some(("BS3", Strong)));
    assert_eq!(oddspath(0.0), Some(("BS3", Strong)));
    // The indeterminate range, and values that cannot be an OddsPath.
    assert_eq!(oddspath(1.0), None);
    assert_eq!(oddspath(-0.1), None);
    assert_eq!(oddspath(f64::NAN), None);
    assert_eq!(oddspath(f64::INFINITY), None);
}

#[test]
fn without_statistics_the_validation_controls_decide_and_stop_at_moderate() {
    // Brnich 2019, step 3: Supporting needs experimental controls and replicates but
    // allows "10 or fewer" validation controls; Moderate needs "at least 11 total
    // validation controls including a mix of benign and pathogenic variants".
    use Strength::*;
    let rules = RuleSet::builtin().unwrap();
    let read = |damaging, experimental, pathogenic, benign| {
        let assay = AssayValidation {
            experimental_controls: experimental,
            pathogenic_controls: pathogenic,
            benign_controls: benign,
        };
        rules.read_functional_controls(damaging, assay).map(|r| (r.code, r.strength))
    };
    // Without normal and abnormal experimental controls and replicates: no evidence.
    assert_eq!(read(true, false, 20, 20), None);
    assert_eq!(read(false, false, 0, 0), None);
    // With them, few or no validation controls still give Supporting.
    assert_eq!(read(true, true, 0, 0), Some(("PS3", Supporting)));
    assert_eq!(read(true, true, 5, 5), Some(("PS3", Supporting)));
    assert_eq!(read(false, true, 6, 4), Some(("BS3", Supporting)));
    // Eleven or more, with both kinds, reach Moderate.
    assert_eq!(read(true, true, 6, 5), Some(("PS3", Moderate)));
    assert_eq!(read(false, true, 1, 499), Some(("BS3", Moderate)));
    // Eleven of one kind only is not "a mix": Supporting.
    assert_eq!(read(true, true, 11, 0), Some(("PS3", Supporting)));
    assert_eq!(read(true, true, 0, 30), Some(("PS3", Supporting)));
}

#[test]
fn functional_limits_that_run_the_wrong_way_are_refused() {
    assert!(matches!(
        edited(r#"{ strength = "strong", at = 18.7 }"#, r#"{ strength = "strong", at = 3.0 }"#),
        Err(RuleError::BadFunctional(_))
    ));
    assert!(matches!(
        edited(r#"{ strength = "supporting", at = 0.48 }"#, r#"{ strength = "supporting", at = 3.0 }"#),
        Err(RuleError::BadFunctional(_))
    ));
    assert!(matches!(
        edited("controls_moderate_min = 11", "controls_moderate_min = 0"),
        Err(RuleError::BadFunctional(_))
    ));
    assert!(matches!(
        edited("controls_moderate_min = 11", "controls_moderate_min = 1"),
        Err(RuleError::BadFunctional(_))
    ));
    assert!(matches!(
        edited("source = \"brnich2019\"\ncontrols", "source = \"nowhere\"\ncontrols"),
        Err(RuleError::UnknownSource { .. })
    ));
}
