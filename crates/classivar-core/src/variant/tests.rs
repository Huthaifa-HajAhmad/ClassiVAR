//! What the curator may type, what it becomes, and what is refused with a reason.
//! The BRCA1 example (c.5266dup, CA003404, VCV 17677, rs80357906) was checked
//! against the ClinGen Allele Registry on 2026-11-21.

use super::*;

fn query(text: &str, build: Option<Build>) -> Result<RegistryQuery, String> {
    let notation = parse(text).map_err(|p| p.message())?;
    notation.query(build).map_err(|p| p.message().to_string())
}

fn hgvs(text: &str, build: Option<Build>) -> String {
    match query(text, build) {
        Ok(RegistryQuery::Hgvs(hgvs)) => hgvs,
        other => panic!("{text}: {other:?}"),
    }
}

#[test]
fn transcript_hgvs_is_passed_on_as_typed() {
    assert_eq!(hgvs("NM_007294.4:c.5266dup", None), "NM_007294.4:c.5266dup");
    assert_eq!(hgvs("  ENST00000357654.9:c.5266dup ", None), "ENST00000357654.9:c.5266dup");
    assert_eq!(hgvs("NR_027676.2:n.100A>G", None), "NR_027676.2:n.100A>G");
    // HGVS's own brackets stay; only builds and ClinVar's gene label go.
    assert_eq!(hgvs("NM_000492.4:c.(53+1_54-1)_(164+1_165-1)del", None), "NM_000492.4:c.(53+1_54-1)_(164+1_165-1)del");
}

#[test]
fn clinvar_display_names_are_read() {
    // ClinVar writes the gene in brackets and the protein change after a space.
    assert_eq!(hgvs("NM_007294.4(BRCA1):c.5266dup (p.Gln1756fs)", None), "NM_007294.4:c.5266dup");
    assert_eq!(hgvs("NM_007294.4(GRCh38):c.5266dup", None), "NM_007294.4:c.5266dup");
}

#[test]
fn genomic_hgvs_needs_its_build_only_when_the_sequence_does_not_name_it() {
    assert_eq!(hgvs("NC_000017.11:g.43057065dup", None), "NC_000017.11:g.43057065dup");
    assert_eq!(hgvs("chr17:g.43057065dup", Some(Build::GRCh38)), "NC_000017.11:g.43057065dup");
    assert_eq!(hgvs("chr17(hg19):g.41209082dup", None), "NC_000017.10:g.41209082dup");
    assert!(parse("chr17:g.43057065dup").unwrap().needs_build());
    assert!(query("chr17:g.43057065dup", None).unwrap_err().contains("Which genome build"));
    // The mitochondrial sequence is the same in both builds.
    assert_eq!(hgvs("chrM:m.3243A>G", None), "NC_012920.1:m.3243A>G");
}

#[test]
fn a_description_that_names_two_builds_is_refused() {
    assert_eq!(parse("chr17(GRCh37)(GRCh38):g.1A>G"), Err(NotationProblem::TwoBuilds));
    // The accession says GRCh38 and the bracket says GRCh37.
    assert_eq!(parse("NC_000017.11(GRCh37):g.43057065dup"), Err(NotationProblem::TwoBuilds));
}

#[test]
fn positions_are_written_as_genomic_hgvs_on_the_chosen_build() {
    // The BRCA1 example as a VCF row: the T is the anchor base, G is inserted.
    let expected = "NC_000017.11:g.43057062_43057063insG";
    assert_eq!(hgvs("17-43057062-T-TG", Some(Build::GRCh38)), expected);
    assert_eq!(hgvs("chr17:43057062:T:TG", Some(Build::GRCh38)), expected);
    assert_eq!(hgvs("17 43057062 rs80357906 T TG", Some(Build::GRCh38)), expected);
    assert_eq!(hgvs("17:43057062 T>TG", Some(Build::GRCh38)), expected);
    assert_eq!(hgvs("chr7-140753336-A-T", Some(Build::GRCh38)), "NC_000007.14:g.140753336A>T");
    assert_eq!(hgvs("1-1000-ACT-A", Some(Build::GRCh37)), "NC_000001.10:g.1001_1002del");
    assert_eq!(hgvs("1-1000-A-G", Some(Build::GRCh37)), "NC_000001.10:g.1000A>G");
    assert_eq!(hgvs("1-1000-AC-GT", Some(Build::GRCh37)), "NC_000001.10:g.1000_1001delinsGT");
    // Never a guessed build.
    assert!(parse("17-43057062-T-TG").unwrap().needs_build());
    assert_eq!(parse("17-43057062-T-TG").unwrap().query(None), Err(QueryProblem::NeedsBuild));
}

