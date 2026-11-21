//! Genome builds and the RefSeq chromosome accessions that name them.
//!
//! `NC_000017.10:g.41209082dup` (GRCh37) and `NC_000017.11:g.43057065dup`
//! (GRCh38) are the same variant: the accession's version encodes the build. A
//! bare position (`17-43057062-T-TG`) does not, so its build must be stated;
//! this module never guesses one.
//!
//! The table is ported from the Missense-Structural-Explorer project
//! (`src/shared/assembly.ts`), where all 50 accessions were checked against their
//! NCBI nuccore records. The mitochondrial accession (`NC_012920.1`, the rCRS)
//! is the same in both builds, so it says nothing about the build.

use serde::{Deserialize, Serialize};

/// A human reference genome build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Build {
    GRCh37,
    GRCh38,
}

impl Build {
    pub const ALL: [Build; 2] = [Build::GRCh38, Build::GRCh37];

    pub fn label(self) -> &'static str {
        match self {
            Build::GRCh37 => "GRCh37 (hg19)",
            Build::GRCh38 => "GRCh38 (hg38)",
        }
    }

    /// Read a build token as people write it: `GRCh38`, `GRCh38.p14`, `hg38`,
    /// `GRCh37`, `hg19`. Case does not matter. Anything else is not a build.
    pub fn from_token(token: &str) -> Option<Build> {
        let token = token.trim().to_ascii_lowercase();
        let release = token.split_once(".p").map_or(token.as_str(), |(major, patch)| {
            if !patch.is_empty() && patch.bytes().all(|b| b.is_ascii_digit()) { major } else { "" }
        });
        match release {
            "grch37" | "hg19" => Some(Build::GRCh37),
            "grch38" | "hg38" => Some(Build::GRCh38),
            _ => None,
        }
    }
}

const CHROMOSOMES: [&str; 25] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
    "21", "22", "X", "Y", "MT",
];

const GRCH37: [&str; 25] = [
    "NC_000001.10",
    "NC_000002.11",
    "NC_000003.11",
    "NC_000004.11",
    "NC_000005.9",
    "NC_000006.11",
    "NC_000007.13",
    "NC_000008.10",
    "NC_000009.11",
    "NC_000010.10",
    "NC_000011.9",
    "NC_000012.11",
    "NC_000013.10",
    "NC_000014.8",
    "NC_000015.9",
    "NC_000016.9",
    "NC_000017.10",
    "NC_000018.9",
    "NC_000019.9",
    "NC_000020.10",
    "NC_000021.8",
    "NC_000022.10",
    "NC_000023.10",
    "NC_000024.9",
    "NC_012920.1",
];

const GRCH38: [&str; 25] = [
    "NC_000001.11",
    "NC_000002.12",
    "NC_000003.12",
    "NC_000004.12",
    "NC_000005.10",
    "NC_000006.12",
    "NC_000007.14",
    "NC_000008.11",
    "NC_000009.12",
    "NC_000010.11",
    "NC_000011.10",
    "NC_000012.12",
    "NC_000013.11",
    "NC_000014.9",
    "NC_000015.10",
    "NC_000016.10",
    "NC_000017.11",
    "NC_000018.10",
    "NC_000019.10",
    "NC_000020.11",
    "NC_000021.9",
    "NC_000022.11",
    "NC_000023.11",
    "NC_000024.10",
    "NC_012920.1",
];

fn table(build: Build) -> &'static [&'static str; 25] {
    match build {
        Build::GRCh37 => &GRCH37,
        Build::GRCh38 => &GRCH38,
    }
}

/// The canonical chromosome name: `chr17` → `17`, `chrM` and `M` → `MT`.
/// None for anything that is not 1-22, X, Y or MT.
pub fn chromosome(token: &str) -> Option<&'static str> {
    let token = token.trim();
    let bare = token.strip_prefix("chr").or_else(|| token.strip_prefix("Chr")).or_else(|| token.strip_prefix("CHR"));
    let upper = bare.unwrap_or(token).to_ascii_uppercase();
    let name = if upper == "M" { "MT" } else { upper.as_str() };
    CHROMOSOMES.iter().find(|c| **c == name).copied()
}

/// The RefSeq accession of a chromosome in a build.
pub fn accession(build: Build, chromosome: &str) -> Option<&'static str> {
    CHROMOSOMES.iter().position(|c| *c == chromosome).map(|at| table(build)[at])
}

/// The build an accession names: `Some(build)` for a nuclear chromosome
/// accession, `None` for MT (it is in both builds) or for an accession this
/// table does not know (never guessed).
pub fn build_of_accession(accession: &str) -> Option<Build> {
    let upper = accession.trim().to_ascii_uppercase();
    if upper == "NC_012920.1" {
        return None;
    }
    Build::ALL.into_iter().find(|build| table(*build).contains(&upper.as_str()))
}

/// Is this a chromosome accession of either build (including MT)?
pub fn is_chromosome_accession(accession: &str) -> bool {
    let upper = accession.trim().to_ascii_uppercase();
    GRCH37.contains(&upper.as_str()) || GRCH38.contains(&upper.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tokens_follow_the_hg_and_grch_naming() {
        assert_eq!(Build::from_token("GRCh38"), Some(Build::GRCh38));
        assert_eq!(Build::from_token("grch38.p14"), Some(Build::GRCh38));
        assert_eq!(Build::from_token("hg38"), Some(Build::GRCh38));
        assert_eq!(Build::from_token("hg19"), Some(Build::GRCh37));
        assert_eq!(Build::from_token("GRCh37"), Some(Build::GRCh37));
        // hg19 is GRCh37: a naive "contains 38" or "37" test would get these wrong.
        assert_eq!(Build::from_token("hg37"), None);
        assert_eq!(Build::from_token("GRCh39"), None);
        assert_eq!(Build::from_token("GRCh37.9x"), None);
        assert_eq!(Build::from_token("GRCh38.p"), None);
    }

    #[test]
    fn the_accession_names_the_build_and_mt_names_none() {
        assert_eq!(build_of_accession("NC_000017.10"), Some(Build::GRCh37));
        assert_eq!(build_of_accession("nc_000017.11"), Some(Build::GRCh38));
        assert_eq!(build_of_accession("NC_012920.1"), None, "the rCRS is in both builds");
        assert_eq!(build_of_accession("NC_000017.12"), None, "an unknown version is never guessed");
        assert_eq!(accession(Build::GRCh38, "X"), Some("NC_000023.11"));
        assert_eq!(accession(Build::GRCh37, "MT"), Some("NC_012920.1"));
        assert_eq!(accession(Build::GRCh38, "23"), None);
    }

    #[test]
    fn chromosome_names_are_read_with_or_without_chr() {
        assert_eq!(chromosome("chr17"), Some("17"));
        assert_eq!(chromosome("Chr1"), Some("1"));
        assert_eq!(chromosome("x"), Some("X"));
        assert_eq!(chromosome("chrM"), Some("MT"));
        assert_eq!(chromosome("23"), None);
        assert_eq!(chromosome("chr0"), None);
    }

    #[test]
    fn both_tables_cover_every_chromosome_once() {
        for build in Build::ALL {
            let mut seen = std::collections::HashSet::new();
            for accession in table(build) {
                assert!(seen.insert(*accession), "{accession} twice");
                assert!(accession.starts_with("NC_0"), "{accession}");
            }
        }
    }
}
