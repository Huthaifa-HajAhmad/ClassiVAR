//! Plain-text descriptions of the rules and of the gene sources, and the points scale.

use super::*;

pub(super) fn plural(n: i32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// The points scale: its range and the zones, taken from the rule set so the
/// scale and the class badge cannot disagree.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Scale {
    pub(super) min: i32,
    pub(super) max: i32,
    /// (class, first point, first point of the next zone)
    pub(super) zones: Vec<(Class, i32, i32)>,
}

pub(super) fn scale_for(rules: &RuleSet) -> Scale {
    let t = &rules.class_min_points;
    // Two points of room beyond the first and last class limit, for the Benign and Pathogenic zones.
    let (min, max) = (t.likely_benign - 2, t.pathogenic + 2);
    Scale {
        min,
        max,
        zones: vec![
            (Class::Benign, min, t.likely_benign),
            (Class::LikelyBenign, t.likely_benign, t.vus),
            (Class::Vus, t.vus, t.likely_pathogenic),
            (Class::LikelyPathogenic, t.likely_pathogenic, t.pathogenic),
            (Class::Pathogenic, t.pathogenic, max),
        ],
    }
}

/// How many detail lines a source card shows before "Show all".
pub(super) const CARD_LINES: usize = 2;

pub(super) fn describe_panelapp(found: &Fetched<PanelAppGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.entries.is_empty() {
        return (
            format!("{} is on no panel. That does not prove it is unrelated to disease.", gene.symbol),
            Vec::new(),
        );
    }
    let headline = format!(
        "{} is on {} panel{}: {} green, {} amber, {} red.",
        gene.symbol,
        gene.entries.len(),
        plural(gene.entries.len() as i32),
        gene.count_at(Confidence::Green),
        gene.count_at(Confidence::Amber),
        gene.count_at(Confidence::Red),
    );
    let lines = gene
        .entries
        .iter()
        .map(|e| {
            format!("• {} (v{}): {} · {}", e.panel_name, e.panel_version, e.confidence.label(), e.mode_of_inheritance)
        })
        .collect();
    (headline, lines)
}

pub(super) fn describe_g2p(found: &Fetched<G2pGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.records.is_empty() {
        return (format!("G2P has no record for {}.", gene.symbol), Vec::new());
    }
    let headline = format!("{} gene-disease record{}.", gene.records.len(), plural(gene.records.len() as i32));
    let lines = gene
        .records
        .iter()
        .map(|r| {
            format!("• {}: {} · {} · {}", r.disease, r.confidence, r.genotype.replace('_', " "), r.molecular_mechanism)
        })
        .collect();
    (headline, lines)
}

pub(super) fn describe_clingen(found: &Fetched<ClinGenGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.curations.is_empty() {
        return (format!("ClinGen has not rated {}.", gene.symbol), Vec::new());
    }
    let headline = format!("{} gene-disease rating{}.", gene.curations.len(), plural(gene.curations.len() as i32));
    let lines = gene
        .curations
        .iter()
        .map(|c| {
            format!(
                "• {} ({}): {} · {} · {}",
                c.disease, c.inheritance, c.classification, c.classified_on, c.expert_panel
            )
        })
        .collect();
    (headline, lines)
}

/// What the curator should read before trusting a class: the review status and rule version.
pub(super) fn rules_tag(rules: &RuleSet) -> String {
    let meta = &rules.meta;
    match meta.status {
        ReviewStatus::Unverified => format!("UNVERIFIED rules v{}", meta.version),
        ReviewStatus::Reviewed => format!(
            "rules v{}, reviewed by {} on {}",
            meta.version,
            meta.reviewed_by.as_deref().unwrap_or("?"),
            meta.reviewed_on.as_deref().unwrap_or("?")
        ),
    }
}

/// Does the loaded rule file give official point ranges to every VUS sub-tier?
/// Only then may the screen name one for new evidence (`Assessment::vus_tier`
/// follows the same rule).
pub(super) fn tiers_are_ranged(rules: &RuleSet) -> bool {
    !rules.vus_tiers.is_empty() && rules.vus_tiers.iter().all(|t| t.min_points.is_some())
}

