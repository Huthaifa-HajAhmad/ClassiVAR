//! The published tables behind the evidence helpers: predictor limits, assay
//! limits, PP1/PP4 points, BS4 situations, de novo points, PM3 points and the BA1
//! reference list. Each reading type says what a helper found in its table.

use serde::Deserialize;

use crate::model::Strength;

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

/// What the curator knows about how an assay was validated (Brnich 2019, step 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssayValidation {
    /// The assay includes normal (wild-type) and abnormal (null) experimental
    /// controls, and technical or biological replicates. Without them, Brnich
    /// says functional evidence should not be applied.
    pub experimental_controls: bool,
    /// Validation controls: variants classified pathogenic or likely pathogenic
    /// without functional data.
    pub pathogenic_controls: u32,
    /// Validation controls classified benign or likely benign without functional data.
    pub benign_controls: u32,
}

/// How strong functional-assay evidence is (Brnich 2019). Without a statistical
/// analysis, the number of validation controls decides: at most `Moderate`. With
/// one, the OddsPath value decides, and its limits are crossed strictly.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionalRules {
    pub source: String,
    /// An assay with at least this many validation controls, at least one
    /// pathogenic and one benign, reaches Moderate. With fewer (or only one kind)
    /// it reaches Supporting, provided it has experimental controls and replicates.
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
