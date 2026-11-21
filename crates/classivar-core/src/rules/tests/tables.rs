//! BS4 situations, PM3 points and the BA1 reference list.

use super::*;

#[test]
fn bs4_applies_only_where_the_paper_supports_it() {
    let rules = RuleSet::builtin().unwrap();
    let read = |id| rules.read_non_segregation(id).map(|r| (r.code, r.strength));
    assert_eq!(read("follows-not"), Some(("BS4", Strength::Strong)));
    // The paper: little to no evidence for compound heterozygotes; no points for
    // unaffected carriers under reduced penetrance; formal analysis for common
    // phenocopies, conflicting results and very large families.
    assert_eq!(read("compound-heterozygous"), None);
    assert_eq!(read("reduced-penetrance"), None);
    assert_eq!(read("high-phenocopy"), None);
    assert_eq!(read("conflicting-or-large"), None);
    assert_eq!(read("not-a-situation"), None);
}

#[test]
fn the_bs4_strength_is_the_minus_4_points_the_paper_recommends() {
    // ClinGen 2024: -4.0 points for non-segregation. No number in the rules changed.
    let rules = RuleSet::builtin().unwrap();
    let strength = rules.non_segregation.as_ref().unwrap().strength;
    assert_eq!(rules.magnitude(strength), 4);
    assert_eq!(rules.criterion("BS4").unwrap().strength, strength);
}

#[test]
fn a_bs4_table_that_could_never_offer_or_misapply_bs4_is_refused() {
    let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadNonSegregation(_)));
    let own_source = "source = \"clingen-pp1-pp4-2024\"\nstrength = \"strong\"";
    // No situation applies.
    assert!(is_bad(edited("applies = true", "applies = false")));
    // A strength BS4 does not allow (benign criteria stop at Strong), and one that is not -4 points.
    assert!(is_bad(edited(own_source, "source = \"clingen-pp1-pp4-2024\"\nstrength = \"very_strong\"")));
    assert!(is_bad(edited(own_source, "source = \"clingen-pp1-pp4-2024\"\nstrength = \"moderate\"")));
    // A repeated id.
    assert!(is_bad(edited("id = \"high-phenocopy\"", "id = \"follows-not\"")));
    // An empty id, label or text.
    assert!(is_bad(edited("id = \"follows-not\"", "id = \"\"")));
    assert!(is_bad(edited("label = \"Does not follow the disease\"", "label = \"\"")));
    assert!(is_bad(with_line_replaced("text = \"Autosomal dominant, X-linked", "text = \"\"")));
    // A situation that does not say whether BS4 applies cannot even be read.
    assert!(matches!(without_lines("applies = "), Err(RuleError::Parse(_))));
    // An unknown source.
    assert!(matches!(
        edited("source = \"clingen-pp1-pp4-2024\"\nstrength", "source = \"nowhere\"\nstrength"),
        Err(RuleError::UnknownSource { .. })
    ));
}

#[test]
fn pm3_follows_pah_example_and_every_strength_edge() {
    use Strength::*;
    let rules = RuleSet::builtin().unwrap();
    // ClinGen 2019: one confirmed LP partner and one confirmed P partner.
    let example = rules.read_pm3(&[2, 0, 0, 0, 0]).unwrap();
    assert_eq!(example.points, 2.0);
    assert_eq!(example.evidence.unwrap().strength, Strong);
    for (count, expected) in [
        (0, None),
        (1, None),
        (2, Some(Supporting)),
        (3, Some(Supporting)),
        (4, Some(Moderate)),
        (7, Some(Moderate)),
        (8, Some(Strong)),
        (15, Some(Strong)),
        (16, Some(VeryStrong)),
    ] {
        let r = rules.read_pm3(&[0, 0, count, 0, 0]).unwrap();
        assert_eq!(r.points, f64::from(count) * 0.25);
        assert_eq!(r.evidence.map(|e| e.strength), expected, "{count}");
    }
    assert_eq!(rules.read_pm3(&[0, 1, 0, 0, 0]).unwrap().points, 0.5);
    let applied = crate::Applied::new("PM3", VeryStrong);
    assert!(rules.assess(&[applied]).is_ok());
}

#[test]
fn pm3_limits_homozygous_and_uncertain_rows_separately() {
    let rules = RuleSet::builtin().unwrap();
    for (counts, points, strength) in [
        ([0, 0, 0, 100, 0], 1.0, Strength::Moderate),
        ([0, 0, 0, 0, 100], 0.5, Strength::Supporting),
        ([0, 0, 0, 100, 100], 1.5, Strength::Moderate),
        ([1, 0, 0, 100, 100], 2.5, Strength::Strong),
    ] {
        let r = rules.read_pm3(&counts).unwrap();
        assert_eq!(r.points, points);
        assert_eq!(r.evidence.unwrap().strength, strength);
    }
    assert!(rules.read_pm3(&[]).is_none());
    assert!(rules.read_pm3(&[1]).is_none());
}

