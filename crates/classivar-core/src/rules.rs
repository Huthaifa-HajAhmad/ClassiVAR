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
    vus_tiers: Vec<VusTier>,
    exclusive_pairs: Vec<ExclusivePair>,
    #[serde(default)]
    combination_caps: Vec<CombinationCap>,
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
            // The date must look like YYYY-MM-DD.
            let is_date = |s: &str| {
                s.len() == 10
                    && s.char_indices().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
            };
            if !filled(&self.meta.reviewed_by) || !self.meta.reviewed_on.as_deref().is_some_and(is_date) {
                return Err(RuleError::ReviewIncomplete);
            }
        }

        let known_sources: HashSet<&str> = self.sources.iter().map(|s| s.id.as_str()).collect();
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

        for tier in &self.vus_tiers {
            needs_source(tier.name.clone(), &tier.source)?;
        }
        self.check_tiers()
    }

    /// Tiers are either all unranged (no official cut-offs yet) or all ranged,
    /// with different minimums, the lowest starting where the VUS range starts,
    /// and every minimum inside the VUS range.
    fn check_tiers(&self) -> Result<(), RuleError> {
        let ranged = self.vus_tiers.iter().filter(|t| t.min_points.is_some()).count();
        if ranged == 0 {
            return Ok(());
        }
        let bad = |message: &str| Err(RuleError::BadTiers(message.to_string()));
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
