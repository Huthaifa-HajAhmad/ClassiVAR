//! Turning ClinVar submissions into earlier records, and a ClinGen validity
//! rating into a suggested answer on the gene-disease link. Plain functions,
//! tested here; the screen only calls them.

use std::collections::HashSet;

use classivar_core::{
    Condition, GeneDecision, MAX_CONDITION_CHARS, MAX_LAB_CHARS, MAX_SOURCE_CHARS, MAX_YEAR, MIN_YEAR, RecordSource,
};
use classivar_sources::clinvar::{ClinVarSubmission, ClinVarVariation};

use super::{Evidence, Record};

/// What an import adds, and what it leaves out and why.
pub(super) struct ImportPlan {
    pub(super) records: Vec<Record>,
    /// Submissions whose class is not one of the five ("not provided", "risk factor" …).
    pub(super) unclassified: usize,
    /// Submissions already in the case (same SCV accession).
    pub(super) already_there: usize,
    /// Classified submissions left out because the case holds at most `room` more records.
    pub(super) no_room: usize,
}

/// The records to add from a ClinVar variation. With more classified submissions
/// than `room`, the most recently evaluated are kept. The records come out oldest
/// first, as on the timeline. Their criteria are empty: ClinVar rarely lists them,
/// and the curator enters what the lab used (the lab's own words are kept in the source).
pub(super) fn import_plan(
    variation: &ClinVarVariation,
    existing: &HashSet<String>,
    room: usize,
    today: &str,
    this_year: u16,
) -> ImportPlan {
    let classified: Vec<&ClinVarSubmission> = variation.submissions.iter().filter(|s| s.class.is_some()).collect();
    let mut fresh: Vec<&ClinVarSubmission> =
        classified.iter().copied().filter(|s| !existing.contains(&s.accession)).collect();
    // Newest evaluation first, so a cut keeps the most recent; undated ones last.
    fresh.sort_by(|a, b| b.last_evaluated.cmp(&a.last_evaluated));
    let no_room = fresh.len().saturating_sub(room);
    fresh.truncate(room);
    let mut records: Vec<Record> = fresh.iter().map(|s| record_of(s, today, this_year)).collect();
    records.sort_by_key(|r| r.year);
    ImportPlan {
        records,
        unclassified: variation.submissions.len() - classified.len(),
        already_there: classified.len() - classified.iter().filter(|s| !existing.contains(&s.accession)).count(),
        no_room,
    }
}

fn record_of(submission: &ClinVarSubmission, today: &str, this_year: u16) -> Record {
    let year = submission
        .last_evaluated
        .as_deref()
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<u16>().ok())
        .unwrap_or(this_year)
        .clamp(MIN_YEAR, MAX_YEAR);
    Record {
        // A name longer than a saved case accepts is cut and ends in "…", so it reads as cut.
        label: if submission.submitter.chars().count() > MAX_LAB_CHARS {
            submission.submitter.chars().take(MAX_LAB_CHARS - 1).collect::<String>().trim_end().to_string() + "…"
        } else {
            submission.submitter.clone()
        },
        year,
        class: submission.class,
        tier: None,
        evidence: Evidence::new(),
        condition: submission
            .condition
            .clone()
            .map(|c| Condition { name: c.name.chars().take(MAX_CONDITION_CHARS).collect(), ..c }),
        source: Some(RecordSource {
            accession: submission.accession.clone(),
            review_status: submission.review_status.clone(),
            last_evaluated: submission.last_evaluated.clone(),
            stated_criteria: submission.stated_criteria.as_deref().map(whole_codes_within_limit),
            imported_on: today.to_string(),
        }),
    }
}

/// The lab's list of codes, cut after a whole code and ended with "…" when it is
/// longer than a saved case accepts. Half a code would misstate the lab's words.
fn whole_codes_within_limit(list: &str) -> String {
    if list.chars().count() <= MAX_SOURCE_CHARS {
        return list.to_string();
    }
    let room = MAX_SOURCE_CHARS - ", …".chars().count();
    let mut kept = String::new();
    for code in list.split(", ") {
        let longer = if kept.is_empty() { code.to_string() } else { format!("{kept}, {code}") };
        if longer.chars().count() > room {
            break;
        }
        kept = longer;
    }
    if kept.is_empty() { "…".to_string() } else { format!("{kept}, …") }
}

/// One sentence about an import, for the status banner.
pub(super) fn import_summary(plan: &ImportPlan, accession: &str) -> String {
    let mut parts = vec![format!("Imported {} records from ClinVar {accession}.", plan.records.len())];
    if plan.already_there > 0 {
        parts.push(format!("{} were already in the case.", plan.already_there));
    }
    if plan.unclassified > 0 {
        parts.push(format!(
            "{} had no class from the five (for example \"not provided\" or \"risk factor\") and were left out.",
            plan.unclassified
        ));
    }
    if plan.no_room > 0 {
        parts.push(format!("{} older ones did not fit: a case holds at most 30 records.", plan.no_room));
    }
    parts.push("ClinVar rarely lists the criteria a lab used: enter them for each record.".to_string());
    parts.join(" ")
}

