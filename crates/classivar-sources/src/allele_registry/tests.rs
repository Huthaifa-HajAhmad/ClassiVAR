use super::*;

const CA003404: &str = include_str!("../../tests/fixtures/allele-registry-ca003404.json");
const UNREGISTERED: &str = include_str!("../../tests/fixtures/allele-registry-unregistered.json");

fn found(status: u16, body: &str) -> Vec<VariantIdentity> {
    match read_reply(status, body, "2026-11-21") {
        Ok(Lookup::Found(list)) => list,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_registered_allele_gives_both_builds_the_mane_transcript_and_its_links() {
    let [identity] = found(200, CA003404).try_into().expect("one allele");
    assert_eq!(identity.allele_id.as_deref(), Some("CA003404"));
    assert_eq!(identity.grch38.as_deref(), Some("NC_000017.11:g.43057065dup"));
    assert_eq!(identity.grch37.as_deref(), Some("NC_000017.10:g.41209082dup"));
    assert_eq!(identity.mane_select.as_deref(), Some("NM_007294.4:c.5266dup"), "the RefSeq MANE entry, not Ensembl's");
    assert_eq!(identity.gene.as_deref(), Some("BRCA1"));
    assert!(identity.protein.as_deref().is_some_and(|p| p.starts_with("NP_009225.1:p.Gln1756")));
    assert_eq!(identity.clinvar_variation, Some(17677));
    assert_eq!(identity.dbsnp, Some(80357906));
    assert_eq!((identity.checked_with.as_str(), identity.checked_on.as_str()), (SOURCE, "2026-11-21"));
}

#[test]
fn an_allele_the_registry_checked_but_never_registered_has_no_id() {
    let [identity] = found(200, UNREGISTERED).try_into().expect("one allele");
    assert_eq!(identity.allele_id, None, "`_:CA` is not an id");
    assert_eq!(identity.grch38.as_deref(), Some("NC_000017.11:g.43060123T>A"));
    assert_eq!(identity.clinvar_variation, None);
}

#[test]
fn a_list_answer_keeps_every_allele_and_an_empty_list_is_not_found() {
    let two = format!("[{CA003404},{UNREGISTERED}]");
    assert_eq!(found(200, &two).len(), 2, "a multi-allelic rsID must not be cut to one");
    assert_eq!(read_reply(200, "[]", "2026-11-21").unwrap(), Lookup::NotFound);
}

#[test]
fn not_found_is_only_the_registry_saying_so() {
    let body =
        r#"{"errorType":"NotFound","description":"The system does not contain any data about requested resource."}"#;
    assert_eq!(read_reply(404, body, "2026-11-21").unwrap(), Lookup::NotFound);
    assert!(matches!(read_reply(500, "", "2026-11-21"), Err(SourceError::Status(500))));
    assert!(matches!(read_reply(200, "<html>maintenance</html>", "2026-11-21"), Err(SourceError::Unexpected(_))));
}

#[test]
fn a_refusal_carries_the_registrys_reason_in_plain_words() {
    let wrong_base = r#"{"errorType":"IncorrectReferenceAllele","description":"Given allele from reference sequence is incorrect."}"#;
    let error = read_reply(400, wrong_base, "2026-11-21").unwrap_err().to_string();
    assert!(error.contains("reference base does not match"), "{error}");
    let unreadable = r#"{"errorType":"HgvsParsingError","description":"Given HGVS expressions cannot be parsed."}"#;
    assert!(read_reply(400, unreadable, "2026-11-21").unwrap_err().to_string().contains("cannot read this HGVS"));
    let other = r#"{"errorType":"Something","description":"Some new reason."}"#;
    assert!(read_reply(400, other, "2026-11-21").unwrap_err().to_string().contains("Some new reason."));
}

#[test]
#[ignore = "calls the live ClinGen Allele Registry"]
fn live_every_id_kind_reaches_the_same_allele() {
    for query in [
        RegistryQuery::Hgvs("NM_007294.4:c.5266dup".into()),
        RegistryQuery::Hgvs("NC_000017.11:g.43057062_43057063insG".into()),
        RegistryQuery::ClinGenAllele("CA003404".into()),
        RegistryQuery::ClinVar(17677),
        RegistryQuery::DbSnp(80357906),
    ] {
        let Lookup::Found(list) = lookup(&query).unwrap().value else { panic!("{query:?} not found") };
        assert_eq!(list[0].allele_id.as_deref(), Some("CA003404"), "{query:?}");
    }
    let wrong = lookup(&RegistryQuery::Hgvs("NM_007294.4:c.5266A>T".into())).unwrap_err().to_string();
    assert!(wrong.contains("reference base"), "{wrong}");
}