/// A one-paragraph description of the loaded rule file. It must stay true for
/// any file loaded with `--rules`, so it names only what the loader guarantees.
pub(super) fn rules_about_text(rules: &RuleSet) -> String {
    let meta = &rules.meta;
    let status = match meta.status {
        ReviewStatus::Unverified => "UNVERIFIED draft".to_string(),
        ReviewStatus::Reviewed => format!(
            "reviewed by {} on {}",
            meta.reviewed_by.as_deref().unwrap_or("?"),
            meta.reviewed_on.as_deref().unwrap_or("?")
        ),
    };
    format!(
        "The loaded rules are {} v{} ({}). Criteria whose default differs from the 2015 paper say so in their notes, and criteria the experts no longer recommend are listed but not counted. The evidence helpers above the criteria apply published tables: predictor scores (PP3/BP4), functional assays (PS3/BS3), family and phenotype points (PP1/PP4), non-segregation (BS4), de novo patients (PS2 or PM6) and recessive patients (PM3). A dated BA1 exceptions reference is also available.",
        meta.id, meta.version, status
    )
}

/// Does a discouraged criterion get its "not counted" row label? A new
/// assessment lists it without counting, but an earlier record is scored with
/// it, so the same row label there would contradict the notes.
pub(super) fn discouraged_row_note(discouraged: bool, editing: Editing) -> Option<&'static str> {
    (discouraged && editing == Editing::Today).then_some("not counted")
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose: `use super::*` would also bring in GPUI's own
    // `#[test]` macro (through `gpui_kit::*`), which replaces Rust's.
    use classivar_core::{Class, RuleSet};

    use super::{Editing, discouraged_row_note, rules_about_text, scale_for};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    #[test]
    fn the_scale_follows_the_rule_set() {
        let scale = scale_for(rules());
        assert_eq!((scale.min, scale.max), (-8, 12));
        let expected = vec![
            (Class::Benign, -8, -6),
            (Class::LikelyBenign, -6, 0),
            (Class::Vus, 0, 6),
            (Class::LikelyPathogenic, 6, 10),
            (Class::Pathogenic, 10, 12),
        ];
        assert_eq!(scale.zones, expected);
        // Every point from the bottom to the top of the scale lies in the zone of the class the rules give it.
        for points in scale.min..scale.max {
            let zone = scale.zones.iter().find(|(_, from, to)| (*from..*to).contains(&points)).unwrap();
            assert_eq!(zone.0, rules().class_for_points(points), "{points} points");
        }
    }

    #[test]
    fn the_scale_moves_when_the_rule_set_changes() {
        let text =
            include_str!("../../../../rules/acmg-amp-points.toml").replacen("pathogenic = 10", "pathogenic = 12", 1);
        let changed = RuleSet::parse(&text).unwrap();
        assert_eq!(scale_for(&changed).max, 14);
        assert!(scale_for(&changed).zones.contains(&(Class::LikelyPathogenic, 6, 12)));
    }

    #[test]
    fn a_discouraged_row_says_not_counted_only_for_todays_evidence() {
        assert_eq!(discouraged_row_note(true, Editing::Today), Some("not counted"));
        // An earlier record counts it; the notes say so, the row must not deny it.
        assert_eq!(discouraged_row_note(true, Editing::Record(0)), None);
        assert_eq!(discouraged_row_note(false, Editing::Today), None);
    }

    #[test]
    fn the_about_text_names_the_loaded_rules_itself() {
        let text = rules_about_text(rules());
        assert!(text.contains("acmg-amp-points"), "{text}");
        assert!(text.contains("UNVERIFIED draft"), "{text}");
        // True for any rule file, so it must not name this file's own deviations.
        assert!(!text.contains("PVS1"), "{text}");
        assert!(!text.contains("PM2"), "{text}");
    }
}
