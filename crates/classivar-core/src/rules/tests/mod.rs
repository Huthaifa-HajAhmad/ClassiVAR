//! Tests of the rule set, grouped by the table or the check they cover.

use super::*;
use crate::model::{Direction, Strength};

mod de_novo;
mod functional;
mod general;
mod locus;
mod predictors;
mod tables;

/// The built-in file with one piece of text replaced.
fn edited(old: &str, new: &str) -> Result<RuleSet, RuleError> {
    assert!(BUILTIN.contains(old), "the test text '{old}' is not in the rule file");
    RuleSet::parse(&BUILTIN.replacen(old, new, 1))
}

/// The built-in file with every line that starts with `prefix` removed.
fn without_lines(prefix: &str) -> Result<RuleSet, RuleError> {
    let kept: Vec<&str> = BUILTIN.lines().filter(|line| !line.trim_start().starts_with(prefix)).collect();
    assert!(kept.len() < BUILTIN.lines().count(), "no line starts with '{prefix}'");
    RuleSet::parse(&kept.join("\n"))
}

/// The built-in file with every line that starts with `prefix` replaced by `new_line`.
fn with_line_replaced(prefix: &str, new_line: &str) -> Result<RuleSet, RuleError> {
    let text: Vec<&str> = BUILTIN.lines().map(|line| if line.starts_with(prefix) { new_line } else { line }).collect();
    assert_ne!(text.join("\n"), BUILTIN.replace("\r\n", "\n"), "no line starts with '{prefix}'");
    RuleSet::parse(&text.join("\n"))
}
