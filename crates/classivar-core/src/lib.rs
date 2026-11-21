//! The rules and scoring behind the ClassiVAR.
//!
//! This crate has no screen code. It loads a rule file, adds up the evidence a
//! curator applied, and explains the result. Keeping it separate makes it easy
//! to test and lets other programs reuse it.

pub mod assess;
pub mod benchmark;
pub mod case;
pub mod combining_table;
pub mod condition;
pub mod date;
pub mod model;
pub mod reevaluate;
pub mod rules;
pub mod variant;

pub use assess::{Applied, AssessError, Assessment, Conflict, Counting, Item, MAX_REASON_CHARS, NextClass, Warning};
pub use case::{
    CASE_FORMAT, CaseError, CaseFile, GeneDecision, MAX_LAB_CHARS, MAX_RECORDS, MAX_SOURCE_CHARS, MAX_VARIANT_CHARS,
    MAX_YEAR, MIN_YEAR, RecordSource, SavedRecord,
};
pub use combining_table::{Need, ROWS as TABLE_ROWS, TableRow};
pub use condition::{Condition, ConditionMatch, Inheritance, MAX_CONDITION_CHARS};
pub use date::is_calendar_date;
pub use model::{Category, Class, Direction, Strength};
pub use reevaluate::{Consistency, Reason, Reevaluation, Rescored, Row, Submission, Verdict};
pub use rules::{
    AssayValidation, BA1_SOURCE, Ba1Exception, Criterion, Cutoff, DeNovoPhenotype, DeNovoReading, DeNovoRules,
    DiseaseArchitecture, EvidenceReading, FunctionalRules, HELPER_CODES, LocusBand, LocusChoice, LocusReading,
    LocusRules, NonSegregationRules, NonSegregationSituation, Pm3Reading, Pm3Row, Pm3Rules, PopulationFacts,
    PopulationFinding, PopulationRules, Predictor, RuleError, RuleSet, RuleSetRef, SegregationKind, YieldStep,
    max_credible_frequency,
};
