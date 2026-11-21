//! The class of a variant in a gene whose link to the disease is not established.
//!
//! Richards 2015, "Evaluating and Reporting Variants in Genes of Uncertain
//! Significance": variants found in a gene of uncertain significance (GUS), "if
//! reported, should always be classified as 'Uncertain Significance'", because
//! the rules were built for genes with a validated link to the phenotype.
//!
//! This program applies that to a pathogenic result only: Pathogenic and Likely
//! pathogenic become VUS. A benign result is left alone (the source speaks about
//! candidate variants; a common or benign variant is not one). The points and
//! the pictures are not changed, so the curator still sees what the evidence adds up to.

use crate::model::Class;

use super::Assessment;

impl Assessment {
    /// The same result for a variant in a gene of uncertain significance: a
    /// shown Pathogenic or Likely pathogenic class becomes VUS, and the hints
    /// stop pointing at classes the gene link rules out.
    pub fn in_gene_of_uncertain_significance(mut self) -> Assessment {
        self.gene_of_uncertain_significance = true;
        // No pathogenic class is reachable until the link is established.
        if self.to_upgrade.is_some_and(|next| next.class > Class::Vus) {
            self.to_upgrade = None;
        }
        // From a lowered class, "the way down" from Likely pathogenic is not the way down from VUS.
        if self.points_class > Class::Vus {
            self.to_downgrade = None;
        }
        self
    }

    /// True when a gene of uncertain significance lowered the class shown.
    pub fn lowered_for_gene(&self) -> bool {
        self.gene_of_uncertain_significance && self.class_before_gene_limit().is_some_and(|class| class > Class::Vus)
    }
}
