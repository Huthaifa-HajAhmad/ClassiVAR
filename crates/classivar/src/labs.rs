//! Lab names offered in the record's name box.
//!
//! A typing convenience only, like the gene list: a short list of laboratories
//! that often submit to ClinVar, plus the names already used in this case. It
//! is not a register of labs and it never decides anything: any name can be
//! typed. The screen says so next to the list.

/// Laboratories that often submit clinical interpretations to ClinVar. Not exhaustive.
const COMMON: &[&str] = &[
    "Ambry Genetics",
    "ARUP Laboratories",
    "Baylor Genetics",
    "Blueprint Genetics",
    "CeGaT",
    "Centogene",
    "Color Diagnostics",
    "Fulgent Genetics",
    "GeneDx",
    "Illumina Laboratory Services",
    "Invitae",
    "Labcorp Genetics",
    "Mayo Clinic Laboratories",
    "Myriad Genetics",
    "Natera",
    "PreventionGenetics",
    "Quest Diagnostics",
    "Revvity Omics",
    "ClinGen expert panel (VCEP)",
];

/// Names to offer for what is typed: names already used in this case first, then
/// the common list. A name matches when it contains the typed text, ignoring case;
/// an exact match is left out (there is nothing left to fill in). Empty text
/// offers everything. At most `limit` names, each once.
pub fn suggestions_for(typed: &str, in_case: &[&str], limit: usize) -> Vec<String> {
    let typed = typed.trim().to_lowercase();
    let mut out: Vec<String> = Vec::new();
    for name in in_case.iter().copied().chain(COMMON.iter().copied()) {
        let lower = name.to_lowercase();
        let fits = lower.contains(&typed) && lower != typed;
        if fits && !out.iter().any(|kept| kept.eq_ignore_ascii_case(name)) {
            out.push(name.to_string());
        }
        if out.len() == limit {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{COMMON, suggestions_for};

    #[test]
    fn names_from_this_case_come_first_and_each_name_once() {
        let found = suggestions_for("gene", &["GeneDx", "My lab"], 10);
        assert_eq!(found.first().map(String::as_str), Some("GeneDx"));
        assert_eq!(found.iter().filter(|name| name.as_str() == "GeneDx").count(), 1);
        assert!(found.iter().any(|name| name == "Ambry Genetics"), "a match inside the name counts");
    }

    #[test]
    fn empty_text_offers_everything_up_to_the_limit_and_an_exact_name_is_left_out() {
        assert_eq!(suggestions_for("", &[], 5).len(), 5);
        assert_eq!(suggestions_for("  ", &[], 100).len(), COMMON.len());
        assert!(!suggestions_for("genedx", &[], 10).iter().any(|name| name == "GeneDx"), "already typed in full");
        assert!(suggestions_for("zzz", &[], 10).is_empty());
    }
}
