//! The rule set: loaded from a TOML data file and checked once, at load time.
//! After that the rest of the program can trust it.
//!
//! A `RuleSet` is checked when it is made through `RuleSet::parse`,
//! `RuleSet::builtin`, or serde (which runs the same checks), so a bad rule file
//! does not reach the scoring code. The fields stay public, so code that changes
//! them afterwards (some tests do) skips the checks.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::model::{Category, Class, Direction, Strength};

/// The built-in rule set, embedded in the program.
const BUILTIN: &str = include_str!("../../../rules/acmg-amp-points.toml");

/// Point values and class limits above this are almost certainly typing mistakes.
/// The limit also keeps every sum far away from the edge of `i32`.
const MAX_POINTS: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Unverified,
    Reviewed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: ReviewStatus,
    pub note: String,
    /// Required when `status` is `reviewed`: who checked the file.
    pub reviewed_by: Option<String>,
    /// Required when `status` is `reviewed`: the date of the review, `YYYY-MM-DD`.
    pub reviewed_on: Option<String>,
}

/// Names one rule set. Saved results and records carry this, so a result can be
/// matched to the rules that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSetRef {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub id: String,
    pub citation: String,
    pub doi: Option<String>,
    pub url: Option<String>,
}

/// Points per strength (always positive; the sign comes from the direction).
#[derive(Debug, Clone, Deserialize)]
pub struct Points {
    pub supporting: i32,
    pub moderate: i32,
    pub strong: i32,
    pub very_strong: i32,
    pub stand_alone: i32,
}

/// The lowest total that still gives each class. Below `likely_benign` is Benign.
#[derive(Debug, Clone, Deserialize)]
pub struct ClassMinPoints {
    pub pathogenic: i32,
    pub likely_pathogenic: i32,
    pub vus: i32,
    pub likely_benign: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VusTier {
    pub name: String,
    pub description: String,
    pub source: String,
    /// Lowest total for this tier. Empty until official ranges are published.
    pub min_points: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExclusivePair {
    pub a: String,
    pub b: String,
    pub reason: String,
    pub source: String,
}

/// A limit on two criteria used together. ClinGen's 2024 phenotype and
/// co-segregation guidance caps the pair at 5 points, so two Strong answers are
/// not allowed even though each is allowed alone.
#[derive(Debug, Clone, Deserialize)]
pub struct CombinationCap {
    pub a: String,
    pub b: String,
    /// The most the two may add up to, in this program's points.
    pub max_points: i32,
    pub reason: String,
    pub source: String,
}

/// One published score limit of a prediction tool. For a pathogenic limit a score
/// at or above `at` reaches `strength`; for a benign limit, at or below `at`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Cutoff {
    pub strength: Strength,
    pub at: f64,
}

/// A prediction tool whose scores were calibrated to evidence strengths for PP3
/// (pathogenic) and BP4 (benign). The numbers come from the cited source only.
#[derive(Debug, Clone, Deserialize)]
pub struct Predictor {
    pub id: String,
    pub name: String,
    pub source: String,
    /// The paper and table the limits come from, as it is written in a saved reason.
    /// A file without it is refused by the loader with a clear message, not by serde.
    #[serde(default)]
    pub reference: String,
    /// What to know before using the tool, for example which version was calibrated.
    pub note: Option<String>,
    /// The evidence strength the source calibrated its limits to, when it advises
    /// applying the codes at a lower strength (Walker 2023). `None` when the two agree.
    #[serde(default)]
    pub calibrated: Option<Strength>,
    /// The score range the tool reports. Set both or neither; a score outside it
    /// gives no evidence. `None` when the rule file does not state a range.
    #[serde(default)]
    pub min_score: Option<f64>,
    #[serde(default)]
    pub max_score: Option<f64>,
    /// A condition the curator must confirm before a score is applied, in the
    /// source's words. `None` when the tool has no extra condition.
    #[serde(default)]
    pub eligibility: Option<String>,
    /// Weakest first. Each stronger limit lies further from the middle of the scale.
    pub pathogenic: Vec<Cutoff>,
    pub benign: Vec<Cutoff>,
}

/// How strong functional-assay evidence is (Brnich 2019). Without a statistical
/// analysis, the number of validation controls decides: at most `Moderate`. With
/// one, the OddsPath value decides, and its limits are crossed strictly.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionalRules {
    pub source: String,
    /// An assay needs at least this many validation controls to give any evidence.
    /// Brnich 2019's Supporting level assumes experimental controls, so 0 gives none.
    pub controls_supporting_min: u32,
    /// An assay with at least this many validation controls (a mix of pathogenic
    /// and benign variants) reaches Moderate; one with fewer (but at least
    /// `controls_supporting_min`) reaches Supporting.
    pub controls_moderate_min: u32,
    /// OddsPath strictly above `at` reaches the strength (PS3). Weakest first.
    pub pathogenic: Vec<Cutoff>,
    /// OddsPath strictly below `at` reaches the strength (BS3). Weakest first.
    pub benign: Vec<Cutoff>,
}

/// One row of the PP4 table: a diagnostic yield (in percent) and the points it earns.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct YieldStep {
    pub at: f64,
    pub points: f64,
}

/// One kind of co-segregation evidence for PP1, and what each informative
/// family member is worth.
#[derive(Debug, Clone, Deserialize)]
pub struct SegregationKind {
    /// Stable id, for the program to refer to the kind.
    pub id: String,
    pub label: String,
    pub points_each: f64,
    /// Who may be counted, in the paper's words.
    pub note: Option<String>,
}

/// One strength choice the paper allows for a number of points. A criterion left
/// out of the choice is not part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct LocusChoice {
    pub pp1: Option<Strength>,
    pub pp4: Option<Strength>,
}

/// The choices the paper allows once the points reach `min_points` (and until the next band starts).
#[derive(Debug, Clone, Deserialize)]
pub struct LocusBand {
    pub min_points: f64,
    pub options: Vec<LocusChoice>,
}

/// Co-segregation (PP1) and phenotype specificity (PP4) in points, ClinGen 2024.
/// The paper limits the two together to `cap_points`.
#[derive(Debug, Clone, Deserialize)]
pub struct LocusRules {
    pub source: String,
    pub cap_points: f64,
    /// The paper's text asks for a yield over this (percent); its table starts lower.
    /// The screen warns for a yield below it.
    pub yield_text_minimum: f64,
    /// What the paper says when only one gene can cause the phenotype: PP1 does not count.
    pub homogeneity_text: String,
    /// Cautions for PP4 from the paper, shown with the helper.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Weakest first. A yield at or above `at` earns the points; between two rows it rounds down.
    pub pp4_yield: Vec<YieldStep>,
    pub segregation: Vec<SegregationKind>,
    /// Lowest points first.
    pub bands: Vec<LocusBand>,
}

/// What the PP1 and PP4 points add up to and which strengths the paper allows for them.
#[derive(Debug, Clone, PartialEq)]
pub struct LocusReading {
    /// The sum of the PP1 and PP4 points, before the cap.
    pub sum: f64,
    /// The sum after the cap.
    pub total: f64,
    /// True when the sum was above the cap and was cut down to it.
    pub capped: bool,
    /// Empty when the points are below the lowest band: the paper gives no evidence.
    pub choices: Vec<LocusChoice>,
}

/// One situation in which non-segregation is, or is not, evidence against a variant (BS4).
#[derive(Debug, Clone, Deserialize)]
pub struct NonSegregationSituation {
    /// Stable id, for the program to refer to the situation.
    pub id: String,
    /// A few words for the button.
    pub label: String,
    /// True when the source supports BS4 in this situation.
    pub applies: bool,
    /// What the source says about this situation, in plain words.
    pub text: String,
}

/// When BS4 applies (ClinGen 2024). Where it does, it is applied at `strength`.
#[derive(Debug, Clone, Deserialize)]
pub struct NonSegregationRules {
    pub source: String,
    pub strength: Strength,
    pub situations: Vec<NonSegregationSituation>,
}

/// One row of the de novo points table (ClinGen SVI v1.1, Table 1): how well a
/// patient's phenotype fits the gene, and the points one such patient earns.
#[derive(Debug, Clone, Deserialize)]
pub struct DeNovoPhenotype {
    /// Stable id, for the program to refer to the row.
    pub id: String,
    pub label: String,
    /// Points for a patient whose parental relationships were confirmed.
    pub confirmed: f64,
    /// Points for a patient whose parental relationships were not confirmed.
    pub unconfirmed: f64,
    /// The most this row may add to the total, when the paper limits it.
    pub max_points: Option<f64>,
}

/// De novo evidence for PS2 and PM6 (ClinGen SVI v1.1): points per patient, and
/// the strength each total reaches.
#[derive(Debug, Clone, Deserialize)]
pub struct DeNovoRules {
    pub source: String,
    /// Said on screen when the recessive rule is asked about.
    pub recessive_text: String,
    pub notes: Vec<String>,
    /// Weakest first. A total at or above `at` reaches `strength` (Table 2).
    pub thresholds: Vec<Cutoff>,
    pub phenotypes: Vec<DeNovoPhenotype>,
}

/// What a list of de novo patients adds up to.
#[derive(Debug, Clone, PartialEq)]
pub struct DeNovoReading {
    /// The total points, after the limit of any row that has one.
    pub points: f64,
    /// `None` when the points are below the lowest threshold, or the recessive
    /// rule lowered the strength below the lowest level.
    pub evidence: Option<EvidenceReading>,
    /// The same level read as PM6, which Table 2 allows instead of PS2. Lowered to
    /// PM6's own limit, so it can be weaker than `evidence`.
    pub pm6: Option<EvidenceReading>,
    /// True when the recessive rule lowered the strength by one level.
    pub lowered_for_recessive: bool,
}

/// One patient group in the PM3 table. Counts must not overlap.
#[derive(Debug, Clone, Deserialize)]
pub struct Pm3Row {
    pub id: String,
    pub label: String,
    pub points: f64,
    pub max_points: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pm3Rules {
    pub source: String,
    pub notes: Vec<String>,
    pub rows: Vec<Pm3Row>,
    pub thresholds: Vec<Cutoff>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pm3Reading {
    pub points: f64,
    pub evidence: Option<EvidenceReading>,
}

/// The source every rule file with BA1 exception entries must define: ClinGen's
/// published list of common pathogenic-variant exceptions (July 2018).
pub const BA1_SOURCE: &str = "clingen-ba1-2018";

/// The criteria this program's helpers fill in from published tables. The
/// helpers cannot answer a strength guide (that is what the PVS1-style guide
/// screen is for), so a guide on one of these would produce evidence the
/// engine refuses; the loader rejects it instead.
pub const HELPER_CODES: [&str; 10] = ["PP3", "BP4", "PS3", "BS3", "PP1", "PP4", "BS4", "PS2", "PM6", "PM3"];

/// A dated reference entry, not an automatic variant match.
#[derive(Debug, Clone, Deserialize)]
pub struct Ba1Exception {
    pub gene: String,
    pub variant: String,
    pub clinvar_id: String,
    pub allele_id: String,
}

/// Points come in steps of 0.1 or more; rounding removes sums such as 2.9999999999999996.
fn round_points(points: f64) -> f64 {
    (points * 100.0).round() / 100.0
}

/// What a score or an assay result means for one criterion. Between the two
/// sets of limits there is no reading, so the evidence is neither side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReading {
    /// The criterion the evidence supports, for example `PP3`, `BP4`, `PS3`, `BS3`.
    pub code: &'static str,
    /// The strength the source assigns to the code for this score. When the source
    /// calibrated the evidence to a different strength, `Predictor::calibrated` holds it.
    pub published: Strength,
    /// The strength this program lets the curator apply: `published`, lowered to
    /// the criterion's limit when the source goes higher.
    pub strength: Strength,
}

/// One situation that sets a criterion's strength.
#[derive(Debug, Clone, Deserialize)]
pub struct StrengthGuide {
    /// Stable id saved with the case. Keep it unchanged when the wording changes.
    pub id: String,
    /// A few words for the collapsed row and the accessible name.
    pub short: String,
    pub situation: String,
    /// None means the criterion does not apply in this situation.
    pub strength: Option<Strength>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Criterion {
    pub code: String,
    pub direction: Direction,
    /// Default strength. The user can change it per case.
    pub strength: Strength,
    /// The highest strength a curator may choose. Empty means no limit beyond
    /// "very strong" (and "stand-alone" for BA1).
    pub max_strength: Option<Strength>,
    pub category: Category,
    pub title: String,
    pub summary: String,
    pub source: String,
    /// Set only when a later recommendation changed the strength. This is the
    /// strength in the 2015 paper, so the app can explain old classifications.
    pub original_strength: Option<Strength>,
    /// The source of that strength change. Falls back to `source` when empty.
    pub strength_source: Option<String>,
    #[serde(default)]
    pub discouraged: bool,
    pub note: Option<String>,
    /// When not empty, the curator answers this instead of getting the default strength.
    #[serde(default)]
    pub guide: Vec<StrengthGuide>,
    /// The source of the guide. Required when there is a guide.
    pub guide_source: Option<String>,
}

impl Criterion {
    /// The strengths a curator may choose for this criterion, weakest first.
    /// "Stand-alone" is only offered to a criterion whose default is stand-alone (BA1).
    pub fn allowed_strengths(&self) -> Vec<Strength> {
        let mut list = self.recorded_strengths();
        if let Some(limit) = self.max_strength
            && let Some(at) = list.iter().position(|s| *s == limit)
        {
            list.truncate(at + 1);
        }
        list
    }

    /// Every strength an earlier record could have used: the same as
    /// `allowed_strengths`, but without today's `max_strength` limit. Old records
    /// were made before the limit existed, and they must still be readable.
    ///
    /// BA1 is the exception: stand-alone is its only level. A record cannot
    /// legitimately hold BA1 at a weaker strength, so offering one would let a
    /// common variant be turned into a Likely benign or VUS by a single click.
    pub fn recorded_strengths(&self) -> Vec<Strength> {
        if self.direction == Direction::Benign {
            // The 2015 table has no Very Strong benign row. BA1 is stand-alone only.
            return if self.strength == Strength::StandAlone {
                vec![Strength::StandAlone]
            } else {
                vec![Strength::Supporting, Strength::Moderate, Strength::Strong]
            };
        }
        vec![Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong]
    }

