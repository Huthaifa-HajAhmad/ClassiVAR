//! De novo points for PS2 and PM6 (ClinGen SVI v1.1).

use super::*;

/// Reads de novo counts as (points, strength) with the table's four rows in
/// order: highly specific, consistent, consistent but many genes, not consistent.
fn de_novo(counts: [(u32, u32); 4], recessive: bool) -> (f64, Option<Strength>, bool) {
    let r = RuleSet::builtin().unwrap().read_de_novo(&counts, recessive).unwrap();
    (r.points, r.evidence.map(|e| e.strength), r.lowered_for_recessive)
}

#[test]
fn de_novo_points_follow_the_papers_worked_examples() {
    use Strength::*;
    let none = (0, 0);
    // NIPBL: one patient with confirmed parents (2 points) and two without (1 + 1) = 4, Very Strong.
    assert_eq!(de_novo([(1, 2), none, none, none], false), (4.0, Some(VeryStrong), false));
    // SIK1: a patient who fits but not specifically, confirmed: 1 point, Moderate.
    assert_eq!(de_novo([none, (1, 0), none, none], false), (1.0, Some(Moderate), false));
    // Two more such patients: 3 points, Strong (not yet Very Strong).
    assert_eq!(de_novo([none, (3, 0), none, none], false), (3.0, Some(Strong), false));
    // ASH1L: many genes can cause it. One patient 0.5 points, Supporting; two give 1, Moderate.
    assert_eq!(de_novo([none, none, (1, 0), none], false), (0.5, Some(Supporting), false));
    assert_eq!(de_novo([none, none, (2, 0), none], false), (1.0, Some(Moderate), false));
    // NIPBL with a phenotype that does not fit: no points, no evidence.
    assert_eq!(de_novo([none, none, none, (0, 1)], false), (0.0, None, false));
}

#[test]
fn de_novo_strength_changes_exactly_at_the_table_2_limits() {
    use Strength::*;
    let none = (0, 0);
    // Each patient whose phenotype fits the gene but not specifically, with parents
    // not confirmed, earns 0.5 points.
    let edge = |count_half: u32| de_novo([none, (0, count_half), none, none], false);
    assert_eq!(edge(0).1, None);
    assert_eq!(edge(1), (0.5, Some(Supporting), false));
    assert_eq!(edge(2), (1.0, Some(Moderate), false));
    assert_eq!(edge(3), (1.5, Some(Moderate), false));
    assert_eq!(edge(4), (2.0, Some(Strong), false));
    assert_eq!(edge(7), (3.5, Some(Strong), false));
    assert_eq!(edge(8), (4.0, Some(VeryStrong), false));
    // A quarter point (many genes, parents not confirmed) adds up exactly: 3 x 0.25 is below Supporting.
    assert_eq!(de_novo([none, none, (0, 3), none], false), (0.75, Some(Supporting), false));
}

#[test]
fn the_many_genes_row_adds_at_most_one_point() {
    use Strength::*;
    let none = (0, 0);
    // Four confirmed patients would be 2 points; the table's note limits the row to 1.
    assert_eq!(de_novo([none, none, (4, 0), none], false), (1.0, Some(Moderate), false));
    // The limit is for that row only; other rows still add to it.
    assert_eq!(de_novo([(1, 0), none, (4, 0), none], false), (3.0, Some(Strong), false));
}

#[test]
fn a_recessive_gene_without_a_second_variant_lowers_the_strength_one_level() {
    use Strength::*;
    let none = (0, 0);
    assert_eq!(de_novo([(1, 2), none, none, none], true), (4.0, Some(Strong), true));
    assert_eq!(de_novo([(1, 0), none, none, none], true), (2.0, Some(Moderate), true));
    assert_eq!(de_novo([none, (1, 0), none, none], true), (1.0, Some(Supporting), true));
    // Supporting has no lower level: no evidence, and the screen is told why.
    assert_eq!(de_novo([none, none, (1, 0), none], true), (0.5, None, true));
    // Nothing to lower when the points reach no level.
    assert_eq!(de_novo([none, none, none, (3, 0)], true), (0.0, None, false));
}

