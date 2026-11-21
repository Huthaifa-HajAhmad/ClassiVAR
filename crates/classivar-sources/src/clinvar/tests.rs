use super::*;

const VCV17677: &str = include_str!("../../tests/fixtures/clinvar-vcv000017677.xml");

fn variation() -> ClinVarVariation {
    parse_variation(VCV17677).unwrap().expect("the fixture is one variation")
}

fn scv<'a>(v: &'a ClinVarVariation, accession: &str) -> &'a ClinVarSubmission {
    v.submissions.iter().find(|s| s.accession == accession).expect(accession)
}

#[test]
fn each_germline_submission_is_read_with_its_lab_date_class_and_review_status() {
    let v = variation();
    assert_eq!(v.accession, "VCV000017677");
    assert_eq!(v.name, "NM_007294.4(BRCA1):c.5266dup (p.Gln1756fs)");
    assert_eq!(v.submissions.len(), 4);
    let lmm = scv(&v, "SCV000271320");
    assert!(lmm.submitter.starts_with("Laboratory for Molecular Medicine"));
    assert_eq!(lmm.last_evaluated.as_deref(), Some("2022-11-23"));
    assert_eq!(lmm.class, Some(Class::Pathogenic));
    assert_eq!(lmm.review_status, "criteria provided, single submitter");
}

#[test]
fn a_class_that_is_not_one_of_the_five_is_kept_as_text_only() {
    let v = variation();
    let gene_reviews = scv(&v, "SCV000086949");
    assert_eq!(gene_reviews.classification, "not provided");
    assert_eq!(gene_reviews.class, None);
    assert_eq!(gene_reviews.last_evaluated, None);
    assert_eq!(class_of("Pathogenic/Likely pathogenic"), None);
    assert_eq!(class_of("likely benign"), Some(Class::LikelyBenign));
}

#[test]
fn the_condition_uses_medgens_name_and_keeps_a_mondo_id_the_lab_gave() {
    let v = variation();
    let innsbruck = scv(&v, "SCV000212010").condition.clone().unwrap();
    assert_eq!(innsbruck.name, "Breast-ovarian cancer, familial, susceptibility to, 1");
    assert_eq!(innsbruck.mondo, None);
    let igm = scv(&v, "SCV007449512").condition.clone().unwrap();
    assert_eq!(igm.name, "Hereditary cancer-predisposing syndrome");
    assert_eq!(igm.mondo.as_deref(), Some("MONDO:0015356"));
    assert_eq!(igm.inheritance, Inheritance::Unknown, "never guessed");
}

#[test]
fn criteria_codes_in_a_comment_are_kept_as_the_labs_words() {
    let v = variation();
    // The lab wrote "ACMG/AMP Criteria applied: PS4, PVS1."
    assert_eq!(scv(&v, "SCV000271320").stated_criteria.as_deref(), Some("PS4, PVS1"));
    assert_eq!(scv(&v, "SCV000212010").stated_criteria, None);
    assert_eq!(stated_criteria("ps3_Supporting; PM2 and PM2, BP7."), Some("ps3_Supporting, PM2, BP7".into()));
    // Look-alikes are not codes.
    assert_eq!(stated_criteria("PM7, PS0, BP10, PVS2, PMS2, PS01, mPM2"), None);
}

#[test]
fn an_empty_answer_is_no_such_variation_and_other_text_is_an_error() {
    let empty = "<?xml version=\"1.0\"?>\n<ClinVarResult-Set><set/></ClinVarResult-Set>";
    assert_eq!(parse_variation(empty).unwrap(), None);
    assert!(matches!(parse_variation("<html>busy</html>"), Err(SourceError::Unexpected(_))));
    assert!(matches!(parse_variation("not xml"), Err(SourceError::Unexpected(_))));
}

#[test]
#[ignore = "calls the live ClinVar service"]
fn live_a_known_variation_has_submissions_and_an_unknown_one_is_none() {
    let v = lookup_variation(17677).unwrap().value.expect("VCV000017677 exists");
    assert_eq!(v.accession, "VCV000017677");
    assert!(v.submissions.len() > 50);
    assert!(lookup_variation(999_999_999).unwrap().value.is_none());
}
