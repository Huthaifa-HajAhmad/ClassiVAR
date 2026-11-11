//! The load-time checks that every rule file must pass: the points, the review
//! status, the sources, the criteria, and the exclusive pairs and caps. The
//! helper tables have their own checks in the files next to this one.

use std::collections::HashSet;

use crate::model::{Direction, Strength};

use super::{Criterion, Cutoff, HELPER_CODES, MAX_POINTS, ReviewStatus, RuleError, RuleSet};

/// The strengths a score or points limit may name, weakest first. Stand-alone is for BA1 only.
const LIMIT_STRENGTHS: [Strength; 4] =
    [Strength::Supporting, Strength::Moderate, Strength::Strong, Strength::VeryStrong];

/// Checks that a list of limits names only strengths from supporting to very strong,
/// weakest first and each once. The error is a sentence for the rule-file author.
pub(super) fn check_strength_ladder(limits: &[Cutoff]) -> Result<(), &'static str> {
    let ranks: Option<Vec<usize>> =
        limits.iter().map(|cut| LIMIT_STRENGTHS.iter().position(|s| *s == cut.strength)).collect();
    let Some(ranks) = ranks else {
        return Err("a limit uses a strength other than supporting, moderate, strong or very_strong");
    };
    if ranks.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("limits must list strengths from weakest to strongest, once each");
    }
    Ok(())
}

/// Do stronger limits lie further from the middle of the scale? Pathogenic limits
/// rise with the score (`rising`); benign limits fall.
pub(super) fn limits_move_away_from_the_middle(limits: &[Cutoff], rising: bool) -> bool {
    limits.windows(2).all(|pair| if rising { pair[0].at < pair[1].at } else { pair[0].at > pair[1].at })
}

/// A rule or table must cite a source that the file lists.
pub(super) fn needs_source(known: &HashSet<&str>, item: impl Into<String>, source_id: &str) -> Result<(), RuleError> {
    if known.contains(source_id) {
        Ok(())
    } else {
        Err(RuleError::UnknownSource { item: item.into(), source_id: source_id.to_string() })
    }
}

impl RuleSet {
    /// Every check, in the order the first problem is reported. The rest of the
    /// program relies on all of them having passed.
    pub(super) fn check(&self) -> Result<(), RuleError> {
        self.check_scale()?;
        self.check_review()?;
        let known = self.known_sources()?;
        self.check_criteria(&known)?;
        self.check_pairs(&known)?;
        self.check_caps(&known)?;
        self.check_predictors(&known)?;
        self.check_functional(&known)?;
        self.check_locus(&known)?;
        self.check_non_segregation(&known)?;
        self.check_de_novo(&known)?;
        self.check_pm3(&known)?;
        self.check_ba1(&known)?;
        for tier in &self.vus_tiers {
            needs_source(&known, tier.name.as_str(), &tier.source)?;
        }
        self.check_tiers()
    }

    /// The class limits and the points per strength.
    fn check_scale(&self) -> Result<(), RuleError> {
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
        if points_ok { Ok(()) } else { Err(RuleError::BadPoints) }
    }

    /// A file marked reviewed must say who reviewed it and when.
    fn check_review(&self) -> Result<(), RuleError> {
        if self.meta.status != ReviewStatus::Reviewed {
            return Ok(());
        }
        let named = self.meta.reviewed_by.as_deref().is_some_and(|name| !name.trim().is_empty());
        let dated = self.meta.reviewed_on.as_deref().is_some_and(crate::date::is_calendar_date);
        if named && dated { Ok(()) } else { Err(RuleError::ReviewIncomplete) }
    }

    /// The ids of the listed sources. A repeated id would make `source()` ambiguous,
    /// so the file must say which one it means.
    fn known_sources(&self) -> Result<HashSet<&str>, RuleError> {
        let mut ids = HashSet::new();
        for source in &self.sources {
            if source.id.trim().is_empty() || !ids.insert(source.id.as_str()) {
                return Err(RuleError::DuplicateSource(source.id.clone()));
            }
        }
        Ok(ids)
    }

