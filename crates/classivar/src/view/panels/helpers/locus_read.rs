//! What the PP1/PP4 helper makes of what is typed: points, notes and the saved reason.
//! No drawing here, so the numbers can be tested without a window.

use super::*;
use classivar_core::{LocusReading, LocusRules};

/// What the boxes and buttons of the helper hold, before any reading.
pub(super) struct LocusTyped {
    /// The diagnostic yield in percent (see `parse_number`).
    pub(super) yield_percent: Option<Result<f64, ()>>,
    /// Informative family members (see `parse_count`).
    pub(super) family: Option<Result<u32, ()>>,
    /// Variants sharing the allele (see `parse_count`).
    pub(super) variants: Option<Result<u32, ()>>,
    pub(super) genes: PhenotypeGenes,
    /// Which kind of `LocusRules::segregation` the family count belongs to.
    pub(super) kind: usize,
}

/// What the boxes of the PP1/PP4 helper hold, read as points.
pub(super) struct LocusInputs {
    pub(super) pp1: f64,
    pub(super) pp1_note: String,
    pub(super) pp4: f64,
    pub(super) pp4_note: String,
    /// What each box added, in words, for the reason that is saved with the case.
    pub(super) parts: Vec<String>,
    /// The yield box or the family count box has something typed in it.
    pub(super) typed_any: bool,
    /// A box holds something that cannot be used. No reading is made then.
    pub(super) unusable: bool,
    /// PP1 counts only when several genes can cause the phenotype.
    pub(super) pp1_allowed: bool,
}

/// Reads the typed values as points. A box with something unusable in it stops the
/// reading; an empty box counts as zero. PP1 counts only when the curator said
/// several genes can cause the phenotype, and not when several variants share the
/// allele (the paper's sharing worksheet is not built in).
pub(super) fn read_locus_inputs(rules: &RuleSet, table: &LocusRules, typed: &LocusTyped) -> LocusInputs {
    let mut out = LocusInputs {
        pp1: 0.0,
        pp1_note: String::new(),
        pp4: 0.0,
        pp4_note: String::new(),
        parts: Vec::new(),
        typed_any: typed.yield_percent.is_some() || typed.family.is_some(),
        unusable: false,
        pp1_allowed: typed.genes == PhenotypeGenes::Several,
    };
    // Variants on the allele: an empty box means one, and zero is not a number of variants.
    let variants = match typed.variants {
        None => Some(1),
        Some(Ok(n)) if n >= 1 => Some(n),
        Some(_) => None,
    };
    read_pp4(rules, table, typed.yield_percent, variants, &mut out);
    read_pp1(rules, table, typed, variants, &mut out);
    out
}

fn read_pp4(
    rules: &RuleSet,
    table: &LocusRules,
    typed_yield: Option<Result<f64, ()>>,
    variants: Option<u32>,
    out: &mut LocusInputs,
) {
    out.pp4_note = match (typed_yield, variants) {
        (_, None) => {
            out.unusable = true;
            "Variants on the same allele: use a whole number, 1 or more.".to_string()
        }
        (None, Some(_)) => "Leave the yield empty if there is no phenotype evidence.".to_string(),
        (Some(Ok(percent)), Some(variants)) if (0.0..=100.0).contains(&percent) => {
            let shared = percent / f64::from(variants);
            let sharing = if variants > 1 {
                format!(" shared by {variants} variants ({shared:.1}% each)")
            } else {
                String::new()
            };
            match rules.pp4_points(percent, variants) {
                Some(points) => {
                    out.pp4 = points;
                    out.parts.push(format!("diagnostic yield {percent}%{sharing} = {points} points for PP4"));
                    let mut note = format!(
                        "A yield of {percent}%{sharing} earns {points} points for PP4 (a yield between two rows of the table takes the lower row)."
                    );
                    if shared < table.yield_text_minimum {
                        note.push_str(&format!(
                            " Careful: the paper's text asks for a yield over {}%; its table starts lower.",
                            table.yield_text_minimum
                        ));
                    }
                    note
                }
                None => format!(
                    "A yield of {percent}%{sharing} is below the first row of the table ({}%): no PP4 points.",
                    table.pp4_yield.first().map_or(0.0, |step| step.at)
                ),
            }
        }
        (Some(_), Some(_)) => {
            out.unusable = true;
            "Type a percentage between 0 and 100, for example 60.".to_string()
        }
    };
}

fn read_pp1(rules: &RuleSet, table: &LocusRules, typed: &LocusTyped, variants: Option<u32>, out: &mut LocusInputs) {
    let kind = &table.segregation[typed.kind];
    out.pp1_note = match typed.family {
        None => "Leave the count empty if there is no family evidence.".to_string(),
        Some(Err(())) => {
            out.unusable = true;
            "Use a whole number of family members, for example 3.".to_string()
        }
        Some(Ok(0)) => "0 family members: no PP1 points.".to_string(),
        Some(Ok(_)) if typed.genes == PhenotypeGenes::Unanswered => {
            out.unusable = true;
            "Answer the question about other genes first: the paper does not allow PP1 when only one gene can cause the phenotype.".to_string()
        }
        Some(Ok(_)) if !out.pp1_allowed => {
            "PP1 is not counted: only this gene causes the phenotype (see the paper's rule above).".to_string()
        }
        Some(Ok(_)) if variants.is_some_and(|v| v > 1) => {
            out.unusable = true;
            "The paper shares PP1 points among variants on one allele with a worksheet (its Table S1) that this program does not have. Do not divide the points yourself. Set the variants to 1 or leave the family count empty.".to_string()
        }
        Some(Ok(count)) => {
            // The kind comes from the same rules, so it always has points.
            let points = rules.pp1_points(&kind.id, count).unwrap_or_default();
            out.pp1 = points;
            out.parts.push(format!("{count} co-segregations ({}) = {points} points for PP1", kind.label));
            format!("{count} co-segregations, {}: {points} points for PP1.", kind.label)
        }
    };
}

