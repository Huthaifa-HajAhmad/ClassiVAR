//! Score every expert-panel classification in the ERepo snapshot with the
//! built-in rules and print the agreement, in Markdown.
//!
//! `cargo run -p classivar-core --example erepo_benchmark > docs/review/BENCHMARK-erepo.md`

use std::collections::BTreeMap;

use classivar_core::RuleSet;
use classivar_core::benchmark::{Outcome, outcome, parse_met, parse_snapshot};

const SNAPSHOT: &str = include_str!("../tests/data/erepo-2026-11-21.tsv");

fn main() {
    let rules = RuleSet::builtin().expect("valid built-in rules");
    let rows = parse_snapshot(SNAPSHOT).expect("a readable snapshot");
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut differ: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut unreadable: BTreeMap<String, usize> = BTreeMap::new();
    for row in &rows {
        let result = outcome(rules, row);
        *counts.entry(format!("{result:?}")).or_default() += 1;
        match result {
            Outcome::Differs | Outcome::Withheld => {
                let got = parse_met(rules, &row.met)
                    .ok()
                    .and_then(|a| rules.assess_as(&a, classivar_core::Counting::AsRecorded).ok())
                    .map_or("?".to_string(), |a| a.class().map_or("withheld".into(), |c| c.short().to_string()));
                *differ.entry((row.class.short().to_string(), got)).or_default() += 1;
            }
            Outcome::Unreadable => {
                let why = parse_met(rules, &row.met).err().unwrap_or_else(|| "not scorable".into());
                *unreadable.entry(why).or_default() += 1;
            }
            _ => {}
        }
    }
    let n = rows.len() as f64;
    let pct = |k: &str| 100.0 * *counts.get(k).unwrap_or(&0) as f64 / n;
    println!("# ERepo benchmark: rules {} v{}\n", rules.meta.id, rules.meta.version);
    println!(
        "{} expert-panel classifications (ClinGen Evidence Repository export of 2026-11-21, retracted rows removed), each scored from the panel's own met criteria, as recorded.\n",
        rows.len()
    );
    println!("| Outcome | Rows | Share |\n|---|---|---|");
    for key in ["PointsAgree", "TableAgrees", "NoEvidence", "Withheld", "Differs", "Unreadable"] {
        println!("| {key} | {} | {:.1}% |", counts.get(key).unwrap_or(&0), pct(key));
    }
    println!(
        "\n## Where the points differ from the panel (panel class → engine class)\n\n| Panel | Engine | Rows |\n|---|---|---|"
    );
    let mut pairs: Vec<_> = differ.into_iter().collect();
    pairs.sort_by_key(|pair| std::cmp::Reverse(pair.1));
    for ((panel, engine), count) in pairs {
        println!("| {panel} | {engine} | {count} |");
    }
    println!("\n## Rows the rules cannot read\n\n| Reason | Rows |\n|---|---|");
    let mut reasons: Vec<_> = unreadable.into_iter().collect();
    reasons.sort_by_key(|reason| std::cmp::Reverse(reason.1));
    for (why, count) in reasons.into_iter().take(15) {
        println!("| {why} | {count} |");
    }
}