    /// The first guide situation that sets this strength, if any. Returns its id.
    pub fn situation_for(&self, strength: Strength) -> Option<&str> {
        self.guide.iter().find(|g| g.strength == Some(strength)).map(|g| g.id.as_str())
    }

    /// Like `situation_for`, but only when exactly one guide answer sets this
    /// strength. Reconstructing a saved answer needs a unique answer: picking
    /// the first of several would invent a reason the save never gave.
    pub fn unique_situation_for(&self, strength: Strength) -> Option<&str> {
        let mut matches = self.guide.iter().filter(|g| g.strength == Some(strength));
        let first = matches.next()?;
        matches.next().is_none().then_some(first.id.as_str())
    }

    /// The guide option with this id, for display and validation.
    pub fn guide_option(&self, id: &str) -> Option<&StrengthGuide> {
        self.guide.iter().find(|g| g.id == id)
    }

    /// The source that explains the current default strength.
    pub fn strength_source_id(&self) -> &str {
        self.strength_source.as_deref().unwrap_or(&self.source)
    }
}

/// The rule file as written. It becomes a `RuleSet` only after `RuleSet::check` passes.
#[derive(Deserialize)]
struct RuleSetFile {
    meta: Meta,
    sources: Vec<Source>,
    points: Points,
    class_min_points: ClassMinPoints,
    exclusive_pairs: Vec<ExclusivePair>,
    #[serde(default)]
    combination_caps: Vec<CombinationCap>,
    #[serde(default)]
    predictors: Vec<Predictor>,
    functional: Option<FunctionalRules>,
    locus_evidence: Option<LocusRules>,
    non_segregation: Option<NonSegregationRules>,
    de_novo: Option<DeNovoRules>,
    pm3: Option<Pm3Rules>,
    // A file may have no tier table at all (the engine treats that as "no tiers").
    #[serde(default)]
    vus_tiers: Vec<VusTier>,
    #[serde(default)]
    ba1_exceptions: Vec<Ba1Exception>,
    criteria: Vec<Criterion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "RuleSetFile")]
pub struct RuleSet {
    pub meta: Meta,
    pub sources: Vec<Source>,
    pub points: Points,
    pub class_min_points: ClassMinPoints,
    pub vus_tiers: Vec<VusTier>,
    pub exclusive_pairs: Vec<ExclusivePair>,
    /// Pairs whose combined points are limited (empty in older rule files).
    pub combination_caps: Vec<CombinationCap>,
    /// Calibrated prediction tools for PP3 and BP4 (empty in older rule files).
    pub predictors: Vec<Predictor>,
    /// Functional-assay evidence limits for PS3 and BS3 (absent in older rule files).
    pub functional: Option<FunctionalRules>,
    /// PP1 and PP4 points and the strengths the paper allows (absent in older rule files).
    pub locus_evidence: Option<LocusRules>,
    /// The situations in which BS4 applies (absent in older rule files).
    pub non_segregation: Option<NonSegregationRules>,
    /// De novo points for PS2 (absent in older rule files).
    pub de_novo: Option<DeNovoRules>,
    pub pm3: Option<Pm3Rules>,
    pub ba1_exceptions: Vec<Ba1Exception>,
    pub criteria: Vec<Criterion>,
}

impl TryFrom<RuleSetFile> for RuleSet {
    type Error = RuleError;

    fn try_from(file: RuleSetFile) -> Result<Self, RuleError> {
        let rules = RuleSet {
            meta: file.meta,
            sources: file.sources,
            points: file.points,
            class_min_points: file.class_min_points,
            vus_tiers: file.vus_tiers,
            exclusive_pairs: file.exclusive_pairs,
            combination_caps: file.combination_caps,
            predictors: file.predictors,
            functional: file.functional,
            locus_evidence: file.locus_evidence,
            non_segregation: file.non_segregation,
            de_novo: file.de_novo,
            pm3: file.pm3,
            ba1_exceptions: file.ba1_exceptions,
            criteria: file.criteria,
        };
        rules.check()?;
        Ok(rules)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("could not read the rule file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("criterion {0} appears twice")]
    DuplicateCode(String),
    #[error("{item} cites source '{source_id}', which is not listed under [[sources]]")]
    UnknownSource { item: String, source_id: String },
    #[error("exclusive pair {a}/{b} names a criterion that does not exist")]
    UnknownPairCode { a: String, b: String },
    #[error("combination cap {a}/{b}: {problem}")]
    BadCombinationCap { a: String, b: String, problem: String },
    #[error("criterion {0}: a strength-guide situation is empty or appears twice")]
    BadSituation(String),
    #[error("criterion {0}: its strength guide offers no answer that applies")]
    GuideWithoutAnswer(String),
    #[error("criterion {0}: guide_source is set but the criterion has no guide")]
    GuideSourceWithoutGuide(String),
    #[error("source {0} appears twice under [[sources]]")]
    DuplicateSource(String),
    #[error(
        "criterion {0} has a strength guide, but the helper that sets {0} cannot answer a guide; remove the guide from {0}"
    )]
    GuideOnHelperCriterion(String),
    #[error("criterion {0}: original_strength must be a recorded strength different from today's default")]
    BadOriginalStrength(String),
    #[error("exclusive pair {0}: the two criteria must differ and the pair must appear once")]
    BadExclusivePair(String),
    #[error(
        "class thresholds must fall in strict order (pathogenic > likely_pathogenic > vus > likely_benign) and vus must be 0"
    )]
    BadThresholds,
    #[error(
        "point values must satisfy 0 < supporting < moderate < strong < very_strong, and stand_alone must be enough on its own for Benign (more than the likely_benign limit); all at most {MAX_POINTS}"
    )]
    BadPoints,
    #[error("criterion {code} has strength stand_alone but is not a benign criterion")]
    StandAloneNotBenign { code: String },
    #[error("criterion {code}: its default strength is not among the strengths a curator may choose")]
    DefaultNotAllowed { code: String },
    #[error("criterion {code}: its strength guide has no source, or offers a strength the criterion does not allow")]
    BadGuide { code: String },
    #[error("criterion {code}: max_strength is not a strength a record can use")]
    MaxStrengthNotRecorded { code: String },
    #[error("the rule file has no criteria, or has criteria for only one direction")]
    MissingCriteria,
    #[error("the rule set is marked reviewed but has no reviewer name or no review date in the form YYYY-MM-DD")]
    ReviewIncomplete,
    #[error("functional evidence limits: {0}")]
    BadFunctional(String),
    #[error("co-segregation and phenotype limits: {0}")]
    BadLocusEvidence(String),
    #[error("non-segregation situations: {0}")]
    BadNonSegregation(String),
    #[error("de novo points: {0}")]
    BadDeNovo(String),
    #[error("PM3 points: {0}")]
    BadPm3(String),
    #[error("BA1 reference: {0}")]
    BadBa1(String),
    #[error("predictor {id}: {problem}")]
    BadPredictor { id: String, problem: String },
    #[error("VUS tiers: {0}")]
    BadTiers(String),
}

impl RuleSet {
    /// Read and check a rule file.
    pub fn parse(text: &str) -> Result<RuleSet, RuleError> {
        let file: RuleSetFile = toml::from_str(text)?;
        RuleSet::try_from(file)
    }

    /// The rule set embedded in the program. A test proves it is valid; the
    /// program also checks it once at start-up and shows the error if it is not.
    pub fn builtin() -> Result<&'static RuleSet, &'static RuleError> {
        static RULES: OnceLock<Result<RuleSet, RuleError>> = OnceLock::new();
        RULES.get_or_init(|| RuleSet::parse(BUILTIN)).as_ref()
    }

    /// The id and version of this rule set.
    pub fn reference(&self) -> RuleSetRef {
        RuleSetRef { id: self.meta.id.clone(), version: self.meta.version.clone() }
    }

    pub fn criterion(&self, code: &str) -> Option<&Criterion> {
        self.criteria.iter().find(|c| c.code == code)
    }

