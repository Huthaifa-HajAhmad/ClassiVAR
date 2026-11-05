//! The small helpers that sit inside the criteria list: the predictor score
//! (PP3/BP4), the functional assay (PS3/BS3), family and phenotype points
//! (PP1/PP4), non-segregation (BS4) and the reason prompt for a raised strength.
//! Each one only fills in a criterion; the curator still decides.

use super::*;
use classivar_core::LocusRules;

/// What the boxes of the PP1/PP4 helper hold, read as points.
struct LocusInputs {
    pp1: f64,
    pp1_note: String,
    pp4: f64,
    pp4_note: String,
    /// What each box added, in words, for the reason that is saved with the case.
    parts: Vec<String>,
    /// The yield box or the family count box has something typed in it.
    typed_any: bool,
    /// A box holds something that cannot be used. No reading is made then.
    unusable: bool,
    /// PP1 counts only when several genes can cause the phenotype.
    pp1_allowed: bool,
}

impl Workbench {
    /// Turns a prediction tool's score into PP3 or BP4 with the strength the
    /// published calibration gives. It only fills in the criterion; the curator
    /// still presses Apply.
    pub(super) fn predictor_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let open = self.predictor_open;
        let mut panel =
            div().p_3().rounded_md().border_1().border_color(border).bg(background).flex().flex_col().gap_2();
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child("Predictor score → PP3 or BP4"))
                .child(
                    Button::new("predictor-toggle")
                        .label(if open { "Close" } else { "Use a score" })
                        .accessibility_label(if open {
                            "Close the predictor score helper"
                        } else {
                            "Open the predictor score helper"
                        })
                        .small()
                        .outline()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.predictor_open = !this.predictor_open;
                            cx.notify();
                        })),
                ),
        );
        if !open {
            return panel.into_any_element();
        }

        let mut tools =
            div().flex().flex_wrap().items_center().gap_2().child(div().text_xs().text_color(muted).child("Tool"));
        for (i, tool) in self.rules.predictors.iter().enumerate() {
            tools = tools.child(
                Button::new(SharedString::from(format!("predictor-tool-{}", tool.id)))
                    .label(tool.name.clone())
                    .small()
                    .selected(self.predictor_tool == i)
                    .toggled(self.predictor_tool == i)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.predictor_tool = i;
                        cx.notify();
                    })),
            );
        }
        let Some(tool) = self.rules.predictors.get(self.predictor_tool) else {
            return panel.into_any_element();
        };
        panel = panel.child(tools).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().text_xs().text_color(muted).child("Score"))
                .child(div().w(px(140.)).child(Input::new(&self.predictor_input))),
        );

        let (message, reading) = match self.typed_score(cx) {
            None => ("Type the score the tool gave.".to_string(), None),
            Some(Err(())) => ("That is not a number. Use digits and a point, for example 0.85.".to_string(), None),
            Some(Ok(score)) => match self.rules.read_predictor(&tool.id, score) {
                None => (
                    format!("{} {score} lies between the limits: this tool gives neither PP3 nor BP4.", tool.name),
                    None,
                ),
                Some(reading) => {
                    let mut text = format!(
                        "{} {score}: the paper rates this {} for {}.",
                        tool.name,
                        reading.published.label(),
                        reading.code
                    );
                    if reading.published != reading.strength {
                        text.push_str(&format!(
                            " This program stops {} at {}, so it applies {}.",
                            reading.code,
                            reading.strength.label(),
                            reading.strength.label()
                        ));
                    }
                    (text, Some(reading))
                }
            },
        };
        panel = panel.child(div().text_sm().child(message));
        if let Some(reading) = reading {
            let label = format!("Apply {} {}", reading.code, reading.strength.label());
            let score = self.predictor_input.read(cx).value().trim().to_string();
            let reason = format!(
                "{} score {score}: {} for {} (Pejaver 2022, Table 2)",
                tool.name,
                reading.published.label(),
                reading.code
            );
            panel = panel.child(
                div().child(
                    Button::new("predictor-apply")
                        .label(label.clone())
                        .accessibility_label(format!("{label} to today's evidence"))
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_reading(&reading, reason.clone());
                            cx.notify();
                        })),
                ),
            );
        }
        let citation = self.rules.source(&tool.source).map(|s| s.citation.clone());
        panel
            .when_some(tool.note.clone(), |d, note| d.child(div().text_xs().text_color(muted).child(note)))
            .child(div().text_xs().text_color(muted).child(
                "Choose one tool before you see its result, and use it for the whole case. The tool, score and source are saved with the case as the reason for the strength.",
            ))
            .when_some(citation, |d, citation| {
                d.child(div().text_xs().text_color(muted).child(format!("Source: {citation}")))
            })
            .into_any_element()
    }

    /// Turns functional-assay facts into PS3 or BS3 at the strength Brnich 2019
    /// gives. Like the predictor helper, it only fills in the criterion.
    pub(super) fn functional_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let open = self.functional_open;
        let mut panel =
            div().p_3().rounded_md().border_1().border_color(border).bg(background).flex().flex_col().gap_2();
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child("Functional assay → PS3 or BS3"),
                )
                .child(
                    Button::new("functional-toggle")
                        .label(if open { "Close" } else { "Use an assay" })
                        .accessibility_label(if open {
                            "Close the functional evidence helper"
                        } else {
                            "Open the functional evidence helper"
                        })
                        .small()
                        .outline()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.functional_open = !this.functional_open;
                            cx.notify();
                        })),
                ),
        );
        let Some(rules) = self.rules.functional.as_ref().filter(|_| open) else {
            return panel.into_any_element();
        };

        let route_button = |id: &'static str, label: &'static str, route: AssayRoute, cx: &mut Context<Self>| {
            Button::new(id)
                .label(label)
                .small()
                .selected(self.functional_route == route)
                .toggled(self.functional_route == route)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.functional_route = route;
                    cx.notify();
                }))
        };
        panel = panel
            .child(div().text_xs().text_color(muted).child(
                "First do the paper's four steps yourself: define the disease mechanism, judge the class of assay, judge this assay, then apply it to the variant.",
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(muted).child("What is known"))
                    .child(route_button("functional-route-controls", "Number of controls", AssayRoute::Controls, cx))
                    .child(route_button("functional-route-oddspath", "OddsPath value", AssayRoute::OddsPath, cx)),
            );

        let (message, reading) = match self.functional_route {
            AssayRoute::Controls => {
                let direction_button =
                    |id: &'static str, label: &'static str, damaging: bool, cx: &mut Context<Self>| {
                        Button::new(id)
                            .label(label)
                            .small()
                            .selected(self.functional_damaging == damaging)
                            .toggled(self.functional_damaging == damaging)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.functional_damaging = damaging;
                                cx.notify();
                            }))
                    };
                panel = panel
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().text_color(muted).child("The assay showed"))
                            .child(direction_button("functional-damage", "Damage (PS3)", true, cx))
                            .child(direction_button("functional-no-damage", "No damage (BS3)", false, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().text_color(muted).child("Validation controls"))
                            .child(div().w(px(120.)).child(Input::new(&self.controls_input))),
                    );
                match self.typed_number(&self.controls_input, cx) {
                    None => (
                        "Type how many validation controls the assay used (pathogenic and benign variants together)."
                            .to_string(),
                        None,
                    ),
                    Some(Ok(n)) if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) => {
                        let controls = n as u32;
                        match self.rules.read_functional_controls(self.functional_damaging, controls) {
                            Some(reading) => (
                                format!(
                                    "{controls} controls, no statistical analysis: {} for {}. {} or more controls reach Moderate; fewer reach Supporting; this route goes no higher.",
                                    reading.strength.label(),
                                    reading.code,
                                    rules.controls_moderate_min
                                ),
                                Some(reading),
                            ),
                            None if controls < rules.controls_supporting_min => (
                                "An assay with no validation controls gives no evidence (Brnich 2019).".to_string(),
                                None,
                            ),
                            None => ("The rules have no functional evidence table.".to_string(), None),
                        }
                    }
                    Some(_) => ("Use a whole number of controls, for example 11.".to_string(), None),
                }
            }
            AssayRoute::OddsPath => {
                panel = panel.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_xs().text_color(muted).child("OddsPath"))
                        .child(div().w(px(120.)).child(Input::new(&self.oddspath_input))),
                );
                match self.typed_number(&self.oddspath_input, cx) {
                    None => ("Type the OddsPath value from the assay's statistical analysis.".to_string(), None),
                    Some(Err(())) => {
                        ("That is not a number. Use digits and a point, for example 5.2.".to_string(), None)
                    }
                    Some(Ok(value)) => match self.rules.read_oddspath(value) {
                        Some(reading) => (
                            format!("OddsPath {value}: {} for {}.", reading.strength.label(), reading.code),
                            Some(reading),
                        ),
                        None => (
                            format!(
                                "OddsPath {value} is not a valid value, or lies in the range with no evidence (between {} and {}).",
                                rules.benign.first().map_or(0.0, |c| c.at),
                                rules.pathogenic.first().map_or(0.0, |c| c.at)
                            ),
                            None,
                        ),
                    },
                }
            }
        };
        panel = panel.child(div().text_sm().child(message));
        if let Some(reading) = reading {
            let label = format!("Apply {} {}", reading.code, reading.strength.label());
            let reason = match self.functional_route {
                AssayRoute::Controls => format!(
                    "Functional assay with {} validation controls, no statistical analysis (Brnich 2019)",
                    self.controls_input.read(cx).value().trim()
                ),
                AssayRoute::OddsPath => {
                    format!("OddsPath {} (Brnich 2019)", self.oddspath_input.read(cx).value().trim())
                }
            };
            if reading.code == "PS3" && reading.strength == Strength::VeryStrong {
                panel = panel.child(div().text_xs().text_color(muted).child(
                    "Even Very strong functional evidence is not enough alone for a Pathogenic class (Brnich 2019). The program does not check this for you.",
                ));
            }
            panel = panel.child(
                div().child(
                    Button::new("functional-apply")
                        .label(label.clone())
                        .accessibility_label(format!("{label} to today's evidence"))
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_reading(&reading, reason.clone());
                            cx.notify();
                        })),
                ),
            );
        }
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        panel
            .when_some(citation, |d, citation| {
                d.child(div().text_xs().text_color(muted).child(format!("Source: {citation}")))
            })
            .into_any_element()
    }

    /// The frame of a helper panel: its title and the button that opens and closes it.
    /// The two newest helpers share it; `toggle` flips the helper's open flag.
    fn helper_start(
        &self,
        ids: (&'static str, &'static str),
        label: &'static str,
        what: &'static str,
        open: bool,
        toggle: fn(&mut Self),
        cx: &mut Context<Self>,
    ) -> Div {
        let (border, background) = (cx.theme().border, cx.theme().background);
        let (button_id, title) = ids;
        div().p_3().rounded_md().border_1().border_color(border).bg(background).flex().flex_col().gap_2().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(
                    Button::new(button_id)
                        .label(if open { "Close" } else { label })
                        .accessibility_label(if open {
                            format!("Close the {what}")
                        } else {
                            format!("Open the {what}")
                        })
                        .small()
                        .outline()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            toggle(this);
                            cx.notify();
                        })),
                ),
        )
    }

    /// Turns family and phenotype facts into PP1 and PP4 strengths with the
    /// arithmetic of ClinGen 2024. It lists the choices the paper allows for the
    /// total; the curator picks one. Like the other helpers, it only fills in criteria.
    pub(super) fn locus_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let panel = self.helper_start(
            ("locus-toggle", "Family and phenotype → PP1 and PP4"),
            "Use points",
            "co-segregation and phenotype helper",
            self.locus_open,
            |this| this.locus_open = !this.locus_open,
            cx,
        );
        let Some(rules) = self.rules.locus_evidence.as_ref().filter(|_| self.locus_open) else {
            return panel.into_any_element();
        };
        let inputs = self.read_locus_inputs(rules, cx);
        panel
            .child(self.locus_phenotype_part(rules, &inputs, cx))
            .child(self.locus_family_part(rules, &inputs, cx))
            .child(self.locus_result(rules, &inputs, cx))
            .into_any_element()
    }

    /// The PP4 half: the diagnostic yield, how many variants share the allele, and the paper's cautions.
    fn locus_phenotype_part(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let row = |label: &'static str, input: &Entity<InputState>| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().text_xs().text_color(muted).child(label))
                .child(div().w(px(120.)).child(Input::new(input)))
        };
        let mut part = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().text_xs().text_color(muted).child(
                    "Fill in one or both parts. An empty box counts as 0 points. The paper divides the yield, never the points, when several variants share an allele.",
                ),
            )
            .child(row("Diagnostic yield of testing for the phenotype (%)", &self.yield_input))
            .child(row("Variants on the same allele (1 if alone)", &self.variants_input))
            .child(div().text_sm().child(inputs.pp4_note.clone()));
        for note in &rules.notes {
            part = part.child(div().text_xs().text_color(muted).child(note.clone()));
        }
        part
    }

    /// The PP1 half: how many genes can cause the phenotype (it decides whether PP1 counts),
    /// the kind of family, and how many informative members there are.
    fn locus_family_part(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let genes_button = |id: &'static str, label: &'static str, genes: PhenotypeGenes, cx: &mut Context<Self>| {
            Button::new(id)
                .label(label)
                .small()
                .selected(self.locus_genes == genes)
                .toggled(self.locus_genes == genes)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.locus_genes = genes;
                    cx.notify();
                }))
        };
        let mut kinds =
            div().flex().flex_wrap().items_center().gap_2().child(div().text_xs().text_color(muted).child("Family"));
        for (i, kind) in rules.segregation.iter().enumerate() {
            kinds = kinds.child(
                Button::new(SharedString::from(format!("locus-kind-{}", kind.id)))
                    .label(kind.label.clone())
                    .small()
                    .selected(self.locus_kind == i)
                    .toggled(self.locus_kind == i)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.locus_kind = i;
                        cx.notify();
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(muted).child("Can other genes cause this phenotype?"))
                    .child(genes_button(
                        "locus-genes-several",
                        "Several genes can cause it",
                        PhenotypeGenes::Several,
                        cx,
                    ))
                    .child(genes_button("locus-genes-one", "Only this gene causes it", PhenotypeGenes::One, cx)),
            )
            .when(self.locus_genes == PhenotypeGenes::One, |d| {
                d.child(div().text_xs().text_color(muted).child(rules.homogeneity_text.clone()))
            })
            .child(kinds)
            .when_some(rules.segregation[self.locus_kind].note.clone(), |d, note| {
                d.child(div().text_xs().text_color(muted).child(note))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(muted).child("Informative family members (see the note)"))
                    .child(div().w(px(120.)).child(Input::new(&self.segregations_input))),
            )
            .child(div().text_sm().child(inputs.pp1_note.clone()))
    }

    /// The total, the choices the paper allows for it, and the source.
    fn locus_result(&self, rules: &LocusRules, inputs: &LocusInputs, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let mut result = div().flex().flex_col().gap_2();
        let reading = (!inputs.unusable && inputs.typed_any)
            .then(|| self.rules.read_locus(inputs.pp1, inputs.pp4, inputs.pp1_allowed))
            .flatten();
        if let Some(reading) = reading {
            let mut summary = format!("Total {} points for PP1 and PP4 together.", reading.total);
            let mut counted = format!("total {} points", reading.total);
            if reading.capped {
                summary.push_str(&format!(
                    " The sum, {}, is above the limit of {} points, so it counts as {}.",
                    reading.sum, rules.cap_points, rules.cap_points
                ));
                counted = format!("sum {} points, counted as {} (the limit)", reading.sum, rules.cap_points);
            }
            if !inputs.pp1_allowed {
                summary.push_str(" PP1 is not counted, so only PP4 choices are listed.");
            }
            if reading.choices.is_empty() {
                let first = rules.bands.first().map_or(1.0, |band| band.min_points);
                summary.push_str(&format!(" The paper gives no evidence below {first} point."));
            } else {
                summary.push_str(
                    " Pick one of the choices the paper allows. A criterion a choice does not name is left as it is.",
                );
            }
            result = result.child(div().text_sm().child(summary));
            let reason = format!("{}; {counted} ({}, Tables 2 to 4)", inputs.parts.join("; "), rules.source);
            let mut choices = div().flex().flex_wrap().gap_2();
            for choice in reading.choices {
                let slug = |strength: Option<Strength>| {
                    strength.map_or("none".to_string(), |s| s.label().to_lowercase().replace(' ', "-"))
                };
                let label = [("PP1", choice.pp1), ("PP4", choice.pp4)]
                    .into_iter()
                    .filter_map(|(code, strength)| strength.map(|s| format!("{code} {}", s.label())))
                    .collect::<Vec<_>>()
                    .join(" + ");
                let reason = reason.clone();
                choices = choices.child(
                    Button::new(SharedString::from(format!("locus-choice-{}-{}", slug(choice.pp1), slug(choice.pp4))))
                        .label(format!("Apply {label}"))
                        .accessibility_label(format!("Apply {label} to today's evidence"))
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_locus_choice(choice, reason.clone());
                            cx.notify();
                        })),
                );
            }
            result = result.child(choices);
        } else if !inputs.unusable {
            result = result.child(div().text_sm().child("Fill in at least one box to see the points."));
        }
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        result.when_some(citation, |d, citation| {
            d.child(div().text_xs().text_color(muted).child(format!("Source: {citation}")))
        })
    }

    /// Asks which situation fits and applies BS4 only where ClinGen 2024 supports
    /// it. Where it does not, the panel says why. Like the other helpers, it only
    /// fills in the criterion.
    pub(super) fn non_segregation_helper(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let panel = self.helper_start(
            ("segregation-toggle", "Non-segregation → BS4"),
            "Check the situation",
            "non-segregation helper",
            self.segregation_open,
            |this| this.segregation_open = !this.segregation_open,
            cx,
        );
        let Some(rules) = self.rules.non_segregation.as_ref().filter(|_| self.segregation_open) else {
            return panel.into_any_element();
        };

        let mut situations = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().text_xs().text_color(muted).child("What fits the family"));
        for (i, situation) in rules.situations.iter().enumerate() {
            situations = situations.child(
                Button::new(SharedString::from(format!("segregation-situation-{}", situation.id)))
                    .label(situation.label.clone())
                    .small()
                    .selected(self.segregation_situation == i)
                    .toggled(self.segregation_situation == i)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.segregation_situation = i;
                        cx.notify();
                    })),
            );
        }
        let situation = &rules.situations[self.segregation_situation];
        let mut panel = panel.child(situations).child(div().text_sm().child(situation.text.clone()));
        if let Some(reading) = self.rules.read_non_segregation(&situation.id) {
            let label = format!("Apply {} {}", reading.code, reading.strength.label());
            let reason = format!(
                "Non-segregation: {} ({}, −{} points)",
                situation.label,
                rules.source,
                self.rules.magnitude(reading.strength)
            );
            panel = panel.child(
                div().child(
                    Button::new("segregation-apply")
                        .label(label.clone())
                        .accessibility_label(format!("{label} to today's evidence"))
                        .small()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.apply_reading(&reading, reason.clone());
                            cx.notify();
                        })),
                ),
            );
        }
        let citation = self.rules.source(&rules.source).map(|s| s.citation.clone());
        panel
            .when_some(citation, |d, citation| {
                d.child(div().text_xs().text_color(muted).child(format!("Source: {citation}")))
            })
            .into_any_element()
    }

    /// Reads the number boxes of the PP1/PP4 helper as points. A box with something
    /// unusable in it stops the reading; an empty box counts as zero. PP1 counts
    /// only when the curator said several genes can cause the phenotype, and not
    /// when several variants share the allele (the paper's sharing worksheet is not built in).
    fn read_locus_inputs(&self, rules: &LocusRules, cx: &App) -> LocusInputs {
        let mut out = LocusInputs {
            pp1: 0.0,
            pp1_note: String::new(),
            pp4: 0.0,
            pp4_note: String::new(),
            parts: Vec::new(),
            typed_any: false,
            unusable: false,
            pp1_allowed: self.locus_genes == PhenotypeGenes::Several,
        };
        let typed_yield = self.typed_number(&self.yield_input, cx);
        let typed_count = self.typed_number(&self.segregations_input, cx);
        out.typed_any = typed_yield.is_some() || typed_count.is_some();

        // Variants on the allele: an empty box means one.
        let variants = match self.typed_number(&self.variants_input, cx) {
            None => Some(1),
            Some(Ok(n)) if n >= 1.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) => Some(n as u32),
            Some(_) => None,
        };
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
                match self.rules.pp4_points(percent, variants) {
                    Some(points) => {
                        out.pp4 = points;
                        out.parts.push(format!("diagnostic yield {percent}%{sharing} = {points} points for PP4"));
                        let mut note = format!(
                            "A yield of {percent}%{sharing} earns {points} points for PP4 (a yield between two rows of the table takes the lower row)."
                        );
                        if shared < rules.yield_text_minimum {
                            note.push_str(&format!(
                                " Careful: the paper's text asks for a yield over {}%; its table starts lower.",
                                rules.yield_text_minimum
                            ));
                        }
                        note
                    }
                    None => format!(
                        "A yield of {percent}%{sharing} is below the first row of the table ({}%): no PP4 points.",
                        rules.pp4_yield.first().map_or(0.0, |step| step.at)
                    ),
                }
            }
            (Some(_), Some(_)) => {
                out.unusable = true;
                "Type a percentage between 0 and 100, for example 60.".to_string()
            }
        };

        let kind = &rules.segregation[self.locus_kind];
        out.pp1_note = match typed_count {
            None => "Leave the count empty if there is no family evidence.".to_string(),
            Some(Ok(n)) if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) => {
                let count = n as u32;
                if count == 0 {
                    "0 family members: no PP1 points.".to_string()
                } else if self.locus_genes == PhenotypeGenes::Unanswered {
                    out.unusable = true;
                    "Answer the question about other genes first: the paper does not allow PP1 when only one gene can cause the phenotype.".to_string()
                } else if !out.pp1_allowed {
                    "PP1 is not counted: only this gene causes the phenotype (see the paper's rule above).".to_string()
                } else if variants.is_some_and(|v| v > 1) {
                    out.unusable = true;
                    "The paper shares PP1 points among variants on one allele with a worksheet (its Table S1) that this program does not have. Do not divide the points yourself. Set the variants to 1 or leave the family count empty.".to_string()
                } else {
                    // The kind comes from the same rules, so it always has points.
                    let points = self.rules.pp1_points(&kind.id, count).unwrap_or_default();
                    out.pp1 = points;
                    out.parts.push(format!("{count} co-segregations ({}) = {points} points for PP1", kind.label));
                    format!("{count} co-segregations, {}: {points} points for PP1.", kind.label)
                }
            }
            Some(_) => {
                out.unusable = true;
                "Use a whole number of family members, for example 3.".to_string()
            }
        };
        out
    }

    /// Asks why a strength is raised above its default. The raise is only made
    /// once a reason is written; Cancel (or Escape) leaves the strength as it was.
    pub(super) fn raise_prompt(&self, c: &Criterion, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let to = self.raising.as_ref().map_or(c.strength, |raise| raise.to);
        let empty = self.reason_input.read(cx).value().trim().is_empty();
        div()
            .mt_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(background)
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!(
                "Why raise {} from {} (the default) to {}?",
                c.code,
                c.strength.label(),
                to.label()
            )))
            .child(div().text_xs().text_color(muted).child(
                "A stronger strength than the guideline default needs a reason. It is saved with the case and shown next to the criterion.",
            ))
            .child(Input::new(&self.reason_input))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new(SharedString::from(format!("raise-save-{}", c.code)))
                            .label("Save reason and raise")
                            .accessibility_label(format!("Save the reason and raise {} to {}", c.code, to.label()))
                            .small()
                            .disabled(empty)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.confirm_raise(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("raise-cancel-{}", c.code)))
                            .label("Cancel")
                            .accessibility_label(format!("Do not raise {}", c.code))
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.raising = None;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }
}
