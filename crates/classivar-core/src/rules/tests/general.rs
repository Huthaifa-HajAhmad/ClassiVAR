//! The checks every rule file must pass: guides, review status, tiers, caps and pairs.

use super::*;

#[test]
fn no_supporting_criterion_can_reach_very_strong() {
    let rules = RuleSet::builtin().unwrap();
    for c in rules.criteria.iter().filter(|c| c.strength == Strength::Supporting && !c.discouraged) {
        assert!(!c.allowed_strengths().contains(&Strength::VeryStrong), "{}", c.code);
    }
    // Old records are read as they were made.
    let pp2 = rules.criterion("PP2").unwrap();
    assert!(pp2.recorded_strengths().contains(&Strength::VeryStrong));
}

/// The strongest level a curator may choose for each criterion in the generic
/// rule file, and the published route that allows it. A level above this needs a
/// gene-specific rule file (an expert panel specification), not a typed reason.
const PUBLISHED_CEILINGS: [(&str, Strength, &str); 26] = [
    ("PVS1", Strength::VeryStrong, "Abou Tayoun 2018 tree"),
    ("PS1", Strength::Strong, "Richards 2015"),
    ("PS2", Strength::VeryStrong, "ClinGen SVI de novo v1.1, Table 2"),
    ("PS3", Strength::VeryStrong, "Brnich 2019, OddsPath above 350"),
    ("PS4", Strength::Strong, "Richards 2015"),
    ("PM1", Strength::Moderate, "Richards 2015"),
    ("PM2", Strength::Moderate, "Richards 2015 (ClinGen 2020 lowered the default)"),
    ("PM3", Strength::VeryStrong, "ClinGen SVI PM3 v1.0, Table 2"),
    ("PM4", Strength::Moderate, "Richards 2015"),
    ("PM5", Strength::Moderate, "Richards 2015"),
    ("PM6", Strength::VeryStrong, "ClinGen SVI de novo v1.1, Table 2"),
    ("PP1", Strength::Strong, "ClinGen 2024, Table 4"),
    ("PP2", Strength::Supporting, "Richards 2015"),
    ("PP3", Strength::Strong, "Pejaver 2022"),
    ("PP4", Strength::Strong, "ClinGen 2024, Table 4"),
    ("BA1", Strength::StandAlone, "Richards 2015"),
    ("BS1", Strength::Strong, "Richards 2015"),
    ("BS2", Strength::Strong, "Richards 2015"),
    ("BS3", Strength::Strong, "Brnich 2019"),
    ("BS4", Strength::Strong, "ClinGen 2024"),
    ("BP1", Strength::Supporting, "Richards 2015"),
    ("BP2", Strength::Supporting, "Richards 2015"),
    ("BP3", Strength::Supporting, "Richards 2015"),
    ("BP4", Strength::Moderate, "Pejaver 2022"),
    ("BP5", Strength::Supporting, "Richards 2015"),
    ("BP7", Strength::Strong, "Walker 2023, BP7_Strong (RNA)"),
];

#[test]
fn no_criterion_can_be_raised_above_its_published_ceiling() {
    let rules = RuleSet::builtin().unwrap();
    for (code, ceiling, route) in PUBLISHED_CEILINGS {
        let allowed = rules.criterion(code).unwrap().allowed_strengths();
        assert_eq!(allowed.last(), Some(&ceiling), "{code}: the strongest choice must be the one {route} supports");
    }
}

#[test]
fn bp7_offers_only_its_two_published_levels() {
    // Walker 2023: BP7 at Supporting from predictions, BP7_Strong (RNA) from an
    // assay. No source gives a Moderate BP7, so the menu must not offer one.
    let bp7 = RuleSet::builtin().unwrap().criterion("BP7").unwrap().clone();
    assert_eq!(bp7.allowed_strengths(), [Strength::Supporting, Strength::Strong]);
    assert_eq!(bp7.recorded_strengths(), [Strength::Supporting, Strength::Moderate, Strength::Strong]);
}

