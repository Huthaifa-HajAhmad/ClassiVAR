//! Turning a description into a question the ClinGen Allele Registry answers.
//!
//! The registry reads HGVS on a RefSeq or Ensembl sequence, and finds alleles by
//! their ClinVar, dbSNP or CA id. A position (`17-43057062-T-TG`) and SPDI are
//! written as genomic HGVS on the build's chromosome accession first. The
//! registry then checks the reference bases and normalises the variant (for
//! example an insertion that repeats the base before it becomes a `dup`).

use super::accessions::{Build, accession};
use super::notation::Notation;

/// One lookup the registry can answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryQuery {
    Hgvs(String),
    ClinGenAllele(String),
    ClinVar(u64),
    DbSnp(u64),
}

/// Why a description cannot be looked up yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryProblem {
    /// A bare chromosome position: the curator must say which build.
    NeedsBuild,
    /// The alleles describe no change, or the change starts before the chromosome.
    NoChange,
}

impl QueryProblem {
    pub fn message(self) -> &'static str {
        match self {
            QueryProblem::NeedsBuild => {
                "Which genome build is this position on? The same number points to different bases in GRCh37 and GRCh38."
            }
            QueryProblem::NoChange => "These alleles describe no change that can be written in HGVS.",
        }
    }
}

impl Notation {
    /// Does this description need a build before it can be looked up?
    pub fn needs_build(&self) -> bool {
        match self {
            Notation::Hgvs { reference, build, .. } => is_bare_nuclear(reference) && build.is_none(),
            Notation::Position { chromosome, build, .. } => *chromosome != "MT" && build.is_none(),
            _ => false,
        }
    }

    /// The registry question for this description. `chosen` is the build the
    /// curator picked, used only where the description names none.
    pub fn query(&self, chosen: Option<Build>) -> Result<RegistryQuery, QueryProblem> {
        match self {
            Notation::Hgvs { reference, body, build } => {
                let sequence = sequence_for(reference, build.or(chosen))?;
                Ok(RegistryQuery::Hgvs(format!("{sequence}:{body}")))
            }
            Notation::Position { chromosome, position, reference, alternate, build } => {
                let sequence = sequence_for(chromosome, build.or(chosen))?;
                genomic_hgvs(sequence, *position, reference, alternate).map(RegistryQuery::Hgvs)
            }
            Notation::Spdi { accession, position, deleted, inserted } => {
                if deleted.bytes().all(|b| b.is_ascii_digit()) {
                    // SPDI with a deleted length: the registry's HGVS reader cannot take it as is.
                    return Err(QueryProblem::NoChange);
                }
                // SPDI counts from 0 between bases; HGVS and VCF count bases from 1.
                genomic_hgvs(accession, position + 1, deleted, inserted).map(RegistryQuery::Hgvs)
            }
            Notation::ClinVar(id) => Ok(RegistryQuery::ClinVar(*id)),
            Notation::DbSnp(id) => Ok(RegistryQuery::DbSnp(*id)),
            Notation::ClinGenAllele(id) => Ok(RegistryQuery::ClinGenAllele(id.clone())),
        }
    }
}

/// A bare nuclear chromosome name (`17`, `X`): it does not say its build. MT and
/// accessions do (MT is the same sequence in both builds).
fn is_bare_nuclear(reference: &str) -> bool {
    super::accessions::chromosome(reference).is_some_and(|c| c != "MT")
}

/// The sequence to write before `:g.`: an accession as given, or the
/// chromosome's accession in the build.
fn sequence_for(reference: &str, build: Option<Build>) -> Result<&str, QueryProblem> {
    match super::accessions::chromosome(reference) {
        Some("MT") => Ok("NC_012920.1"),
        Some(chromosome) => {
            accession(build.ok_or(QueryProblem::NeedsBuild)?, chromosome).ok_or(QueryProblem::NeedsBuild)
        }
        None => Ok(reference),
    }
}

/// Write a VCF-style change (1-based position, reference and alternate bases) as
/// genomic HGVS. The bases both alleles share at the start and end are dropped
/// first, as VCF's anchor base is not part of the change.
pub fn genomic_hgvs(sequence: &str, position: u64, reference: &str, alternate: &str) -> Result<String, QueryProblem> {
    let (r, a) = (reference.as_bytes(), alternate.as_bytes());
    let start_shared = r.iter().zip(a).take_while(|(x, y)| x == y).count();
    let (r, a) = (&r[start_shared..], &a[start_shared..]);
    let end_shared = r.iter().rev().zip(a.iter().rev()).take_while(|(x, y)| x == y).count();
    let (r, a) = (&r[..r.len() - end_shared], &a[..a.len() - end_shared]);
    let start = position + start_shared as u64;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    let span = |len: usize| {
        let end = start + len as u64 - 1;
        if len == 1 { start.to_string() } else { format!("{start}_{end}") }
    };
    let change = match (r.len(), a.len()) {
        (0, 0) => return Err(QueryProblem::NoChange),
        // An insertion lies between the base before `start` and `start`.
        (0, _) if start < 2 => return Err(QueryProblem::NoChange),
        (0, _) => format!("{}_{}ins{}", start - 1, start, text(a)),
        (1, 1) => format!("{start}{}>{}", text(r), text(a)),
        (len, 0) => format!("{}del", span(len)),
        (len, _) => format!("{}delins{}", span(len), text(a)),
    };
    Ok(format!("{sequence}:g.{change}"))
}
