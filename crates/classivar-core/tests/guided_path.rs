use classivar_core::case::{CaseReview, MAX_REVIEW_ANSWER_CHARS, MAX_REVIEW_NAME_CHARS};
use classivar_core::{RuleError, RuleSet};

const FILE: &str = include_str!("../../../rules/acmg-amp-points.toml");

/// The built-in rules without their facts or path, so a test can write its own.
fn base() -> &'static str {
    let before_facts = FILE.split("\n[[facts]]").next().unwrap();
    before_facts.split("\n[[path]]").next().unwrap()
}

/// One step per criterion, with `extra` lines added to the step for `code`.
fn one_step_each(code: &str, extra: &str) -> String {
    let rules = RuleSet::parse(base()).unwrap();
    let mut text = base().to_string();
    for c in &rules.criteria {
        let more = if c.code == code { extra } else { "" };
        text.push_str(&format!(
            "
[[path]]
id = '{0}'
question = 'Review {0}'
criteria = ['{0}']
{more}",
            c.code
        ));
    }
    text
}

#[test]
fn the_built_in_path_has_the_approved_ten_steps() {
    let rules = RuleSet::parse(FILE).unwrap();
    let path = rules.path.expect("built-in rules carry a review path");
    let ids: Vec<&str> = path.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "population",
            "variant-type",
            "predictors",
            "functional",
            "case-data",
            "segregation",
            "de-novo",
            "other-allele",
            "other-cause",
            "other-sources"
        ]
    );
    let helpers: Vec<Option<&str>> = path.iter().map(|s| s.helper.as_deref()).collect();
    assert_eq!(
        helpers,
        [
            Some("population"),
            None,
            Some("predictor"),
            Some("functional"),
            None,
            Some("segregation"),
            Some("de-novo"),
            Some("pm3"),
            None,
            None
        ]
    );
}

#[test]
fn a_step_may_hold_more_than_its_helper_owns() {
    let mut text = base().to_string();
    let rules = RuleSet::parse(base()).unwrap();
    for c in rules.criteria.iter().filter(|c| c.code != "PM3" && c.code != "BP2") {
        text.push_str(&format!(
            "
[[path]]
id = '{0}'
question = 'Review {0}'
criteria = ['{0}']
",
            c.code
        ));
    }
    text.push_str(
        "
[[path]]
id = 'allele'
question = 'Other allele'
criteria = ['PM3', 'BP2']
helper = 'pm3'
",
    );
    assert!(RuleSet::parse(&text).is_ok());
}

#[test]
fn a_helper_must_find_all_its_criteria_in_its_step() {
    assert!(
        RuleSet::parse(&one_step_each(
            "PP1",
            "helper = 'locus'
"
        ))
        .is_err(),
        "PP4 is outside the step"
    );
    assert!(
        RuleSet::parse(&one_step_each(
            "PP1",
            "helper = 'unknown'
"
        ))
        .is_err()
    );
}

#[test]
fn an_incomplete_path_cannot_hide_criteria() {
    let text = format!(
        "{}\n[[path]]\nid = 'population'\nquestion = 'Review frequency'\ncriteria = ['BA1', 'BS1', 'PM2']\nhelper = 'population'\n",
        base()
    );
    assert!(RuleSet::parse(&text).is_err(), "a partial path must be refused");
}

#[test]
fn path_steps_have_unique_ids_and_known_criteria() {
    for criteria in ["['UNKNOWN']", "['BA1', 'BA1']"] {
        let text = format!("{}\n[[path]]\nid = 'step'\nquestion = 'Review evidence'\ncriteria = {criteria}\n", base());
        assert!(RuleSet::parse(&text).is_err());
    }
}

#[test]
fn complete_path_questions_are_in_the_review_packet() {
    let loaded = RuleSet::parse(&one_step_each("PVS1", "")).unwrap();
    assert!(loaded.review_items().iter().any(|item| item.id == "path:PVS1"));
}

fn refused_path(text: &str) -> String {
    match RuleSet::parse(text) {
        Err(RuleError::BadPath(why)) => why,
        other => panic!("expected a refused path, got {:?}", other.map(|_| "rules that load")),
    }
}

#[test]
fn a_step_needs_a_unique_id_a_question_and_criteria() {
    let good = one_step_each("PVS1", "");
    for (from, to) in
        [("id = 'PS1'", "id = 'PVS1'"), ("id = 'PS1'", "id = ' '"), ("question = 'Review PS1'", "question = ' '")]
    {
        assert!(good.contains(from));
        assert!(refused_path(&good.replacen(from, to, 1)).contains("unique nonempty"), "{to}");
    }
    let empty = format!("{good}\n[[path]]\nid = 'empty'\nquestion = 'Nothing'\ncriteria = []\n");
    assert!(refused_path(&empty).contains("unique nonempty"));
}

// Test wording only; these are not questions or hints of the built-in rules.
const FACT: &str = "\n[[facts]]\nid = 'kind'\nquestion = 'Test question'\nanswers = ['first', 'other', 'unknown']\n";
const HINT: &str = "[[path.hints]]\nfact = 'kind'\nanswer = 'first'\nsource = 'richards2015'\ntext = 'Test hint'\n";

