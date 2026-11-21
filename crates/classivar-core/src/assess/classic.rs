//! The ACMG/AMP 2015 combining table, read from the counted criteria. The rows
//! themselves live in `combining_table`.

use crate::combining_table::{self, TableRow};
use crate::model::Class;

/// What the 2015 table (see `combining_table`) gives for the evidence, given the
/// rows it meets. Every strength a criterion can hold has a row (the loader
/// refuses Very Strong benign), so the table always gives a class.
pub(super) fn classic_class(met: &[&TableRow]) -> Class {
    // If both sides are met, the 2015 guidance says VUS.
    match combining_table::sides(met) {
        (Some(_), Some(_)) | (None, None) => Class::Vus,
        (Some(class), None) | (None, Some(class)) => class,
    }
}