#[test]
fn listed_levels_must_be_readable_and_include_the_default() {
    let line = "levels = [\"supporting\", \"strong\"]";
    assert!(edited(line, "levels = [\"strong\"]").is_err(), "the default (Supporting) must be a level");
    assert!(edited(line, "levels = [\"supporting\", \"very_strong\"]").is_err(), "no Very Strong benign level");
    assert!(edited(line, "levels = [\"strong\", \"supporting\"]").is_err(), "weakest first");
}

#[test]
fn benign_evidence_has_no_very_strong_level() {
    // The 2015 table has no very strong benign row. BA1 is the one stand-alone exception.
    let rules = RuleSet::builtin().unwrap();
    for c in rules.criteria.iter().filter(|c| c.direction == Direction::Benign && c.code != "BA1" && !c.discouraged) {
        assert!(!c.allowed_strengths().contains(&Strength::VeryStrong), "{}", c.code);
        assert!(!c.recorded_strengths().contains(&Strength::VeryStrong), "{}", c.code);
    }
}

#[test]
fn the_pvs1_guide_follows_the_clingen_tree() {
    // Abou Tayoun 2018: "generally does not recommend assigning PVS1 or PVS1_Strong for start loss variants".
    let pvs1 = RuleSet::builtin().unwrap().criterion("PVS1").unwrap();
    let strengths_for = |text: &str| -> Vec<Option<Strength>> {
        pvs1.guide.iter().filter(|g| g.situation.starts_with(text)).map(|g| g.strength).collect()
    };
    assert_eq!(strengths_for("Start-loss"), [Some(Strength::Moderate), Some(Strength::Supporting)]);
    assert_eq!(strengths_for("Decay is not predicted, the role"), [Some(Strength::Strong), Some(Strength::Moderate)]);
    assert_eq!(strengths_for("Whole-gene deletion"), [Some(Strength::VeryStrong)]);
    assert!(pvs1.guide.iter().any(|g| g.strength.is_none()), "the curator can answer that PVS1 does not apply");
}

#[test]
fn a_bad_strength_guide_is_refused() {
    // No source.
    let no_source = BUILTIN.replacen("guide_source = \"abou-tayoun2018\"\n", "", 1);
    assert!(matches!(RuleSet::parse(&no_source), Err(RuleError::BadGuide { .. })));
    // A strength the criterion may not use.
    let bad = BUILTIN.replacen(
        "strength = \"supporting\"\n\n[[criteria.guide]]",
        "strength = \"stand_alone\"\n\n[[criteria.guide]]",
        1,
    );
    assert!(matches!(RuleSet::parse(&bad), Err(RuleError::BadGuide { .. })));
}

#[test]
fn the_answer_a_saved_case_can_get_back_is_unique() {
    let pvs1 = RuleSet::builtin().unwrap().criterion("PVS1").unwrap();
    // Two guide answers set Very Strong, so the loader must not pick one
    // silently; a saved case with that strength and no answer asks again.
    assert_eq!(pvs1.unique_situation_for(Strength::VeryStrong), None);
    assert!(pvs1.situation_for(Strength::VeryStrong).is_some());
    // One answer sets Supporting, so a saved Supporting strength is
    // reconstructable without inventing anything.
    let unique = pvs1.unique_situation_for(Strength::Supporting).unwrap();
    assert_eq!(pvs1.guide_option(unique).unwrap().strength, Some(Strength::Supporting));
}

#[test]
fn a_source_id_or_a_tier_name_cannot_appear_twice() {
    let source_twice = BUILTIN.replacen("id = \"richards2015\"", "id = \"clingen-svi-pm2\"", 1);
    assert!(matches!(RuleSet::parse(&source_twice), Err(RuleError::DuplicateSource(_))));
    let empty_id = BUILTIN.replacen("id = \"richards2015\"", "id = \"\"", 1);
    assert!(matches!(RuleSet::parse(&empty_id), Err(RuleError::DuplicateSource(_))));
    let tier_twice = BUILTIN.replacen("name = \"VUS-mid\"", "name = \"VUS-low\"", 1);
    assert!(matches!(RuleSet::parse(&tier_twice), Err(RuleError::BadTiers(_))));
}

