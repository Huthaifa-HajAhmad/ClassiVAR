//! PP3 and BP4 score limits of the prediction tools.

use super::*;

fn reading(tool: &str, score: f64) -> Option<(&'static str, Strength, Strength)> {
    RuleSet::builtin().unwrap().read_predictor(tool, score).map(|r| (r.code, r.published, r.strength))
}

#[test]
fn revel_scores_follow_pejaver_2022_table_2() {
    use Strength::*;
    // Each pair of lines: a score just inside a band, and the limit itself.
    assert_eq!(reading("revel", 0.644), Some(("PP3", Supporting, Supporting)));
    assert_eq!(reading("revel", 0.772), Some(("PP3", Supporting, Supporting)));
    assert_eq!(reading("revel", 0.773), Some(("PP3", Moderate, Moderate)));
    assert_eq!(reading("revel", 0.931), Some(("PP3", Moderate, Moderate)));
    assert_eq!(reading("revel", 0.932), Some(("PP3", Strong, Strong)));
    assert_eq!(reading("revel", 1.0), Some(("PP3", Strong, Strong)));
    assert_eq!(reading("revel", 0.290), Some(("BP4", Supporting, Supporting)));
    assert_eq!(reading("revel", 0.183), Some(("BP4", Moderate, Moderate)));
    // Between the two sides, the table gives no evidence at all.
    assert_eq!(reading("revel", 0.291), None);
    assert_eq!(reading("revel", 0.5), None);
    assert_eq!(reading("revel", 0.643), None);
}

#[test]
fn a_benign_score_the_paper_rates_above_moderate_is_lowered_to_moderate() {
    use Strength::*;
    // The paper rates REVEL <= 0.016 as Strong and <= 0.003 as Very Strong;
    // this program stops BP4 at Moderate and keeps the published level for display.
    assert_eq!(reading("revel", 0.016), Some(("BP4", Strong, Moderate)));
    assert_eq!(reading("revel", 0.003), Some(("BP4", VeryStrong, Moderate)));
    assert_eq!(reading("revel", 0.0), Some(("BP4", VeryStrong, Moderate)));
}

#[test]
fn the_other_calibrated_tools_read_their_own_limits() {
    use Strength::*;
    assert_eq!(reading("bayesdel-noaf", 0.50), Some(("PP3", Strong, Strong)));
    assert_eq!(reading("bayesdel-noaf", -0.36), Some(("BP4", Moderate, Moderate)));
    assert_eq!(reading("bayesdel-noaf", -0.18), Some(("BP4", Supporting, Supporting)));
    assert_eq!(reading("bayesdel-noaf", 0.0), None);
    assert_eq!(reading("mutpred2", 0.829), Some(("PP3", Moderate, Moderate)));
    assert_eq!(reading("mutpred2", 0.01), Some(("BP4", Strong, Moderate)));
    assert_eq!(reading("vest4", 0.965), Some(("PP3", Strong, Strong)));
    assert_eq!(reading("vest4", 0.302), Some(("BP4", Moderate, Moderate)));
}

#[test]
fn spliceai_follows_walker_2023_and_never_goes_above_supporting() {
    use Strength::*;
    let rules = RuleSet::builtin().unwrap();
    // PP3 at 0.2 or more, BP4 at 0.1 or less; both are applied at Supporting
    // although the paper calibrated them to Moderate.
    assert_eq!(reading("spliceai", 0.2), Some(("PP3", Supporting, Supporting)));
    assert_eq!(reading("spliceai", 0.99), Some(("PP3", Supporting, Supporting)));
    assert_eq!(reading("spliceai", 0.1), Some(("BP4", Supporting, Supporting)));
    assert_eq!(reading("spliceai", 0.0), Some(("BP4", Supporting, Supporting)));
    // Between 0.1 and 0.2 the paper gives no evidence either way.
    assert_eq!(reading("spliceai", 0.11), None);
    assert_eq!(reading("spliceai", 0.19), None);
    // SpliceAI delta scores run from 0 to 1; a typo outside that gives nothing.
    assert_eq!(reading("spliceai", -0.01), None);
    assert_eq!(reading("spliceai", 1.01), None);
    let tool = rules.predictor("spliceai").unwrap();
    assert_eq!(tool.calibrated, Some(Moderate));
    assert_eq!((tool.min_score, tool.max_score), (Some(0.0), Some(1.0)));
    assert!(tool.eligibility.as_deref().is_some_and(|text| text.contains("donor")));
}

