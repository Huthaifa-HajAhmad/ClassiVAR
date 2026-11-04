//! The small helpers that sit inside the criteria list: the predictor score
//! (PP3/BP4), the functional assay (PS3/BS3) and the reason prompt for a raised
//! strength. Each one only fills in a criterion; the curator still decides.

use super::*;

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
