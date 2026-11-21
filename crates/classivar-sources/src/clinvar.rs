//! ClinVar: what each laboratory submitted for a variant (its SCV records).
//!
//! A submission holds the lab, the date it last evaluated the variant, its class,
//! ClinVar's review status, and the condition. It rarely lists the ACMG criteria
//! in a structured way; when a comment names codes ("PP5, PS4_moderate, PVS1"),
//! they are kept as the lab's words for the curator to read. Nothing here applies
//! a criterion: the curator does that.

use classivar_core::{Class, Condition, Inheritance};
use roxmltree::{Document, Node};

use crate::{Fetched, SourceError};

const SOURCE: &str = "ClinVar";
const URL: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi";
/// Popular variants have hundreds of submissions; a full record can be many MB.
const MAX_BYTES: u64 = 50 * 1024 * 1024;

/// One variation (a VCV record) and the germline submissions under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClinVarVariation {
    /// For example `VCV000017677`.
    pub accession: String,
    pub name: String,
    pub submissions: Vec<ClinVarSubmission>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClinVarSubmission {
    /// For example `SCV000271320`.
    pub accession: String,
    pub submitter: String,
    /// `YYYY-MM-DD`, when the lab gave it.
    pub last_evaluated: Option<String>,
    /// The class as ClinVar writes it ("Pathogenic", "not provided", "risk factor" …).
    pub classification: String,
    /// One of the five classes, or None when the text is not one of them.
    pub class: Option<Class>,
    pub review_status: String,
    pub condition: Option<Condition>,
    /// The criteria codes the lab's comments name, in order, as written.
    pub stated_criteria: Option<String>,
}

/// Fetch one variation by its id (the number in VCV000017677). `Ok(None)` when
/// ClinVar answers that it has no such variation. Blocks; call from a background thread.
pub fn lookup_variation(id: u64) -> Result<Fetched<Option<ClinVarVariation>>, SourceError> {
    let id = id.to_string();
    let query =
        [("db", "clinvar"), ("rettype", "vcv"), ("is_variationid", ""), ("id", id.as_str()), ("tool", "ClassiVAR")];
    let body = crate::get_ok(URL, &query, "application/xml", MAX_BYTES)?;
    Ok(Fetched::now(SOURCE, parse_variation(&body)?))
}

/// Read a VCV answer. An empty result set means "no such variation".
pub(crate) fn parse_variation(xml: &str) -> Result<Option<ClinVarVariation>, SourceError> {
    let document = Document::parse(xml).map_err(|e| SourceError::Unexpected(format!("not ClinVar XML ({e})")))?;
    let root = document.root_element();
    if root.tag_name().name() != "ClinVarResult-Set" {
        return Err(SourceError::Unexpected("not a ClinVar result".into()));
    }
    let Some(archive) = child(root, "VariationArchive") else { return Ok(None) };
    let accession = archive.attribute("Accession").ok_or_else(|| missing("the VCV accession"))?;
    let mappings: Vec<Node> = archive.descendants().filter(|n| n.has_tag_name("TraitMapping")).collect();
    let submissions = archive
        .descendants()
        .filter(|n| n.has_tag_name("ClinicalAssertion"))
        .filter_map(|assertion| submission(assertion, &mappings).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(ClinVarVariation {
        accession: accession.to_string(),
        name: archive.attribute("VariationName").unwrap_or(accession).to_string(),
        submissions,
    }))
}

fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|n| n.has_tag_name(name))
}

fn text_of<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    child(node, name).and_then(|n| n.text()).map(str::trim)
}

fn missing(what: &str) -> SourceError {
    SourceError::Unexpected(format!("a ClinVar record has no {what}"))
}

/// One germline submission. `Ok(None)` for a somatic one (no germline class).
fn submission(assertion: Node, mappings: &[Node]) -> Result<Option<ClinVarSubmission>, SourceError> {
    let scv = child(assertion, "ClinVarAccession").ok_or_else(|| missing("SCV accession"))?;
    let classification = child(assertion, "Classification").ok_or_else(|| missing("classification"))?;
    let Some(germline) = text_of(classification, "GermlineClassification") else { return Ok(None) };
    let comments: Vec<&str> =
        assertion.descendants().filter(|n| n.has_tag_name("Comment")).filter_map(|n| n.text()).collect();
    Ok(Some(ClinVarSubmission {
        accession: scv.attribute("Accession").ok_or_else(|| missing("SCV accession"))?.to_string(),
        submitter: scv.attribute("SubmitterName").unwrap_or("Unnamed submitter").to_string(),
        last_evaluated: classification.attribute("DateLastEvaluated").map(str::to_string),
        classification: germline.to_string(),
        class: class_of(germline),
        review_status: text_of(classification, "ReviewStatus").unwrap_or("").to_string(),
        condition: condition(assertion, mappings),
        stated_criteria: stated_criteria(&comments.join(" ")),
    }))
}

