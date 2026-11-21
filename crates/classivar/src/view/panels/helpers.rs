//! The small helpers that sit inside the criteria list: the predictor score
//! (PP3/BP4), the functional assay (PS3/BS3), family and phenotype points
//! (PP1/PP4), non-segregation (BS4), de novo patients (PS2 or PM6), recessive patients
//! (PM3), the dated BA1 exceptions reference, and the reason prompt for a raised
//! strength. Each one only fills in a criterion; the curator still decides.
//!
//! One file per helper. Its state lives in `view/helper_state.rs`.

use super::*;

mod de_novo;
mod functional;
mod locus;
mod locus_read;
mod pm3;
mod population;

pub(in crate::view) use population::POPULATION_BOXES;
mod predictor;
mod raise;
mod segregation;
