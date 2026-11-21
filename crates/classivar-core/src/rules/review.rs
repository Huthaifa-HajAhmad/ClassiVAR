//! The review packet: every entry of a rule file a reviewer must sign off, the
//! sign-offs recorded so far (`rules/review-signoff.toml`), and the checks that
//! keep them honest. An AI sign-off is a record of what was checked, never a
//! review: the rule file may say "reviewed" only when every entry has a human one.

use serde::Deserialize;

use super::{ReviewStatus, RuleSet};

/// One entry a reviewer signs off, with what it holds in a few words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItem {
    pub id: String,
    pub summary: String,
}

/// One sign-off from the sign-off file.
#[derive(Debug, Clone, Deserialize)]
pub struct Signoff {
    pub item: String,
    pub reviewer: String,
    /// "ai" or "human".
    pub kind: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// "matches source", "matches source, reading noted", "not checked", or for a
    /// human sign-off also "corrected" or "disagrees".
    pub verdict: String,
    #[serde(default)]
    pub checked_against: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize)]
struct SignoffFile {
    signoff: Vec<Signoff>,
}

const VERDICTS: [&str; 5] =
    ["matches source", "matches source, reading noted", "not checked", "corrected", "disagrees"];

/// Read a sign-off file.
pub fn parse_signoffs(text: &str) -> Result<Vec<Signoff>, String> {
    toml::from_str::<SignoffFile>(text).map(|file| file.signoff).map_err(|e| e.to_string())
}

