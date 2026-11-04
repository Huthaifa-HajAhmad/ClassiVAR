//! Gene symbols offered as you type in the gene box.
//!
//! This is a typing convenience only. The list is a set of common clinical
//! genetics genes in HGNC style, not a curated or validated set, and it never
//! decides anything: a gene that is missing here can still be checked against
//! the sources. The screen says so next to the suggestions.

/// Common clinical gene symbols, HGNC style. Not exhaustive.
///
/// Keep every entry an upper-case HGNC symbol. Order does not matter: matching
/// sorts its answers, and a test rejects duplicates and non-symbol entries.
const SYMBOLS: &[&str] = &[
    "ABCA4", "ACADS", "ACADVL", "ACTA2", "ACTC1", "ACVRL1", "ADA", "AIP", "ALDOB", "ALK", "APC", "APOB", "APP", "AR",
    "ARID1B", "ASXL1", "ATM", "ATP7B", "ATXN1", "ATXN2", "ATXN3", "BAG3", "BARD1", "BBS1", "BBS10", "BEST1", "BMPR1A",
    "BRAF", "BRCA1", "BRCA2", "BRIP1", "BTK", "C9orf72", "CACNA1C", "CACNA1S", "CALM1", "CALM2", "CALM3", "CAPN3",
    "CASQ2", "CDH1", "CDH23", "CDKL5", "CEP290", "CFTR", "CHD2", "CHD7", "CHEK2", "CHM", "COL1A1", "COL1A2", "COL3A1",
    "COL4A3", "COL4A4", "COL4A5", "COL5A1", "COL5A2", "CPT2", "CRB1", "CREBBP", "CYLD", "DEPDC5", "DES", "DICER1",
    "DMD", "DSC2", "DSG2", "DSP", "DYSF", "EGFR", "EMD", "ENG", "EP300", "F11", "F2", "F5", "F8", "F9", "FBN1", "FBN2",
    "FGFR1", "FGFR2", "FGFR3", "FH", "FLCN", "FLNC", "FMR1", "FOXG1", "FUS", "G6PD", "GAA", "GABRA1", "GABRG2",
    "GATA4", "GBA1", "GJB2", "GLA", "GLB1", "GRN", "HBA1", "HBA2", "HBB", "HFE", "HEXA", "HRAS", "HTT", "IDS", "IKBKG",
    "JAG1", "KCNE1", "KCNE2", "KCNH2", "KCNJ2", "KCNQ1", "KCNT1", "KRAS", "LDLR", "LDLRAP1", "LMNA", "LZTR1", "MAPT",
    "MAX", "MECP2", "MEN1", "MET", "MLH1", "MSH2", "MSH6", "MUTYH", "MYBPC3", "MYH11", "MYH7", "MYL2", "MYL3",
    "MYO15A", "MYO7A", "NBN", "NF1", "NF2", "NKX2-5", "NPC1", "NPC2", "NRAS", "OTC", "OTOF", "PAH", "PALB2", "PCSK9",
    "PKLR", "PKP2", "PMS2", "POLD1", "POLE", "POT1", "PRKAG2", "PRPH2", "PRSS1", "PSEN1", "PSEN2", "PTEN", "PTPN11",
    "RAD51C", "RAD51D", "RAF1", "RB1", "RECQL4", "RET", "RPE65", "RPGR", "RS1", "RYR1", "RYR2", "SCN1A", "SCN2A",
    "SCN5A", "SCN8A", "SDHA", "SDHAF2", "SDHB", "SDHC", "SDHD", "SERPINA1", "SHANK3", "SHOC2", "SLC12A3", "SLC22A5",
    "SLC25A13", "SLC26A4", "SMAD3", "SMAD4", "SMN1", "SMPD1", "SOD1", "SOS1", "SPINK1", "STK11", "TAZ", "TERT",
    "TGFBR1", "TGFBR2", "TMEM127", "TMEM43", "TMC1", "TNNI3", "TNNT2", "TP53", "TPM1", "TSC1", "TSC2", "TTN", "TTR",
    "USH2A", "VHL", "VWF", "WAS", "WT1", "WRN",
];

/// Symbols to offer for what is in the gene box: those that start with the typed
/// text, ignoring case, minus an exact match (there is nothing left to fill in).
/// Results are sorted and capped at `limit`, so the row is stable as you type.
pub fn suggestions_for(typed: &str, limit: usize) -> Vec<&'static str> {
    let typed = typed.trim().to_ascii_uppercase();
    if typed.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut found: Vec<&'static str> = SYMBOLS
        .iter()
        .copied()
        .filter(|symbol| {
            let upper = symbol.to_ascii_uppercase();
            upper.starts_with(&typed) && upper != typed
        })
        .collect();
    found.sort_unstable();
    found.truncate(limit);
    found
}

/// Is this something worth sending to the gene sources? Letters, digits and
/// hyphens, up to 20 characters. The same shape the source crate accepts.
pub fn is_gene_symbol(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && text.len() <= 20 && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::{SYMBOLS, is_gene_symbol, suggestions_for};
    use std::collections::HashSet;

    #[test]
    fn only_symbol_shaped_text_is_checked() {
        assert!(is_gene_symbol("BRCA1"));
        assert!(is_gene_symbol("NKX2-5"));
        assert!(is_gene_symbol(" c9orf72 "));
        assert!(!is_gene_symbol(""));
        assert!(!is_gene_symbol("   "));
        assert!(!is_gene_symbol("BRCA1; DROP"));
        assert!(!is_gene_symbol("a b"));
        assert!(!is_gene_symbol(&"A".repeat(21)));
    }

    #[test]
    fn the_list_holds_gene_symbols_without_duplicates() {
        let mut seen = HashSet::new();
        for symbol in SYMBOLS {
            assert!(!symbol.is_empty());
            // HGNC symbols are upper case apart from the "orf" genes, e.g. C9orf72.
            let symbol_shape = symbol
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-' || c == 'o' || c == 'r' || c == 'f');
            assert!(symbol_shape, "{symbol} is not a gene symbol (only the orf genes may mix case)");
            assert!(
                symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "{symbol} is not a gene-symbol shape"
            );
            assert!(seen.insert(symbol.to_ascii_uppercase()), "{symbol} appears twice");
        }
    }

    #[test]
    fn matching_ignores_case_and_offers_the_full_symbols() {
        assert_eq!(suggestions_for("brc", 6), vec!["BRCA1", "BRCA2"]);
        assert_eq!(suggestions_for("bRcA", 6), vec!["BRCA1", "BRCA2"]);
        assert_eq!(suggestions_for("SCN", 3), vec!["SCN1A", "SCN2A", "SCN5A"]);
        assert_eq!(suggestions_for("c9o", 6), vec!["C9orf72"]);
    }

    #[test]
    fn an_exact_symbol_or_nothing_typed_offers_nothing() {
        assert!(suggestions_for("BRCA1", 6).is_empty());
        assert!(suggestions_for("", 6).is_empty());
        assert!(suggestions_for("   ", 6).is_empty());
        assert!(suggestions_for("NOTAGENE", 6).is_empty());
        assert!(suggestions_for("BRC", 0).is_empty());
    }

    #[test]
    fn the_answer_is_capped() {
        // One letter matches many symbols; the cap keeps the row readable.
        assert_eq!(suggestions_for("S", 4).len(), 4);
        let all = suggestions_for("S", SYMBOLS.len());
        assert!(all.len() > 4, "the test needs more matches than the cap");
        assert!(all.len() <= SYMBOLS.len());
    }
}