    pub fn source(&self, id: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn predictor(&self, id: &str) -> Option<&Predictor> {
        self.predictors.iter().find(|p| p.id == id)
    }

    /// What a tool's score means for the evidence, or `None` when the score lies
    /// between the limits, is outside the tool's stated range, is not a finite
    /// number, or the tool id is unknown.
    pub fn read_predictor(&self, id: &str, score: f64) -> Option<EvidenceReading> {
        let tool = self.predictor(id)?;
        if !score.is_finite() {
            return None;
        }
        if let (Some(min), Some(max)) = (tool.min_score, tool.max_score)
            && !(min..=max).contains(&score)
        {
            return None;
        }
        // Limits run weakest first, so the last one a score reaches is the strongest.
        if let Some(cut) = tool.pathogenic.iter().rfind(|cut| score >= cut.at) {
            self.reading("PP3", cut.strength)
        } else {
            let cut = tool.benign.iter().rfind(|cut| score <= cut.at)?;
            self.reading("BP4", cut.strength)
        }
    }

    /// The strength of functional evidence from the number of validation controls
    /// alone (no statistical analysis). `damaging` says whether the assay showed
    /// damage (PS3) or no damage (BS3). `None` when the rules have no functional table
    /// or the assay has too few controls to give any evidence.
    pub fn read_functional_controls(&self, damaging: bool, controls: u32) -> Option<EvidenceReading> {
        let rules = self.functional.as_ref()?;
        if controls < rules.controls_supporting_min {
            return None;
        }
        let strength = if controls >= rules.controls_moderate_min { Strength::Moderate } else { Strength::Supporting };
        self.reading(if damaging { "PS3" } else { "BS3" }, strength)
    }

    /// The strength of functional evidence from an OddsPath value. `None` when the
    /// value lies in the indeterminate range, is negative or is not a finite number.
    pub fn read_oddspath(&self, value: f64) -> Option<EvidenceReading> {
        let rules = self.functional.as_ref()?;
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        if let Some(cut) = rules.pathogenic.iter().rfind(|cut| value > cut.at) {
            self.reading("PS3", cut.strength)
        } else {
            let cut = rules.benign.iter().rfind(|cut| value < cut.at)?;
            self.reading("BS3", cut.strength)
        }
    }

    /// The PP4 points a diagnostic yield (percent) earns. When `variants_on_allele`
    /// variants share one allele, the paper divides the yield (a probability) among
    /// them before it reads the table; points must not be divided. `None` when the
    /// rules have no table, the yield is not between 0 and 100, there are no
    /// variants, or the (divided) yield is below the table's first row.
    pub fn pp4_points(&self, diagnostic_yield: f64, variants_on_allele: u32) -> Option<f64> {
        let rules = self.locus_evidence.as_ref()?;
        if !(0.0..=100.0).contains(&diagnostic_yield) || variants_on_allele == 0 {
            return None;
        }
        let shared = diagnostic_yield / f64::from(variants_on_allele);
        rules.pp4_yield.iter().rfind(|step| shared >= step.at).map(|step| step.points)
    }

    /// The PP1 points for `count` informative co-segregations of one kind. `None`
    /// when the rules have no table or the kind id is unknown.
    pub fn pp1_points(&self, kind_id: &str, count: u32) -> Option<f64> {
        let kind = self.locus_evidence.as_ref()?.segregation.iter().find(|kind| kind.id == kind_id)?;
        Some(round_points(kind.points_each * f64::from(count)))
    }

    /// Adds the PP1 and PP4 points, applies the limit for the two together and
    /// lists the strength choices the paper allows. `None` when the rules have no
    /// table or a point value is negative or not a number.
    ///
    /// With `pp1_allowed` false (only one gene can cause the phenotype) only PP4
    /// choices are listed: the highest band that has a PP4-only choice, at or
    /// below the total. Table 4 says "maximum allowable", so a lower band's choice
    /// is also allowed.
    pub fn read_locus(&self, pp1_points: f64, pp4_points: f64, pp1_allowed: bool) -> Option<LocusReading> {
        let rules = self.locus_evidence.as_ref()?;
        if [pp1_points, pp4_points].iter().any(|points| !points.is_finite() || *points < 0.0) {
            return None;
        }
        let sum = round_points(pp1_points + pp4_points);
        let total = sum.min(rules.cap_points);
        let choices = rules
            .bands
            .iter()
            .rev()
            .filter(|band| total >= band.min_points)
            .map(|band| {
                band.options.iter().filter(|choice| pp1_allowed || choice.pp1.is_none()).copied().collect::<Vec<_>>()
            })
            .find(|options| !options.is_empty())
            .unwrap_or_default();
        Some(LocusReading { sum, total, capped: sum > rules.cap_points, choices })
    }

    /// The BS4 evidence for a situation. `None` when the rules have no such table,
    /// the id is unknown, or the source says BS4 does not apply in that situation.
    pub fn read_non_segregation(&self, situation_id: &str) -> Option<EvidenceReading> {
        let rules = self.non_segregation.as_ref()?;
        let situation = rules.situations.iter().find(|s| s.id == situation_id)?;
        if !situation.applies {
            return None;
        }
        self.reading("BS4", rules.strength)
    }

    /// Adds the de novo points of the patients and reads the total as a PS2
    /// strength and as a PM6 strength. `counts` has one `(confirmed, unconfirmed)`
    /// pair of patient numbers for each row of the table, in the table's order.
    /// With `recessive_without_second_variant` true the strength is lowered one
    /// level. `None` when the rules have no table or `counts` does not match its rows.
    pub fn read_de_novo(&self, counts: &[(u32, u32)], recessive_without_second_variant: bool) -> Option<DeNovoReading> {
        let rules = self.de_novo.as_ref()?;
        if counts.len() != rules.phenotypes.len() {
            return None;
        }
        let sum: f64 = rules
            .phenotypes
            .iter()
            .zip(counts)
            .map(|(row, (confirmed, unconfirmed))| {
                let points = row.confirmed * f64::from(*confirmed) + row.unconfirmed * f64::from(*unconfirmed);
                row.max_points.map_or(points, |max| points.min(max))
            })
            .sum();
        let points = round_points(sum);
        // Thresholds run weakest first; the last one the total reaches is the strength.
        let reached = rules.thresholds.iter().rposition(|cut| points >= cut.at);
        let level = if recessive_without_second_variant { reached.and_then(|i| i.checked_sub(1)) } else { reached };
        let strength = level.map(|i| rules.thresholds[i].strength);
        Some(DeNovoReading {
            points,
            evidence: strength.and_then(|s| self.reading("PS2", s)),
            pm6: strength.and_then(|s| self.reading("PM6", s)),
            lowered_for_recessive: recessive_without_second_variant && reached.is_some(),
        })
    }

    /// Adds PM3 patient counts after the curator has checked the source's
    /// eligibility conditions. Counts follow the rule table's row order.
    pub fn read_pm3(&self, counts: &[u32]) -> Option<Pm3Reading> {
        let rules = self.pm3.as_ref()?;
        if counts.len() != rules.rows.len() {
            return None;
        }
        let points = round_points(
            rules
                .rows
                .iter()
                .zip(counts)
                .map(|(row, count)| {
                    let sum = row.points * f64::from(*count);
                    row.max_points.map_or(sum, |max| sum.min(max))
                })
                .sum(),
        );
        let evidence = rules
            .thresholds
            .iter()
            .rev()
            .find(|cut| points >= cut.at)
            .and_then(|cut| self.reading("PM3", cut.strength));
        Some(Pm3Reading { points, evidence })
    }

    /// A published strength for `code`, lowered to the highest this program offers
    /// for that criterion when the source goes higher.
    fn reading(&self, code: &'static str, published: Strength) -> Option<EvidenceReading> {
        let allowed = self.criterion(code)?.allowed_strengths();
        let strength = if allowed.contains(&published) { published } else { *allowed.last()? };
        Some(EvidenceReading { code, published, strength })
    }

    /// Points (always positive) for a strength.
    pub fn magnitude(&self, strength: Strength) -> i32 {
        match strength {
            Strength::Supporting => self.points.supporting,
            Strength::Moderate => self.points.moderate,
            Strength::Strong => self.points.strong,
            Strength::VeryStrong => self.points.very_strong,
            Strength::StandAlone => self.points.stand_alone,
        }
    }

    /// Lowest total for a class. Benign has no lower limit.
    pub fn class_min(&self, class: Class) -> Option<i32> {
        match class {
            Class::Pathogenic => Some(self.class_min_points.pathogenic),
            Class::LikelyPathogenic => Some(self.class_min_points.likely_pathogenic),
            Class::Vus => Some(self.class_min_points.vus),
            Class::LikelyBenign => Some(self.class_min_points.likely_benign),
            Class::Benign => None,
        }
    }

    pub fn class_for_points(&self, total: i32) -> Class {
        let t = &self.class_min_points;
        if total >= t.pathogenic {
            Class::Pathogenic
        } else if total >= t.likely_pathogenic {
            Class::LikelyPathogenic
        } else if total >= t.vus {
            Class::Vus
        } else if total >= t.likely_benign {
            Class::LikelyBenign
        } else {
            Class::Benign
        }
    }

    fn check(&self) -> Result<(), RuleError> {
        let t = &self.class_min_points;
        let in_range = |v: i32| (-MAX_POINTS..=MAX_POINTS).contains(&v);
        let thresholds_ok = t.pathogenic > t.likely_pathogenic
            && t.likely_pathogenic > t.vus
            && t.vus > t.likely_benign
            && t.vus == 0
            && [t.pathogenic, t.likely_pathogenic, t.vus, t.likely_benign].into_iter().all(in_range);
        if !thresholds_ok {
            return Err(RuleError::BadThresholds);
        }
        let p = &self.points;
        let points_ok = 0 < p.supporting
            && p.supporting < p.moderate
            && p.moderate < p.strong
            && p.strong < p.very_strong
            && p.very_strong <= MAX_POINTS
            // BA1 alone must reach Benign: its negative points fall below the likely_benign limit.
            && -p.stand_alone < t.likely_benign
            && p.stand_alone <= MAX_POINTS;
        if !points_ok {
            return Err(RuleError::BadPoints);
        }

        if self.meta.status == ReviewStatus::Reviewed {
            let filled = |v: &Option<String>| v.as_deref().is_some_and(|s| !s.trim().is_empty());
            // The date must look like YYYY-MM-DD with a possible month and day.
            let is_date = |s: &str| {
                s.len() == 10
                    && s.char_indices().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
                    && (1..=12).contains(&s[5..7].parse::<u32>().unwrap_or(0))
                    && (1..=31).contains(&s[8..10].parse::<u32>().unwrap_or(0))
            };
            if !filled(&self.meta.reviewed_by) || !self.meta.reviewed_on.as_deref().is_some_and(is_date) {
                return Err(RuleError::ReviewIncomplete);
            }
        }

        let known_sources: HashSet<&str> = self.sources.iter().map(|s| s.id.as_str()).collect();
        // A repeated source id would make `source()` ambiguous, so the file
        // says which one it means instead.
        let mut source_ids = HashSet::new();
        for source in &self.sources {
            if source.id.trim().is_empty() || !source_ids.insert(source.id.as_str()) {
                return Err(RuleError::DuplicateSource(source.id.clone()));
            }
        }
        let needs_source = |item: String, source_id: &str| {
            if known_sources.contains(source_id) {
                Ok(())
            } else {
                Err(RuleError::UnknownSource { item, source_id: source_id.to_string() })
            }
        };

        let has_direction = |d: Direction| self.criteria.iter().any(|c| c.direction == d);
        if !has_direction(Direction::Pathogenic) || !has_direction(Direction::Benign) {
            return Err(RuleError::MissingCriteria);
        }

        let mut seen = HashSet::new();
        for c in &self.criteria {
            if !seen.insert(c.code.as_str()) {
                return Err(RuleError::DuplicateCode(c.code.clone()));
            }
            needs_source(c.code.clone(), &c.source)?;
            if let Some(source) = &c.strength_source {
                needs_source(format!("{} (strength)", c.code), source)?;
            }
            if c.strength == Strength::StandAlone && c.direction != Direction::Benign {
                return Err(RuleError::StandAloneNotBenign { code: c.code.clone() });
            }
            if !c.allowed_strengths().contains(&c.strength) {
                return Err(RuleError::DefaultNotAllowed { code: c.code.clone() });
            }
            // A cap the loader cannot place would be dropped without a word, and the
            // curator would be offered strengths the rule file meant to forbid.
            if let Some(limit) = c.max_strength
                && !c.recorded_strengths().contains(&limit)
            {
                return Err(RuleError::MaxStrengthNotRecorded { code: c.code.clone() });
            }
            if !c.guide.is_empty() {
                let source_ok = c.guide_source.as_deref().is_some_and(|id| known_sources.contains(id));
                let allowed = c.allowed_strengths();
                let strengths_ok = c.guide.iter().all(|g| g.strength.is_none_or(|s| allowed.contains(&s)));
                if !source_ok || !strengths_ok {
                    return Err(RuleError::BadGuide { code: c.code.clone() });
                }
                let mut situations = HashSet::new();
                let mut ids = HashSet::new();
                for guide in &c.guide {
                    let shaped = !guide.id.trim().is_empty()
                        && !guide.short.trim().is_empty()
                        && !guide.situation.trim().is_empty();
                    if !shaped || !ids.insert(guide.id.as_str()) || !situations.insert(guide.situation.as_str()) {
                        return Err(RuleError::BadSituation(c.code.clone()));
                    }
                }
                if c.guide.iter().all(|guide| guide.strength.is_none()) {
                    return Err(RuleError::GuideWithoutAnswer(c.code.clone()));
                }
            } else if c.guide_source.is_some() {
                return Err(RuleError::GuideSourceWithoutGuide(c.code.clone()));
            }
            // The helpers set their criteria from tables and cannot answer a
            // guide, so a guide on one of them could never be scored.
            if !c.guide.is_empty() && HELPER_CODES.contains(&c.code.as_str()) {
                return Err(RuleError::GuideOnHelperCriterion(c.code.clone()));
            }
            if let Some(original) = c.original_strength
                && (original == c.strength || !c.recorded_strengths().contains(&original))
            {
                return Err(RuleError::BadOriginalStrength(c.code.clone()));
            }
        }
        let mut pairs_seen = HashSet::new();
        for pair in &self.exclusive_pairs {
            let key =
                if pair.a <= pair.b { (pair.a.as_str(), pair.b.as_str()) } else { (pair.b.as_str(), pair.a.as_str()) };
            if pair.a == pair.b || !pairs_seen.insert(key) {
                return Err(RuleError::BadExclusivePair(format!("{}/{}", pair.a, pair.b)));
            }
            needs_source(format!("{}/{}", pair.a, pair.b), &pair.source)?;
            if !seen.contains(pair.a.as_str()) || !seen.contains(pair.b.as_str()) {
                return Err(RuleError::UnknownPairCode { a: pair.a.clone(), b: pair.b.clone() });
            }
        }

        for cap in &self.combination_caps {
            needs_source(format!("{}/{}", cap.a, cap.b), &cap.source)?;
            let problem = if cap.a == cap.b {
                Some("names the same criterion twice".to_string())
            } else {
                match (self.criterion(&cap.a), self.criterion(&cap.b)) {
                    (Some(a), Some(b)) => {
                        let defaults = self.magnitude(a.strength) + self.magnitude(b.strength);
                        (cap.max_points <= 0
                            || cap.max_points > MAX_POINTS
                            || defaults > cap.max_points)
                            .then(|| {
                                format!(
                                    "max_points must be at least the default pair total ({defaults}) and at most {MAX_POINTS}"
                                )
                            })
                    }
                    _ => Some("names a criterion that does not exist".to_string()),
                }
            };
            if let Some(problem) = problem {
                return Err(RuleError::BadCombinationCap { a: cap.a.clone(), b: cap.b.clone(), problem });
            }
        }

        self.check_predictors(&known_sources)?;
        self.check_functional(&known_sources)?;
        self.check_locus(&known_sources)?;
        self.check_non_segregation(&known_sources)?;
        self.check_de_novo(&known_sources)?;
        self.check_pm3(&known_sources)?;
        self.check_ba1(&known_sources)?;

        for tier in &self.vus_tiers {
            needs_source(tier.name.clone(), &tier.source)?;
        }
        self.check_tiers()
    }

    /// The functional table needs a known source, the criteria PS3 and BS3, and
    /// limits that run away from the indeterminate middle without overlapping.
    fn check_functional(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.functional else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadFunctional(problem.to_string()));
        if !known_sources.contains(rules.source.as_str()) {
            return Err(RuleError::UnknownSource {
                item: "functional evidence".to_string(),
                source_id: rules.source.clone(),
            });
        }
        // `read_functional_controls` and `read_oddspath` read the damage as PS3
        // and the absence of damage as BS3, so the codes must sit that way.
        let (Some(ps3), Some(bs3)) = (self.criterion("PS3"), self.criterion("BS3")) else {
            return bad("it needs the criteria PS3 and BS3");
        };
        if ps3.direction != Direction::Pathogenic
            || ps3.discouraged
            || bs3.direction != Direction::Benign
            || bs3.discouraged
        {
            return bad("PS3 must be a counted pathogenic criterion and BS3 a counted benign one");
        }
        if rules.controls_supporting_min == 0 || rules.controls_supporting_min >= rules.controls_moderate_min {
            return bad("controls_supporting_min must be at least 1 and below controls_moderate_min");
        }
        let order = [Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong];
        for (limits, rising) in [(&rules.pathogenic, true), (&rules.benign, false)] {
            let ranks: Option<Vec<usize>> =
                limits.iter().map(|cut| order.iter().position(|o| *o == cut.strength)).collect();
            let Some(ranks) = ranks else { return bad("a limit uses a strength other than supporting to very_strong") };
            if ranks.windows(2).any(|w| w[0] >= w[1]) {
                return bad("limits must list strengths from weakest to strongest, once each");
            }
            // An OddsPath value cannot be negative.
            if limits.iter().any(|cut| !cut.at.is_finite() || cut.at < 0.0) {
                return bad("an OddsPath limit is not a number of zero or more");
            }
            if limits.windows(2).any(|w| if rising { w[0].at >= w[1].at } else { w[0].at <= w[1].at }) {
                return bad("stronger limits must lie further from the middle of the scale");
            }
        }
        if let (Some(p), Some(b)) = (rules.pathogenic.first(), rules.benign.first())
            && b.at > p.at
        {
            return bad("the benign and pathogenic limits overlap");
        }
        Ok(())
    }

    /// The BA1 entries are a dated manual reference, not evidence logic. The list
    /// only makes sense with the source that published it, and each entry must
    /// still be findable in ClinVar.
    fn check_ba1(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        let bad = |s: &str| Err(RuleError::BadBa1(s.to_string()));
        if self.ba1_exceptions.is_empty() {
            return Ok(());
        }
        if !known_sources.contains(BA1_SOURCE) {
            return bad("BA1 entries need the clingen-ba1-2018 source");
        }
        let mut ids = HashSet::new();
        for row in &self.ba1_exceptions {
            if row.gene.trim().is_empty()
                || row.variant.trim().is_empty()
                || row.clinvar_id.is_empty()
                || !row.clinvar_id.bytes().all(|c| c.is_ascii_digit())
                || !row.allele_id.starts_with("CA")
                || !ids.insert(&row.allele_id)
            {
                return bad("BA1 entries need a gene, variant, numeric ClinVar id and unique allele id");
            }
        }
        Ok(())
    }