#[test]
fn de_novo_evidence_is_counted_as_ps2_and_can_reach_very_strong() {
    let rules = RuleSet::builtin().unwrap();
    let reading = rules.read_de_novo(&[(2, 0), (0, 0), (0, 0), (0, 0)], false).unwrap().evidence.unwrap();
    assert_eq!(
        (reading.code, reading.published, reading.strength),
        ("PS2", Strength::VeryStrong, Strength::VeryStrong)
    );
}

#[test]
fn a_de_novo_total_can_also_be_applied_as_pm6_up_to_very_strong() {
    use Strength::*;
    let rules = RuleSet::builtin().unwrap();
    let pm6 = |counts: [(u32, u32); 4], recessive: bool| {
        let reading = rules.read_de_novo(&counts, recessive).unwrap();
        reading.pm6.map(|r| (r.code, r.published, r.strength))
    };
    let none = (0, 0);
    // Table 2 lets PS2 or PM6 carry the same level: 1 point is Moderate for both.
    assert_eq!(pm6([none, (1, 0), none, none], false), Some(("PM6", Moderate, Moderate)));
    assert_eq!(pm6([none, (3, 0), none, none], false), Some(("PM6", Strong, Strong)));
    // Table 2 lists PM6 at Very Strong too, and the program follows it.
    assert_eq!(pm6([(1, 2), none, none, none], false), Some(("PM6", VeryStrong, VeryStrong)));
    // The recessive rule lowers PM6 exactly as it lowers PS2; no points, no PM6.
    assert_eq!(pm6([(1, 2), none, none, none], true), Some(("PM6", Strong, Strong)));
    assert_eq!(pm6([none, none, (1, 0), none], true), None);
    assert_eq!(pm6([none, none, none, (3, 0)], false), None);
}

#[test]
fn de_novo_counts_that_do_not_match_the_table_give_nothing() {
    let rules = RuleSet::builtin().unwrap();
    assert!(rules.read_de_novo(&[(1, 0)], false).is_none());
    assert!(rules.read_de_novo(&[], false).is_none());
}

#[test]
fn a_de_novo_table_that_could_overstate_the_evidence_is_refused() {
    let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadDeNovo(_)));
    // Less for confirmed parents than for unconfirmed ones makes no sense.
    assert!(is_bad(edited("confirmed = 2.0, unconfirmed = 1.0", "confirmed = 1.0, unconfirmed = 2.0")));
    // Negative points.
    assert!(is_bad(edited("confirmed = 2.0, unconfirmed = 1.0", "confirmed = 2.0, unconfirmed = -1.0")));
    // A repeated row id, and an empty label.
    assert!(is_bad(edited("id = \"consistent\"", "id = \"highly-specific\"")));
    assert!(is_bad(edited("label = \"Phenotype not consistent with the gene\"", "label = \"\"")));
    // A limit that is not above zero.
    assert!(is_bad(edited("max_points = 1.0", "max_points = 0.0")));
    // Thresholds out of order, repeated or not rising.
    assert!(is_bad(edited("{ at = 2.0, strength = \"strong\" }", "{ at = 0.7, strength = \"strong\" }")));
    assert!(is_bad(edited("{ at = 1.0, strength = \"moderate\" }", "{ at = 0.5, strength = \"moderate\" }")));
    assert!(is_bad(edited("{ at = 4.0, strength = \"very_strong\" }", "{ at = 4.0, strength = \"strong\" }")));
    // A strength that is not a pathogenic level.
    assert!(is_bad(edited("{ at = 4.0, strength = \"very_strong\" }", "{ at = 4.0, strength = \"stand_alone\" }")));
    // An unknown source.
    assert!(matches!(
        edited("source = \"clingen-svi-de-novo\"\nrecessive_text", "source = \"nowhere\"\nrecessive_text"),
        Err(RuleError::UnknownSource { .. })
    ));
}

#[test]
fn a_rule_file_without_the_de_novo_table_still_loads_and_gives_nothing() {
    let start = BUILTIN.find("\n[de_novo]\n").unwrap();
    let end = BUILTIN[start..].find("\n[[criteria]]").unwrap() + start;
    let rules = RuleSet::parse(&format!("{}{}", &BUILTIN[..start], &BUILTIN[end..])).unwrap();
    assert!(rules.read_de_novo(&[(1, 0), (0, 0), (0, 0), (0, 0)], false).is_none());
}