/// The five classes, as ClinVar spells them (case varies between submitters).
/// Combined or other values ("Pathogenic/Likely pathogenic", "risk factor") are None.
pub fn class_of(text: &str) -> Option<Class> {
    match text.trim().to_ascii_lowercase().as_str() {
        "pathogenic" => Some(Class::Pathogenic),
        "likely pathogenic" => Some(Class::LikelyPathogenic),
        "uncertain significance" => Some(Class::Vus),
        "likely benign" => Some(Class::LikelyBenign),
        "benign" => Some(Class::Benign),
        _ => None,
    }
}

/// The condition a submission names: MedGen's name from ClinVar's mapping of
/// the submitter's trait, else the submitter's own name, else its cross-reference.
/// The MONDO id is kept when the submitter gave one and named a single trait.
fn condition(assertion: Node, mappings: &[Node]) -> Option<Condition> {
    let id = assertion.attribute("ID")?;
    let traits: Vec<Node> = assertion.descendants().filter(|n| n.has_tag_name("Trait")).collect();
    let names: Vec<String> = traits.iter().filter_map(|t| trait_name(*t, id, mappings)).collect();
    if names.is_empty() {
        return None;
    }
    let mondo = match traits.as_slice() {
        [one] => one.children().find(|x| x.has_tag_name("XRef") && x.attribute("DB") == Some("MONDO")),
        _ => None,
    };
    Some(Condition {
        name: names.join("; "),
        mondo: mondo.and_then(|x| x.attribute("ID")).map(str::to_string),
        inheritance: Inheritance::Unknown,
    })
}

fn trait_name(trait_node: Node, assertion_id: &str, mappings: &[Node]) -> Option<String> {
    let own = trait_node.descendants().find(|n| n.has_tag_name("ElementValue")).and_then(|n| n.text());
    let xrefs: Vec<(&str, &str)> = trait_node
        .children()
        .filter(|x| x.has_tag_name("XRef"))
        .filter_map(|x| Some((x.attribute("DB")?, x.attribute("ID")?)))
        .collect();
    let mapped = mappings.iter().filter(|m| m.attribute("ClinicalAssertionID") == Some(assertion_id)).find(|m| {
        let value = m.attribute("MappingValue");
        match m.attribute("MappingType") {
            Some("Name") => value == own,
            Some("XRef") => xrefs.iter().any(|(db, id)| m.attribute("MappingRef") == Some(db) && value == Some(id)),
            _ => false,
        }
    });
    let medgen = mapped.and_then(|m| m.children().find(|c| c.has_tag_name("MedGen"))).and_then(|m| m.attribute("Name"));
    medgen.or(own).map(|name| name.trim().to_string()).or_else(|| xrefs.first().map(|(db, id)| format!("{db} {id}")))
}

/// The ACMG/AMP criteria codes a comment names, in order, each once, with any
/// strength suffix as written ("PS4_moderate"). None when it names none.
pub fn stated_criteria(text: &str) -> Option<String> {
    let mut found: Vec<String> = Vec::new();
    for token in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        let code = token.split('_').next().unwrap_or("").to_ascii_uppercase();
        if is_criterion_code(&code) && !found.iter().any(|f| f.eq_ignore_ascii_case(token)) {
            found.push(token.to_string());
        }
    }
    (!found.is_empty()).then(|| found.join(", "))
}

fn is_criterion_code(code: &str) -> bool {
    let ranges: [(&str, u8); 7] = [("PVS", 1), ("PS", 4), ("PM", 6), ("PP", 5), ("BA", 1), ("BS", 4), ("BP", 7)];
    ranges.iter().any(|(prefix, max)| {
        code.strip_prefix(prefix)
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=*max).contains(&n) && !code[prefix.len()..].starts_with('0'))
    })
}

#[cfg(test)]
mod tests;
