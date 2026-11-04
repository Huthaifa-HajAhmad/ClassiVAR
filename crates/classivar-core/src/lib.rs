//! The rules and scoring behind the ClassiVAR.
//!
//! This crate has no screen code. It loads a rule file, adds up the evidence a
//! curator applied, and explains the result. Keeping it separate makes it easy
//! to test and lets other programs reuse it.

pub mod assess;
pub mod model;
pub mod reevaluate;
pub mod rules;

pub use assess::{Applied, AssessError, Assessment, Item, NextClass, Warning};
pub use model::{Category, Class, Direction, Strength};
pub use reevaluate::{Reason, Reevaluation, Rescored, Row, Submission, Verdict};
pub use rules::{Criterion, RuleError, RuleSet};
