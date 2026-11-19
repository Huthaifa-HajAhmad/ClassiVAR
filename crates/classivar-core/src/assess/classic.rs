//! The ACMG/AMP 2015 combining table, read from the counted criteria. The rows
//! themselves live in `combining_table`.

use crate::combining_table::{self, TableRow};
use crate::model::{Class, Direction, Strength};

use super::Item;

/// What the 2015 table (see `combining_table`) gives for the evidence, given the
/// rows it meets. None when the table has no row for a strength in use.
pub(super) fn classic_class(items: &[Item], met: &[&TableRow]) -> Option<Class> {
    // The 2015 table has no moderate or very strong benign rows.
    let uncovered = items.iter().any(|i| {
        i.counted && i.direction == Direction::Benign && matches!(i.strength, Strength::Moderate | Strength::VeryStrong)
    });
    if uncovered {
        return None;
    }
    // If both sides are met, the 2015 guidance says VUS.
    Some(match combining_table::sides(met) {
        (Some(_), Some(_)) | (None, None) => Class::Vus,
        (Some(class), None) | (None, Some(class)) => class,
    })
}
