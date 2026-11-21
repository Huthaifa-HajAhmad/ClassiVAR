//! What kind of variant description the curator typed, read once, here.
//!
//! The grammar follows the Missense-Structural-Explorer project's
//! `variantNotation.ts` (build annotations, the refusal of two builds, delimited
//! and VCF rows, rsID and VCV, the cDNA reference checks), widened from missense
//! changes to every kind of variant ClassiVAR classifies. It only reads the
//! shape: whether the variant exists, and whether its reference base is right,
//! is the ClinGen Allele Registry's answer.

use super::accessions::{Build, build_of_accession, chromosome, is_chromosome_accession};

/// A variant description that can be looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notation {
    /// HGVS on a named sequence: `NM_007294.4:c.5266dup`, `NC_000017.11:g.43057065dup`,
    /// `chr17:g.43057065dup` (a bare chromosome then needs a build), `NC_012920.1:m.3243A>G`.
    Hgvs { reference: String, body: String, build: Option<Build> },
    /// A position and two alleles, as in a VCF row: `17-43057062-T-TG`. 1-based.
    Position { chromosome: &'static str, position: u64, reference: String, alternate: String, build: Option<Build> },
    /// NCBI SPDI: `NC_000017.11:43057062:T:TG`. The position is 0-based.
    Spdi { accession: String, position: u64, deleted: String, inserted: String },
    /// A ClinVar variation: `VCV000017677`.
    ClinVar(u64),
    /// A dbSNP id: `rs80357906`. It may name several alleles.
    DbSnp(u64),
    /// A ClinGen Allele Registry id: `CA003404`.
    ClinGenAllele(String),
}

/// Why a description cannot be looked up as it is. Each says what to type instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotationProblem {
    Empty,
    /// Two different builds in one description.
    TwoBuilds,
    /// `c.123A>G` with no transcript.
    MissingTranscript,
    /// `BRCA1:c.123A>G`: a gene symbol where the transcript belongs.
    GeneInsteadOfTranscript(String),
    /// `NM_007294:c.…`: the transcript has no version.
    UnversionedTranscript(String),
    /// A protein change alone: several DNA changes can give it.
    ProteinOnly,
    /// `NP_…:c.…`: a protein accession before a `c.` position.
    ProteinAccession,
    /// `C.123A>G`: HGVS type prefixes are lower case.
    PrefixCase,
    /// An RCV or SCV accession: ClinVar records, not the variant.
    ClinVarRecord(String),
    Unknown,
}

impl NotationProblem {
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "Type a variant.".into(),
            Self::TwoBuilds => "This names two genome builds. Keep one.".into(),
            Self::MissingTranscript => "Name the transcript before c., for example NM_007294.4:c.5266dup.".into(),
            Self::GeneInsteadOfTranscript(gene) => format!(
                "{gene} is a gene, not a transcript. A c. position means a different base on each transcript; name it, for example its MANE Select transcript NM_….N:c.…"
            ),
            Self::UnversionedTranscript(id) => {
                format!("Add the version to {id} (for example {id}.4): versions can number the bases differently.")
            }
            Self::ProteinOnly => "A protein change alone is not enough: several DNA changes give the same protein change. Type the DNA change (c. or g.), a position, or an id.".into(),
            Self::ProteinAccession => "NP_ is a protein; c. positions belong to a transcript such as NM_….".into(),
            Self::PrefixCase => "HGVS types are lower case: c., g., n., m.".into(),
            Self::ClinVarRecord(id) => format!(
                "{id} is one ClinVar record, not the variant. Use the variation (VCV…) it belongs to."
            ),
            Self::Unknown => "This is not a form ClassiVAR reads. Use HGVS (NM_…:c.…, NC_…:g.…), a position (17-43057062-T-TG), SPDI, a ClinVar VCV, an rsID or a ClinGen CA id.".into(),
        }
    }
}