#[test]
fn a_misspelled_key_is_refused_instead_of_ignored() {
    assert!(RuleSet::parse(&one_step_each("PM3", "helpers = 'pm3'\n")).is_err());
    let fact = format!("{}{FACT}note = 'x'\n", one_step_each("PVS1", ""));
    assert!(RuleSet::parse(&fact).is_err());
    let hint = format!("{}{FACT}", one_step_each("PVS1", &format!("{HINT}criterion = 'PVS1'\n")));
    assert!(RuleSet::parse(&hint).is_err());
}

#[test]
fn facts_and_hints_load_and_join_the_review_packet() {
    let rules = RuleSet::parse(&format!("{}{FACT}", one_step_each("PVS1", HINT))).unwrap();
    assert_eq!(rules.facts[0].id, "kind");
    assert_eq!(rules.path.as_ref().unwrap()[0].hints[0].text, "Test hint");
    assert_eq!(rules.path.as_ref().unwrap()[0].hints[0].source, "richards2015");
    let items = rules.review_items();
    let summary = |id: &str| items.iter().find(|i| i.id == id).map(|i| i.summary.as_str());
    assert_eq!(summary("fact:kind"), Some("Test question (first / other / unknown)"));
    assert_eq!(summary("hint:PVS1:kind=first"), Some("Test hint"));
}

#[test]
fn a_hint_must_point_at_a_listed_fact_and_answer_once() {
    for hint in [
        HINT.replace("fact = 'kind'", "fact = 'missing'"),
        HINT.replace("answer = 'first'", "answer = 'missing'"),
        HINT.replace("text = 'Test hint'", "text = ' '"),
        format!("{HINT}{HINT}"),
    ] {
        let text = format!("{}{FACT}", one_step_each("PVS1", &hint));
        assert!(refused_path(&text).contains("hint"), "{hint}");
    }
    let bad_source = format!("{}{FACT}", one_step_each("PVS1", &HINT.replace("richards2015", "no-such-paper")));
    assert!(matches!(RuleSet::parse(&bad_source), Err(RuleError::UnknownSource { .. })));
}

#[test]
fn built_in_facts_and_hints_separate_predicted_and_observed_splicing_and_match_answers() {
    let rules = RuleSet::parse(FILE).unwrap();
    let fact_ids: Vec<&str> = rules.facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(fact_ids, ["consequence", "splicing"]);
    let splicing = rules.fact("splicing").unwrap();
    assert!(splicing.lists("predicted splicing impact"));
    assert!(splicing.lists("observed abnormal splicing (RNA)"));
    assert!(splicing.lists("unknown") && splicing.lists("other"));
    let path = rules.path.as_ref().unwrap();
    let variant_step = path.iter().find(|s| s.id == "variant-type").unwrap();
    let mut answers = std::collections::BTreeMap::new();
    assert!(variant_step.matching_hints(&answers).is_empty());
    answers.insert("consequence".into(), "missense".into());
    answers.insert("splicing".into(), "observed normal splicing (RNA)".into());
    let matched = variant_step.matching_hints(&answers);
    assert_eq!(matched.len(), 2);
    assert!(matched.iter().any(|h| h.source == "richards2015"));
    assert!(matched.iter().any(|h| h.source == "walker2023" && h.text.contains("BP7_Strong (RNA)")));
    assert_eq!(classivar_core::rules::helper_criteria("population"), Some(&["BA1", "BS1", "PM2"][..]));
    assert_eq!(classivar_core::rules::helper_criteria("unknown"), None);
}

#[test]
fn a_fact_needs_a_unique_id_a_question_and_unknown_and_other_answers() {
    let long_id = format!("id = '{}'", "x".repeat(MAX_REVIEW_NAME_CHARS + 1));
    let long_answer = format!("'{}', 'other'", "x".repeat(MAX_REVIEW_ANSWER_CHARS + 1));
    for fact in [
        format!("{FACT}{FACT}"),
        FACT.replace("id = 'kind'", "id = ' '"),
        FACT.replace("id = 'kind'", &long_id),
        FACT.replace("question = 'Test question'", "question = ' '"),
        FACT.replace(", 'unknown'", ""),
        FACT.replace("'other', ", ""),
        FACT.replace("'first', 'other'", "'other', 'other'"),
        FACT.replace("'first', 'other'", "' ', 'other'"),
        FACT.replace("'first', 'other'", &long_answer),
    ] {
        assert!(refused_path(&format!("{}{fact}", base())).contains("fact"), "{fact}");
    }
}

#[test]
fn a_fact_answer_these_rules_do_not_list_is_set_aside() {
    let rules = RuleSet::parse(&format!("{}{FACT}", base())).unwrap();
    let mut review = CaseReview::default();
    review.facts.insert("kind".into(), "first".into());
    assert!(review.set_aside(&rules).is_empty());
    review.facts.insert("kind".into(), "an answer from other rules".into());
    assert_eq!(review.set_aside(&rules), ["kind"]);
}
