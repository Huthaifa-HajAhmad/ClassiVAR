//! The ACMG/AMP 2015 combining table, read from the counted criteria.

use crate::model::{Class, Direction, Strength};

use super::Item;

/// The ACMG/AMP 2015 combining table (Richards et al., Table 5), plus one
/// addition: ClinGen's PM2 recommendation of 4 September 2020 lowered PM2 to
/// Supporting and, so that PVS1 + PM2 stays Likely pathogenic, added the rule
/// "one Very Strong + one Supporting = Likely pathogenic". That rule is the
/// `pvs >= 1 && pp >= 1` line below.
pub(super) fn classic_class(items: &[Item]) -> Option<Class> {
    // The 2015 table has no moderate or very strong benign rows.
    let uncovered = items.iter().any(|i| {
        i.counted && i.direction == Direction::Benign && matches!(i.strength, Strength::Moderate | Strength::VeryStrong)
    });
    if uncovered {
        return None;
    }
    // If both sides are met, the 2015 guidance says VUS.
    Some(match table_sides(items) {
        (Some(_), Some(_)) | (None, None) => Class::Vus,
        (Some(class), None) | (None, Some(class)) => class,
    })
}

/// What the rows of the 2015 table give on each side: (pathogenic side, benign side).
pub(super) fn table_sides(items: &[Item]) -> (Option<Class>, Option<Class>) {
    let counted = || items.iter().filter(|i| i.counted);
    let count = |direction: Direction, strength: Strength| {
        counted().filter(|i| i.direction == direction && i.strength == strength).count()
    };
    let (pvs, ps, pm, pp) = (
        count(Direction::Pathogenic, Strength::VeryStrong),
        count(Direction::Pathogenic, Strength::Strong),
        count(Direction::Pathogenic, Strength::Moderate),
        count(Direction::Pathogenic, Strength::Supporting),
    );
    let (ba, bs, bp) = (
        count(Direction::Benign, Strength::StandAlone),
        count(Direction::Benign, Strength::Strong),
        count(Direction::Benign, Strength::Supporting),
    );
    let pathogenic = (pvs >= 1 && (ps >= 1 || pm >= 2 || (pm >= 1 && pp >= 1) || pp >= 2))
        || ps >= 2
        || (ps >= 1 && (pm >= 3 || (pm >= 2 && pp >= 2) || (pm >= 1 && pp >= 4)));
    let likely_pathogenic = (pvs >= 1 && pm >= 1)
        || (pvs >= 1 && pp >= 1) // ClinGen 2020 addition, see above
        || (ps >= 1 && pm >= 1)
        || (ps >= 1 && pp >= 2)
        || pm >= 3
        || (pm >= 2 && pp >= 2)
        || (pm >= 1 && pp >= 4);
    let benign = ba >= 1 || bs >= 2;
    let likely_benign = (bs >= 1 && bp >= 1) || bp >= 2;

    let pathogenic_class = if pathogenic {
        Some(Class::Pathogenic)
    } else if likely_pathogenic {
        Some(Class::LikelyPathogenic)
    } else {
        None
    };
    let benign_class = if benign {
        Some(Class::Benign)
    } else if likely_benign {
        Some(Class::LikelyBenign)
    } else {
        None
    };

    (pathogenic_class, benign_class)
}