    fn check_criteria(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let has_direction = |d: Direction| self.criteria.iter().any(|c| c.direction == d);
        if !has_direction(Direction::Pathogenic) || !has_direction(Direction::Benign) {
            return Err(RuleError::MissingCriteria);
        }
        let mut seen = HashSet::new();
        for c in &self.criteria {
            if !seen.insert(c.code.as_str()) {
                return Err(RuleError::DuplicateCode(c.code.clone()));
            }
            self.check_criterion(c, known)?;
        }
        Ok(())
    }

    fn check_criterion(&self, c: &Criterion, known: &HashSet<&str>) -> Result<(), RuleError> {
        needs_source(known, c.code.as_str(), &c.source)?;
        if let Some(source) = &c.strength_source {
            needs_source(known, format!("{} (strength)", c.code), source)?;
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
        check_guide(c, known)?;
        if let Some(original) = c.original_strength
            && (original == c.strength || !c.recorded_strengths().contains(&original))
        {
            return Err(RuleError::BadOriginalStrength(c.code.clone()));
        }
        Ok(())
    }

    fn check_pairs(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        let mut seen_pairs = HashSet::new();
        for pair in &self.exclusive_pairs {
            let key =
                if pair.a <= pair.b { (pair.a.as_str(), pair.b.as_str()) } else { (pair.b.as_str(), pair.a.as_str()) };
            if pair.a == pair.b || !seen_pairs.insert(key) {
                return Err(RuleError::BadExclusivePair(format!("{}/{}", pair.a, pair.b)));
            }
            needs_source(known, format!("{}/{}", pair.a, pair.b), &pair.source)?;
            if self.criterion(&pair.a).is_none() || self.criterion(&pair.b).is_none() {
                return Err(RuleError::UnknownPairCode { a: pair.a.clone(), b: pair.b.clone() });
            }
        }
        Ok(())
    }

    fn check_caps(&self, known: &HashSet<&str>) -> Result<(), RuleError> {
        for cap in &self.combination_caps {
            needs_source(known, format!("{}/{}", cap.a, cap.b), &cap.source)?;
            let problem = if cap.a == cap.b {
                Some("names the same criterion twice".to_string())
            } else {
                match (self.criterion(&cap.a), self.criterion(&cap.b)) {
                    (Some(a), Some(b)) => {
                        let defaults = self.magnitude(a.strength) + self.magnitude(b.strength);
                        (cap.max_points <= 0 || cap.max_points > MAX_POINTS || defaults > cap.max_points).then(|| {
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
        Ok(())
    }
}

/// A strength guide needs a source, answers the criterion allows, distinct and
/// filled-in situations, and at least one answer that applies. The criteria the
/// helpers fill in from tables cannot answer a guide, so they may not carry one.
fn check_guide(c: &Criterion, known: &HashSet<&str>) -> Result<(), RuleError> {
    if c.guide.is_empty() {
        return match c.guide_source {
            Some(_) => Err(RuleError::GuideSourceWithoutGuide(c.code.clone())),
            None => Ok(()),
        };
    }
    let source_ok = c.guide_source.as_deref().is_some_and(|id| known.contains(id));
    let allowed = c.allowed_strengths();
    let strengths_ok = c.guide.iter().all(|g| g.strength.is_none_or(|s| allowed.contains(&s)));
    if !source_ok || !strengths_ok {
        return Err(RuleError::BadGuide { code: c.code.clone() });
    }
    let mut situations = HashSet::new();
    let mut ids = HashSet::new();
    for guide in &c.guide {
        let shaped =
            !guide.id.trim().is_empty() && !guide.short.trim().is_empty() && !guide.situation.trim().is_empty();
        if !shaped || !ids.insert(guide.id.as_str()) || !situations.insert(guide.situation.as_str()) {
            return Err(RuleError::BadSituation(c.code.clone()));
        }
    }
    if c.guide.iter().all(|guide| guide.strength.is_none()) {
        return Err(RuleError::GuideWithoutAnswer(c.code.clone()));
    }
    if HELPER_CODES.contains(&c.code.as_str()) {
        return Err(RuleError::GuideOnHelperCriterion(c.code.clone()));
    }
    Ok(())
}
