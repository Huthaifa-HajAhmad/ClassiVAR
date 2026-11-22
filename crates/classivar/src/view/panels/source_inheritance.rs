//! How the gene sources' own wording is read as a condition: PanelApp's mode of
//! inheritance and G2P's genotype. Anything unclear stays "unknown"; the curator
//! can change the inheritance after picking.

use classivar_core::{Condition, Inheritance, MAX_CONDITION_CHARS};

/// PanelApp's mode of inheritance, for example "MONOALLELIC, autosomal or pseudoautosomal, NOT imprinted".
pub(super) fn inheritance_from_panelapp(mode: &str) -> Inheritance {
    let mode = mode.trim().to_ascii_uppercase();
    if mode.starts_with("X-LINKED") {
        Inheritance::XLinked
    } else if mode.starts_with("MONOALLELIC") {
        Inheritance::AutosomalDominant
    } else if mode.starts_with("BIALLELIC") {
        Inheritance::AutosomalRecessive
    } else if mode.starts_with("MITOCHONDRIAL") {
        Inheritance::Mitochondrial
    } else {
        Inheritance::Unknown
    }
}

/// G2P's genotype, for example `monoallelic_autosomal` or `monoallelic_X_hem`.
pub(super) fn inheritance_from_g2p(genotype: &str) -> Inheritance {
    match genotype.trim().to_ascii_lowercase().as_str() {
        "monoallelic_autosomal" => Inheritance::AutosomalDominant,
        "biallelic_autosomal" => Inheritance::AutosomalRecessive,
        "mitochondrial" => Inheritance::Mitochondrial,
        g if g.contains("_x_") || g.ends_with("_x") => Inheritance::XLinked,
        _ => Inheritance::Unknown,
    }
}

/// A condition by name only, from a source that gives no MONDO id.
pub(super) fn named(name: &str, inheritance: Inheritance) -> Condition {
    Condition { name: name.trim().chars().take(MAX_CONDITION_CHARS).collect(), mondo: None, inheritance }
}

#[cfg(test)]
mod tests {
    use classivar_core::Inheritance;

    use super::{inheritance_from_g2p, inheritance_from_panelapp};

    #[test]
    fn panelapp_wording_is_read_as_an_inheritance_and_anything_unclear_stays_unknown() {
        let cases = [
            ("MONOALLELIC, autosomal or pseudoautosomal, NOT imprinted", Inheritance::AutosomalDominant),
            ("BIALLELIC, autosomal or pseudoautosomal", Inheritance::AutosomalRecessive),
            ("X-LINKED: hemizygous mutation in males, biallelic mutations in females", Inheritance::XLinked),
            ("MITOCHONDRIAL", Inheritance::Mitochondrial),
            ("BOTH monoallelic and biallelic, autosomal or pseudoautosomal", Inheritance::Unknown),
            ("", Inheritance::Unknown),
        ];
        for (mode, expected) in cases {
            assert_eq!(inheritance_from_panelapp(mode), expected, "{mode}");
        }
    }

    #[test]
    fn g2p_genotypes_are_read_as_an_inheritance_and_anything_unclear_stays_unknown() {
        assert_eq!(inheritance_from_g2p("monoallelic_autosomal"), Inheritance::AutosomalDominant);
        assert_eq!(inheritance_from_g2p("biallelic_autosomal"), Inheritance::AutosomalRecessive);
        assert_eq!(inheritance_from_g2p("monoallelic_X_hem"), Inheritance::XLinked);
        assert_eq!(inheritance_from_g2p("mitochondrial"), Inheritance::Mitochondrial);
        assert_eq!(inheritance_from_g2p("monoallelic_PAR"), Inheritance::Unknown);
        assert_eq!(inheritance_from_g2p("biallelic_PAR"), Inheritance::Unknown);
    }
}
