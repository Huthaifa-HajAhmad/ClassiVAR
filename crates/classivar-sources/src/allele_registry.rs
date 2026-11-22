//! The ClinGen Allele Registry: one service that reads HGVS (transcript or
//! genomic), finds alleles by ClinVar, dbSNP or CA id, checks the reference
//! bases, normalises the variant, and returns its canonical id with both builds,
//! the MANE Select transcript and the ClinVar and dbSNP links.
//!
//! Only the variant description is sent: never the case, its evidence or its labs.

use classivar_core::variant::{MAX_CODING_NAMES, RegistryQuery, VariantIdentity};
use serde::Deserialize;

use crate::{Fetched, SourceError};

const SOURCE: &str = "ClinGen Allele Registry";
const BASE: &str = "https://reg.clinicalgenome.org";
/// One allele record is about 30 KB; a dbSNP id can name a few alleles.
const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// What the registry knows about a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// One allele, or several when an id names several (a multi-allelic rsID).
    Found(Vec<VariantIdentity>),
    /// The registry says it has no such allele.
    NotFound,
}

/// Ask the registry. Blocks; call from a background thread.
pub fn lookup(query: &RegistryQuery) -> Result<Fetched<Lookup>, SourceError> {
    let (url, key, value) = match query {
        RegistryQuery::Hgvs(hgvs) => (format!("{BASE}/allele"), "hgvs", hgvs.clone()),
        RegistryQuery::ClinGenAllele(id) => (format!("{BASE}/allele/{id}"), "", String::new()),
        RegistryQuery::ClinVar(id) => (format!("{BASE}/alleles"), "ClinVar.variationId", id.to_string()),
        RegistryQuery::DbSnp(id) => (format!("{BASE}/alleles"), "dbSNP.rs", id.to_string()),
    };
    let query: Vec<(&str, &str)> = if key.is_empty() { vec![] } else { vec![(key, value.as_str())] };
    let reply = crate::get(&url, &query, "application/json", MAX_BYTES)?;
    let today = crate::utc_today();
    Ok(Fetched::now(SOURCE, read_reply(reply.status, &reply.body, &today)?))
}

#[derive(Deserialize)]
struct RawError {
    #[serde(rename = "errorType")]
    kind: Option<String>,
    description: Option<String>,
}

/// Read an answer. A 400 from the registry is a refusal with its own reason
/// (a wrong reference base, HGVS it cannot read), a 404 is "no such allele".
pub(crate) fn read_reply(status: u16, body: &str, today: &str) -> Result<Lookup, SourceError> {
    match status {
        200 => {
            let alleles: Vec<RawAllele> = if body.trim_start().starts_with('[') {
                serde_json::from_str(body)
            } else {
                serde_json::from_str(body).map(|one| vec![one])
            }
            .map_err(|e| SourceError::Unexpected(format!("not an allele record ({e})")))?;
            if alleles.is_empty() {
                return Ok(Lookup::NotFound);
            }
            Ok(Lookup::Found(alleles.into_iter().map(|raw| raw.identity(today)).collect()))
        }
        404 => Ok(Lookup::NotFound),
        400 => Err(refusal(body)),
        other => Err(SourceError::Status(other)),
    }
}

fn refusal(body: &str) -> SourceError {
    let raw: RawError = match serde_json::from_str(body) {
        Ok(raw) => raw,
        Err(_) => return SourceError::Status(400),
    };
    let reason = match raw.kind.as_deref() {
        Some("IncorrectReferenceAllele") => {
            "the reference base does not match the reference sequence at that position. Check the build, the position and the bases."
                .to_string()
        }
        Some("HgvsParsingError") => "it cannot read this HGVS. Check the spelling against the HGVS rules.".to_string(),
        _ => raw.description.unwrap_or_else(|| "it refused the request without a reason".to_string()),
    };
    SourceError::Refused(reason)
}

#[derive(Deserialize)]
struct RawAllele {
    #[serde(rename = "@id")]
    id: String,
    #[serde(default, rename = "genomicAlleles")]
    genomic: Vec<RawGenomic>,
    #[serde(default, rename = "transcriptAlleles")]
    transcripts: Vec<RawTranscript>,
    #[serde(default, rename = "externalRecords")]
    external: RawExternal,
}

