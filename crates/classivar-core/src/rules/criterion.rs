//! One criterion of the rule file, and the strength guide a criterion may carry.

use serde::Deserialize;

use crate::model::{Category, Direction, Strength};

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

/// One answer to the gene-level question that comes before a guide (PVS1:
/// Abou Tayoun 2018, Table 1). It lowers the strength the guide gives.
#[derive(Debug, Clone, Deserialize)]
pub struct GeneLevel {
    /// Stable id saved with the case. Keep it unchanged when the wording changes.
    pub id: String,
    pub short: String,
    pub situation: String,
    /// How many levels the guide's strength is lowered. None: the criterion
    /// does not apply at all for this gene.
    pub steps: Option<u8>,
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
    /// The exact strengths a curator may choose, weakest first, when the
    /// published levels have a gap (BP7: Supporting from predictions, Strong
    /// from an RNA assay, no Moderate). Empty means every step up to `max_strength`.
    #[serde(default)]
    pub levels: Vec<Strength>,
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
    /// The gene-level question asked before the guide, when the rule file has one.
    #[serde(default)]
    pub gene_level: Vec<GeneLevel>,
    /// The source of the gene-level question. Required when there is one.
    pub gene_level_source: Option<String>,
}

impl Criterion {
    /// The strengths a curator may choose for this criterion, weakest first.
    /// "Stand-alone" is only offered to a criterion whose default is stand-alone (BA1).
    pub fn allowed_strengths(&self) -> Vec<Strength> {
        if !self.levels.is_empty() {
            return self.levels.clone();
        }
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

    /// The gene-level answer with this id.
    pub fn gene_level_option(&self, id: &str) -> Option<&GeneLevel> {
        self.gene_level.iter().find(|g| g.id == id)
    }

    /// The strength a guide answer and a gene-level answer give together: the
    /// guide's strength lowered by the gene-level step. None when the criterion
    /// does not apply (either answer says so, or the step goes below Supporting).
    /// Without a gene-level question, the guide's strength.
    pub fn strength_for(&self, situation: &str, gene_level: Option<&str>) -> Option<Strength> {
        let guided = self.guide_option(situation)?.strength?;
        if self.gene_level.is_empty() {
            return Some(guided);
        }
        let steps = self.gene_level_option(gene_level?)?.steps?;
        guided.lowered(steps)
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
