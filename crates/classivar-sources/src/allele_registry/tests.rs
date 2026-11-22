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
fn without_a_mane_select_the_refseq_coding_names_are_listed_and_ensembl_and_predicted_ones_are_not() {
    let body = r#"{"@id":"http://reg.genome.network/allele/CA645165536","genomicAlleles":[],
      "transcriptAlleles":[
        {"hgvs":["ENST00000320857.7:c.-15+2T>C"],"geneSymbol":"SSR4"},
        {"hgvs":["NM_001204526.1:c.19+2T>C"],"geneSymbol":"SSR4"},
        {"hgvs":["NR_037927.1:n.137+2T>C"],"geneSymbol":"SSR4"},
        {"hgvs":["XM_011531186.1:c.-14-87T>C"],"geneSymbol":"SSR4"},
        {"hgvs":["NM_006280.2:c.-101T>C"],"geneSymbol":"SSR4"}]}"#;
    let [identity] = found(200, body).try_into().expect("one allele");
    assert_eq!(identity.mane_select, None);
    assert_eq!(identity.coding, ["NM_001204526.1:c.19+2T>C", "NM_006280.2:c.-101T>C"]);
    assert_eq!(identity.gene.as_deref(), Some("SSR4"), "the gene comes from the coding transcripts too");
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
        assert_eq!(list.first().and_then(|a| a.allele_id.as_deref()), Some("CA003404"), "{query:?}");
    }
    let wrong = lookup(&RegistryQuery::Hgvs("NM_007294.4:c.5266A>T".into())).unwrap_err().to_string();
    assert!(wrong.contains("reference base"), "{wrong}");
}

/// Real expert-panel variants: the first allele of each panel in the dated ERepo
/// snapshot (up to 30 panels), so many genes and variant kinds are covered. Each
/// one must be found by its CA id, and the HGVS the registry gives back must be
/// read by the app's own parser and lead back to the same allele.
#[test]
#[ignore = "calls the live ClinGen Allele Registry about 60 times"]
fn live_expert_panel_variants_resolve_and_their_hgvs_reads_back_to_the_same_allele() {
    let snapshot = include_str!("../../../classivar-core/tests/data/erepo-2026-11-21.tsv");
    let mut panels: Vec<&str> = Vec::new();
    let mut alleles: Vec<&str> = Vec::new();
    for row in snapshot.lines().skip(1) {
        let mut cells = row.split('\t');
        let (Some(allele), Some(panel)) = (cells.next(), cells.next()) else { continue };
        if allele.starts_with("CA") && !panels.contains(&panel) && panels.len() < 30 {
            panels.push(panel);
            alleles.push(allele);
        }
    }
    let mut failures = Vec::new();
    for allele in &alleles {
        let found = lookup(&RegistryQuery::ClinGenAllele((*allele).into()));
        let Ok(Fetched { value: Lookup::Found(list), .. }) = found else {
            failures.push(format!("{allele}: not found by id ({found:?})"));
            continue;
        };
        let Some(first) = list.first() else {
            failures.push(format!("{allele}: found, but with no allele"));
            continue;
        };
        let Some(hgvs) = first.mane_select.clone().or(first.grch38.clone()) else {
            failures.push(format!("{allele}: no MANE or GRCh38 HGVS"));
            continue;
        };
        let query = classivar_core::variant::parse(&hgvs).map_err(|e| format!("{e:?}")).and_then(|notation| {
            notation.query(Some(classivar_core::variant::Build::GRCh38)).map_err(|e| format!("{e:?}"))
        });
        let back = query.map(|query| lookup(&query));
        match back {
            Ok(Ok(Fetched { value: Lookup::Found(again), .. }))
                if again.first().is_some_and(|a| a.allele_id.as_deref() == Some(*allele)) => {}
            other => failures.push(format!("{allele} via {hgvs}: {other:?}")),
        }
    }
    assert!(alleles.len() >= 20, "the snapshot covers at least 20 panels");
    assert!(failures.is_empty(), "{} of {} failed:\n{}", failures.len(), alleles.len(), failures.join("\n"));
}
