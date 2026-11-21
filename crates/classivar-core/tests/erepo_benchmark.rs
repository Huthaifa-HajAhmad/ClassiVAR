//! The engine against ClinGen expert-panel classifications (ERepo export of
//! 2026-11-21, `tests/data/erepo-2026-11-21.tsv`, made by
//! `scripts/erepo-snapshot.py`). The full report is `docs/review/BENCHMARK-erepo.md`.
//!
//! The floors are what the rules reached when the snapshot was taken. A rule
//! change that lowers them must say why in `docs/ROADMAP.md`, or be undone.

use classivar_core::benchmark::{Outcome, outcome, parse_snapshot};
use classivar_core::{Class, RuleSet};

const SNAPSHOT: &str = include_str!("data/erepo-2026-11-21.tsv");

fn pathogenic_side(class: Class) -> bool {
    class > Class::Vus
}

#[test]
fn the_engine_reaches_the_panels_class_from_the_panels_own_criteria() {
    let rules = RuleSet::builtin().unwrap();
    let rows = parse_snapshot(SNAPSHOT).unwrap();
    assert!(rows.len() >= 13_000, "the snapshot is complete: {}", rows.len());
    let outcomes: Vec<Outcome> = rows.iter().map(|row| outcome(rules, row)).collect();
    let count = |kind: Outcome| outcomes.iter().filter(|o| **o == kind).count();
    let with_evidence = rows.len() - count(Outcome::NoEvidence);
    let agree = count(Outcome::PointsAgree) + count(Outcome::TableAgrees);
    let share = agree as f64 / with_evidence as f64;
    assert!(share >= 0.98, "agreement fell to {:.2}% ({agree} of {with_evidence})", share * 100.0);
    assert!(count(Outcome::Withheld) * 1000 <= with_evidence * 2, "withheld in {} rows", count(Outcome::Withheld));
    assert!(count(Outcome::Unreadable) <= 10, "{} rows unreadable", count(Outcome::Unreadable));
}

#[test]
fn the_engine_never_puts_a_panels_class_on_the_other_side() {
    // A pathogenic call shown as benign, or the reverse, is the error that matters most.
    let rules = RuleSet::builtin().unwrap();
    for row in parse_snapshot(SNAPSHOT).unwrap() {
        let Ok(applied) = classivar_core::benchmark::parse_met(rules, &row.met) else { continue };
        let Ok(assessment) = rules.assess_as(&applied, classivar_core::Counting::AsRecorded) else { continue };
        let Some(class) = assessment.class() else { continue };
        let crossed =
            (pathogenic_side(row.class) && class < Class::Vus) || (row.class < Class::Vus && pathogenic_side(class));
        assert!(!crossed, "{} ({}): panel {:?}, engine {:?} from {}", row.allele, row.panel, row.class, class, row.met);
    }
}
