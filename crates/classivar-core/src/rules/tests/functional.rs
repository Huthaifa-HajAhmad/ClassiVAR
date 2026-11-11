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
fn without_statistics_the_number_of_controls_decides_and_stops_at_moderate() {
    use Strength::*;
    let rules = RuleSet::builtin().unwrap();
    let read = |damaging, controls| rules.read_functional_controls(damaging, controls).map(|r| (r.code, r.strength));
    // No validation controls: no evidence at all, in either direction.
    assert_eq!(read(true, 0), None);
    assert_eq!(read(false, 0), None);
    assert_eq!(read(true, 1), Some(("PS3", Supporting)));
    assert_eq!(read(true, 10), Some(("PS3", Supporting)));
    assert_eq!(read(true, 11), Some(("PS3", Moderate)));
    assert_eq!(read(true, 500), Some(("PS3", Moderate)));
    assert_eq!(read(false, 10), Some(("BS3", Supporting)));
    assert_eq!(read(false, 11), Some(("BS3", Moderate)));
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
        edited("controls_supporting_min = 1", "controls_supporting_min = 0"),
        Err(RuleError::BadFunctional(_))
    ));
    assert!(matches!(
        edited("controls_supporting_min = 1", "controls_supporting_min = 11"),
        Err(RuleError::BadFunctional(_))
    ));
    assert!(matches!(
        edited("source = \"brnich2019\"\ncontrols", "source = \"nowhere\"\ncontrols"),
        Err(RuleError::UnknownSource { .. })
    ));
}
