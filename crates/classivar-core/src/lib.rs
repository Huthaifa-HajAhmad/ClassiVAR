//! The rules and scoring behind the ClassiVAR.
//!
//! This crate has no screen code. It loads a rule file, adds up the evidence a
//! curator applied, and explains the result. Keeping it separate makes it easy
//! to test and lets other programs reuse it.

pub mod assess;
pub mod case;
pub mod model;
pub mod reevaluate;
pub mod rules;

pub use assess::{Applied, AssessError, Assessment, Conflict, Counting, Item, MAX_REASON_CHARS, NextClass, Warning};
pub use case::{CASE_FORMAT, CaseError, CaseFile, GeneDecision, SavedRecord};
pub use model::{Category, Class, Direction, Strength};
pub use reevaluate::{Consistency, Reason, Reevaluation, Rescored, Row, Submission, Verdict};
pub use rules::{Criterion, Cutoff, EvidenceReading, FunctionalRules, Predictor, RuleError, RuleSet, RuleSetRef};