/// Read a description. Spaces around it, a build in brackets (`(GRCh38)`), a
/// gene in brackets as ClinVar writes it (`NM_007294.4(BRCA1):c.…`) and a
/// protein change after it (`… (p.Gln1756fs)`) are allowed.
pub fn parse(input: &str) -> Result<Notation, NotationProblem> {
    let text = drop_trailing_protein(input.trim());
    if text.is_empty() {
        return Err(NotationProblem::Empty);
    }
    let (text, build) = take_brackets(text)?;
    if let Some(found) = identifier(&text) {
        return found;
    }
    if text.contains('.') && text.contains(':') || is_hgvs_body(&text) {
        return hgvs(&text, build);
    }
    if let Some(spdi) = spdi(&text) {
        return Ok(spdi);
    }
    position(&text, build)
        .ok_or_else(|| if looks_like_protein(&text) { NotationProblem::ProteinOnly } else { NotationProblem::Unknown })
}

/// `NM_007294.4(BRCA1):c.5266dup (p.Gln1756fs)` → without the protein part.
fn drop_trailing_protein(text: &str) -> &str {
    match text.split_once(char::is_whitespace) {
        Some((first, rest)) if rest.trim_start().starts_with("(p.") => first,
        _ => text,
    }
}

/// Remove bracketed builds and gene symbols, keeping HGVS's own brackets
/// (`c.(123+1_124-1)_(456+1_457-1)del`). Two builds are refused.
fn take_brackets(text: &str) -> Result<(String, Option<Build>), NotationProblem> {
    let mut out = String::with_capacity(text.len());
    let mut builds = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|c| open + c) else { break };
        let inside = &rest[open + 1..close];
        let before_colon = !rest[..open].contains(':') && rest[close..].starts_with("):");
        out.push_str(&rest[..open]);
        if let Some(build) = Build::from_token(inside) {
            builds.push(build);
        } else if !(before_colon && is_symbol(inside)) {
            out.push_str(&rest[open..=close]);
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    match builds.as_slice() {
        [] => Ok((out, None)),
        [one] => Ok((out, Some(*one))),
        _ => Err(NotationProblem::TwoBuilds),
    }
}

fn is_symbol(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 20
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && text.starts_with(|c: char| c.is_ascii_alphabetic())
}

fn digits(text: &str) -> Option<u64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())).then(|| text.parse().ok()).flatten()
}

/// rsID, VCV, RCV/SCV and CA ids, in any letter case.
///
/// A few gene symbols have the same shape (`RS1`, `CA2`) and are read as ids. This
/// is accepted on purpose: the lookup shows the variant the Registry returned,
/// and a gene that differs from the gene box is flagged, so nothing is saved under
/// a guess. Refusing the shape would also refuse a real `RS80357906`.
fn identifier(text: &str) -> Option<Result<Notation, NotationProblem>> {
    let upper = text.to_ascii_uppercase();
    if let Some(id) = upper.strip_prefix("RS").and_then(digits) {
        return Some(Ok(Notation::DbSnp(id)));
    }
    if let Some(rest) = upper.strip_prefix("VCV") {
        let number = rest.split_once('.').map_or(rest, |(n, v)| if digits(v).is_some() { n } else { "" });
        return digits(number).map(|id| Ok(Notation::ClinVar(id)));
    }
    for prefix in ["RCV", "SCV"] {
        if upper.strip_prefix(prefix).is_some_and(|rest| digits(rest.split('.').next().unwrap_or("")).is_some()) {
            return Some(Err(NotationProblem::ClinVarRecord(upper)));
        }
    }
    upper.strip_prefix("CA").and_then(digits).map(|_| Ok(Notation::ClinGenAllele(upper.clone())))
}

/// True for an HGVS body alone, with no sequence: `c.123A>G`, `C.123`.
fn is_hgvs_body(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["c.", "g.", "n.", "m.", "p.", "r."].iter().any(|t| lower.starts_with(t))
}

fn looks_like_protein(text: &str) -> bool {
    let body = text.split_once(':').map_or(text, |(_, b)| b);
    let body = body.strip_prefix("p.").unwrap_or(body);
    let mut chars = body.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && body.chars().any(|c| c.is_ascii_digit())
}