#[test]
fn pm3_rejects_bad_tables_and_supports_old_rule_files() {
    for replacement in ["points = -1.0", "points = nan"] {
        let text = BUILTIN.replacen("points = 1.0\n", &format!("{replacement}\n"), 1);
        assert!(RuleSet::parse(&text).is_err());
    }
    for (from, to) in [
        ("source = \"clingen-svi-pm3\"\nnotes", "source = \"missing\"\nnotes"),
        ("id = \"unknown_p\"", "id = \"confirmed\""),
        ("max_points = 1.0\n", "max_points = 0.0\n"),
        ("{ strength = \"strong\", at = 2.0 }", "{ strength = \"strong\", at = 0.25 }"),
    ] {
        assert!(RuleSet::parse(&BUILTIN.replacen(from, to, 1)).is_err(), "{to}");
    }
    let start = BUILTIN.find("\n[pm3]\n").unwrap();
    let end = BUILTIN[start..].find("# ----- Benign -----").unwrap() + start;
    let old = format!("{}{}", &BUILTIN[..start], &BUILTIN[end..]);
    let old = RuleSet::parse(&old).unwrap();
    assert!(old.read_pm3(&[1, 0, 0, 0, 0]).is_none());
    assert!(old.ba1_exceptions.is_empty());
}

#[test]
fn ba1_reference_has_the_nine_published_identifiers() {
    let rules = RuleSet::builtin().unwrap();
    let actual: Vec<_> =
        rules.ba1_exceptions.iter().map(|r| (r.gene.as_str(), r.clinvar_id.as_str(), r.allele_id.as_str())).collect();
    assert_eq!(
        actual,
        vec![
            ("ACAD9", "1018", "CA114709"),
            ("GJB2", "17023", "CA172210"),
            ("HFE", "10", "CA113797"),
            ("HFE", "9", "CA113795"),
            ("MEFV", "2551", "CA280114"),
            ("MEFV", "2552", "CA280116"),
            ("PIBF1", "217689", "CA210261"),
            ("ACADS", "3830", "CA312214"),
            ("BTD", "1900", "CA090886"),
        ]
    );
    assert!(RuleSet::parse(&BUILTIN.replace("allele_id = \"CA090886\"", "allele_id = \"CA312214\"")).is_err());
    assert!(RuleSet::parse(&BUILTIN.replace("clinvar_id = \"1900\"", "clinvar_id = \"bad\"")).is_err());
}

#[test]
fn pm3_calculation_reason_survives_case_reopening() {
    use crate::{Applied, CASE_FORMAT, CaseFile, GeneDecision};
    let rules = RuleSet::builtin().unwrap();
    let r = rules.read_pm3(&[4, 0, 0, 0, 0]).unwrap();
    let item = Applied::new("PM3", r.evidence.unwrap().strength)
        .with_reason("PM3: 4 x confirmed = 4 patient points after row limits; conditions checked (clingen-svi-pm3)");
    let case = CaseFile {
        format: CASE_FORMAT,
        app_version: "test".into(),
        saved_on: "2026-11-05".into(),
        rule_set: rules.reference(),
        variant: "test".into(),
        gene: "PAH".into(),
        gene_decision: GeneDecision::NotDecided,
        today: vec![item],
        records: vec![],
        identity: None,
        condition: None,
    };
    let reopened = CaseFile::from_json(&case.to_json(), rules).unwrap();
    assert_eq!(reopened.today, case.today);
    assert_eq!(rules.assess(&reopened.today).unwrap().class(), rules.assess(&case.today).unwrap().class());
}

#[test]
fn a_rule_file_without_the_bs4_table_still_loads_and_gives_nothing() {
    let start = BUILTIN.find("\n[non_segregation]\n").unwrap();
    let end = BUILTIN[start..].find("\n[[criteria]]").unwrap() + start;
    let rules = RuleSet::parse(&format!("{}{}", &BUILTIN[..start], &BUILTIN[end..])).unwrap();
    assert!(rules.read_non_segregation("follows-not").is_none());
}

#[test]
fn a_rule_file_without_the_locus_table_still_loads_and_gives_nothing() {
    // Cut only the PP1/PP4 section: from its header to the BS4 section's header.
    let start = BUILTIN.find("\n[locus_evidence]\n").unwrap();
    let end = BUILTIN.find("\n[non_segregation]\n").unwrap();
    let rules = RuleSet::parse(&format!("{}{}", &BUILTIN[..start], &BUILTIN[end..])).unwrap();
    assert!(rules.pp4_points(90.0, 1).is_none());
    assert!(rules.pp1_points("dominant", 3).is_none());
    assert!(rules.read_locus(1.0, 1.0, true).is_none());
    // The BS4 table is still there.
    assert!(rules.read_non_segregation("follows-not").is_some());
}