#[test]
fn every_predictor_names_the_table_its_numbers_come_from() {
    let rules = RuleSet::builtin().unwrap();
    assert!(rules.predictors.iter().all(|p| !p.reference.trim().is_empty()));
    assert_eq!(rules.predictor("spliceai").unwrap().reference, "Walker 2023, Table 1");
    assert_eq!(rules.predictor("revel").unwrap().reference, "Pejaver 2022, Table 2");
}

#[test]
fn a_score_that_is_not_a_number_or_an_unknown_tool_gives_nothing() {
    assert_eq!(reading("revel", f64::NAN), None);
    assert_eq!(reading("revel", f64::INFINITY), None);
    assert_eq!(reading("not-a-tool", 0.9), None);
}

#[test]
fn predictor_limits_that_run_the_wrong_way_are_refused() {
    // Strong must lie further from the middle than Moderate.
    assert!(matches!(
        edited(
            r#"{ strength = "strong", at = 0.932 }]
benign = [{ strength = "supporting", at = 0.290 }"#,
            r#"{ strength = "strong", at = 0.700 }]
benign = [{ strength = "supporting", at = 0.290 }"#
        ),
        Err(RuleError::BadPredictor { .. })
    ));
    // The benign and pathogenic sides may not overlap.
    assert!(matches!(
        edited(r#"{ strength = "supporting", at = 0.290 }"#, r#"{ strength = "supporting", at = 0.700 }"#),
        Err(RuleError::BadPredictor { .. })
    ));
    // A predictor must cite a source the file lists.
    assert!(matches!(
        edited(
            "id = \"vest4\"\nname = \"VEST4\"\nsource = \"pejaver2022\"",
            "id = \"vest4\"\nname = \"VEST4\"\nsource = \"nowhere\""
        ),
        Err(RuleError::UnknownSource { .. })
    ));
}

#[test]
fn a_predictor_must_name_the_table_its_limits_come_from() {
    // The reference is saved with the reason, so an empty one is refused.
    assert!(matches!(
        edited(r#"reference = "Pejaver 2022, Table 2""#, r#"reference = """#),
        Err(RuleError::BadPredictor { .. })
    ));
    // A file written before the field existed gets the loader's message, not a serde error.
    assert!(matches!(edited("reference = \"Pejaver 2022, Table 2\"\n", ""), Err(RuleError::BadPredictor { .. })));
}

#[test]
fn predictor_metadata_that_would_mislead_is_refused() {
    // The range must be a pair with min below max.
    assert!(matches!(
        edited("min_score = 0.0\nmax_score = 1.0", "min_score = 1.0\nmax_score = 0.0"),
        Err(RuleError::BadPredictor { .. })
    ));
    assert!(matches!(
        edited("min_score = 0.0\nmax_score = 1.0", "min_score = 0.0"),
        Err(RuleError::BadPredictor { .. })
    ));
    // An empty confirmation text would render an empty condition.
    assert!(matches!(
        edited(
            r#"eligibility = "the variant is outside the donor and acceptor +/-1,2 positions""#,
            r#"eligibility = """#
        ),
        Err(RuleError::BadPredictor { .. })
    ));
    // The applied code cannot be stronger than the evidence the paper calibrated.
    let stronger_than_calibrated =
        BUILTIN.replacen(r#"calibrated = "moderate""#, r#"calibrated = "supporting""#, 1).replacen(
            r#"pathogenic = [{ strength = "supporting", at = 0.2 }]"#,
            r#"pathogenic = [{ strength = "moderate", at = 0.2 }]"#,
            1,
        );
    assert!(matches!(RuleSet::parse(&stronger_than_calibrated), Err(RuleError::BadPredictor { .. })));
}