/// `REFERENCE:type.body`, or a body alone (which is refused with a reason).
fn hgvs(text: &str, build: Option<Build>) -> Result<Notation, NotationProblem> {
    let (reference, body) = match text.split_once(':') {
        Some((reference, body)) if is_hgvs_body(body) => (reference.trim(), body.trim()),
        Some(_) => return spdi(text).ok_or(NotationProblem::Unknown),
        None => ("", text),
    };
    let kind = &body[..2];
    if kind.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Err(NotationProblem::PrefixCase);
    }
    if kind == "p." {
        return Err(NotationProblem::ProteinOnly);
    }
    if kind == "r." || body.len() < 3 {
        return Err(NotationProblem::Unknown);
    }
    match kind {
        "c." | "n." => transcript_hgvs(reference, body, build),
        _ => genomic_hgvs(reference, body, build),
    }
}

fn transcript_hgvs(reference: &str, body: &str, build: Option<Build>) -> Result<Notation, NotationProblem> {
    let upper = reference.to_ascii_uppercase();
    if upper.is_empty() {
        return Err(NotationProblem::MissingTranscript);
    }
    if ["NP_", "XP_", "YP_", "ENSP"].iter().any(|p| upper.starts_with(p)) {
        return Err(NotationProblem::ProteinAccession);
    }
    let accession_like = upper.contains('_') || upper.starts_with("ENST") || upper.starts_with("LRG");
    if !accession_like && is_symbol(reference) {
        return Err(NotationProblem::GeneInsteadOfTranscript(reference.to_string()));
    }
    let versioned = upper.starts_with("LRG") || upper.split_once('.').is_some_and(|(_, v)| digits(v).is_some());
    if !versioned {
        return Err(NotationProblem::UnversionedTranscript(reference.to_string()));
    }
    Ok(Notation::Hgvs { reference: upper, body: body.to_string(), build })
}

fn genomic_hgvs(reference: &str, body: &str, build: Option<Build>) -> Result<Notation, NotationProblem> {
    if reference.is_empty() {
        return Err(NotationProblem::Unknown);
    }
    let upper = reference.to_ascii_uppercase();
    if let Some(chromosome) = chromosome(reference) {
        return Ok(Notation::Hgvs { reference: chromosome.to_string(), body: body.to_string(), build });
    }
    let stated = build_of_accession(&upper);
    let build = match (stated, build) {
        (Some(a), Some(b)) if a != b => return Err(NotationProblem::TwoBuilds),
        (stated, typed) => stated.or(typed),
    };
    Ok(Notation::Hgvs { reference: upper, body: body.to_string(), build })
}

/// `NC_000017.11:43057062:T:TG`. The deleted part may be a length (`:1:`); then
/// it is not checked here and the lookup reads it.
fn spdi(text: &str) -> Option<Notation> {
    let parts: Vec<&str> = text.split(':').collect();
    let [accession, position, deleted, inserted] = parts.as_slice() else { return None };
    if !is_chromosome_accession(accession) {
        return None;
    }
    let bases = |s: &str| s.bytes().all(|b| b"ACGTacgt".contains(&b));
    if !(bases(deleted) || digits(deleted).is_some()) || !bases(inserted) {
        return None;
    }
    Some(Notation::Spdi {
        accession: accession.to_ascii_uppercase(),
        position: digits(position)?,
        deleted: deleted.to_ascii_uppercase(),
        inserted: inserted.to_ascii_uppercase(),
    })
}

/// `17-43057062-T-TG`, `chr17:43057062:T:TG`, `17:43057062 T>TG`, `17 43057062 T TG`,
/// or a VCF row `17 43057062 rs80357906 T TG` (the third column is the VCF ID).
fn position(text: &str, build: Option<Build>) -> Option<Notation> {
    let separators = |c: char| c == '-' || c == ':' || c == '>' || c == '/' || c.is_whitespace();
    let tokens: Vec<&str> = text.split(separators).filter(|t| !t.is_empty()).collect();
    let (chrom, pos, reference, alternate) = match tokens.as_slice() {
        [c, p, r, a] | [c, p, _, r, a] => (*c, *p, *r, *a),
        _ => return None,
    };
    let bases = |s: &str| !s.is_empty() && s.bytes().all(|b| b"ACGTacgt".contains(&b));
    if !bases(reference) || !bases(alternate) || reference.eq_ignore_ascii_case(alternate) {
        return None;
    }
    Some(Notation::Position {
        chromosome: chromosome(chrom)?,
        position: digits(pos).filter(|p| *p >= 1)?,
        reference: reference.to_ascii_uppercase(),
        alternate: alternate.to_ascii_uppercase(),
        build,
    })
}
