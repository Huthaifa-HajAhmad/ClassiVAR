//! The evidence a curator applied, and how it is carried onto today's rules.

use super::*;

pub(super) fn to_applied(evidence: &Evidence) -> Vec<Applied> {
    evidence
        .iter()
        .map(|(code, choice)| Applied {
            code: code.clone(),
            strength: choice.strength,
            situation: choice.situation.clone(),
            gene_level: choice.gene_level.clone(),
            reason: choice.reason.clone(),
        })
        .collect()
}

pub(super) fn evidence_of(list: &[(&str, Strength)]) -> Evidence {
    list.iter().map(|(code, strength)| (code.to_string(), Choice::new(*strength))).collect()
}

/// One thing that changed when evidence moved onto today's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    /// The strength was above today's limit and was lowered to it.
    Lowered { code: String, to: Strength },
    /// The criterion is not in today's rules, so it was left out.
    Dropped(String),
    /// The criterion has a strength guide and no single answer fits the strength, so
    /// the curator must answer again. Picking one would invent a reason nobody gave.
    NeedsGuide(String),
    /// The criterion has a strength guide and the curator never gave an answer for this
    /// strength, so it was rebuilt: exactly one answer sets the strength. The curator should
    /// check it. A saved answer that still fits is kept and is not a change.
    GuideRebuilt(String),
    /// The criterion asks a gene-level question first (PVS1) and the evidence
    /// has no answer to it, so the curator must answer. It is never assumed.
    NeedsGeneLevel(String),
}

/// Bring evidence onto today's rules: lower a strength above today's limit, leave
/// out a criterion that no longer exists, and keep or rebuild the guide answer.
/// A reason explains one strength, so it is dropped when the strength was lowered.
/// The changes are returned so the screen can say them; nothing is repaired silently.
pub(super) fn carry_forward(rules: &RuleSet, list: &[Applied]) -> (Vec<(String, Choice)>, Vec<Change>) {
    let mut changes = Vec::new();
    let mut out = Vec::new();
    for applied in list {
        let Some(criterion) = rules.criterion(&applied.code) else {
            changes.push(Change::Dropped(applied.code.clone()));
            continue;
        };
        let allowed = criterion.allowed_strengths();
        let strength = if allowed.contains(&applied.strength) {
            applied.strength
        } else {
            let highest = *allowed.last().unwrap_or(&criterion.strength);
            changes.push(Change::Lowered { code: applied.code.clone(), to: highest });
            highest
        };
        let level = applied.gene_level.as_deref().filter(|id| criterion.gene_level_option(id).is_some());
        if !criterion.gene_level.is_empty() && level.is_none() {
            changes.push(Change::NeedsGeneLevel(applied.code.clone()));
            continue;
        }
        let situation = if criterion.guide.is_empty() {
            None
        } else {
            let fits = |id: &str| criterion.strength_for(id, level) == Some(strength);
            let saved = applied.situation.as_deref().filter(|s| fits(s));
            match saved.map(|kept| (kept, false)).or_else(|| unique_fit(criterion, &fits).map(|built| (built, true))) {
                Some((situation, rebuilt)) => {
                    if rebuilt {
                        changes.push(Change::GuideRebuilt(applied.code.clone()));
                    }
                    Some(situation.to_string())
                }
                None => {
                    changes.push(Change::NeedsGuide(applied.code.clone()));
                    continue;
                }
            }
        };
        let reason = applied.reason.clone().filter(|_| strength == applied.strength).map(bounded_reason);
        out.push((applied.code.clone(), Choice { strength, situation, gene_level: level.map(str::to_string), reason }));
    }
    (out, changes)
}

/// The one guide answer that fits, or None when none or several do. Rebuilding
/// an answer from several would invent a reason nobody gave.
fn unique_fit<'a>(criterion: &'a Criterion, fits: &impl Fn(&str) -> bool) -> Option<&'a str> {
    let mut matches = criterion.guide.iter().filter(|g| fits(&g.id));
    let first = matches.next()?;
    matches.next().is_none().then_some(first.id.as_str())
}