/// A suggested answer on the gene-disease link from a ClinGen rating. Definitive,
/// Strong and Moderate suggest "established"; Limited, Disputed, Refuted and "No
/// Known Disease Relationship" suggest "not established" (a gene of uncertain
/// significance). The curator can change it; the screen says where it came from.
pub(super) fn decision_for_validity(classification: &str) -> GeneDecision {
    match classification {
        "Definitive" | "Strong" | "Moderate" => GeneDecision::Established,
        "Limited" | "Disputed" | "Refuted" | "No Known Disease Relationship" => GeneDecision::NotEstablished,
        _ => GeneDecision::NotDecided,
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{Class, Condition, Inheritance};

    use super::*;

    fn submission(accession: &str, class: Option<Class>, evaluated: Option<&str>) -> ClinVarSubmission {
        ClinVarSubmission {
            accession: accession.into(),
            submitter: format!("Lab {accession}"),
            last_evaluated: evaluated.map(str::to_string),
            classification: class.map_or("not provided", Class::label).into(),
            class,
            review_status: "criteria provided, single submitter".into(),
            condition: Some(Condition { name: "HBOC".into(), mondo: None, inheritance: Inheritance::Unknown }),
            stated_criteria: Some("PS4, PVS1".into()),
        }
    }

    fn variation(submissions: Vec<ClinVarSubmission>) -> ClinVarVariation {
        ClinVarVariation { accession: "VCV000017677".into(), name: "test".into(), submissions }
    }

    #[test]
    fn classified_submissions_become_records_oldest_first_with_their_source() {
        let v = variation(vec![
            submission("SCV2", Some(Class::Pathogenic), Some("2022-11-23")),
            submission("SCV1", Some(Class::LikelyPathogenic), Some("2015-02-11")),
            submission("SCV3", None, None),
        ]);
        let plan = import_plan(&v, &HashSet::new(), 30, "2026-11-21", 2026);
        let years: Vec<u16> = plan.records.iter().map(|r| r.year).collect();
        assert_eq!(years, [2015, 2022]);
        assert_eq!(plan.unclassified, 1);
        let first = &plan.records[0];
        assert_eq!(first.class, Some(Class::LikelyPathogenic));
        assert!(first.evidence.is_empty(), "criteria are never invented from ClinVar");
        let source = first.source.as_ref().unwrap();
        assert_eq!((source.accession.as_str(), source.stated_criteria.as_deref()), ("SCV1", Some("PS4, PVS1")));
        assert_eq!(first.condition.as_ref().unwrap().name, "HBOC");
    }

    #[test]
    fn a_second_import_skips_what_is_there_and_a_full_case_keeps_the_newest() {
        let v = variation(vec![
            submission("SCV1", Some(Class::Vus), Some("2015-01-01")),
            submission("SCV2", Some(Class::Vus), Some("2020-01-01")),
            submission("SCV3", Some(Class::Vus), Some("2024-01-01")),
        ]);
        let existing: HashSet<String> = ["SCV3".to_string()].into();
        let plan = import_plan(&v, &existing, 1, "2026-11-21", 2026);
        assert_eq!(plan.already_there, 1);
        assert_eq!(plan.no_room, 1);
        assert_eq!(plan.records.len(), 1);
        assert_eq!(plan.records[0].year, 2020, "the newest one that fits");
        let summary = import_summary(&plan, "VCV000017677");
        assert!(summary.contains("1 were already") && summary.contains("did not fit"), "{summary}");
        assert!(summary.contains("enter them for each record"), "{summary}");
    }

    #[test]
    fn an_undated_submission_gets_this_year_and_a_long_lab_name_is_cut() {
        let mut long = submission("SCV9", Some(Class::Benign), None);
        long.submitter = "L".repeat(200);
        let plan = import_plan(&variation(vec![long]), &HashSet::new(), 30, "2026-11-21", 2026);
        assert_eq!(plan.records[0].year, 2026);
        assert_eq!(plan.records[0].label.chars().count(), MAX_LAB_CHARS);
        assert!(plan.records[0].label.ends_with('…'), "a cut name says it was cut");
    }

    #[test]
    fn long_lab_text_is_cut_to_what_a_saved_case_accepts() {
        let mut long = submission("SCV9", Some(Class::Pathogenic), Some("2020-01-01"));
        long.stated_criteria = Some(vec!["PS4_moderate"; 40].join(", "));
        long.condition.as_mut().unwrap().name = "C".repeat(MAX_CONDITION_CHARS + 50);
        let plan = import_plan(&variation(vec![long]), &HashSet::new(), 30, "2026-11-21", 2026);
        let record = &plan.records[0];
        let stated = record.source.as_ref().unwrap().stated_criteria.as_deref().unwrap();
        assert!(stated.chars().count() <= MAX_SOURCE_CHARS, "{stated}");
        assert!(stated.ends_with("PS4_moderate, …"), "a cut list says it was cut, at a whole code: {stated}");
        assert_eq!(record.condition.as_ref().unwrap().name.chars().count(), MAX_CONDITION_CHARS);
        let short = submission("SCV1", Some(Class::Vus), None);
        let kept = import_plan(&variation(vec![short]), &HashSet::new(), 30, "2026-11-21", 2026);
        assert_eq!(kept.records[0].source.as_ref().unwrap().stated_criteria.as_deref(), Some("PS4, PVS1"));
    }

    #[test]
    fn clingen_ratings_suggest_an_answer_and_unknown_ones_suggest_none() {
        assert_eq!(decision_for_validity("Definitive"), GeneDecision::Established);
        assert_eq!(decision_for_validity("Moderate"), GeneDecision::Established);
        assert_eq!(decision_for_validity("Limited"), GeneDecision::NotEstablished);
        assert_eq!(decision_for_validity("Refuted"), GeneDecision::NotEstablished);
        assert_eq!(decision_for_validity("Something new"), GeneDecision::NotDecided);
    }
}