#[test]
fn a_helper_criterion_may_not_carry_a_strength_guide() {
    // The predictor helper applies PP3 from its tables; a guide on PP3
    // would make every Apply produce an assessment error, so the loader
    // refuses the file. (PP3 appears before BP4, so the first occurrence
    // of the marker line ends PP3's block.)
    let pp3_with_guide = BUILTIN.replacen(
        "Choose one tool before you see its result.\"\n\n[[criteria]]\ncode = \"PP4\"",
        "Choose one tool before you see its result.\"\nguide_source = \"pejaver2022\"\n\n[[criteria.guide]]\nid = \"calibrated\"\nshort = \"Calibrated table\"\nsituation = \"The score comes from the tool this case is using.\"\nstrength = \"supporting\"\n\n[[criteria]]\ncode = \"PP4\"",
        1,
    );
    assert!(matches!(RuleSet::parse(&pp3_with_guide), Err(RuleError::GuideOnHelperCriterion(_))));
}

#[test]
fn a_review_date_must_name_a_possible_day() {
    let ok = edited(
        "status = \"unverified\"",
        "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-11-08\"",
    );
    assert!(ok.is_ok());
    let month = edited(
        "status = \"unverified\"",
        "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-99-08\"",
    );
    assert!(matches!(month, Err(RuleError::ReviewIncomplete)));
    let day = edited(
        "status = \"unverified\"",
        "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-11-39\"",
    );
    assert!(matches!(day, Err(RuleError::ReviewIncomplete)));
}

#[test]
fn the_built_in_file_is_valid() {
    assert!(RuleSet::builtin().is_ok());
}

#[test]
fn point_systems_that_would_misclassify_are_rejected() {
    // No evidence would fall below the VUS range.
    assert!(matches!(edited("vus = 0", "vus = 1"), Err(RuleError::BadThresholds)));
    // Stronger evidence worth fewer points than weaker evidence.
    assert!(matches!(
        edited("supporting = 1\nmoderate = 2", "supporting = 4\nmoderate = 2"),
        Err(RuleError::BadPoints)
    ));
    // BA1 alone could not reach Benign.
    assert!(matches!(edited("stand_alone = 8", "stand_alone = 4"), Err(RuleError::BadPoints)));
    // Numbers so large that adding them could overflow.
    assert!(matches!(edited("very_strong = 8", "very_strong = 2000000000"), Err(RuleError::BadPoints)));
}

#[test]
fn a_rule_file_read_straight_through_serde_is_checked_too() {
    let bad = BUILTIN.replacen("vus = 0", "vus = 1", 1);
    assert!(toml::from_str::<RuleSet>(&bad).is_err());
    assert!(toml::from_str::<RuleSet>(BUILTIN).is_ok());
}

#[test]
fn reviewed_needs_a_reviewer_and_a_date() {
    assert!(matches!(edited("status = \"unverified\"", "status = \"reviewed\""), Err(RuleError::ReviewIncomplete)));
    let ok = edited(
        "status = \"unverified\"",
        "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-12-01\"",
    );
    assert_eq!(ok.unwrap().meta.status, ReviewStatus::Reviewed);
    let bad_date = edited(
        "status = \"unverified\"",
        "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"December\"",
    );
    assert!(matches!(bad_date, Err(RuleError::ReviewIncomplete)));
}

#[test]
fn vus_tiers_must_be_all_or_nothing_and_cover_the_vus_range() {
    let tiers = |a: &str, b: &str, c: &str| {
        let mut text = BUILTIN.to_string();
        for (name, min) in [("VUS-high", a), ("VUS-mid", b), ("VUS-low", c)] {
            let old = format!("name = \"{name}\"\n");
            text = text.replacen(&old, &format!("{old}min_points = {min}\n"), 1);
        }
        RuleSet::parse(&text)
    };
    assert!(tiers("4", "2", "0").is_ok());
    assert!(matches!(tiers("4", "4", "0"), Err(RuleError::BadTiers(_))), "same minimum twice");
    assert!(matches!(tiers("4", "2", "1"), Err(RuleError::BadTiers(_))), "gap at the bottom of the range");
    assert!(matches!(tiers("6", "2", "0"), Err(RuleError::BadTiers(_))), "starts inside Likely pathogenic");
    // Only one tier with a range.
    let text = BUILTIN.replacen("name = \"VUS-high\"\n", "name = \"VUS-high\"\nmin_points = 4\n", 1);
    assert!(matches!(RuleSet::parse(&text), Err(RuleError::BadTiers(_))));
}