/// What to tell the curator after a case was opened. A guide answer that was kept is
/// not a change.
pub(super) fn open_notes(changes: &[Change]) -> Vec<String> {
    changes
        .iter()
        .filter_map(|change| match change {
            Change::Lowered { code, to } => Some(format!("{code} lowered to {}.", to.label())),
            Change::Dropped(code) => Some(format!("{code} (criterion no longer exists, dropped)")),
            Change::NeedsGuide(code) => Some(format!("{code} needs its strength guide answered again.")),
            Change::NeedsGeneLevel(code) => {
                Some(format!("{code} needs its gene-level question answered (new in rules 0.14)."))
            }
            Change::GuideRebuilt(_) => None,
        })
        .collect()
}

/// What to tell the curator after a record's evidence was copied into today's
/// evidence, or `None` when nothing needs saying.
pub(super) fn copy_note(changes: &[Change]) -> Option<String> {
    let lowered: Vec<String> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Lowered { code, to } => Some(format!("{code} to {}", to.label())),
            _ => None,
        })
        .collect();
    let left_out: Vec<String> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Dropped(code) => Some(format!("{code} (criterion no longer exists)")),
            Change::NeedsGuide(code) => Some(format!("{code} needs the guide answered again")),
            Change::NeedsGeneLevel(code) => Some(format!("{code} needs its gene-level question answered")),
            _ => None,
        })
        .collect();
    let guided: Vec<&str> = changes
        .iter()
        .filter_map(|c| match c {
            Change::GuideRebuilt(code) => Some(code.as_str()),
            _ => None,
        })
        .collect();
    let mut notes = Vec::new();
    if !lowered.is_empty() {
        notes.push(format!("Lowered to today's limit: {}.", lowered.join(", ")));
    }
    if !left_out.is_empty() {
        notes.push(format!("Not carried over: {}.", left_out.join(", ")));
    }
    if !guided.is_empty() {
        notes.push(format!(
            "Check the strength of {}: click its strength button to answer the guide again.",
            guided.join(", ")
        ));
    }
    (!notes.is_empty()).then(|| notes.join(" "))
}

impl Workbench {
    /// Copy the latest record's evidence into today's evidence, to adjust from there.
    pub(super) fn start_from_latest(&mut self) {
        let latest = self.records.iter().enumerate().max_by_key(|(i, r)| (r.year, *i)).map(|(_, r)| &r.evidence);
        let Some(evidence) = latest else { return };
        let (today, changes) = carry_forward(self.rules, &to_applied(evidence));
        self.preview = false;
        self.today = today.into_iter().collect();
        if let Some(note) = copy_note(&changes) {
            self.file_status = Some(Status::attention(note));
        }
    }
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose: `use super::*` would also bring in GPUI's own
    // `#[test]` macro (through `gpui_kit::*`), which replaces Rust's.
    use classivar_core::{Applied, RuleSet, Strength};

    use super::{Change, carry_forward, copy_note, open_notes};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    #[test]
    fn old_evidence_is_brought_onto_todays_rules_not_rejected() {
        // PP3 VeryStrong is above today's cap; PVS1 VeryStrong without an answer
        // has two guide answers, so the criterion asks again instead of guessing.
        let (today, changes) = carry_forward(
            rules(),
            &[
                Applied::new("PP3", Strength::VeryStrong),
                Applied::new("PVS1", Strength::VeryStrong).with_gene_level("established"),
            ],
        );
        let today: std::collections::BTreeMap<_, _> = today.into_iter().collect();
        assert_eq!(today["PP3"].strength, Strength::Strong, "today's cap lowers the saved strength");
        assert!(!today.contains_key("PVS1"), "an ambiguous guide answer is not guessed");
        assert_eq!(
            changes,
            vec![Change::Lowered { code: "PP3".into(), to: Strength::Strong }, Change::NeedsGuide("PVS1".into())]
        );
        assert_eq!(
            open_notes(&changes),
            vec!["PP3 lowered to Strong.".to_string(), "PVS1 needs its strength guide answered again.".to_string()]
        );
    }

    #[test]
    fn a_saved_guide_answer_that_the_rules_accept_is_kept_without_a_note() {
        let situation = rules().criterion("PVS1").unwrap().situation_for(Strength::Supporting).unwrap().to_string();
        // One entry with the saved answer, one without: the Supporting answer is
        // unique, so a missing answer is rebuilt as the same one.
        let (today, changes) = carry_forward(
            rules(),
            &[
                Applied::new("PVS1", Strength::Supporting).with_gene_level("established"),
                Applied::guided("PVS1", Strength::Supporting, situation).with_gene_level("established"),
            ],
        );
        assert_eq!(today.len(), 2);
        assert!(today.iter().all(|(code, choice)| code == "PVS1" && choice.situation.is_some()));
        assert!(open_notes(&changes).is_empty(), "{changes:?}");
    }

