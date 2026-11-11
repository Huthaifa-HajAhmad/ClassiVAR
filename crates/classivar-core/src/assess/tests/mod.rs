//! Tests of scoring, grouped by what they protect.

use super::*;
use crate::model::Strength::*;

mod classes;
mod safety;

fn rules() -> &'static RuleSet {
    RuleSet::builtin().expect("the built-in rules are valid")
}

fn applied(list: &[(&str, Strength)]) -> Vec<Applied> {
    list.iter()
        .map(|(code, strength)| match rules().criterion(code).and_then(|c| c.situation_for(*strength)) {
            Some(situation) => Applied::guided(*code, *strength, situation),
            None => Applied::new(*code, *strength),
        })
        .collect()
}

fn apply(list: &[(&str, Strength)]) -> Assessment {
    rules().assess(&applied(list)).expect("valid evidence")
}