/// The sentence under the total, and the reason saved with whichever choice the curator applies.
pub(super) fn locus_summary(table: &LocusRules, reading: &LocusReading, inputs: &LocusInputs) -> (String, String) {
    let mut summary = format!("Total {} points for PP1 and PP4 together.", reading.total);
    let mut counted = format!("total {} points", reading.total);
    if reading.capped {
        summary.push_str(&format!(
            " The sum, {}, is above the limit of {} points, so it counts as {}.",
            reading.sum, table.cap_points, table.cap_points
        ));
        counted = format!("sum {} points, counted as {} (the limit)", reading.sum, table.cap_points);
    }
    if !inputs.pp1_allowed {
        summary.push_str(" PP1 is not counted, so only PP4 choices are listed.");
    }
    if reading.choices.is_empty() {
        let first = table.bands.first().map_or(1.0, |band| band.min_points);
        summary.push_str(&format!(" The paper gives no evidence below {first} point."));
    } else {
        summary.push_str(
            " Pick one of the choices the paper allows. A criterion a choice does not name is left as it is.",
        );
    }
    let reason = format!("{}; {counted} ({}, Tables 2 to 4)", inputs.parts.join("; "), table.source);
    (summary, reason)
}

#[cfg(test)]
mod tests {
    use classivar_core::RuleSet;

    use super::{LocusInputs, LocusTyped, PhenotypeGenes, locus_summary, read_locus_inputs};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn read(typed: LocusTyped) -> LocusInputs {
        read_locus_inputs(rules(), rules().locus_evidence.as_ref().unwrap(), &typed)
    }

    fn typed() -> LocusTyped {
        LocusTyped { yield_percent: None, family: None, variants: None, genes: PhenotypeGenes::Several, kind: 0 }
    }

    #[test]
    fn empty_boxes_read_as_nothing_typed() {
        let out = read(typed());
        assert!(!out.typed_any);
        assert!(!out.unusable);
        assert_eq!((out.pp1, out.pp4), (0.0, 0.0));
    }

    #[test]
    fn a_yield_earns_points_and_a_bad_one_stops_the_reading() {
        let out = read(LocusTyped { yield_percent: Some(Ok(60.0)), ..typed() });
        assert!(out.pp4 > 0.0 && !out.unusable, "{}", out.pp4_note);
        assert_eq!(out.parts.len(), 1);
        assert!(read(LocusTyped { yield_percent: Some(Ok(120.0)), ..typed() }).unusable);
        assert!(read(LocusTyped { yield_percent: Some(Err(())), ..typed() }).unusable);
        assert!(read(LocusTyped { variants: Some(Ok(0)), ..typed() }).unusable, "zero variants is not a count");
    }

    #[test]
    fn variants_sharing_the_allele_divide_the_yield_and_say_so() {
        let alone = read(LocusTyped { yield_percent: Some(Ok(90.0)), ..typed() });
        let shared = read(LocusTyped { yield_percent: Some(Ok(90.0)), variants: Some(Ok(3)), ..typed() });
        assert!(shared.pp4 < alone.pp4);
        assert!(shared.pp4_note.contains("shared by 3 variants"), "{}", shared.pp4_note);
    }

    #[test]
    fn pp1_needs_the_other_genes_question_answered_and_one_variant() {
        let family = Some(Ok(3));
        let unanswered = read(LocusTyped { family, genes: PhenotypeGenes::Unanswered, ..typed() });
        assert!(unanswered.unusable);
        let one_gene = read(LocusTyped { family, genes: PhenotypeGenes::One, ..typed() });
        assert!(!one_gene.unusable && one_gene.pp1 == 0.0 && one_gene.pp1_note.contains("not counted"));
        let shared = read(LocusTyped { family, variants: Some(Ok(2)), ..typed() });
        assert!(shared.unusable, "the paper's sharing worksheet is not built in");
        let counted = read(LocusTyped { family, ..typed() });
        assert!(counted.pp1 > 0.0 && !counted.unusable, "{}", counted.pp1_note);
        assert!(counted.parts[0].contains("co-segregations"));
        assert_eq!(read(LocusTyped { family: Some(Ok(0)), ..typed() }).pp1, 0.0);
    }

    #[test]
    fn the_summary_says_when_the_cap_applied_and_what_is_saved() {
        let table = rules().locus_evidence.as_ref().unwrap();
        let out = read(LocusTyped { yield_percent: Some(Ok(99.9)), family: Some(Ok(30)), ..typed() });
        let reading = rules().read_locus(out.pp1, out.pp4, out.pp1_allowed).unwrap();
        let (summary, reason) = locus_summary(table, &reading, &out);
        assert!(reading.capped);
        assert!(summary.contains("above the limit"), "{summary}");
        assert!(reason.contains("(the limit)"), "{reason}");
        assert!(reason.contains(&table.source), "{reason}");
    }
}