    #[test]
    fn a_kept_guide_answer_is_no_change_and_a_rebuilt_one_asks_to_be_checked() {
        let situation = rules().criterion("PVS1").unwrap().situation_for(Strength::Supporting).unwrap().to_string();
        let (_, kept) = carry_forward(
            rules(),
            &[Applied::guided("PVS1", Strength::Supporting, situation).with_gene_level("established")],
        );
        assert!(kept.is_empty(), "{kept:?}");
        assert_eq!(copy_note(&kept), None, "nothing to re-answer when the saved answer still fits");
        let (_, rebuilt) =
            carry_forward(rules(), &[Applied::new("PVS1", Strength::Supporting).with_gene_level("established")]);
        assert_eq!(rebuilt, vec![Change::GuideRebuilt("PVS1".into())]);
        assert!(copy_note(&rebuilt).unwrap().contains("Check the strength of PVS1"));
    }
    #[test]
    fn an_unknown_criterion_is_dropped_and_said() {
        let (today, changes) = carry_forward(rules(), &[Applied::new("ZZ9", Strength::Strong)]);
        assert!(today.is_empty());
        assert_eq!(changes, vec![Change::Dropped("ZZ9".into())]);
        assert!(open_notes(&changes)[0].contains("ZZ9"));
        assert!(copy_note(&changes).unwrap().contains("Not carried over: ZZ9"));
    }

    #[test]
    fn a_reason_is_kept_with_its_strength_and_dropped_when_the_strength_is_lowered() {
        let kept = Applied::new("PM2", Strength::Moderate).with_reason("founder population");
        let lowered = Applied::new("PP3", Strength::VeryStrong).with_reason("high score");
        let (today, _) = carry_forward(rules(), &[kept, lowered]);
        let today: std::collections::BTreeMap<_, _> = today.into_iter().collect();
        assert_eq!(today["PM2"].reason.as_deref(), Some("founder population"));
        assert_eq!(today["PP3"].reason, None);
    }

    #[test]
    fn copying_a_record_says_what_it_lowered_left_out_and_asks_to_check() {
        let situation = rules().criterion("PVS1").unwrap().situation_for(Strength::Supporting).unwrap().to_string();
        let (_, changes) = carry_forward(
            rules(),
            &[
                Applied::new("PP3", Strength::VeryStrong),
                Applied::new("PVS1", Strength::VeryStrong).with_gene_level("established"),
                Applied::guided("PVS1x", Strength::Supporting, situation.clone()),
                Applied::new("PVS1", Strength::Supporting).with_gene_level("established"),
            ],
        );
        let note = copy_note(&changes).unwrap();
        assert!(note.contains("Lowered to today's limit: PP3 to Strong."), "{note}");
        assert!(
            note.contains("Not carried over: PVS1 needs the guide answered again, PVS1x (criterion no longer exists)."),
            "{note}"
        );
        assert!(note.contains("Check the strength of PVS1"), "{note}");
        assert_eq!(copy_note(&[]), None);
    }

    #[test]
    fn a_pvs1_saved_before_the_gene_level_question_is_set_aside_and_said() {
        // Rules 0.14 ask Abou Tayoun's Table 1 first. A PVS1 saved without that answer
        // keeps no strength today: the answer is never assumed.
        let old = Applied::guided("PVS1", Strength::VeryStrong, "nmd");
        let (today, changes) = carry_forward(rules(), &[old]);
        assert!(today.is_empty());
        assert_eq!(changes, vec![Change::NeedsGeneLevel("PVS1".into())]);
        assert!(open_notes(&changes)[0].contains("gene-level question"));
        assert!(copy_note(&changes).unwrap().contains("PVS1 needs its gene-level question answered"));
        // With the answer, the lowered strength is kept.
        let lowered = Applied::guided("PVS1", Strength::Strong, "nmd").with_gene_level("lower_one");
        let (today, changes) = carry_forward(rules(), &[lowered]);
        assert_eq!((today[0].1.strength, today[0].1.gene_level.as_deref()), (Strength::Strong, Some("lower_one")));
        assert!(changes.is_empty(), "{changes:?}");
    }
}