#[test]
fn a_cap_at_the_default_is_accepted_and_still_limits_later() {
    // PP3 is Supporting by default; a cap at Supporting leaves only that level.
    let rules = edited(
        "max_strength = \"strong\"\nnote = \"ClinGen later calibrated",
        "max_strength = \"supporting\"\nnote = \"ClinGen later calibrated",
    )
    .unwrap();
    assert_eq!(rules.criterion("PP3").unwrap().allowed_strengths(), vec![Strength::Supporting]);
    // PM1 is Moderate by default; capping it at Supporting would exclude its own default.
    let text = BUILTIN.replacen("code = \"PS3\"\n", "code = \"PS3\"\nmax_strength = \"supporting\"\n", 1);
    assert!(matches!(RuleSet::parse(&text), Err(RuleError::DefaultNotAllowed { .. })));
}

#[test]
fn a_cap_that_is_not_a_recorded_strength_is_rejected() {
    // Stand-alone only exists for BA1. Capping PP3 at it would otherwise be
    // dropped without a word, and the curator would be offered every strength.
    assert!(matches!(
        edited("max_strength = \"strong\"", "max_strength = \"stand_alone\""),
        Err(RuleError::MaxStrengthNotRecorded { .. })
    ));
}

#[test]
fn an_empty_guide_situation_is_rejected() {
    let line = "situation = \"Start-loss, no transcript uses an alternative start codon, and no pathogenic variants lie before the next in-frame methionine.\"";
    assert!(matches!(edited(line, "situation = \"\""), Err(RuleError::BadSituation { .. })));
}

#[test]
fn a_repeated_exclusive_pair_is_rejected() {
    let text = format!(
        "{BUILTIN}\n[[exclusive_pairs]]\na = \"PS3\"\nb = \"BS3\"\nreason = \"again\"\nsource = \"richards2015\"\n"
    );
    assert!(matches!(RuleSet::parse(&text), Err(RuleError::BadExclusivePair(_))));
}

#[test]
fn a_combination_cap_below_the_default_pair_is_rejected() {
    assert!(matches!(edited("max_points = 5", "max_points = 1"), Err(RuleError::BadCombinationCap { .. })));
}

#[test]
fn original_strength_must_be_a_different_recorded_strength() {
    // PM2's default is Supporting; an "original" equal to it explains nothing.
    assert!(matches!(
        edited("original_strength = \"moderate\"", "original_strength = \"supporting\""),
        Err(RuleError::BadOriginalStrength { .. })
    ));
}

#[test]
fn a_gene_level_question_needs_a_guide_a_source_and_a_level_that_keeps_the_strength() {
    assert!(edited("gene_level_source = \"abou-tayoun2018\"", "gene_level_source = \"nowhere\"").is_err());
    assert!(edited("steps = 0", "steps = 1").is_err(), "one answer must keep the guide's strength");
    assert!(edited("steps = 2", "steps = 4").is_err(), "four steps go past Supporting");
    assert!(edited("id = \"lower_two\"", "id = \"lower_one\"").is_err(), "ids must differ");
    let pvs1 = RuleSet::builtin().unwrap().criterion("PVS1").unwrap().clone();
    assert_eq!(pvs1.strength_for("nmd", Some("lower_one")), Some(Strength::Strong));
    assert_eq!(pvs1.strength_for("nmd", None), None, "the answer is needed");
    assert_eq!(pvs1.strength_for("not_applicable", Some("established")), None);
}