#[test]
fn spdi_counts_from_zero_and_is_written_like_a_position() {
    assert_eq!(hgvs("NC_000017.11:43057062:T:TG", None), "NC_000017.11:g.43057063_43057064insG");
    assert_eq!(hgvs("NC_000007.14:140753335:A:T", None), "NC_000007.14:g.140753336A>T");
}

#[test]
fn ids_are_looked_up_by_id() {
    assert_eq!(query("VCV000017677", None), Ok(RegistryQuery::ClinVar(17677)));
    assert_eq!(query("vcv000017677.178", None), Ok(RegistryQuery::ClinVar(17677)));
    assert_eq!(query("rs80357906", None), Ok(RegistryQuery::DbSnp(80357906)));
    assert_eq!(query("ca003404", None), Ok(RegistryQuery::ClinGenAllele("CA003404".into())));
}

#[test]
fn each_refusal_says_what_to_type_instead() {
    assert_eq!(parse(""), Err(NotationProblem::Empty));
    assert_eq!(parse("c.5266dup"), Err(NotationProblem::MissingTranscript));
    assert_eq!(parse("BRCA1:c.5266dup"), Err(NotationProblem::GeneInsteadOfTranscript("BRCA1".into())));
    assert_eq!(parse("NM_007294:c.5266dup"), Err(NotationProblem::UnversionedTranscript("NM_007294".into())));
    assert_eq!(parse("NP_009225.1:c.5266dup"), Err(NotationProblem::ProteinAccession));
    assert_eq!(parse("C.5266dup"), Err(NotationProblem::PrefixCase));
    assert_eq!(parse("RCV000019246"), Err(NotationProblem::ClinVarRecord("RCV000019246".into())));
    for protein in ["p.Gln1756fs", "NP_009225.1:p.Gln1756fs", "R408W", "Arg408Trp"] {
        assert_eq!(parse(protein), Err(NotationProblem::ProteinOnly), "{protein}");
    }
    for nonsense in ["hello", "5382insC", "17-0-T-TG", "17-100-T-T", "23-100-A-G", "17-100-N-A"] {
        assert!(parse(nonsense).is_err(), "{nonsense}");
    }
    for problem in [NotationProblem::Unknown, NotationProblem::TwoBuilds, NotationProblem::ProteinOnly] {
        assert!(!problem.message().is_empty());
    }
}

#[test]
fn an_insertion_before_the_first_base_or_no_change_is_refused() {
    assert_eq!(query::genomic_hgvs("NC_000001.11", 1, "A", "A"), Err(QueryProblem::NoChange));
    assert_eq!(query::genomic_hgvs("NC_000001.11", 1, "", "G"), Err(QueryProblem::NoChange));
}

#[test]
fn the_display_name_prefers_the_mane_transcript() {
    let mut identity = VariantIdentity {
        allele_id: Some("CA003404".into()),
        grch38: Some("NC_000017.11:g.43057065dup".into()),
        grch37: None,
        mane_select: Some("NM_007294.4:c.5266dup".into()),
        protein: None,
        gene: Some("BRCA1".into()),
        clinvar_variation: Some(17677),
        dbsnp: Some(80357906),
        checked_with: "ClinGen Allele Registry".into(),
        checked_on: "2026-11-21".into(),
    };
    assert_eq!(identity.display_name(), Some("NM_007294.4:c.5266dup"));
    assert_eq!(identity.too_long(), None);
    identity.mane_select = None;
    assert_eq!(identity.display_name(), Some("NC_000017.11:g.43057065dup"));
    identity.gene = Some("A".repeat(MAX_IDENTITY_CHARS + 1));
    assert_eq!(identity.too_long(), Some("gene"));
}