    /// The PM3 table needs a known source, the criterion PM3, rows with unique
    /// ids and nonnegative finite points, and thresholds that rise with the
    /// strength, offering only strengths the criterion may use.
    fn check_pm3(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        let bad = |s: &str| Err(RuleError::BadPm3(s.to_string()));
        let Some(table) = &self.pm3 else { return Ok(()) };
        if !known_sources.contains(table.source.as_str()) {
            return Err(RuleError::UnknownSource { item: "PM3 points".into(), source_id: table.source.clone() });
        }
        let Some(criterion) = self.criterion("PM3").filter(|c| c.direction == Direction::Pathogenic && !c.discouraged)
        else {
            return bad("PM3 must be a counted pathogenic criterion");
        };
        let mut ids = HashSet::new();
        if table.rows.is_empty()
            || table.rows.iter().any(|r| {
                r.id.trim().is_empty()
                    || r.label.trim().is_empty()
                    || !ids.insert(&r.id)
                    || !r.points.is_finite()
                    || r.points < 0.0
                    || r.max_points.is_some_and(|m| !m.is_finite() || m <= 0.0)
            })
        {
            return bad("PM3 rows need unique ids, labels, nonnegative finite points and positive finite limits");
        }
        let order = [Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong];
        let ranks: Option<Vec<usize>> =
            table.thresholds.iter().map(|c| order.iter().position(|s| *s == c.strength)).collect();
        if ranks.is_none_or(|r| r.is_empty() || r.windows(2).any(|w| w[0] >= w[1]))
            || table.thresholds.iter().any(|c| !c.at.is_finite() || c.at <= 0.0)
            || table.thresholds.windows(2).any(|w| w[0].at >= w[1].at)
        {
            return bad("PM3 thresholds must rise in points and strength from Supporting to Very Strong");
        }
        if table.thresholds.iter().any(|cut| !criterion.allowed_strengths().contains(&cut.strength)) {
            return bad("a PM3 threshold offers a strength the PM3 criterion does not let a curator use");
        }
        Ok(())
    }

