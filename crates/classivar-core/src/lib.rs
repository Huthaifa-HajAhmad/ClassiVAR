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
pub use rules::{
    BA1_SOURCE, Ba1Exception, Criterion, Cutoff, DeNovoPhenotype, DeNovoReading, DeNovoRules, EvidenceReading,
    FunctionalRules, LocusBand, LocusChoice, LocusReading, LocusRules, NonSegregationRules, NonSegregationSituation,
    Pm3Reading, Pm3Row, Pm3Rules, Predictor, RuleError, RuleSet, RuleSetRef, SegregationKind, YieldStep,
};
