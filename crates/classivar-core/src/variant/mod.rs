//! What variant a case is about: reading what the curator typed, the genome
//! build, the question to ask the ClinGen Allele Registry, and the checked
//! identity a case saves. Pure: the lookup itself is in `classivar-sources`.

pub mod accessions;
mod identity;
pub mod notation;
pub mod query;
#[cfg(test)]
mod tests;

pub use accessions::Build;
pub use identity::{MAX_IDENTITY_CHARS, VariantIdentity};
pub use notation::{Notation, NotationProblem, parse};
pub use query::{QueryProblem, RegistryQuery};
