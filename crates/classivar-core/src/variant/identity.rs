//! A checked variant identity, as saved in a case file.

use serde::{Deserialize, Serialize};

/// The longest text in a saved identity, in characters. HGVS for a long
/// deletion-insertion can be long; anything past this is not from the registry.
pub const MAX_IDENTITY_CHARS: usize = 300;

/// The most RefSeq coding names a saved identity keeps.
pub const MAX_CODING_NAMES: usize = 4;

/// What the ClinGen Allele Registry said the variant is. Every field the
/// registry did not give is `None`; nothing here is filled in by guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantIdentity {
    /// The registry's canonical id, for example `CA003404`. None when the
    /// registry checked the variant but has not registered it.
    pub allele_id: Option<String>,
    /// Genomic HGVS on GRCh38, for example `NC_000017.11:g.43057065dup`.
    pub grch38: Option<String>,
    pub grch37: Option<String>,
    /// HGVS on the MANE Select transcript, for example `NM_007294.4:c.5266dup`.
    pub mane_select: Option<String>,
    /// When the registry names no MANE Select transcript: its RefSeq coding
    /// names (`NM_…:c.…`), one per transcript, at most `MAX_CODING_NAMES`.
    /// Empty when there is a MANE Select name. Files saved before this field
    /// existed open with it empty.
    #[serde(default)]
    pub coding: Vec<String>,
    /// The protein change on that transcript, as the registry writes it.
    pub protein: Option<String>,
    /// The gene of the MANE Select transcript.
    pub gene: Option<String>,
    pub clinvar_variation: Option<u64>,
    pub dbsnp: Option<u64>,
    /// The service that checked it, and the UTC date (`YYYY-MM-DD`).
    pub checked_with: String,
    pub checked_on: String,
}

impl VariantIdentity {
    /// The name to show: the MANE Select HGVS, else the first RefSeq coding name, else GRCh38, else GRCh37.
    pub fn display_name(&self) -> Option<&str> {
        self.mane_select
            .as_deref()
            .or(self.coding.first().map(String::as_str))
            .or(self.grch38.as_deref())
            .or(self.grch37.as_deref())
    }

    /// Does the identity list more RefSeq coding names than a saved identity keeps?
    pub fn too_many_coding(&self) -> bool {
        self.coding.len() > MAX_CODING_NAMES
    }

    /// The first text field over `MAX_IDENTITY_CHARS`, by name. A saved case
    /// with one is refused, as for every other field.
    pub fn too_long(&self) -> Option<&'static str> {
        let fields = [
            ("allele id", self.allele_id.as_deref()),
            ("GRCh38 HGVS", self.grch38.as_deref()),
            ("GRCh37 HGVS", self.grch37.as_deref()),
            ("MANE Select HGVS", self.mane_select.as_deref()),
            ("protein change", self.protein.as_deref()),
            ("gene", self.gene.as_deref()),
            ("checking service", Some(self.checked_with.as_str())),
            ("checking date", Some(self.checked_on.as_str())),
        ];
        let long_coding = self.coding.iter().any(|name| name.chars().count() > MAX_IDENTITY_CHARS);
        fields
            .into_iter()
            .find(|(_, text)| text.is_some_and(|t| t.chars().count() > MAX_IDENTITY_CHARS))
            .map(|(n, _)| n)
            .or(long_coding.then_some("coding name"))
    }
}