#[derive(Deserialize)]
struct RawGenomic {
    #[serde(rename = "referenceGenome")]
    genome: Option<String>,
    #[serde(default)]
    hgvs: Vec<String>,
}

#[derive(Deserialize)]
struct RawTranscript {
    #[serde(default)]
    hgvs: Vec<String>,
    #[serde(rename = "geneSymbol")]
    gene: Option<String>,
    #[serde(rename = "proteinEffect")]
    protein: Option<RawProtein>,
    #[serde(rename = "MANE")]
    mane: Option<RawMane>,
}

#[derive(Deserialize)]
struct RawProtein {
    hgvs: Option<String>,
}

#[derive(Deserialize)]
struct RawMane {
    #[serde(rename = "maneStatus")]
    status: Option<String>,
}

#[derive(Default, Deserialize)]
struct RawExternal {
    #[serde(default, rename = "ClinVarVariations")]
    clinvar: Vec<RawClinVar>,
    #[serde(default, rename = "dbSNP")]
    dbsnp: Vec<RawDbSnp>,
}

#[derive(Deserialize)]
struct RawClinVar {
    #[serde(rename = "variationId")]
    id: u64,
}

#[derive(Deserialize)]
struct RawDbSnp {
    rs: u64,
}

impl RawAllele {
    /// The RefSeq coding names (`NM_`, not Ensembl, `NR_` or predicted `XM_`), one per
    /// transcript accession (its newest version), in accession order, a few at most.
    fn coding_names(&self) -> Vec<String> {
        let mut names: Vec<&String> =
            self.transcripts.iter().flat_map(|t| &t.hgvs).filter(|h| h.starts_with("NM_")).collect();
        let accession = |hgvs: &String| hgvs.split(['.', ':']).next().unwrap_or_default().to_string();
        let version = |hgvs: &String| {
            hgvs.split(':').next().and_then(|id| id.split('.').nth(1)).and_then(|v| v.parse::<u32>().ok())
        };
        names.sort_by(|a, b| accession(a).cmp(&accession(b)).then(version(b).cmp(&version(a))));
        names.dedup_by(|later, kept| accession(later) == accession(kept));
        names.into_iter().take(MAX_CODING_NAMES).cloned().collect()
    }

    fn identity(self, today: &str) -> VariantIdentity {
        // A registered allele is `…/allele/CA003404`; an unregistered one is `_:CA`.
        let allele_id =
            self.id.rsplit('/').next().filter(|id| id.starts_with("CA") && id.len() > 2).map(str::to_string);
        let genomic = |build: &str| {
            self.genomic
                .iter()
                .filter(|g| g.genome.as_deref() == Some(build))
                .find_map(|g| g.hgvs.iter().find(|h| h.starts_with("NC_")).cloned())
        };
        // MANE Select has an RefSeq (NM_) and an Ensembl entry; the RefSeq one is shown.
        let mane: Vec<&RawTranscript> = self
            .transcripts
            .iter()
            .filter(|t| t.mane.as_ref().and_then(|m| m.status.as_deref()) == Some("MANE Select"))
            .collect();
        let refseq = mane.iter().find(|t| t.hgvs.iter().any(|h| h.starts_with("NM_"))).or(mane.first());
        let coding = if mane.is_empty() { self.coding_names() } else { Vec::new() };
        let gene = refseq.and_then(|t| t.gene.clone()).or_else(|| {
            self.transcripts.iter().find(|t| t.hgvs.iter().any(|h| h.starts_with("NM_"))).and_then(|t| t.gene.clone())
        });
        VariantIdentity {
            allele_id,
            grch38: genomic("GRCh38"),
            grch37: genomic("GRCh37"),
            mane_select: refseq.and_then(|t| t.hgvs.iter().find(|h| h.starts_with("NM_")).or(t.hgvs.first()).cloned()),
            protein: refseq.and_then(|t| t.protein.as_ref()).and_then(|p| p.hgvs.clone()),
            gene,
            coding,
            clinvar_variation: self.external.clinvar.first().map(|c| c.id),
            dbsnp: self.external.dbsnp.first().map(|d| d.rs),
            checked_with: SOURCE.to_string(),
            checked_on: today.to_string(),
        }
    }
}

#[cfg(test)]
mod tests;