    /// The de novo table needs a known source, the criterion PS2, rows that never
    /// give less for confirmed parents, and thresholds that rise with the strength.
    fn check_de_novo(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.de_novo else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadDeNovo(problem.to_string()));
        if !known_sources.contains(rules.source.as_str()) {
            return Err(RuleError::UnknownSource {
                item: "de novo points".to_string(),
                source_id: rules.source.clone(),
            });
        }
        match self.criterion("PS2") {
            Some(ps2) if ps2.direction == Direction::Pathogenic && !ps2.discouraged => {}
            _ => return bad("it needs the criterion PS2, as a pathogenic criterion that is not discouraged"),
        }
        let points_ok = |points: f64| points.is_finite() && points >= 0.0;
        let mut ids = HashSet::new();
        let rows_ok = rules.phenotypes.iter().all(|row| {
            !row.id.trim().is_empty()
                && !row.label.trim().is_empty()
                && points_ok(row.confirmed)
                && points_ok(row.unconfirmed)
                && row.unconfirmed <= row.confirmed
                && row.max_points.is_none_or(|max| max.is_finite() && max > 0.0)
        });
        if rules.phenotypes.is_empty() || !rows_ok {
            return bad(
                "each row needs an id, a label, points of zero or more that are not higher without confirmed parents, and a limit above zero if it has one",
            );
        }
        if !rules.phenotypes.iter().all(|row| ids.insert(row.id.as_str())) {
            return bad("row ids must be unique");
        }
        let order = [Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong];
        let ranks: Option<Vec<usize>> =
            rules.thresholds.iter().map(|cut| order.iter().position(|o| *o == cut.strength)).collect();
        let Some(ranks) = ranks.filter(|ranks| !ranks.is_empty()) else {
            return bad("thresholds must use the strengths supporting to very_strong");
        };
        if ranks.windows(2).any(|w| w[0] >= w[1]) {
            return bad("thresholds must list strengths from weakest to strongest, once each");
        }
        let at_ok = rules.thresholds.iter().all(|cut| cut.at.is_finite() && cut.at > 0.0);
        if !at_ok || rules.thresholds.windows(2).any(|w| w[0].at >= w[1].at) {
            return bad("threshold points must be above zero and rise with the strength");
        }
        let allowed = self.criterion("PS2").map(|c| c.allowed_strengths()).unwrap_or_default();
        if rules.thresholds.iter().any(|cut| !allowed.contains(&cut.strength)) {
            return bad("a threshold offers a strength the PS2 criterion does not let a curator use");
        }
        if rules.recessive_text.trim().is_empty() {
            return bad("the recessive text is empty");
        }
        Ok(())
    }

    /// The BS4 table needs a known source, the criterion BS4 at a strength it allows,
    /// and situations with unique ids, of which at least one applies.
    fn check_non_segregation(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.non_segregation else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadNonSegregation(problem.to_string()));
        if !known_sources.contains(rules.source.as_str()) {
            return Err(RuleError::UnknownSource {
                item: "non-segregation situations".to_string(),
                source_id: rules.source.clone(),
            });
        }
        let Some(bs4) = self.criterion("BS4") else { return bad("it needs the criterion BS4") };
        if bs4.direction != Direction::Benign || bs4.discouraged {
            return bad("BS4 must be a benign criterion that is not discouraged");
        }
        if !bs4.allowed_strengths().contains(&rules.strength) {
            return bad("the strength is one that BS4 does not allow");
        }
        // The paper recommends -4 points; the helper says so on screen.
        if self.magnitude(rules.strength) != 4 {
            return bad("the strength must be worth 4 points, the paper's -4");
        }
        let mut ids = HashSet::new();
        let shaped = rules
            .situations
            .iter()
            .all(|s| !s.id.trim().is_empty() && !s.label.trim().is_empty() && !s.text.trim().is_empty());
        if rules.situations.is_empty() || !shaped {
            return bad("there must be situations, each with an id, a label and a text");
        }
        if !rules.situations.iter().all(|s| ids.insert(s.id.as_str())) {
            return bad("situation ids must be unique");
        }
        if !rules.situations.iter().any(|s| s.applies) {
            return bad("at least one situation must apply, or BS4 could never be offered");
        }
        Ok(())
    }

    /// The PP1/PP4 tables need a known source, the criteria PP1 and PP4, rising
    /// rows, and strength choices that the criteria allow and that never promise
    /// more points than the evidence has earned.
    fn check_locus(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        let Some(rules) = &self.locus_evidence else { return Ok(()) };
        let bad = |problem: &str| Err(RuleError::BadLocusEvidence(problem.to_string()));
        if !known_sources.contains(rules.source.as_str()) {
            return Err(RuleError::UnknownSource {
                item: "co-segregation and phenotype limits".to_string(),
                source_id: rules.source.clone(),
            });
        }
        let (Some(pp1), Some(pp4)) = (self.criterion("PP1"), self.criterion("PP4")) else {
            return bad("it needs the criteria PP1 and PP4");
        };
        if [pp1, pp4].iter().any(|c| c.direction != Direction::Pathogenic || c.discouraged) {
            return bad("PP1 and PP4 must be pathogenic criteria that are not discouraged");
        }
        // Table 4 is written in the standard points, so a different scale would make its choices wrong.
        if (self.points.supporting, self.points.moderate, self.points.strong) != (1, 2, 4) {
            return bad("it needs the standard point scale: Supporting 1, Moderate 2, Strong 4");
        }
        let positive = |x: f64| x.is_finite() && x > 0.0;
        if !positive(rules.cap_points) || rules.cap_points > f64::from(MAX_POINTS) {
            return bad("cap_points must be a positive number");
        }
        // The class is withheld by the combination cap, so it must match the limit the helper uses.
        let pair_cap = self
            .combination_caps
            .iter()
            .find(|cap| (cap.a == "PP1" && cap.b == "PP4") || (cap.a == "PP4" && cap.b == "PP1"));
        if pair_cap.is_none_or(|cap| f64::from(cap.max_points) != rules.cap_points) {
            return bad("it needs a PP1/PP4 combination cap equal to cap_points");
        }
        if !positive(rules.yield_text_minimum) || rules.yield_text_minimum > 100.0 {
            return bad("yield_text_minimum must be a percentage above 0");
        }
        if rules.homogeneity_text.trim().is_empty() || rules.notes.iter().any(|note| note.trim().is_empty()) {
            return bad("the homogeneity text and every note must be filled in");
        }
        if rules.pp4_yield.is_empty()
            || rules.pp4_yield.iter().any(|step| !positive(step.points) || !positive(step.at) || step.at > 100.0)
        {
            return bad("every yield row needs a yield between 0 and 100 and positive points, and there must be rows");
        }
        if rules.pp4_yield.windows(2).any(|w| w[0].at >= w[1].at || w[0].points >= w[1].points) {
            return bad("yield rows must rise in both yield and points");
        }
        let mut ids = HashSet::new();
        if rules.segregation.is_empty()
            || rules
                .segregation
                .iter()
                .any(|kind| kind.id.trim().is_empty() || kind.label.trim().is_empty() || !positive(kind.points_each))
        {
            return bad("every co-segregation kind needs an id, a label and positive points, and there must be kinds");
        }
        if !rules.segregation.iter().all(|kind| ids.insert(kind.id.as_str())) {
            return bad("co-segregation ids must be unique");
        }
        if rules.bands.is_empty()
            || rules.bands.iter().any(|band| !positive(band.min_points) || band.min_points > rules.cap_points)
            || rules.bands.windows(2).any(|w| w[0].min_points >= w[1].min_points)
        {
            return bad("bands must start above zero, rise, and not start above cap_points");
        }
        for band in &rules.bands {
            if band.options.is_empty() {
                return bad("every band needs at least one strength choice");
            }
            for choice in &band.options {
                if choice.pp1.is_none() && choice.pp4.is_none() {
                    return bad("a strength choice names neither PP1 nor PP4");
                }
                let allowed = |criterion: &Criterion, strength: Option<Strength>| {
                    strength.is_none_or(|s| criterion.allowed_strengths().contains(&s))
                };
                if !allowed(pp1, choice.pp1) || !allowed(pp4, choice.pp4) {
                    return bad("a strength choice uses a strength the criterion does not allow");
                }
                let worth: i32 = [choice.pp1, choice.pp4].into_iter().flatten().map(|s| self.magnitude(s)).sum();
                if f64::from(worth) > band.min_points || f64::from(worth) > rules.cap_points {
                    return bad("a strength choice is worth more points than its band has earned");
                }
            }
        }
        Ok(())
    }

    /// Every predictor needs a known source, and its limits must run in order, so
    /// a stronger level can never be reached by a score that misses a weaker one.
    fn check_predictors(&self, known_sources: &HashSet<&str>) -> Result<(), RuleError> {
        if self.predictors.is_empty() {
            return Ok(());
        }
        // `read_predictor` maps a high score to PP3 and a low one to BP4, so the
        // codes must sit that way, and PP3/BP4 cannot be discouraged pair members.
        match (self.criterion("PP3"), self.criterion("BP4")) {
            (Some(pp3), Some(bp4))
                if pp3.direction == Direction::Pathogenic
                    && !pp3.discouraged
                    && bp4.direction == Direction::Benign
                    && !bp4.discouraged => {}
            (Some(_), Some(_)) => {
                return Err(RuleError::BadPredictor {
                    id: "<predictors>".to_string(),
                    problem: "PP3 must be a counted pathogenic criterion and BP4 a counted benign one".to_string(),
                });
            }
            _ => {
                return Err(RuleError::BadPredictor {
                    id: "<predictors>".to_string(),
                    problem: "predictors need the criteria PP3 and BP4".to_string(),
                });
            }
        }
        let mut ids = HashSet::new();
        for tool in &self.predictors {
            let bad =
                |problem: &str| Err(RuleError::BadPredictor { id: tool.id.clone(), problem: problem.to_string() });
            if tool.id.trim().is_empty()
                || tool.name.trim().is_empty()
                || tool.reference.trim().is_empty()
                || !ids.insert(tool.id.as_str())
            {
                return bad("the id, name and reference must be filled in, and the id must be unique");
            }
            if !known_sources.contains(tool.source.as_str()) {
                return Err(RuleError::UnknownSource {
                    item: format!("predictor {}", tool.id),
                    source_id: tool.source.clone(),
                });
            }
            if tool.pathogenic.is_empty() && tool.benign.is_empty() {
                return bad("it has no score limits");
            }
            if tool.eligibility.as_deref().is_some_and(|text| text.trim().is_empty()) {
                return bad("the eligibility text is empty");
            }
            match (tool.min_score, tool.max_score) {
                (None, None) => {}
                (Some(min), Some(max)) if min.is_finite() && max.is_finite() && min < max => {}
                _ => {
                    return bad(
                        "min_score and max_score must be set together, be finite, and put min_score below max_score",
                    );
                }
            }
            let order = [Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong];
            if let Some(calibrated) = tool.calibrated {
                let Some(calibrated_rank) = order.iter().position(|o| *o == calibrated) else {
                    return bad("the calibrated strength must be supporting, moderate, strong or very_strong");
                };
                if tool
                    .pathogenic
                    .iter()
                    .chain(&tool.benign)
                    .any(|cut| order.iter().position(|o| *o == cut.strength).is_some_and(|rank| rank > calibrated_rank))
                {
                    return bad("a limit is stronger than the calibrated evidence it describes");
                }
            }
            for (limits, rising) in [(&tool.pathogenic, true), (&tool.benign, false)] {
                let ranks: Option<Vec<usize>> =
                    limits.iter().map(|cut| order.iter().position(|o| *o == cut.strength)).collect();
                let Some(ranks) = ranks else {
                    return bad("a limit uses a strength other than supporting, moderate, strong or very_strong");
                };
                if ranks.windows(2).any(|w| w[0] >= w[1]) {
                    return bad("limits must list strengths from weakest to strongest, once each");
                }
                if limits.iter().any(|cut| !cut.at.is_finite()) {
                    return bad("a score limit is not a number");
                }
                if limits.windows(2).any(|w| if rising { w[0].at >= w[1].at } else { w[0].at <= w[1].at }) {
                    return bad("stronger limits must lie further from the middle of the scale");
                }
            }
            // A score must never be both PP3 and BP4.
            if let (Some(p), Some(b)) = (tool.pathogenic.first(), tool.benign.first())
                && b.at >= p.at
            {
                return bad("the benign and pathogenic limits overlap");
            }
        }
        Ok(())
    }

    /// Tiers are either all unranged (no official cut-offs yet) or all ranged,
    /// with different minimums, the lowest starting where the VUS range starts,
    /// and every minimum inside the VUS range.
    fn check_tiers(&self) -> Result<(), RuleError> {
        let bad = |message: &str| Err(RuleError::BadTiers(message.to_string()));
        // Both unranged and ranged tiers need distinct names: the record screens
        // use the name as the id of the tier buttons and to compare answers.
        let mut names = HashSet::new();
        for tier in &self.vus_tiers {
            if tier.name.trim().is_empty() || !names.insert(tier.name.as_str()) {
                return bad("tier names must be filled in and unique");
            }
        }
        let ranged = self.vus_tiers.iter().filter(|t| t.min_points.is_some()).count();
        if ranged == 0 {
            return Ok(());
        }
        if ranged != self.vus_tiers.len() {
            return bad("either every tier has min_points or none does");
        }
        let (low, high) = (self.class_min_points.vus, self.class_min_points.likely_pathogenic);
        let mut minimums: Vec<i32> = self.vus_tiers.iter().filter_map(|t| t.min_points).collect();
        minimums.sort_unstable();
        if minimums.windows(2).any(|w| w[0] == w[1]) {
            return bad("two tiers start at the same point value");
        }
        if minimums.first() != Some(&low) {
            return bad("the lowest tier must start where the VUS range starts");
        }
        if minimums.last().is_some_and(|m| *m >= high) {
            return bad("a tier starts outside the VUS range");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in file with one piece of text replaced.
    fn edited(old: &str, new: &str) -> Result<RuleSet, RuleError> {
        assert!(BUILTIN.contains(old), "the test text '{old}' is not in the rule file");
        RuleSet::parse(&BUILTIN.replacen(old, new, 1))
    }

    fn reading(tool: &str, score: f64) -> Option<(&'static str, Strength, Strength)> {
        RuleSet::builtin().unwrap().read_predictor(tool, score).map(|r| (r.code, r.published, r.strength))
    }

    #[test]
    fn revel_scores_follow_pejaver_2022_table_2() {
        use Strength::*;
        // Each pair of lines: a score just inside a band, and the limit itself.
        assert_eq!(reading("revel", 0.644), Some(("PP3", Supporting, Supporting)));
        assert_eq!(reading("revel", 0.772), Some(("PP3", Supporting, Supporting)));
        assert_eq!(reading("revel", 0.773), Some(("PP3", Moderate, Moderate)));
        assert_eq!(reading("revel", 0.931), Some(("PP3", Moderate, Moderate)));
        assert_eq!(reading("revel", 0.932), Some(("PP3", Strong, Strong)));
        assert_eq!(reading("revel", 1.0), Some(("PP3", Strong, Strong)));
        assert_eq!(reading("revel", 0.290), Some(("BP4", Supporting, Supporting)));
        assert_eq!(reading("revel", 0.183), Some(("BP4", Moderate, Moderate)));
        // Between the two sides, the table gives no evidence at all.
        assert_eq!(reading("revel", 0.291), None);
        assert_eq!(reading("revel", 0.5), None);
        assert_eq!(reading("revel", 0.643), None);
    }

    #[test]
    fn a_benign_score_the_paper_rates_above_moderate_is_lowered_to_moderate() {
        use Strength::*;
        // The paper rates REVEL <= 0.016 as Strong and <= 0.003 as Very Strong;
        // this program stops BP4 at Moderate and keeps the published level for display.
        assert_eq!(reading("revel", 0.016), Some(("BP4", Strong, Moderate)));
        assert_eq!(reading("revel", 0.003), Some(("BP4", VeryStrong, Moderate)));
        assert_eq!(reading("revel", 0.0), Some(("BP4", VeryStrong, Moderate)));
    }

    #[test]
    fn the_other_calibrated_tools_read_their_own_limits() {
        use Strength::*;
        assert_eq!(reading("bayesdel-noaf", 0.50), Some(("PP3", Strong, Strong)));
        assert_eq!(reading("bayesdel-noaf", -0.36), Some(("BP4", Moderate, Moderate)));
        assert_eq!(reading("bayesdel-noaf", -0.18), Some(("BP4", Supporting, Supporting)));
        assert_eq!(reading("bayesdel-noaf", 0.0), None);
        assert_eq!(reading("mutpred2", 0.829), Some(("PP3", Moderate, Moderate)));
        assert_eq!(reading("mutpred2", 0.01), Some(("BP4", Strong, Moderate)));
        assert_eq!(reading("vest4", 0.965), Some(("PP3", Strong, Strong)));
        assert_eq!(reading("vest4", 0.302), Some(("BP4", Moderate, Moderate)));
    }

    #[test]
    fn spliceai_follows_walker_2023_and_never_goes_above_supporting() {
        use Strength::*;
        let rules = RuleSet::builtin().unwrap();
        // PP3 at 0.2 or more, BP4 at 0.1 or less; both are applied at Supporting
        // although the paper calibrated them to Moderate.
        assert_eq!(reading("spliceai", 0.2), Some(("PP3", Supporting, Supporting)));
        assert_eq!(reading("spliceai", 0.99), Some(("PP3", Supporting, Supporting)));
        assert_eq!(reading("spliceai", 0.1), Some(("BP4", Supporting, Supporting)));
        assert_eq!(reading("spliceai", 0.0), Some(("BP4", Supporting, Supporting)));
        // Between 0.1 and 0.2 the paper gives no evidence either way.
        assert_eq!(reading("spliceai", 0.11), None);
        assert_eq!(reading("spliceai", 0.19), None);
        // SpliceAI delta scores run from 0 to 1; a typo outside that gives nothing.
        assert_eq!(reading("spliceai", -0.01), None);
        assert_eq!(reading("spliceai", 1.01), None);
        let tool = rules.predictor("spliceai").unwrap();
        assert_eq!(tool.calibrated, Some(Moderate));
        assert_eq!((tool.min_score, tool.max_score), (Some(0.0), Some(1.0)));
        assert!(tool.eligibility.as_deref().is_some_and(|text| text.contains("donor")));
    }

    #[test]
    fn every_predictor_names_the_table_its_numbers_come_from() {
        let rules = RuleSet::builtin().unwrap();
        assert!(rules.predictors.iter().all(|p| !p.reference.trim().is_empty()));
        assert_eq!(rules.predictor("spliceai").unwrap().reference, "Walker 2023, Table 1");
        assert_eq!(rules.predictor("revel").unwrap().reference, "Pejaver 2022, Table 2");
    }

    fn oddspath(value: f64) -> Option<(&'static str, Strength)> {
        RuleSet::builtin().unwrap().read_oddspath(value).map(|r| (r.code, r.strength))
    }

    #[test]
    fn oddspath_limits_follow_brnich_2019_and_are_crossed_strictly() {
        use Strength::*;
        // Pathogenic: above 2.1, 4.3, 18.7 and 350. A value on a limit does not cross it.
        assert_eq!(oddspath(2.1), None);
        assert_eq!(oddspath(2.11), Some(("PS3", Supporting)));
        assert_eq!(oddspath(4.3), Some(("PS3", Supporting)));
        assert_eq!(oddspath(4.31), Some(("PS3", Moderate)));
        assert_eq!(oddspath(18.7), Some(("PS3", Moderate)));
        assert_eq!(oddspath(18.71), Some(("PS3", Strong)));
        assert_eq!(oddspath(350.0), Some(("PS3", Strong)));
        assert_eq!(oddspath(350.1), Some(("PS3", VeryStrong)));
        // Benign: below 0.48, 0.23 and 0.053.
        assert_eq!(oddspath(0.48), None);
        assert_eq!(oddspath(0.47), Some(("BS3", Supporting)));
        assert_eq!(oddspath(0.23), Some(("BS3", Supporting)));
        assert_eq!(oddspath(0.22), Some(("BS3", Moderate)));
        assert_eq!(oddspath(0.053), Some(("BS3", Moderate)));
        assert_eq!(oddspath(0.05), Some(("BS3", Strong)));
        assert_eq!(oddspath(0.0), Some(("BS3", Strong)));
        // The indeterminate range, and values that cannot be an OddsPath.
        assert_eq!(oddspath(1.0), None);
        assert_eq!(oddspath(-0.1), None);
        assert_eq!(oddspath(f64::NAN), None);
        assert_eq!(oddspath(f64::INFINITY), None);
    }

    #[test]
    fn without_statistics_the_number_of_controls_decides_and_stops_at_moderate() {
        use Strength::*;
        let rules = RuleSet::builtin().unwrap();
        let read =
            |damaging, controls| rules.read_functional_controls(damaging, controls).map(|r| (r.code, r.strength));
        // No validation controls: no evidence at all, in either direction.
        assert_eq!(read(true, 0), None);
        assert_eq!(read(false, 0), None);
        assert_eq!(read(true, 1), Some(("PS3", Supporting)));
        assert_eq!(read(true, 10), Some(("PS3", Supporting)));
        assert_eq!(read(true, 11), Some(("PS3", Moderate)));
        assert_eq!(read(true, 500), Some(("PS3", Moderate)));
        assert_eq!(read(false, 10), Some(("BS3", Supporting)));
        assert_eq!(read(false, 11), Some(("BS3", Moderate)));
    }

    #[test]
    fn functional_limits_that_run_the_wrong_way_are_refused() {
        assert!(matches!(
            edited(r#"{ strength = "strong", at = 18.7 }"#, r#"{ strength = "strong", at = 3.0 }"#),
            Err(RuleError::BadFunctional(_))
        ));
        assert!(matches!(
            edited(r#"{ strength = "supporting", at = 0.48 }"#, r#"{ strength = "supporting", at = 3.0 }"#),
            Err(RuleError::BadFunctional(_))
        ));
        assert!(matches!(
            edited("controls_moderate_min = 11", "controls_moderate_min = 0"),
            Err(RuleError::BadFunctional(_))
        ));
        assert!(matches!(
            edited("controls_supporting_min = 1", "controls_supporting_min = 0"),
            Err(RuleError::BadFunctional(_))
        ));
        assert!(matches!(
            edited("controls_supporting_min = 1", "controls_supporting_min = 11"),
            Err(RuleError::BadFunctional(_))
        ));
        assert!(matches!(
            edited("source = \"brnich2019\"\ncontrols", "source = \"nowhere\"\ncontrols"),
            Err(RuleError::UnknownSource { .. })
        ));
    }

    /// ClinGen 2024, Table 2, copied here by hand so the rule file is checked against
    /// an independent copy: (diagnostic yield in percent, points).
    const TABLE_2: [(f64, f64); 23] = [
        (19.1, 1.0),
        (25.4, 1.5),
        (33.0, 2.0),
        (41.5, 2.5),
        (50.6, 3.0),
        (59.6, 3.5),
        (68.0, 4.0),
        (75.4, 4.5),
        (81.6, 5.0),
        (86.4, 5.5),
        (90.2, 6.0),
        (93.0, 6.5),
        (95.0, 7.0),
        (96.5, 7.5),
        (97.5, 8.0),
        (98.3, 8.5),
        (98.8, 9.0),
        (99.2, 9.5),
        (99.4, 10.0),
        (99.6, 10.5),
        (99.7, 11.0),
        (99.8, 11.5),
        (99.9, 12.0),
    ];

    #[test]
    fn pp4_points_follow_every_row_of_table_2_and_round_down_between_rows() {
        let rules = RuleSet::builtin().unwrap();
        let mut below = None;
        for (at, points) in TABLE_2 {
            assert_eq!(rules.pp4_points(at, 1), Some(points), "at {at}%");
            // Just under a row, the row below applies (nothing under the first row).
            assert_eq!(rules.pp4_points(at - 0.01, 1), below, "just under {at}%");
            below = Some(points);
        }
        assert_eq!(rules.pp4_points(0.0, 1), None);
        assert_eq!(rules.pp4_points(100.0, 1), Some(12.0));
        // Not a percentage.
        assert_eq!(rules.pp4_points(100.1, 1), None);
        assert_eq!(rules.pp4_points(-5.0, 1), None);
        assert_eq!(rules.pp4_points(f64::NAN, 1), None);
        assert_eq!(rules.pp4_points(f64::INFINITY, 1), None);
    }

    #[test]
    fn variants_sharing_an_allele_divide_the_yield_not_the_points() {
        let rules = RuleSet::builtin().unwrap();
        // The paper's example (Figure 2B and Section 3.3): a 95.8% yield is 7 points.
        // Shared by two variants, each has 47.9%, which rounds down to the 41.5% row:
        // 2.5 points each. Dividing the 7 points would wrongly give 3.5.
        assert_eq!(rules.pp4_points(95.8, 1), Some(7.0));
        assert_eq!(rules.pp4_points(95.8, 2), Some(2.5));
        // A yield that falls below the table once shared earns nothing.
        assert_eq!(rules.pp4_points(30.0, 2), None);
        // No variants is not a case.
        assert_eq!(rules.pp4_points(95.8, 0), None);
    }

    #[test]
    fn pp1_points_follow_table_3() {
        let rules = RuleSet::builtin().unwrap();
        assert_eq!(rules.pp1_points("dominant", 0), Some(0.0));
        assert_eq!(rules.pp1_points("dominant", 3), Some(3.0));
        assert_eq!(rules.pp1_points("dominant", 5), Some(5.0));
        assert_eq!(rules.pp1_points("x-linked-male", 4), Some(4.0));
        assert_eq!(rules.pp1_points("recessive-affected", 1), Some(2.0));
        assert_eq!(rules.pp1_points("recessive-affected", 5), Some(10.0));
        // 0.4 is not exact in binary; the result must still read as the table does.
        assert_eq!(rules.pp1_points("recessive-unaffected", 3), Some(1.2));
        assert_eq!(rules.pp1_points("recessive-unaffected", 5), Some(2.0));
        // The paper says to keep adding 0.4 for each one above five.
        assert_eq!(rules.pp1_points("recessive-unaffected", 7), Some(2.8));
        assert_eq!(rules.pp1_points("not-a-kind", 3), None);
    }

    /// The PP1 and PP4 strengths of each choice, in the order the rule file lists them.
    type Choices = Vec<(Option<Strength>, Option<Strength>)>;

    fn locus_with(pp1: f64, pp4: f64, pp1_allowed: bool) -> (f64, bool, Choices) {
        let r = RuleSet::builtin().unwrap().read_locus(pp1, pp4, pp1_allowed).unwrap();
        (r.total, r.capped, r.choices.iter().map(|c| (c.pp1, c.pp4)).collect())
    }

    fn locus(pp1: f64, pp4: f64) -> (f64, bool, Choices) {
        locus_with(pp1, pp4, true)
    }

    #[test]
    fn locus_points_give_the_strength_choices_of_table_4() {
        use Strength::*;
        // 0 to 0.9 points: not applicable.
        assert_eq!(locus(0.0, 0.0), (0.0, false, vec![]));
        assert_eq!(locus(0.4, 0.5), (0.9, false, vec![]));
        // 1 to 1.9: one criterion at Supporting.
        let one = vec![(Some(Supporting), None), (None, Some(Supporting))];
        assert_eq!(locus(1.0, 0.0), (1.0, false, one.clone()));
        assert_eq!(locus(0.5, 1.4), (1.9, false, one));
        // 2 to 2.9.
        let two = vec![(Some(Moderate), None), (None, Some(Moderate)), (Some(Supporting), Some(Supporting))];
        assert_eq!(locus(1.0, 1.0), (2.0, false, two.clone()));
        assert_eq!(locus(0.0, 2.9), (2.9, false, two));
        // 3 to 3.9.
        let three = vec![(Some(Supporting), Some(Moderate)), (Some(Moderate), Some(Supporting))];
        assert_eq!(locus(1.0, 2.0), (3.0, false, three.clone()));
        assert_eq!(locus(0.0, 3.5), (3.5, false, three));
        // 4 to 4.9.
        let four = vec![(Some(Strong), None), (None, Some(Strong)), (Some(Moderate), Some(Moderate))];
        assert_eq!(locus(4.0, 0.0), (4.0, false, four.clone()));
        assert_eq!(locus(2.0, 2.5), (4.5, false, four));
        // 5 or more: the largest allowed pair is Strong with Supporting, and the sum stops at 5.
        let five = vec![(Some(Strong), Some(Supporting)), (Some(Supporting), Some(Strong))];
        assert_eq!(locus(3.0, 2.0), (5.0, false, five.clone()));
        assert_eq!(locus(5.0, 12.0), (5.0, true, five));
    }

    #[test]
    fn the_reading_keeps_the_sum_before_the_cap() {
        let reading = RuleSet::builtin().unwrap().read_locus(5.0, 12.0, true).unwrap();
        assert_eq!((reading.sum, reading.total, reading.capped), (17.0, 5.0, true));
    }

    #[test]
    fn when_only_one_gene_causes_the_phenotype_only_pp4_choices_are_listed() {
        use Strength::*;
        // Table 4 says "maximum allowable", so a lower band's PP4-only choice is allowed
        // when the band the total reaches has none (bands 3 and 5 have only pairs).
        assert_eq!(locus_with(0.0, 0.5, false), (0.5, false, vec![]));
        assert_eq!(locus_with(0.0, 1.5, false), (1.5, false, vec![(None, Some(Supporting))]));
        assert_eq!(locus_with(0.0, 2.0, false), (2.0, false, vec![(None, Some(Moderate))]));
        assert_eq!(locus_with(0.0, 3.5, false), (3.5, false, vec![(None, Some(Moderate))]));
        assert_eq!(locus_with(0.0, 4.5, false), (4.5, false, vec![(None, Some(Strong))]));
        // The most PP4 can reach alone is Strong, even at the 5 point limit.
        assert_eq!(locus_with(0.0, 12.0, false), (5.0, true, vec![(None, Some(Strong))]));
    }

    #[test]
    fn a_point_value_that_is_not_a_number_or_is_negative_gives_no_reading() {
        let rules = RuleSet::builtin().unwrap();
        for (pp1, pp4) in [(f64::NAN, 1.0), (1.0, f64::NAN), (f64::INFINITY, 0.0), (0.0, f64::INFINITY), (-1.0, 3.0)] {
            assert_eq!(rules.read_locus(pp1, pp4, true), None, "{pp1}, {pp4}");
        }
    }

    #[test]
    fn locus_points_that_add_to_a_band_edge_are_not_lost_to_rounding_noise() {
        // 0.1 + 0.7 is 0.7999999999999999 in binary arithmetic, and plus 0.2 it
        // lands just under 1.0. The sum is rounded before the band is chosen.
        let (total, _, choices) = locus(0.1 + 0.7, 0.2);
        assert_eq!(total, 1.0);
        assert_eq!(choices.len(), 2);
    }

    #[test]
    fn no_locus_choice_promises_more_than_the_points_earned_or_the_pair_limit() {
        let rules = RuleSet::builtin().unwrap();
        let locus = rules.locus_evidence.as_ref().unwrap();
        let cap = rules.combination_caps.iter().find(|c| c.a == "PP1" && c.b == "PP4").unwrap();
        for band in &locus.bands {
            for choice in &band.options {
                let worth: i32 = [choice.pp1, choice.pp4].into_iter().flatten().map(|s| rules.magnitude(s)).sum();
                assert!(f64::from(worth) <= band.min_points, "{choice:?} in the band from {}", band.min_points);
                assert!(worth <= cap.max_points, "{choice:?} goes over the PP1+PP4 limit");
            }
        }
        assert_eq!(f64::from(cap.max_points), locus.cap_points);
    }

    /// The built-in file with every line that starts with `prefix` removed.
    fn without_lines(prefix: &str) -> Result<RuleSet, RuleError> {
        let kept: Vec<&str> = BUILTIN.lines().filter(|line| !line.trim_start().starts_with(prefix)).collect();
        assert!(kept.len() < BUILTIN.lines().count(), "no line starts with '{prefix}'");
        RuleSet::parse(&kept.join("\n"))
    }

    /// The built-in file with every line that starts with `prefix` replaced by `new_line`.
    fn with_line_replaced(prefix: &str, new_line: &str) -> Result<RuleSet, RuleError> {
        let text: Vec<&str> =
            BUILTIN.lines().map(|line| if line.starts_with(prefix) { new_line } else { line }).collect();
        assert_ne!(text.join("\n"), BUILTIN.replace("\r\n", "\n"), "no line starts with '{prefix}'");
        RuleSet::parse(&text.join("\n"))
    }

    #[test]
    fn locus_tables_that_would_overstate_the_evidence_are_refused() {
        let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadLocusEvidence(_)));
        let band_one = r#"{ min_points = 1.0, options = [{ pp1 = "supporting" }, { pp4 = "supporting" }] }"#;
        // A Moderate choice in the band that starts at one point.
        assert!(is_bad(edited(
            band_one,
            r#"{ min_points = 1.0, options = [{ pp1 = "moderate" }, { pp4 = "supporting" }] }"#
        )));
        // A choice that names neither criterion.
        assert!(is_bad(edited(band_one, r#"{ min_points = 1.0, options = [{ }, { pp4 = "supporting" }] }"#)));
        // A band with no choice.
        assert!(is_bad(edited(band_one, "{ min_points = 1.0, options = [] }")));
        // A choice with a strength PP1 does not allow.
        assert!(is_bad(edited(
            r#"{ pp1 = "strong", pp4 = "supporting" }"#,
            r#"{ pp1 = "stand_alone", pp4 = "supporting" }"#
        )));
        // No bands at all.
        assert!(is_bad(without_lines("{ min_points")));
        // A band that starts above the limit for the pair can never be reached.
        assert!(is_bad(edited("cap_points = 5.0", "cap_points = 3.0")));
        // Yield rows must rise and stay percentages.
        assert!(is_bad(edited("{ at = 25.4, points = 1.5 }", "{ at = 10.0, points = 1.5 }")));
        assert!(is_bad(edited("{ at = 99.9, points = 12.0 }", "{ at = 120.0, points = 12.0 }")));
        // Co-segregation kinds need positive points, unique ids, and there must be some.
        assert!(is_bad(edited("points_each = 0.4", "points_each = 0.0")));
        assert!(is_bad(edited(r#"id = "x-linked-male""#, r#"id = "dominant""#)));
        assert!(is_bad(edited(r#"{ id = "dominant", label"#, r#"{ id = "", label"#)));
        assert!(is_bad(without_lines("{ id = ")));
        // Bands must rise.
        assert!(is_bad(edited("{ min_points = 3.0,", "{ min_points = 1.0,")));
        // The texts shown with the helper must not be empty.
        assert!(is_bad(with_line_replaced("homogeneity_text", r#"homogeneity_text = """#)));
        assert!(is_bad(edited("yield_text_minimum = 20.0", "yield_text_minimum = 0.0")));
        // The table must cite a source the file lists.
        assert!(matches!(
            edited("source = \"clingen-pp1-pp4-2024\"\ncap_points", "source = \"nowhere\"\ncap_points"),
            Err(RuleError::UnknownSource { .. })
        ));
    }

    #[test]
    fn the_locus_table_must_agree_with_the_rest_of_the_rules() {
        let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadLocusEvidence(_)));
        // The class is withheld by the PP1/PP4 combination cap, so it must equal the helper's limit.
        assert!(is_bad(edited("max_points = 5", "max_points = 6")));
        assert!(is_bad(edited("a = \"PP1\"\nb = \"PP4\"", "a = \"PP1\"\nb = \"PM1\"")));
        // Table 4 is written in the standard points.
        assert!(is_bad(edited("strong = 4\nvery_strong = 8", "strong = 5\nvery_strong = 8")));
    }

    #[test]
    fn bs4_applies_only_where_the_paper_supports_it() {
        let rules = RuleSet::builtin().unwrap();
        let read = |id| rules.read_non_segregation(id).map(|r| (r.code, r.strength));
        assert_eq!(read("follows-not"), Some(("BS4", Strength::Strong)));
        // The paper: little to no evidence for compound heterozygotes; no points for
        // unaffected carriers under reduced penetrance; formal analysis for common
        // phenocopies, conflicting results and very large families.
        assert_eq!(read("compound-heterozygous"), None);
        assert_eq!(read("reduced-penetrance"), None);
        assert_eq!(read("high-phenocopy"), None);
        assert_eq!(read("conflicting-or-large"), None);
        assert_eq!(read("not-a-situation"), None);
    }

    #[test]
    fn the_bs4_strength_is_the_minus_4_points_the_paper_recommends() {
        // ClinGen 2024: -4.0 points for non-segregation. No number in the rules changed.
        let rules = RuleSet::builtin().unwrap();
        let strength = rules.non_segregation.as_ref().unwrap().strength;
        assert_eq!(rules.magnitude(strength), 4);
        assert_eq!(rules.criterion("BS4").unwrap().strength, strength);
    }

    #[test]
    fn a_bs4_table_that_could_never_offer_or_misapply_bs4_is_refused() {
        let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadNonSegregation(_)));
        let own_source = "source = \"clingen-pp1-pp4-2024\"\nstrength = \"strong\"";
        // No situation applies.
        assert!(is_bad(edited("applies = true", "applies = false")));
        // A strength BS4 does not allow (benign criteria stop at Strong), and one that is not -4 points.
        assert!(is_bad(edited(own_source, "source = \"clingen-pp1-pp4-2024\"\nstrength = \"very_strong\"")));
        assert!(is_bad(edited(own_source, "source = \"clingen-pp1-pp4-2024\"\nstrength = \"moderate\"")));
        // A repeated id.
        assert!(is_bad(edited("id = \"high-phenocopy\"", "id = \"follows-not\"")));
        // An empty id, label or text.
        assert!(is_bad(edited("id = \"follows-not\"", "id = \"\"")));
        assert!(is_bad(edited("label = \"Does not follow the disease\"", "label = \"\"")));
        assert!(is_bad(with_line_replaced("text = \"Autosomal dominant, X-linked", "text = \"\"")));
        // A situation that does not say whether BS4 applies cannot even be read.
        assert!(matches!(without_lines("applies = "), Err(RuleError::Parse(_))));
        // An unknown source.
        assert!(matches!(
            edited("source = \"clingen-pp1-pp4-2024\"\nstrength", "source = \"nowhere\"\nstrength"),
            Err(RuleError::UnknownSource { .. })
        ));
    }

    /// Reads de novo counts as (points, strength) with the table's four rows in
    /// order: highly specific, consistent, consistent but many genes, not consistent.
    fn de_novo(counts: [(u32, u32); 4], recessive: bool) -> (f64, Option<Strength>, bool) {
        let r = RuleSet::builtin().unwrap().read_de_novo(&counts, recessive).unwrap();
        (r.points, r.evidence.map(|e| e.strength), r.lowered_for_recessive)
    }

    #[test]
    fn pm3_follows_pah_example_and_every_strength_edge() {
        use Strength::*;
        let rules = RuleSet::builtin().unwrap();
        // ClinGen 2019: one confirmed LP partner and one confirmed P partner.
        let example = rules.read_pm3(&[2, 0, 0, 0, 0]).unwrap();
        assert_eq!(example.points, 2.0);
        assert_eq!(example.evidence.unwrap().strength, Strong);
        for (count, expected) in [
            (0, None),
            (1, None),
            (2, Some(Supporting)),
            (3, Some(Supporting)),
            (4, Some(Moderate)),
            (7, Some(Moderate)),
            (8, Some(Strong)),
            (15, Some(Strong)),
            (16, Some(VeryStrong)),
        ] {
            let r = rules.read_pm3(&[0, 0, count, 0, 0]).unwrap();
            assert_eq!(r.points, f64::from(count) * 0.25);
            assert_eq!(r.evidence.map(|e| e.strength), expected, "{count}");
        }
        assert_eq!(rules.read_pm3(&[0, 1, 0, 0, 0]).unwrap().points, 0.5);
        let applied = crate::Applied::new("PM3", VeryStrong);
        assert!(rules.assess(&[applied]).is_ok());
    }

    #[test]
    fn pm3_limits_homozygous_and_uncertain_rows_separately() {
        let rules = RuleSet::builtin().unwrap();
        for (counts, points, strength) in [
            ([0, 0, 0, 100, 0], 1.0, Strength::Moderate),
            ([0, 0, 0, 0, 100], 0.5, Strength::Supporting),
            ([0, 0, 0, 100, 100], 1.5, Strength::Moderate),
            ([1, 0, 0, 100, 100], 2.5, Strength::Strong),
        ] {
            let r = rules.read_pm3(&counts).unwrap();
            assert_eq!(r.points, points);
            assert_eq!(r.evidence.unwrap().strength, strength);
        }
        assert!(rules.read_pm3(&[]).is_none());
        assert!(rules.read_pm3(&[1]).is_none());
    }

    #[test]
    fn pm3_rejects_bad_tables_and_supports_old_rule_files() {
        for replacement in ["points = -1.0", "points = nan"] {
            let text = BUILTIN.replacen("points = 1.0\n", &format!("{replacement}\n"), 1);
            assert!(RuleSet::parse(&text).is_err());
        }
        for (from, to) in [
            ("source = \"clingen-svi-pm3\"\nnotes", "source = \"missing\"\nnotes"),
            ("id = \"unknown_p\"", "id = \"confirmed\""),
            ("max_points = 1.0\n", "max_points = 0.0\n"),
            ("{ strength = \"strong\", at = 2.0 }", "{ strength = \"strong\", at = 0.25 }"),
        ] {
            assert!(RuleSet::parse(&BUILTIN.replacen(from, to, 1)).is_err(), "{to}");
        }
        let start = BUILTIN.find("\n[pm3]\n").unwrap();
        let end = BUILTIN[start..].find("# ----- Benign -----").unwrap() + start;
        let old = format!("{}{}", &BUILTIN[..start], &BUILTIN[end..]);
        let old = RuleSet::parse(&old).unwrap();
        assert!(old.read_pm3(&[1, 0, 0, 0, 0]).is_none());
        assert!(old.ba1_exceptions.is_empty());
    }

    #[test]
    fn ba1_reference_has_the_nine_published_identifiers() {
        let rules = RuleSet::builtin().unwrap();
        let actual: Vec<_> = rules
            .ba1_exceptions
            .iter()
            .map(|r| (r.gene.as_str(), r.clinvar_id.as_str(), r.allele_id.as_str()))
            .collect();
        assert_eq!(
            actual,
            vec![
                ("ACAD9", "1018", "CA114709"),
                ("GJB2", "17023", "CA172210"),
                ("HFE", "10", "CA113797"),
                ("HFE", "9", "CA113795"),
                ("MEFV", "2551", "CA280114"),
                ("MEFV", "2552", "CA280116"),
                ("PIBF1", "217689", "CA210261"),
                ("ACADS", "3830", "CA312214"),
                ("BTD", "1900", "CA090886"),
            ]
        );
        assert!(RuleSet::parse(&BUILTIN.replace("allele_id = \"CA090886\"", "allele_id = \"CA312214\"")).is_err());
        assert!(RuleSet::parse(&BUILTIN.replace("clinvar_id = \"1900\"", "clinvar_id = \"bad\"")).is_err());
    }

    #[test]
    fn pm3_calculation_reason_survives_case_reopening() {
        use crate::{Applied, CASE_FORMAT, CaseFile, GeneDecision};
        let rules = RuleSet::builtin().unwrap();
        let r = rules.read_pm3(&[4, 0, 0, 0, 0]).unwrap();
        let item = Applied::new("PM3", r.evidence.unwrap().strength).with_reason(
            "PM3: 4 x confirmed = 4 patient points after row limits; conditions checked (clingen-svi-pm3)",
        );
        let case = CaseFile {
            format: CASE_FORMAT,
            app_version: "test".into(),
            saved_on: "2026-11-05".into(),
            rule_set: rules.reference(),
            variant: "test".into(),
            gene: "PAH".into(),
            gene_decision: GeneDecision::NotDecided,
            today: vec![item],
            records: vec![],
        };
        let reopened = CaseFile::from_json(&case.to_json(), rules).unwrap();
        assert_eq!(reopened.today, case.today);
        assert_eq!(rules.assess(&reopened.today).unwrap().class(), rules.assess(&case.today).unwrap().class());
    }

    #[test]
    fn de_novo_points_follow_the_papers_worked_examples() {
        use Strength::*;
        let none = (0, 0);
        // NIPBL: one patient with confirmed parents (2 points) and two without (1 + 1) = 4, Very Strong.
        assert_eq!(de_novo([(1, 2), none, none, none], false), (4.0, Some(VeryStrong), false));
        // SIK1: a patient who fits but not specifically, confirmed: 1 point, Moderate.
        assert_eq!(de_novo([none, (1, 0), none, none], false), (1.0, Some(Moderate), false));
        // Two more such patients: 3 points, Strong (not yet Very Strong).
        assert_eq!(de_novo([none, (3, 0), none, none], false), (3.0, Some(Strong), false));
        // ASH1L: many genes can cause it. One patient 0.5 points, Supporting; two give 1, Moderate.
        assert_eq!(de_novo([none, none, (1, 0), none], false), (0.5, Some(Supporting), false));
        assert_eq!(de_novo([none, none, (2, 0), none], false), (1.0, Some(Moderate), false));
        // NIPBL with a phenotype that does not fit: no points, no evidence.
        assert_eq!(de_novo([none, none, none, (0, 1)], false), (0.0, None, false));
    }

    #[test]
    fn de_novo_strength_changes_exactly_at_the_table_2_limits() {
        use Strength::*;
        let none = (0, 0);
        // Each patient whose phenotype fits the gene but not specifically, with parents
        // not confirmed, earns 0.5 points.
        let edge = |count_half: u32| de_novo([none, (0, count_half), none, none], false);
        assert_eq!(edge(0).1, None);
        assert_eq!(edge(1), (0.5, Some(Supporting), false));
        assert_eq!(edge(2), (1.0, Some(Moderate), false));
        assert_eq!(edge(3), (1.5, Some(Moderate), false));
        assert_eq!(edge(4), (2.0, Some(Strong), false));
        assert_eq!(edge(7), (3.5, Some(Strong), false));
        assert_eq!(edge(8), (4.0, Some(VeryStrong), false));
        // A quarter point (many genes, parents not confirmed) adds up exactly: 3 x 0.25 is below Supporting.
        assert_eq!(de_novo([none, none, (0, 3), none], false), (0.75, Some(Supporting), false));
    }

    #[test]
    fn the_many_genes_row_adds_at_most_one_point() {
        use Strength::*;
        let none = (0, 0);
        // Four confirmed patients would be 2 points; the table's note limits the row to 1.
        assert_eq!(de_novo([none, none, (4, 0), none], false), (1.0, Some(Moderate), false));
        // The limit is for that row only; other rows still add to it.
        assert_eq!(de_novo([(1, 0), none, (4, 0), none], false), (3.0, Some(Strong), false));
    }

    #[test]
    fn a_recessive_gene_without_a_second_variant_lowers_the_strength_one_level() {
        use Strength::*;
        let none = (0, 0);
        assert_eq!(de_novo([(1, 2), none, none, none], true), (4.0, Some(Strong), true));
        assert_eq!(de_novo([(1, 0), none, none, none], true), (2.0, Some(Moderate), true));
        assert_eq!(de_novo([none, (1, 0), none, none], true), (1.0, Some(Supporting), true));
        // Supporting has no lower level: no evidence, and the screen is told why.
        assert_eq!(de_novo([none, none, (1, 0), none], true), (0.5, None, true));
        // Nothing to lower when the points reach no level.
        assert_eq!(de_novo([none, none, none, (3, 0)], true), (0.0, None, false));
    }

    #[test]
    fn de_novo_evidence_is_counted_as_ps2_and_can_reach_very_strong() {
        let rules = RuleSet::builtin().unwrap();
        let reading = rules.read_de_novo(&[(2, 0), (0, 0), (0, 0), (0, 0)], false).unwrap().evidence.unwrap();
        assert_eq!(
            (reading.code, reading.published, reading.strength),
            ("PS2", Strength::VeryStrong, Strength::VeryStrong)
        );
    }

    #[test]
    fn a_de_novo_total_can_also_be_applied_as_pm6_at_the_most_this_program_allows() {
        use Strength::*;
        let rules = RuleSet::builtin().unwrap();
        let pm6 = |counts: [(u32, u32); 4], recessive: bool| {
            let reading = rules.read_de_novo(&counts, recessive).unwrap();
            reading.pm6.map(|r| (r.code, r.published, r.strength))
        };
        let none = (0, 0);
        // Table 2 lets PS2 or PM6 carry the same level: 1 point is Moderate for both.
        assert_eq!(pm6([none, (1, 0), none, none], false), Some(("PM6", Moderate, Moderate)));
        assert_eq!(pm6([none, (3, 0), none, none], false), Some(("PM6", Strong, Strong)));
        // Table 2 also lists PM6 at Very Strong, but this program stops PM6 at Strong.
        assert_eq!(pm6([(1, 2), none, none, none], false), Some(("PM6", VeryStrong, Strong)));
        // The recessive rule lowers PM6 exactly as it lowers PS2; no points, no PM6.
        assert_eq!(pm6([(1, 2), none, none, none], true), Some(("PM6", Strong, Strong)));
        assert_eq!(pm6([none, none, (1, 0), none], true), None);
        assert_eq!(pm6([none, none, none, (3, 0)], false), None);
    }

    #[test]
    fn de_novo_counts_that_do_not_match_the_table_give_nothing() {
        let rules = RuleSet::builtin().unwrap();
        assert!(rules.read_de_novo(&[(1, 0)], false).is_none());
        assert!(rules.read_de_novo(&[], false).is_none());
    }

    #[test]
    fn a_de_novo_table_that_could_overstate_the_evidence_is_refused() {
        let is_bad = |result: Result<RuleSet, RuleError>| matches!(result, Err(RuleError::BadDeNovo(_)));
        // Less for confirmed parents than for unconfirmed ones makes no sense.
        assert!(is_bad(edited("confirmed = 2.0, unconfirmed = 1.0", "confirmed = 1.0, unconfirmed = 2.0")));
        // Negative points.
        assert!(is_bad(edited("confirmed = 2.0, unconfirmed = 1.0", "confirmed = 2.0, unconfirmed = -1.0")));
        // A repeated row id, and an empty label.
        assert!(is_bad(edited("id = \"consistent\"", "id = \"highly-specific\"")));
        assert!(is_bad(edited("label = \"Phenotype not consistent with the gene\"", "label = \"\"")));
        // A limit that is not above zero.
        assert!(is_bad(edited("max_points = 1.0", "max_points = 0.0")));
        // Thresholds out of order, repeated or not rising.
        assert!(is_bad(edited("{ at = 2.0, strength = \"strong\" }", "{ at = 0.7, strength = \"strong\" }")));
        assert!(is_bad(edited("{ at = 1.0, strength = \"moderate\" }", "{ at = 0.5, strength = \"moderate\" }")));
        assert!(is_bad(edited("{ at = 4.0, strength = \"very_strong\" }", "{ at = 4.0, strength = \"strong\" }")));
        // A strength that is not a pathogenic level.
        assert!(is_bad(edited("{ at = 4.0, strength = \"very_strong\" }", "{ at = 4.0, strength = \"stand_alone\" }")));
        // An unknown source.
        assert!(matches!(
            edited("source = \"clingen-svi-de-novo\"\nrecessive_text", "source = \"nowhere\"\nrecessive_text"),
            Err(RuleError::UnknownSource { .. })
        ));
    }

    #[test]
    fn a_rule_file_without_the_de_novo_table_still_loads_and_gives_nothing() {
        let start = BUILTIN.find("\n[de_novo]\n").unwrap();
        let end = BUILTIN[start..].find("\n[[criteria]]").unwrap() + start;
        let rules = RuleSet::parse(&format!("{}{}", &BUILTIN[..start], &BUILTIN[end..])).unwrap();
        assert!(rules.read_de_novo(&[(1, 0), (0, 0), (0, 0), (0, 0)], false).is_none());
    }

    #[test]
    fn a_rule_file_without_the_bs4_table_still_loads_and_gives_nothing() {
        let start = BUILTIN.find("\n[non_segregation]\n").unwrap();
        let end = BUILTIN[start..].find("\n[[criteria]]").unwrap() + start;
        let rules = RuleSet::parse(&format!("{}{}", &BUILTIN[..start], &BUILTIN[end..])).unwrap();
        assert!(rules.read_non_segregation("follows-not").is_none());
    }

    #[test]
    fn a_rule_file_without_the_locus_table_still_loads_and_gives_nothing() {
        // Cut only the PP1/PP4 section: from its header to the BS4 section's header.
        let start = BUILTIN.find("\n[locus_evidence]\n").unwrap();
        let end = BUILTIN.find("\n[non_segregation]\n").unwrap();
        let rules = RuleSet::parse(&format!("{}{}", &BUILTIN[..start], &BUILTIN[end..])).unwrap();
        assert!(rules.pp4_points(90.0, 1).is_none());
        assert!(rules.pp1_points("dominant", 3).is_none());
        assert!(rules.read_locus(1.0, 1.0, true).is_none());
        // The BS4 table is still there.
        assert!(rules.read_non_segregation("follows-not").is_some());
    }

    #[test]
    fn a_score_that_is_not_a_number_or_an_unknown_tool_gives_nothing() {
        assert_eq!(reading("revel", f64::NAN), None);
        assert_eq!(reading("revel", f64::INFINITY), None);
        assert_eq!(reading("not-a-tool", 0.9), None);
    }

    #[test]
    fn predictor_limits_that_run_the_wrong_way_are_refused() {
        // Strong must lie further from the middle than Moderate.
        assert!(matches!(
            edited(
                r#"{ strength = "strong", at = 0.932 }]
benign = [{ strength = "supporting", at = 0.290 }"#,
                r#"{ strength = "strong", at = 0.700 }]
benign = [{ strength = "supporting", at = 0.290 }"#
            ),
            Err(RuleError::BadPredictor { .. })
        ));
        // The benign and pathogenic sides may not overlap.
        assert!(matches!(
            edited(r#"{ strength = "supporting", at = 0.290 }"#, r#"{ strength = "supporting", at = 0.700 }"#),
            Err(RuleError::BadPredictor { .. })
        ));
        // A predictor must cite a source the file lists.
        assert!(matches!(
            edited(
                "id = \"vest4\"\nname = \"VEST4\"\nsource = \"pejaver2022\"",
                "id = \"vest4\"\nname = \"VEST4\"\nsource = \"nowhere\""
            ),
            Err(RuleError::UnknownSource { .. })
        ));
    }

    #[test]
    fn a_predictor_must_name_the_table_its_limits_come_from() {
        // The reference is saved with the reason, so an empty one is refused.
        assert!(matches!(
            edited(r#"reference = "Pejaver 2022, Table 2""#, r#"reference = """#),
            Err(RuleError::BadPredictor { .. })
        ));
        // A file written before the field existed gets the loader's message, not a serde error.
        assert!(matches!(edited("reference = \"Pejaver 2022, Table 2\"\n", ""), Err(RuleError::BadPredictor { .. })));
    }

    #[test]
    fn predictor_metadata_that_would_mislead_is_refused() {
        // The range must be a pair with min below max.
        assert!(matches!(
            edited("min_score = 0.0\nmax_score = 1.0", "min_score = 1.0\nmax_score = 0.0"),
            Err(RuleError::BadPredictor { .. })
        ));
        assert!(matches!(
            edited("min_score = 0.0\nmax_score = 1.0", "min_score = 0.0"),
            Err(RuleError::BadPredictor { .. })
        ));
        // An empty confirmation text would render an empty condition.
        assert!(matches!(
            edited(
                r#"eligibility = "the variant is outside the donor and acceptor +/-1,2 positions""#,
                r#"eligibility = """#
            ),
            Err(RuleError::BadPredictor { .. })
        ));
        // The applied code cannot be stronger than the evidence the paper calibrated.
        let stronger_than_calibrated =
            BUILTIN.replacen(r#"calibrated = "moderate""#, r#"calibrated = "supporting""#, 1).replacen(
                r#"pathogenic = [{ strength = "supporting", at = 0.2 }]"#,
                r#"pathogenic = [{ strength = "moderate", at = 0.2 }]"#,
                1,
            );
        assert!(matches!(RuleSet::parse(&stronger_than_calibrated), Err(RuleError::BadPredictor { .. })));
    }

    #[test]
    fn no_supporting_criterion_can_reach_very_strong() {
        let rules = RuleSet::builtin().unwrap();
        for c in rules.criteria.iter().filter(|c| c.strength == Strength::Supporting && !c.discouraged) {
            assert!(!c.allowed_strengths().contains(&Strength::VeryStrong), "{}", c.code);
        }
        // Old records are read as they were made.
        let pp2 = rules.criterion("PP2").unwrap();
        assert!(pp2.recorded_strengths().contains(&Strength::VeryStrong));
    }

    #[test]
    fn benign_evidence_has_no_very_strong_level() {
        // The 2015 table has no very strong benign row. BA1 is the one stand-alone exception.
        let rules = RuleSet::builtin().unwrap();
        for c in rules.criteria.iter().filter(|c| c.direction == Direction::Benign && c.code != "BA1" && !c.discouraged)
        {
            assert!(!c.allowed_strengths().contains(&Strength::VeryStrong), "{}", c.code);
            assert!(!c.recorded_strengths().contains(&Strength::VeryStrong), "{}", c.code);
        }
    }

    #[test]
    fn the_pvs1_guide_follows_the_clingen_tree() {
        // Abou Tayoun 2018: "generally does not recommend assigning PVS1 or PVS1_Strong for start loss variants".
        let pvs1 = RuleSet::builtin().unwrap().criterion("PVS1").unwrap();
        let strengths_for = |text: &str| -> Vec<Option<Strength>> {
            pvs1.guide.iter().filter(|g| g.situation.starts_with(text)).map(|g| g.strength).collect()
        };
        assert_eq!(strengths_for("Start-loss"), [Some(Strength::Moderate), Some(Strength::Supporting)]);
        assert_eq!(
            strengths_for("Decay is not predicted, the role"),
            [Some(Strength::Strong), Some(Strength::Moderate)]
        );
        assert_eq!(strengths_for("Whole-gene deletion"), [Some(Strength::VeryStrong)]);
        assert!(pvs1.guide.iter().any(|g| g.strength.is_none()), "the curator can answer that PVS1 does not apply");
    }

    #[test]
    fn a_bad_strength_guide_is_refused() {
        // No source.
        let no_source = BUILTIN.replacen("guide_source = \"abou-tayoun2018\"\n", "", 1);
        assert!(matches!(RuleSet::parse(&no_source), Err(RuleError::BadGuide { .. })));
        // A strength the criterion may not use.
        let bad = BUILTIN.replacen(
            "strength = \"supporting\"\n\n[[criteria.guide]]",
            "strength = \"stand_alone\"\n\n[[criteria.guide]]",
            1,
        );
        assert!(matches!(RuleSet::parse(&bad), Err(RuleError::BadGuide { .. })));
    }

    #[test]
    fn the_answer_a_saved_case_can_get_back_is_unique() {
        let pvs1 = RuleSet::builtin().unwrap().criterion("PVS1").unwrap();
        // Two guide answers set Very Strong, so the loader must not pick one
        // silently; a saved case with that strength and no answer asks again.
        assert_eq!(pvs1.unique_situation_for(Strength::VeryStrong), None);
        assert!(pvs1.situation_for(Strength::VeryStrong).is_some());
        // One answer sets Supporting, so a saved Supporting strength is
        // reconstructable without inventing anything.
        let unique = pvs1.unique_situation_for(Strength::Supporting).unwrap();
        assert_eq!(pvs1.guide_option(unique).unwrap().strength, Some(Strength::Supporting));
    }

    #[test]
    fn a_source_id_or_a_tier_name_cannot_appear_twice() {
        let source_twice = BUILTIN.replacen("id = \"richards2015\"", "id = \"clingen-svi-pm2\"", 1);
        assert!(matches!(RuleSet::parse(&source_twice), Err(RuleError::DuplicateSource(_))));
        let empty_id = BUILTIN.replacen("id = \"richards2015\"", "id = \"\"", 1);
        assert!(matches!(RuleSet::parse(&empty_id), Err(RuleError::DuplicateSource(_))));
        let tier_twice = BUILTIN.replacen("name = \"VUS-mid\"", "name = \"VUS-low\"", 1);
        assert!(matches!(RuleSet::parse(&tier_twice), Err(RuleError::BadTiers(_))));
    }

    #[test]
    fn a_helper_criterion_may_not_carry_a_strength_guide() {
        // The predictor helper applies PP3 from its tables; a guide on PP3
        // would make every Apply produce an assessment error, so the loader
        // refuses the file. (PP3 appears before BP4, so the first occurrence
        // of the marker line ends PP3's block.)
        let pp3_with_guide = BUILTIN.replacen(
            "Choose one tool before you see its result.\"\n\n[[criteria]]\ncode = \"PP4\"",
            "Choose one tool before you see its result.\"\nguide_source = \"pejaver2022\"\n\n[[criteria.guide]]\nid = \"calibrated\"\nshort = \"Calibrated table\"\nsituation = \"The score comes from the tool this case is using.\"\nstrength = \"supporting\"\n\n[[criteria]]\ncode = \"PP4\"",
            1,
        );
        assert!(matches!(RuleSet::parse(&pp3_with_guide), Err(RuleError::GuideOnHelperCriterion(_))));
    }

    #[test]
    fn a_review_date_must_name_a_possible_day() {
        let ok = edited(
            "status = \"unverified\"",
            "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-11-08\"",
        );
        assert!(ok.is_ok());
        let month = edited(
            "status = \"unverified\"",
            "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-99-08\"",
        );
        assert!(matches!(month, Err(RuleError::ReviewIncomplete)));
        let day = edited(
            "status = \"unverified\"",
            "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-11-39\"",
        );
        assert!(matches!(day, Err(RuleError::ReviewIncomplete)));
    }

    #[test]
    fn the_built_in_file_is_valid() {
        assert!(RuleSet::builtin().is_ok());
    }

    #[test]
    fn point_systems_that_would_misclassify_are_rejected() {
        // No evidence would fall below the VUS range.
        assert!(matches!(edited("vus = 0", "vus = 1"), Err(RuleError::BadThresholds)));
        // Stronger evidence worth fewer points than weaker evidence.
        assert!(matches!(
            edited("supporting = 1\nmoderate = 2", "supporting = 4\nmoderate = 2"),
            Err(RuleError::BadPoints)
        ));
        // BA1 alone could not reach Benign.
        assert!(matches!(edited("stand_alone = 8", "stand_alone = 4"), Err(RuleError::BadPoints)));
        // Numbers so large that adding them could overflow.
        assert!(matches!(edited("very_strong = 8", "very_strong = 2000000000"), Err(RuleError::BadPoints)));
    }

    #[test]
    fn a_rule_file_read_straight_through_serde_is_checked_too() {
        let bad = BUILTIN.replacen("vus = 0", "vus = 1", 1);
        assert!(toml::from_str::<RuleSet>(&bad).is_err());
        assert!(toml::from_str::<RuleSet>(BUILTIN).is_ok());
    }

    #[test]
    fn reviewed_needs_a_reviewer_and_a_date() {
        assert!(matches!(edited("status = \"unverified\"", "status = \"reviewed\""), Err(RuleError::ReviewIncomplete)));
        let ok = edited(
            "status = \"unverified\"",
            "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"2026-12-01\"",
        );
        assert_eq!(ok.unwrap().meta.status, ReviewStatus::Reviewed);
        let bad_date = edited(
            "status = \"unverified\"",
            "status = \"reviewed\"\nreviewed_by = \"A. Curator\"\nreviewed_on = \"December\"",
        );
        assert!(matches!(bad_date, Err(RuleError::ReviewIncomplete)));
    }

    #[test]
    fn vus_tiers_must_be_all_or_nothing_and_cover_the_vus_range() {
        let tiers = |a: &str, b: &str, c: &str| {
            let mut text = BUILTIN.to_string();
            for (name, min) in [("VUS-high", a), ("VUS-mid", b), ("VUS-low", c)] {
                let old = format!("name = \"{name}\"\n");
                text = text.replacen(&old, &format!("{old}min_points = {min}\n"), 1);
            }
            RuleSet::parse(&text)
        };
        assert!(tiers("4", "2", "0").is_ok());
        assert!(matches!(tiers("4", "4", "0"), Err(RuleError::BadTiers(_))), "same minimum twice");
        assert!(matches!(tiers("4", "2", "1"), Err(RuleError::BadTiers(_))), "gap at the bottom of the range");
        assert!(matches!(tiers("6", "2", "0"), Err(RuleError::BadTiers(_))), "starts inside Likely pathogenic");
        // Only one tier with a range.
        let text = BUILTIN.replacen("name = \"VUS-high\"\n", "name = \"VUS-high\"\nmin_points = 4\n", 1);
        assert!(matches!(RuleSet::parse(&text), Err(RuleError::BadTiers(_))));
    }

    #[test]
    fn a_cap_at_the_default_is_accepted_and_still_limits_later() {
        // PP3 is Supporting by default; a cap at Supporting leaves only that level.
        let rules = edited(
            "max_strength = \"strong\"\nnote = \"ClinGen later calibrated",
            "max_strength = \"supporting\"\nnote = \"ClinGen later calibrated",
        )
        .unwrap();
        assert_eq!(rules.criterion("PP3").unwrap().allowed_strengths(), vec![Strength::Supporting]);
        // PM1 is Moderate by default; capping it at Supporting would exclude its own default.
        let text = BUILTIN.replacen("code = \"PS3\"\n", "code = \"PS3\"\nmax_strength = \"supporting\"\n", 1);
        assert!(matches!(RuleSet::parse(&text), Err(RuleError::DefaultNotAllowed { .. })));
    }

    #[test]
    fn a_cap_that_is_not_a_recorded_strength_is_rejected() {
        // Stand-alone only exists for BA1. Capping PP3 at it would otherwise be
        // dropped without a word, and the curator would be offered every strength.
        assert!(matches!(
            edited("max_strength = \"strong\"", "max_strength = \"stand_alone\""),
            Err(RuleError::MaxStrengthNotRecorded { .. })
        ));
    }

    #[test]
    fn an_empty_guide_situation_is_rejected() {
        let line = "situation = \"Whole-gene deletion, where loss of function is an established disease mechanism. Abou Tayoun 2018 says a full-gene deletion of a known haploinsufficient gene warrants Pathogenic even though PVS1 alone is 8 points, so check the rest of the evidence.\"";
        assert!(matches!(edited(line, "situation = \"\""), Err(RuleError::BadSituation { .. })));
    }

    #[test]
    fn a_repeated_exclusive_pair_is_rejected() {
        let text = format!(
            "{BUILTIN}\n[[exclusive_pairs]]\na = \"PS3\"\nb = \"BS3\"\nreason = \"again\"\nsource = \"richards2015\"\n"
        );
        assert!(matches!(RuleSet::parse(&text), Err(RuleError::BadExclusivePair(_))));
    }

    #[test]
    fn a_combination_cap_below_the_default_pair_is_rejected() {
        assert!(matches!(edited("max_points = 5", "max_points = 1"), Err(RuleError::BadCombinationCap { .. })));
    }

    #[test]
    fn original_strength_must_be_a_different_recorded_strength() {
        // PM2's default is Supporting; an "original" equal to it explains nothing.
        assert!(matches!(
            edited("original_strength = \"moderate\"", "original_strength = \"supporting\""),
            Err(RuleError::BadOriginalStrength { .. })
        ));
    }
}