impl RuleSet {
    /// Every entry a reviewer signs off, in the rule file's order of topics.
    pub fn review_items(&self) -> Vec<ReviewItem> {
        let item = |id: String, summary: String| ReviewItem { id, summary };
        let p = &self.points;
        let t = &self.class_min_points;
        let mut items = vec![
            item(
                "points".into(),
                format!(
                    "Supporting {}, Moderate {}, Strong {}, Very Strong {}, stand-alone {}",
                    p.supporting, p.moderate, p.strong, p.very_strong, p.stand_alone
                ),
            ),
            item(
                "class_min_points".into(),
                format!(
                    "P >= {}, LP >= {}, VUS >= {}, LB >= {}",
                    t.pathogenic, t.likely_pathogenic, t.vus, t.likely_benign
                ),
            ),
            item(
                "combining_table".into(),
                "Richards 2015 Table 5 with the Tavtigian 2020 points, ClinGen 2020 and Pejaver 2022 rows".into(),
            ),
            item("vus_tiers".into(), self.vus_tiers.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", ")),
        ];
        for c in &self.criteria {
            let ceiling = c.allowed_strengths().last().map_or("-", |s| s.label());
            items.push(item(
                format!("criterion:{}", c.code),
                format!("{}: default {}, highest {ceiling}", c.title, c.strength.label()),
            ));
            if !c.guide.is_empty() {
                items.push(item(format!("guide:{}", c.code), format!("{} answers", c.guide.len())));
            }
            if !c.gene_level.is_empty() {
                items.push(item(format!("gene_level:{}", c.code), format!("{} answers", c.gene_level.len())));
            }
        }
        items.extend(self.exclusive_pairs.iter().map(|x| item(format!("exclusive:{}/{}", x.a, x.b), x.reason.clone())));
        items.extend(
            self.combination_caps
                .iter()
                .map(|x| item(format!("cap:{}/{}", x.a, x.b), format!("at most {} points", x.max_points))),
        );
        items.extend(self.predictors.iter().map(|x| item(format!("predictor:{}", x.id), x.name.clone())));
        items.extend(self.table_items());
        items
    }

    /// The helper tables a rule file has, one entry each.
    fn table_items(&self) -> Vec<ReviewItem> {
        let tables = [
            ("functional", self.functional.is_some()),
            ("locus_evidence", self.locus_evidence.is_some()),
            ("non_segregation", self.non_segregation.is_some()),
            ("de_novo", self.de_novo.is_some()),
            ("pm3", self.pm3.is_some()),
            ("population", self.population.is_some()),
            ("ba1_exceptions", !self.ba1_exceptions.is_empty()),
        ];
        tables
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(id, _)| ReviewItem { id: id.into(), summary: "table".into() })
            .collect()
    }

    /// Check sign-offs against this rule file: each names an existing entry,
    /// has a known kind, verdict and date, and every entry is signed. While the
    /// file says "reviewed", every entry needs a human sign-off.
    pub fn check_signoffs(&self, signoffs: &[Signoff]) -> Result<(), String> {
        let items = self.review_items();
        for s in signoffs {
            if !items.iter().any(|i| i.id == s.item) {
                return Err(format!("sign-off for '{}', which is not an entry of these rules", s.item));
            }
            let dated = crate::date::is_calendar_date(&s.date);
            if !["ai", "human"].contains(&s.kind.as_str())
                || !VERDICTS.contains(&s.verdict.as_str())
                || !dated
                || s.reviewer.trim().is_empty()
            {
                return Err(format!(
                    "sign-off for '{}' needs a reviewer, kind ai or human, a known verdict and a YYYY-MM-DD date",
                    s.item
                ));
            }
        }
        if let Some(missing) = items.iter().find(|i| !signoffs.iter().any(|s| s.item == i.id)) {
            return Err(format!("no sign-off for '{}'", missing.id));
        }
        let human = |id: &str| signoffs.iter().any(|s| s.item == id && s.kind == "human");
        if self.meta.status == ReviewStatus::Reviewed
            && let Some(item) = items.iter().find(|i| !human(&i.id))
        {
            return Err(format!("the rules say reviewed, but '{}' has no human sign-off", item.id));
        }
        Ok(())
    }

    /// The review packet, as Markdown: one row per entry with its sign-offs.
    pub fn review_packet(&self, signoffs: &[Signoff]) -> String {
        let mut out = vec![
            format!("# Review packet: {} v{}", self.meta.id, self.meta.version),
            String::new(),
            "Generated from the rule file and `rules/review-signoff.toml` by `cargo run -p classivar-core --example review_packet`. An AI sign-off records what was checked; it is not a qualified review.".into(),
            String::new(),
            "| Entry | Holds | Sign-off | Checked against | Note |".into(),
            "|---|---|---|---|---|".into(),
        ];
        let cell = |text: &str| text.replace('|', "/").replace('\n', " ");
        for item in self.review_items() {
            for s in signoffs.iter().filter(|s| s.item == item.id) {
                out.push(format!(
                    "| `{}` | {} | {} ({}, {}): {} | {} | {} |",
                    item.id,
                    cell(&item.summary),
                    s.reviewer,
                    s.kind,
                    s.date,
                    s.verdict,
                    cell(&s.checked_against),
                    cell(&s.note)
                ));
            }
        }
        out.join("\n") + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNOFFS: &str = include_str!("../../../../rules/review-signoff.toml");

    #[test]
    fn every_entry_of_the_built_in_rules_has_a_sign_off_and_each_names_a_real_entry() {
        let rules = RuleSet::builtin().unwrap();
        let signoffs = parse_signoffs(SIGNOFFS).unwrap();
        rules.check_signoffs(&signoffs).unwrap();
        assert!(signoffs.iter().all(|s| s.kind == "ai"), "only an AI review is recorded so far");
        assert_eq!(rules.meta.status, ReviewStatus::Unverified, "an AI review never makes the rules reviewed");
    }

    #[test]
    fn a_reviewed_rule_file_needs_a_human_sign_off_for_every_entry() {
        let mut rules = RuleSet::builtin().unwrap().clone();
        rules.meta.status = ReviewStatus::Reviewed;
        let signoffs = parse_signoffs(SIGNOFFS).unwrap();
        assert!(rules.check_signoffs(&signoffs).unwrap_err().contains("no human sign-off"));
        let human: Vec<Signoff> = signoffs
            .into_iter()
            .map(|s| Signoff { kind: "human".into(), reviewer: "A. Reviewer".into(), ..s })
            .collect();
        assert!(rules.check_signoffs(&human).is_ok());
    }

    #[test]
    fn a_missing_unknown_or_malformed_sign_off_is_refused() {
        let rules = RuleSet::builtin().unwrap();
        let mut signoffs = parse_signoffs(SIGNOFFS).unwrap();
        let first = signoffs.remove(0);
        assert!(rules.check_signoffs(&signoffs).unwrap_err().contains("no sign-off"));
        signoffs.push(Signoff { item: "criterion:XX9".into(), ..first.clone() });
        assert!(rules.check_signoffs(&signoffs).unwrap_err().contains("not an entry"));
        signoffs.pop();
        signoffs.push(Signoff { date: "2026-13-40".into(), ..first });
        assert!(rules.check_signoffs(&signoffs).is_err());
    }

    #[test]
    fn the_packet_has_a_row_per_sign_off() {
        let rules = RuleSet::builtin().unwrap();
        let signoffs = parse_signoffs(SIGNOFFS).unwrap();
        let packet = rules.review_packet(&signoffs);
        assert!(packet.contains("| `gene_level:PVS1` |"), "{packet}");
        assert_eq!(packet.lines().filter(|l| l.starts_with("| `")).count(), signoffs.len());
    }
}
