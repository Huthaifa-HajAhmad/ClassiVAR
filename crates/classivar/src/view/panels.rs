//! The parts of the main screen, drawn from the state kept in `view.rs`.
//!
//! The screen has four areas: a top bar (what this is, which mode, save and open),
//! a case panel (variant, gene, gene sources, earlier records), the criteria list on
//! the left, and the result on the right. The result is the first thing the eye
//! should land on, so it gets the largest type and the strongest colour on screen.
//! Yellow is kept for things that block a result; everything else is quiet.
//!
//! Most functions here return an `AnyElement` (a few small helpers return a `Div`
//! or a `String`). Keeping each one small also keeps the stack use of a debug
//! build low (a single huge `render` once overflowed the stack and closed the app).

use super::*;

mod helpers;

/// A section heading in the results pane.
fn heading(text: impl Into<SharedString>) -> Div {
    div().text_sm().font_weight(FontWeight::SEMIBOLD).pb_2().child(text.into())
}

impl Workbench {
    // ---------- top bar ----------

    pub(super) fn top_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (border, muted, background) = (theme.border, theme.muted_foreground, theme.background);
        let dark = theme.is_dark();
        let mode_button = |id: &'static str, label: &'static str, mode: Mode, cx: &mut Context<Self>| {
            Button::new(id).label(label).small().selected(self.mode == mode).toggled(self.mode == mode).on_click(
                cx.listener(move |this, _, _, cx| {
                    this.set_mode(mode);
                    cx.notify();
                }),
            )
        };

        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_4()
            .px_5()
            .py_3()
            .border_b_1()
            .border_color(border)
            .bg(background)
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("ClassiVAR"))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(mode_button("mode-new", "New variant", Mode::NewVariant, cx))
                    .child(mode_button("mode-reported", "Reported variant", Mode::ReportedVariant, cx)),
            )
            .child(div().flex_1())
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .text_xs()
                    .text_color(muted)
                    .child(format!("{} · research and learning only", rules_tag(self.rules))),
            )
            .child(Button::new("save-case").label("Save case…").small().outline().on_click(cx.listener(
                |this, _, _, cx| {
                    this.save_case(cx);
                },
            )))
            .child(Button::new("open-case").label("Open case…").small().outline().on_click(cx.listener(
                |this, _, window, cx| {
                    this.open_case(window, cx);
                },
            )))
            .child(
                Button::new("theme-toggle")
                    .label(if dark { "Light theme" } else { "Dark theme" })
                    .accessibility_label(if dark {
                        "Theme: dark. Switch to light."
                    } else {
                        "Theme: light. Switch to dark."
                    })
                    .toggled(dark)
                    .small()
                    .outline()
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_theme(window, cx))),
            )
            .child(
                Button::new("learning")
                    .label(if self.learning { "Learning on" } else { "Learning off" })
                    .small()
                    .selected(self.learning)
                    .toggled(self.learning)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.learning = !this.learning;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    // ---------- case panel: variant, gene, sources, records ----------

    pub(super) fn case_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (border, muted, band) = (theme.border, theme.muted_foreground, theme.background);
        let label =
            |text: &'static str| div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child(text);

        let mut decision = div().flex().items_center().gap_2().child(label("Gene causes this disease?"));
        for (choice, text) in [
            (GeneDecision::Established, "Yes"),
            (GeneDecision::NotEstablished, "Doubtful"),
            (GeneDecision::NotDecided, "Not decided"),
        ] {
            decision = decision.child(
                Button::new(SharedString::from(format!("gene-decision-{text}")))
                    .label(text)
                    .small()
                    .selected(self.gene_link == choice)
                    .toggled(self.gene_link == choice)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.gene_link = choice;
                        cx.notify();
                    })),
            );
        }

        let lookups_started = !matches!(self.gene.clingen, Lookup::Idle);
        let mut panel = div().flex().flex_col().gap_3().px_5().py_3().bg(band).border_b_1().border_color(border).child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(label("Variant"))
                .child(div().w(px(300.)).child(Input::new(&self.variant_input)))
                .child(label("Gene"))
                .child(div().w(px(200.)).child(Input::new(&self.gene_input)))
                .child(Button::new("gene-check").label("Check gene").small().on_click(cx.listener(
                    |this, _, window, cx| {
                        this.check_gene(window, cx);
                    },
                )))
                .child(div().flex_1())
                .child(decision),
        );

        panel = panel.children(self.gene_suggestions(cx));
        if let Some(error) = &self.gene_error {
            panel = panel.child(notice(error.clone(), warning_fill(), warning_ink()));
        }

        if lookups_started {
            panel = panel.child(self.gene_summary(cx));
            if self.gene_cards_open {
                panel = panel.child(self.gene_cards(cx));
            }
        }
        if self.mode == Mode::ReportedVariant {
            panel = panel.child(self.records_bar(cx));
        }
        panel.into_any_element()
    }

    /// Symbols that start with what is in the gene box, so the curator can fill
    /// the field with one click. A typing aid only; it is not a gene check.
    fn gene_suggestions(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let typed = self.gene_input.read(cx).value();
        let matches = crate::genes::suggestions_for(&typed, 6);
        if matches.is_empty() {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().text_xs().text_color(muted).child("Fill in the gene box:"));
        for symbol in matches {
            row = row.child(
                Button::new(SharedString::from(format!("gene-suggestion-{symbol}")))
                    .label(symbol)
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.gene_input.update(cx, |input, cx| input.set_value(symbol, window, cx));
                        this.check_gene(window, cx);
                    })),
            );
        }
        row = row.child(
            div()
                .text_xs()
                .text_color(muted)
                .child("A short list for typing only, not a check that the gene is right."),
        );
        Some(row.into_any_element())
    }

    /// One short line per gene source, so the answer is visible without opening anything.
    fn source_chip<T>(
        label: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        border: Hsla,
    ) -> AnyElement {
        let chip = div().px_2().py_1().rounded_md().border_1().max_w(px(430.)).text_xs();
        match lookup {
            Lookup::Idle => div().into_any_element(),
            Lookup::Loading => chip.border_color(border).child(format!("{label}: checking…")).into_any_element(),
            Lookup::Failed(message) => chip
                .bg(warning_fill())
                .border_color(warning_fill())
                .text_color(warning_ink())
                .child(format!("{label}: could not check. {message}"))
                .into_any_element(),
            Lookup::Done(found) => {
                chip.border_color(border).child(format!("{label}: {}", describe(found).0)).into_any_element()
            }
        }
    }

    fn gene_summary(&self, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let g = &self.gene;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(Self::source_chip("ClinGen", &g.clingen, describe_clingen, border))
            .child(Self::source_chip("G2P", &g.g2p, describe_g2p, border))
            .child(Self::source_chip("PanelApp England", &g.england, describe_panelapp, border))
            .child(Self::source_chip("PanelApp Australia", &g.australia, describe_panelapp, border))
            .child(
                Button::new("gene-cards-toggle")
                    .label(if self.gene_cards_open { "Hide details" } else { "Details" })
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.gene_cards_open = !this.gene_cards_open;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("Sources are expert opinion about the gene, not your variant, and they can disagree."),
            )
            .into_any_element()
    }

    fn gene_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        let g = &self.gene;
        div()
            .id("gene-cards")
            .flex()
            .flex_wrap()
            .gap_2()
            // Four cards can be taller than the window; keep them in a short
            // scrolling strip so the criteria list and the result stay reachable.
            .max_h(px(260.))
            .overflow_y_scroll()
            .child(self.source_card("ClinGen gene-disease validity", &g.clingen, describe_clingen, cx))
            .child(self.source_card("Gene2Phenotype (G2P)", &g.g2p, describe_g2p, cx))
            .child(self.source_card("PanelApp England", &g.england, describe_panelapp, cx))
            .child(self.source_card("PanelApp Australia", &g.australia, describe_panelapp, cx))
            .into_any_element()
    }

    /// One source's full answer. A failure is shown as a failure, never as "nothing found".
    fn source_card<T>(
        &self,
        title: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let expanded = self.expanded_cards.contains(title);
        let content: AnyElement = match lookup {
            Lookup::Idle => div().into_any_element(),
            Lookup::Loading => div().text_sm().text_color(muted).child("Checking…").into_any_element(),
            Lookup::Failed(message) => notice(
                format!("Could not check: {message}. This is not \"nothing found\"."),
                warning_fill(),
                warning_ink(),
            )
            .into_any_element(),
            Lookup::Done(found) => {
                let (headline, lines) = describe(found);
                let total = lines.len();
                let mut col = div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().text_color(muted).child(format!("fetched {}", found.fetched_on)))
                    .child(div().text_sm().child(headline));
                let shown = if expanded { total } else { CARD_LINES };
                for line in lines.into_iter().take(shown) {
                    col = col.child(div().text_xs().text_color(muted).child(line));
                }
                if total > CARD_LINES {
                    let label = if expanded { "Show fewer".to_string() } else { format!("Show all {total}") };
                    col = col.child(
                        div().child(
                            Button::new(SharedString::from(format!("card-more-{title}")))
                                .label(label)
                                .small()
                                .ghost()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if !this.expanded_cards.remove(title) {
                                        this.expanded_cards.insert(title);
                                    }
                                    cx.notify();
                                })),
                        ),
                    );
                }
                col.into_any_element()
            }
        };
        div()
            .flex_1()
            .min_w(px(300.))
            .max_w(px(590.))
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(background)
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(title))
            .child(content)
            .into_any_element()
    }

    // ---------- records (reported variant) ----------

    fn records_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let mut chips = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("Earlier records"));
        for (i, r) in self.records.iter().enumerate() {
            chips = chips.child(
                Button::new(SharedString::from(format!("record-{i}")))
                    .label(format!("{} · {}", r.label, r.year))
                    .small()
                    .selected(self.editing == Editing::Record(i))
                    .toggled(self.editing == Editing::Record(i))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.show_record(i, window, cx);
                        cx.notify();
                    })),
            );
        }
        chips = chips
            .child(Button::new("record-add").label("Add record").small().outline().on_click(cx.listener(
                |this, _, window, cx| {
                    this.add_record(window, cx);
                    cx.notify();
                },
            )))
            .child(
                Button::new("record-today")
                    .label("Today (you)")
                    .small()
                    .selected(self.editing == Editing::Today)
                    .toggled(self.editing == Editing::Today)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.editing = Editing::Today;
                        cx.notify();
                    })),
            );
        if self.records.is_empty() {
            chips = chips.child(Button::new("record-example").label("Load example").small().ghost().on_click(
                cx.listener(|this, _, window, cx| {
                    // Loading replaces today's evidence, so ask first when there is some.
                    if this.today.is_empty() {
                        this.load_example(window, cx);
                    } else {
                        this.ask(Confirm::LoadExample);
                    }
                    cx.notify();
                }),
            ));
        }

        let detail = match self.editing {
            Editing::Record(i) => self.record_detail(i, cx),
            Editing::Today => {
                let mut row = div().flex().items_center().gap_2();
                if !self.records.is_empty() {
                    row = row.child(
                        Button::new("copy-latest")
                            .label("Start from the latest record's evidence")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.today.is_empty() {
                                    this.start_from_latest();
                                } else {
                                    this.ask(Confirm::StartFromLatest);
                                }
                                cx.notify();
                            })),
                    );
                }
                row.child(div().text_xs().text_color(muted).child(if self.records.is_empty() {
                    "Add what earlier labs reported and used, then apply your own evidence to compare."
                } else {
                    "Editing your own evidence. The verdict compares it with the latest record."
                }))
                .into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_2()
            .border_t_1()
            .border_color(border)
            .child(chips)
            .child(detail)
            .into_any_element()
    }

    /// The class, year and VUS tier one earlier record reported.
    fn record_detail(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let record = &self.records[i];
        let remove_label = record.label.clone();
        let label = |text: &'static str| div().text_xs().text_color(muted).child(text);

        let name_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(label("Lab or source"))
            .child(div().w(px(260.)).child(Input::new(&self.record_name_input)));
        let mut row = div().flex().flex_wrap().items_center().gap_2().child(label("This record reported"));
        for class in CLASSES {
            row = row.child(
                Button::new(SharedString::from(format!("class-{i}-{}", class.short())))
                    .label(class.short())
                    .accessibility_label(format!("This record reported {}", class.label()))
                    .small()
                    .selected(record.class == Some(class))
                    .toggled(record.class == Some(class))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.records[i].class = Some(class);
                        if class != Class::Vus {
                            // A sub-tier only makes sense for a VUS.
                            this.records[i].tier = None;
                        }
                        cx.notify();
                    })),
            );
        }
        if record.class.is_none() {
            row = row.child(label("(choose one to get a verdict)"));
        }
        if record.class == Some(Class::Vus) {
            row = row.child(div().pl_2().child(label("Tier it named")));
            row = row.child(
                Button::new(SharedString::from(format!("tier-{i}-none")))
                    .label("none stated")
                    .small()
                    .selected(record.tier.is_none())
                    .toggled(record.tier.is_none())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.records[i].tier = None;
                        cx.notify();
                    })),
            );
            for tier in &self.rules.vus_tiers {
                let name = tier.name.clone();
                let chosen = record.tier.as_deref() == Some(tier.name.as_str());
                row = row.child(
                    Button::new(SharedString::from(format!("tier-{i}-{}", tier.name)))
                        .label(tier.name.clone())
                        .small()
                        .selected(chosen)
                        .toggled(chosen)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.records[i].tier = Some(name.clone());
                            cx.notify();
                        })),
                );
            }
        }
        let row = row
            .child(div().pl_2().child(label("Year")))
            .child(
                Button::new(SharedString::from(format!("year-down-{i}")))
                    .label("−")
                    .accessibility_label("One year earlier")
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let year = &mut this.records[i].year;
                        *year = year.saturating_sub(1).max(2000);
                        cx.notify();
                    })),
            )
            .child(div().text_sm().child(record.year.to_string()))
            .child(
                Button::new(SharedString::from(format!("year-up-{i}")))
                    .label("+")
                    .accessibility_label("One year later")
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let year = &mut this.records[i].year;
                        *year = year.saturating_add(1).min(2100);
                        cx.notify();
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("record-remove-{i}")))
                    .label("Remove record")
                    .small()
                    .ghost()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ask(Confirm::RemoveRecord(i, remove_label.clone()));
                        cx.notify();
                    })),
            );
        div().flex().flex_col().gap_2().child(name_row).child(row).into_any_element()
    }

    // ---------- criteria list (left) ----------

    pub(super) fn criteria_panel(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let applied = self.current().len();
        let clear_label = match self.editing {
            Editing::Today => "Clear".to_string(),
            Editing::Record(i) => format!("Clear {}", self.records[i].label),
        };

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_5()
            .py_3()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Criteria · {}", self.editing_label())),
                    )
                    .child(div().text_xs().text_color(muted).child(if applied == 0 {
                        "Click a code to apply it. You decide which criteria apply.".to_string()
                    } else {
                        format!("{applied} applied")
                    })),
            )
            .child(Button::new("clear").label(clear_label).small().outline().on_click(cx.listener(
                |this, _, _, cx| {
                    if !this.current().is_empty() {
                        this.ask(Confirm::Clear(this.editing));
                    }
                    cx.notify();
                },
            )));

        let mut list = div()
            .id("criteria")
            .track_scroll(&self.criteria_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_4();
        if self.editing == Editing::Today && !self.rules.predictors.is_empty() {
            list = list.child(self.predictor_helper(cx));
        }
        if self.editing == Editing::Today && self.rules.functional.is_some() {
            list = list.child(self.functional_helper(cx));
        }
        if self.editing == Editing::Today && self.rules.locus_evidence.is_some() {
            list = list.child(self.locus_helper(cx));
        }
        if self.editing == Editing::Today && self.rules.non_segregation.is_some() {
            list = list.child(self.non_segregation_helper(cx));
        }
        if self.editing == Editing::Today && self.rules.pm3.is_some() {
            list = list.child(self.pm3_helper(cx));
        }
        // Deliberate, not an omission: the BA1 list is a dated static reference,
        // not an evidence helper. It shows for earlier records too and cannot
        // insert evidence, so the `Editing::Today` gates above do not apply to it.
        if !self.rules.ba1_exceptions.is_empty() {
            list = list.child(self.ba1_reference(cx));
        }
        if self.editing == Editing::Today && self.rules.de_novo.is_some() {
            list = list.child(self.de_novo_helper(cx));
        }
        for (direction, title, points) in [
            (Direction::Pathogenic, "For disease (pathogenic)", format!("+{}", result.pathogenic_points)),
            (Direction::Benign, "Against disease (benign)", format!("−{}", result.benign_points.abs())),
        ] {
            let mut group = div().flex().flex_col().gap_0p5().child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .pb_1()
                    .child(div().w(px(10.)).h(px(10.)).rounded_sm().bg(direction_colour(direction)))
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().flex_1())
                    .child(div().text_xs().text_color(muted).child(format!("{points} points"))),
            );
            for category in Category::ALL {
                let in_category: Vec<&Criterion> =
                    self.rules.criteria.iter().filter(|c| c.direction == direction && c.category == category).collect();
                if in_category.is_empty() {
                    continue;
                }
                group = group.child(div().px_2().pt_2().text_xs().text_color(muted).child(category.label()));
                for c in in_category {
                    group = group.child(self.criterion_row(c, cx));
                }
            }
            list = list.child(group);
        }

        div()
            .flex()
            .flex_col()
            .w(px(480.))
            .h_full()
            .flex_none()
            .border_r_1()
            .border_color(border)
            .child(header)
            .child(list)
            .into_any_element()
    }

    fn criterion_row(&self, c: &Criterion, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, accent) = (theme.muted_foreground, theme.accent);
        let choice = self.current().get(&c.code);
        let strength = choice.map(|choice| choice.strength);
        let situation = choice.and_then(|choice| choice.situation.clone());
        let guide_option = situation.as_deref().and_then(|id| c.guide_option(id));
        let open = self.open_details.contains(&c.code);
        let state = if strength.is_some() { "applied" } else { "not applied" };
        let source = self.rules.source(&c.source).map(|s| s.citation.clone());

        let (toggle_target, cycle_target, detail_code) = (c.clone(), c.clone(), c.code.clone());
        let remove_code = c.code.clone();
        let guide_open = self.editing == Editing::Today && self.guide_open.as_deref() == Some(c.code.as_str());
        let raise_open =
            self.editing == Editing::Today && self.raising.as_ref().is_some_and(|raise| raise.code == c.code);
        let reason = choice.and_then(|choice| choice.reason.clone());
        let summary_indent = px(66.);

        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_2()
            .py_1p5()
            .rounded_md()
            .when(strength.is_some(), |d| d.bg(accent))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().w(px(58.)).flex_none().child(
                            Button::new(SharedString::from(format!("apply-{}", c.code)))
                                .label(c.code.clone())
                                .accessibility_label(format!("{}: {} ({state})", c.code, c.title))
                                .small()
                                .selected(strength.is_some())
                                .toggled(strength.is_some())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle(&toggle_target);
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(div().flex_1().min_w_0().text_sm().child(c.title.clone()))
                    .when(c.discouraged, |row| row.child(div().text_xs().text_color(muted).child("not counted")))
                    .when_some(strength, |row, strength| {
                        row.child(
                            Button::new(SharedString::from(format!("strength-{}", c.code)))
                                .label(strength.label())
                                .accessibility_label(format!(
                                    "{} strength: {}. Click to change.",
                                    c.code,
                                    strength.label()
                                ))
                                .small()
                                .outline()
                                .tooltip(if c.guide.is_empty() {
                                    let cap = c
                                        .max_strength
                                        .map(|limit| format!("Capped at {} by this program. ", limit.label()))
                                        .unwrap_or_default();
                                    format!("{cap}Click to change the strength")
                                } else {
                                    "Click to answer the strength guide again".to_string()
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.next_strength(&cycle_target, window, cx);
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        Button::new(SharedString::from(format!("detail-{}", c.code)))
                            .label(if open { "Less" } else { "More" })
                            .small()
                            .ghost()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.open_details.remove(&detail_code) {
                                    this.open_details.insert(detail_code.clone());
                                }
                                cx.notify();
                            })),
                    ),
            )
            .when(self.learning || open, |col| {
                // Learning on: one or two lines of the rule under every criterion. More: all of it.
                let mut text = div().pl(summary_indent).flex().flex_col().gap_1().text_xs().text_color(muted);
                text = text.child(if open {
                    div().child(c.summary.clone())
                } else {
                    div().line_clamp(2).child(c.summary.clone())
                });
                if open && let Some(note) = c.note.clone() {
                    text = text.child(format!("Note: {note}"));
                }
                if let Some(option) = guide_option {
                    let answer = if open {
                        format!("Guide answer: {} — {}", option.short, option.situation)
                    } else {
                        format!("Guide answer: {}", option.short)
                    };
                    let answer = div().child(answer);
                    text = text.child(if open { answer } else { answer.line_clamp(2) });
                }
                if !c.guide.is_empty() && strength.is_none() {
                    text =
                        text.child(div().child(
                            "Click the code to answer a short decision tree; nothing is applied until you choose.",
                        ));
                }
                if open && strength.is_some() && !c.guide.is_empty() && self.editing == Editing::Today {
                    text = text.child(
                        Button::new(SharedString::from(format!("remove-{remove_code}")))
                            .label(format!("Remove {}", c.code))
                            .accessibility_label(format!("Remove {}", remove_code))
                            .small()
                            .ghost()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_criterion(&remove_code);
                                cx.notify();
                            })),
                    );
                }
                if open && let Some(citation) = source {
                    text = text.child(format!("Source: {citation}"));
                }
                col.child(text)
            })
            .when_some(reason, |col, reason| {
                col.child(div().pl(summary_indent).text_xs().text_color(muted).child(format!("Reason: {reason}")))
            })
            .when(raise_open, |col| col.child(self.raise_prompt(c, cx)))
            .when(guide_open, |col| col.child(self.strength_guide(c, cx)))
            .into_any_element()
    }

    /// The questions that set a criterion's strength. Nothing is applied until one is answered.
    fn strength_guide(&self, c: &Criterion, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let citation = c.guide_source.as_deref().and_then(|id| self.rules.source(id)).map(|s| s.citation.clone());
        let mut panel = div()
            .mt_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(background)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Which fits this variant? The answer sets the strength of {}.", c.code)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("guide-cancel-{}", c.code)))
                            .label("Cancel")
                            .accessibility_label(format!("Close the {} strength guide", c.code))
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.guide_open = None;
                                cx.notify();
                            })),
                    ),
            );
        for (i, option) in c.guide.iter().enumerate() {
            let code = c.code.clone();
            let strength = option.strength;
            let situation = option.id.clone();
            let chosen =
                self.current().get(&c.code).and_then(|choice| choice.situation.as_deref()) == Some(option.id.as_str());
            panel = panel.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div().w(px(112.)).flex_none().child(
                            Button::new(SharedString::from(format!("guide-{}-{i}", c.code)))
                                .label(strength.map_or("Does not apply", Strength::label))
                                .accessibility_label(format!(
                                    "{} {}: {}",
                                    c.code,
                                    strength.map_or("does not apply", Strength::label),
                                    option.short
                                ))
                                .small()
                                .outline()
                                .selected(chosen)
                                .toggled(chosen)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.answer_guide(&code, strength, situation.clone());
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(option.short.clone()))
                            .child(div().text_xs().child(option.situation.clone())),
                    ),
            );
        }
        panel
            .when_some(citation, |d, citation| {
                d.child(div().text_xs().text_color(muted).child(format!("Source: {citation}")))
            })
            .into_any_element()
    }

    // ---------- results (right) ----------

    /// A chip for one criterion cell of the history table.
    fn evidence_chip(
        &self,
        code: &str,
        strength: Option<Strength>,
        changed: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (muted, foreground) = (cx.theme().muted_foreground, cx.theme().foreground);
        let cell = div().flex().items_center().gap_1().p_1().rounded_sm();
        let Some(strength) = strength else {
            return cell.text_color(muted).child("–").into_any_element();
        };
        let direction = self.rules.criterion(code).map_or(Direction::Pathogenic, |c| c.direction);
        let points = self.rules.magnitude(strength);
        let sign = if direction == Direction::Pathogenic { "+" } else { "−" };
        cell.when(changed, |d| d.border_2().border_color(foreground))
            .child(
                div()
                    .px_1()
                    .rounded_sm()
                    .bg(direction_colour(direction))
                    .text_color(rgb(0xffffff))
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .child(format!("{sign}{points}")),
            )
            .child(div().text_xs().child(strength.label()))
            .into_any_element()
    }

    /// Who this result is about, the class, the points and where they sit.
    pub(super) fn hero(&self, result: &Assessment, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let dark = cx.theme().is_dark();
        // The gene the evidence was checked against, not text still being typed.
        let gene = self.case_gene.clone();
        let scope = (self.mode == Mode::ReportedVariant).then(|| self.editing_label());

        let title = match variant {
            Some(name) => div().text_xl().font_weight(FontWeight::BOLD).child(name.to_string()),
            None => div().text_xl().text_color(muted).child("Unnamed variant"),
        };
        let mut hero = div().flex().flex_col().gap_3();
        if let Some(scope) = &scope {
            hero = hero.child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("Result for {scope}")));
        }
        hero = hero.child(title);
        let subtitle: Vec<String> = [Some(gene).filter(|g| !g.is_empty())].into_iter().flatten().collect();
        if !subtitle.is_empty() {
            hero = hero.child(div().text_sm().text_color(muted).child(subtitle.join(" · ")));
        }

        match result.class() {
            Some(class) => {
                let sign = if result.total_points > 0 { "+" } else { "" };
                let classic = match (result.classic_class, result.frameworks_agree()) {
                    (Some(c), Some(true)) => {
                        format!("The 2015 table plus ClinGen's 2020 addition also gives {}.", c.label())
                    }
                    (Some(c), _) => format!(
                        "The 2015 table plus ClinGen's 2020 addition gives {}. It differs from the points; the two methods are not identical.",
                        c.label()
                    ),
                    (None, _) => "The 2015 table does not cover the strengths used.".to_string(),
                };
                hero = hero
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_5()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .px_4()
                                    .py_3()
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(border)
                                    .bg(class_fill(class, dark))
                                    .text_color(class_ink(class, dark))
                                    .child(div().text_2xl().font_weight(FontWeight::BOLD).child(class.label()))
                                    .child(div().text_xs().child(rules_tag(self.rules))),
                            )
                            .child(div().text_2xl().font_weight(FontWeight::BOLD).child(format!(
                                "{sign}{} point{}",
                                result.total_points,
                                plural(result.total_points.abs())
                            )))
                            .when_some(result.vus_tier.clone(), |row, tier| row.child(div().text_lg().child(tier))),
                    )
                    .child(self.points_scale(result.total_points, class, cx))
                    .child(div().text_sm().text_color(muted).child(classic));
                // Stand-alone evidence fixes the class; the points total alone would not.
                if result
                    .items
                    .iter()
                    .any(|i| i.counted && i.direction == Direction::Benign && i.strength == Strength::StandAlone)
                {
                    hero = hero.child(div().text_xs().text_color(muted).child(
                        "Stand-alone benign evidence (BA1) fixes the class at Benign, whatever else is applied. The points total is shown for transparency.",
                    ));
                }
                if class == Class::Vus {
                    hero = hero.child(self.vus_note(result.total_points, cx));
                }
                hero = hero.child(div().text_xs().text_color(muted).child(
                    "Default strengths follow 2015, except PVS1 (guide), PM2 (Supporting, ClinGen 2020) and PP5/BP6 (listed, not counted). Gene-specific rules and later ClinGen refinements (splicing, PM6) can change them and are not built in yet. Six helpers in the criteria list apply published tables: predictor scores (PP3/BP4), functional assays (PS3/BS3), family and phenotype points (PP1/PP4), non-segregation (BS4), de novo patients (PS2) and recessive patients (PM3). A dated BA1 exceptions reference is also available.",
                ));
            }
            None if !result.conflicts.is_empty() => {
                hero = hero
                    .child(notice(
                        "Class withheld: some criteria contradict each other. Remove one of each pair to see a class.",
                        warning_fill(),
                        warning_ink(),
                    ))
                    .children(result.conflicts.iter().map(|c| {
                        div().text_sm().child(format!("• {} and {} cannot both apply. {}", c.a, c.b, c.reason))
                    }))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(format!("Points so far: {} (not a classification).", result.total_points)),
                    );
            }
            None if result.both_sides_met => {
                let side = |direction: Direction| {
                    result
                        .items
                        .iter()
                        .filter(|item| item.counted && item.direction == direction)
                        .map(|item| item.code.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let has_ba1 = result.items.iter().any(|item| item.counted && item.code == "BA1");
                hero = hero
                    .child(notice(
                        "Needs expert judgment: this evidence meets a pathogenic rule and a benign rule at the same time.",
                        warning_fill(),
                        warning_ink(),
                    ))
                    .child(div().text_sm().child(format!(
                        "Pathogenic side: {}. Benign side: {}. The 2015 guidance calls this uncertain (VUS). The points alone would add up to {} and say {}, which hides the conflict, so ClassiVAR shows no class. Check that each criterion really applies.",
                        side(Direction::Pathogenic),
                        side(Direction::Benign),
                        result.total_points,
                        result.points_class.label()
                    )))
                    .when(has_ba1, |card| {
                        card.child(div().text_xs().child(
                            "BA1 is stand-alone, but the pathogenic evidence here reaches a pathogenic rule on its own. A well-known pathogenic variant can be common (founder effect, reduced penetrance), which is why ClinGen keeps a BA1 exception list. Check that list before calling this Benign.",
                        ))
                    });
            }
            None if !result.combination_overuse.is_empty() => {
                let mut card = hero.child(notice(
                    "Needs expert judgment: two criteria together exceed a published combined limit.",
                    warning_fill(),
                    warning_ink(),
                ));
                for over in &result.combination_overuse {
                    card = card.child(div().text_sm().child(format!(
                        "{} + {} are {} points together; the limit is {}. {} Lower one of them (or remove one) to get a class.",
                        over.a, over.b, over.points, over.max_points, over.reason
                    )));
                }
                hero = card;
            }
            None => {
                let (big, small) = if result.items.is_empty() {
                    ("No class yet", "Apply criteria on the left. A class appears once at least one criterion counts.")
                } else {
                    ("No class yet", "Only criteria that no longer count are applied. Apply at least one that counts.")
                };
                hero = hero
                    .child(div().text_2xl().font_weight(FontWeight::BOLD).text_color(muted).child(big))
                    .child(div().text_sm().text_color(muted).child(small));
            }
        }
        hero.into_any_element()
    }

    /// Where in the VUS range the points are. The range comes from the rule set.
    fn vus_note(&self, total: i32, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let low = self.rules.class_min_points.vus;
        let high = self.rules.class_min_points.likely_pathogenic - 1;
        let tiers = self.rules.vus_tiers.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ");
        div()
            .text_sm()
            .text_color(muted)
            .child(format!(
                "VUS covers {low} to {high} points. This variant is at {total}: {} above the bottom and {} below Likely pathogenic. Sub-tiers ({tiers}) have no official point ranges yet, so ClassiVAR does not name one for new evidence.",
                total - low,
                self.rules.class_min_points.likely_pathogenic - total,
            ))
            .into_any_element()
    }

    pub(super) fn points_scale(&self, total: i32, active: Class, cx: &mut Context<Self>) -> AnyElement {
        let (foreground, muted) = (cx.theme().foreground, cx.theme().muted_foreground);
        let dark = cx.theme().is_dark();
        let scale = scale_for(self.rules);
        let x_of = |points: i32| (points - scale.min) as f32 * SCALE_UNIT;
        let marker_x = x_of(total.clamp(scale.min, scale.max - 1)) + SCALE_UNIT / 2.0 - 7.0;
        let width = (scale.max - scale.min) as f32 * SCALE_UNIT;

        let mut bar = div().flex().absolute().top(px(18.)).left(px(0.));
        for (class, from, to) in &scale.zones {
            let is_active = *class == active;
            let fill = class_fill(*class, dark);
            let label = class.short();
            bar = bar.child(
                div()
                    .w(px((to - from) as f32 * SCALE_UNIT))
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .bg(fill)
                    .text_color(class_ink(*class, dark))
                    .text_xs()
                    .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .border_2()
                    .border_color(if is_active { foreground } else { fill.into() })
                    .child(label),
            );
        }

        let mut ticks = div().absolute().top(px(54.)).left(px(0.)).w(px(width));
        for edge in scale.zones.iter().skip(1).map(|(_, from, _)| *from) {
            ticks = ticks.child(
                div()
                    .absolute()
                    .left(px(x_of(edge) - 10.))
                    .w(px(20.))
                    .flex()
                    .justify_center()
                    .text_xs()
                    .text_color(muted)
                    .child(edge.to_string()),
            );
        }

        div()
            .id("scale-scroll")
            .overflow_x_scroll()
            .child(
                div()
                    .relative()
                    .w(px(width))
                    .h(px(74.))
                    .child(div().absolute().top(px(0.)).left(px(marker_x)).text_color(foreground).child("▼"))
                    .child(bar)
                    .child(ticks),
            )
            .into_any_element()
    }

    pub(super) fn evidence_lanes(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let mut lanes = div().flex().flex_col();
        for category in Category::ALL {
            let items: Vec<_> = result
                .items
                .iter()
                .filter(|i| self.rules.criterion(&i.code).is_some_and(|c| c.category == category))
                .collect();
            let mut track = div().flex().flex_wrap().items_center().gap_1().min_h(px(30.)).flex_1();
            if items.is_empty() {
                track = track.child(div().text_xs().text_color(muted).child("none"));
            }
            for item in items {
                if !item.counted {
                    track = track.child(div().text_xs().text_color(muted).pr_2().child(format!(
                        "{} {}: listed, not counted",
                        item.code,
                        item.strength.label()
                    )));
                    continue;
                }
                let width = (item.points.unsigned_abs() as f32 * POINT_WIDTH).max(POINT_WIDTH);
                let sign = if item.points > 0 { "+" } else { "−" };
                track = track
                    .child(
                        div()
                            .w(px(width))
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_r_md()
                            .bg(direction_colour(item.direction))
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child(item.code.clone()),
                    )
                    .child(div().text_xs().text_color(muted).pr_2().child(format!(
                        "{sign}{} {}",
                        item.points.unsigned_abs(),
                        item.strength.label()
                    )));
            }
            lanes = lanes.child(
                div()
                    .flex()
                    .items_center()
                    .py_1()
                    .border_t_1()
                    .border_color(border)
                    .child(div().w(px(120.)).flex_none().text_xs().text_color(muted).child(category.label()))
                    .child(track),
            );
        }
        lanes.into_any_element()
    }

    pub(super) fn what_changes(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        if let Some(next) = result.to_upgrade {
            list = list.child(format!(
                "Up: {} more point{} of pathogenic evidence reaches {}.",
                next.points_needed,
                plural(next.points_needed),
                next.class.label()
            ));
        }
        if let Some(next) = result.to_downgrade {
            list = list.child(format!(
                "Down: {} more point{} of benign evidence reaches {}.",
                next.points_needed,
                plural(next.points_needed),
                next.class.label()
            ));
        }
        list.child(
            div().text_xs().text_color(muted).child("The evidence that could move it is in the lanes marked none."),
        )
        .into_any_element()
    }

    /// Notes about the result. They are information, so they are quiet, not yellow.
    pub(super) fn notes(&self, result: &Assessment, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut texts: Vec<String> = Vec::new();
        if variant.is_none() {
            texts.push(
                "No variant is named. Name it at the top, so this result can be attributed to a variant.".to_string(),
            );
        }
        match self.gene_link {
            GeneDecision::Established => {}
            GeneDecision::NotDecided => texts.push(
                "You have not confirmed that this gene causes the disease. ACMG/AMP criteria assume it does.".to_string(),
            ),
            GeneDecision::NotEstablished => texts.push(
                "You marked the gene-disease link as doubtful. ACMG/AMP criteria do not apply without it; treat any class with great care.".to_string(),
            ),
        }
        texts.extend(result.warnings.iter().map(|w| match w {
            Warning::ConflictingEvidence => {
                "Evidence points both ways. Check that each criterion really applies.".to_string()
            }
            Warning::Discouraged { code, note, counted } => format!(
                "{code} is no longer recommended by the experts. {}{}",
                if *counted {
                    "It is counted here because this is an earlier record."
                } else {
                    "It is listed but not counted."
                },
                note.as_deref().map(|n| format!(" {n}")).unwrap_or_default()
            ),
            Warning::StrengthChanged { code, from, to, reason } => {
                let why = match reason {
                    Some(reason) => format!(" Reason: {reason}"),
                    None if self.rules.magnitude(*to) > self.rules.magnitude(*from) => {
                        " No reason is recorded; click the strength to raise it again with one.".to_string()
                    }
                    None => String::new(),
                };
                format!("{code} strength changed from {} to {}.{why}", from.label(), to.label())
            }
            Warning::DoubleCounted { a, b, reason } => {
                format!("{a} and {b} may count the same observation. {reason}")
            }
        }));
        if texts.is_empty() {
            return div().text_sm().text_color(muted).child("No notes.").into_any_element();
        }
        let mut list = div().flex().flex_col().gap_2().text_sm();
        for text in texts {
            list = list.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .w(px(18.))
                            .h(px(18.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .border_color(muted)
                            .text_xs()
                            .text_color(muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("!"),
                    )
                    .child(div().flex_1().min_w_0().child(text)),
            );
        }
        list.into_any_element()
    }

    // ---------- verdict and history (reported variant) ----------

    pub(super) fn verdict_card(
        &self,
        re: Option<&Reevaluation>,
        incomplete: &[String],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let card = div().flex().flex_col().gap_2().p_4().rounded_lg().border_1().border_color(border);

        let Some(re) = re else {
            return card
                .child(notice(
                    format!(
                        "Choose the class each record reported to get a verdict. Missing for: {}.",
                        incomplete.join(", ")
                    ),
                    warning_fill(),
                    warning_ink(),
                ))
                .into_any_element();
        };
        let latest = re.latest_index.map(|i| &re.records[i]);
        let today = match &re.today {
            Ok(today) => today,
            Err(error) => {
                return card
                    .child(notice(
                        format!("Today's evidence cannot be scored: {error}. Remove that criterion."),
                        warning_fill(),
                        warning_ink(),
                    ))
                    .into_any_element();
            }
        };

        let (Some(verdict), Some(latest), Some(today_class)) = (re.verdict, latest, today.class()) else {
            let hint = if self.records.is_empty() {
                "Add the earlier records, then apply your own evidence to get a verdict."
            } else if !today.conflicts.is_empty() {
                "Your evidence contradicts itself, so there is no class to compare. Remove one of the pair."
            } else if today.both_sides_met {
                "Your evidence meets a pathogenic rule and a benign rule at the same time. It needs expert judgment, so there is no class to compare."
            } else if !today.combination_overuse.is_empty() {
                "Two of your criteria together exceed a published combined limit. Lower or remove one of them; there is no class to compare."
            } else {
                "Apply your evidence, or start from the latest record's evidence, to get a verdict."
            };
            return card.child(div().text_sm().text_color(muted).child(hint)).into_any_element();
        };

        let (symbol, title) = match verdict {
            Verdict::Holds => ("=", "Candidate: the classification still holds"),
            Verdict::UpgradeCandidate => ("↑", "Upgrade candidate"),
            Verdict::DowngradeCandidate => ("↓", "Downgrade candidate"),
            Verdict::FrameworkDiffers => ("≠", "Same evidence, different class"),
            Verdict::RecordInconsistent => ("?", "The latest record does not fit its own evidence"),
        };
        let latest_summary = format!(
            "The latest record ({}, {}) reported {}{}.",
            latest.label,
            latest.year,
            latest.reported_class.label(),
            latest.reported_tier.as_deref().map(|t| format!(" ({t})")).unwrap_or_default()
        );
        let detail = match verdict {
            Verdict::Holds | Verdict::UpgradeCandidate | Verdict::DowngradeCandidate => format!(
                "{latest_summary} Today's evidence gives {} points: {}.",
                today.total_points,
                today_class.label()
            ),
            Verdict::FrameworkDiffers => format!(
                "{latest_summary} No evidence changed, but today's evidence scores {} ({} points). The record may have used the 2015 table, a method without points, or a criterion that no longer counts. This is not a reason to upgrade or downgrade.",
                today_class.label(),
                today.total_points
            ),
            Verdict::RecordInconsistent => {
                let own = latest.assessment.as_ref().ok();
                format!(
                    "{latest_summary} Its own evidence scores {} by points and {} by the 2015 table, so a comparison would mean little. No upgrade or downgrade is suggested.",
                    own.map_or("?", |a| a.points_class.label()),
                    own.and_then(|a| a.classic_class).map_or("not defined", Class::label),
                )
            }
        };

        let mut card = card
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_4()
                    .child(div().text_xl().font_weight(FontWeight::BOLD).child(format!("{symbol}  {title}")))
                    .child(class_badge(today_class, self.rules, "Today: ", border, cx.theme().is_dark())),
            )
            .child(div().text_sm().child(detail));
        if let Some(tier) =
            latest.reported_tier.as_deref().and_then(|name| self.rules.vus_tiers.iter().find(|t| t.name == name))
        {
            let citation = self.rules.source(&tier.source).map_or(tier.source.as_str(), |s| s.citation.as_str());
            card = card.child(div().text_sm().child(format!(
                "{} means: {} ({citation}) ClassiVAR cannot say which tier today's evidence falls in: official point ranges for the tiers are not published yet.",
                tier.name, tier.description
            )));
        }
        card.child(
            div()
                .text_xs()
                .text_color(muted)
                .child("This is a candidate for you to review, not a decision. Check each change below."),
        )
        .into_any_element()
    }

    pub(super) fn reasons_list(&self, re: &Reevaluation, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        if re.reasons.is_empty() {
            return list
                .child(div().text_color(muted).child("No differences from the latest record."))
                .into_any_element();
        }
        let citation_of = |id: &str| self.rules.source(id).map_or(id.to_string(), |s| s.citation.clone());
        for reason in &re.reasons {
            let text = match reason {
                Reason::Added { code, strength } => {
                    format!("{code} added at {}: new evidence since the latest record.", strength.label())
                }
                Reason::Removed { code } => format!("{code} is no longer applied."),
                Reason::StrengthChanged { code, from, to } => {
                    format!("{code} changed from {} to {}.", from.label(), to.label())
                }
                Reason::RuleChanged { code, from, to, source } => format!(
                    "{code}: the record used {} (the strength in the 2015 paper). The current rule is {}. Source: {}",
                    from.label(),
                    to.label(),
                    citation_of(source)
                ),
                Reason::RuleSetChanged { from, to } => {
                    format!(
                        "The record used rules {} v{}. Today's rules are {} v{}.",
                        from.id, from.version, to.id, to.version
                    )
                }
                Reason::TierChanged { from, to } => format!("The record named {from}. Today's evidence falls in {to}."),
            };
            list = list.child(div().flex().gap_2().child("•").child(div().flex_1().min_w_0().child(text)));
        }
        list.into_any_element()
    }

    /// One line for the "Total" row of the history table.
    fn total_text(result: &Result<Assessment, classivar_core::AssessError>) -> String {
        match result {
            Err(error) => format!("could not be scored: {error}"),
            Ok(a) => match a.class() {
                Some(class) => format!("{} pts → {}", a.total_points, class.short()),
                None if !a.has_evidence => "no criteria".to_string(),
                None => format!("{} pts, class withheld", a.total_points),
            },
        }
    }

    pub(super) fn history_table(&self, re: &Reevaluation, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let cell = |content: AnyElement| div().flex_1().min_w(px(130.)).px_1().child(content);
        let first = |content: AnyElement| div().w(px(56.)).flex_none().child(content);
        let latest_cells = re.latest_index;

        let mut header = div().flex().items_start().py_1().child(first(div().into_any_element()));
        for r in &re.records {
            let reported = match &r.reported_tier {
                Some(tier) => format!("reported {} · {tier}", r.reported_class.short()),
                None => format!("reported {}", r.reported_class.short()),
            };
            header = header.child(cell(
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(format!("{} · {}", r.label, r.year)))
                    .child(div().text_xs().text_color(muted).child(reported))
                    .into_any_element(),
            ));
        }
        header = header
            .child(cell(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("Today (you)").into_any_element()));

        let mut table = div().flex().flex_col().min_w(px(56. + 130. * (re.records.len() + 1) as f32)).child(header);
        for row in &re.rows {
            let mut line = div().flex().items_center().py_1().border_t_1().border_color(border).child(first(
                div().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.code.clone()).into_any_element(),
            ));
            let today_index = row.cells.len() - 1;
            for (i, strength) in row.cells.iter().enumerate() {
                // Outline today's cell when it differs from the latest record.
                let changed = i == today_index && latest_cells.is_some_and(|l| row.cells[l] != *strength);
                line = line.child(cell(self.evidence_chip(&row.code, *strength, changed, cx)));
            }
            table = table.child(line);
        }

        let mut total = div()
            .flex()
            .items_start()
            .py_1()
            .border_t_1()
            .border_color(border)
            .child(first(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("Total").into_any_element()));
        for r in &re.records {
            let mut col = div().flex().flex_col().child(div().text_sm().child(Self::total_text(&r.assessment)));
            if r.consistency() == Consistency::Differs {
                col = col.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(format!("! it reported {}; its evidence scores differently", r.reported_class.short())),
                );
            }
            total = total.child(cell(col.into_any_element()));
        }
        total = total.child(cell(
            div().text_sm().font_weight(FontWeight::BOLD).child(Self::total_text(&re.today)).into_any_element(),
        ));
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().id("history-scroll").overflow_x_scroll().child(table.child(total)))
            .child(div().text_xs().text_color(muted).child(format!(
                "Records are scored the way they were made (criteria that no longer count are still counted). Today's evidence uses today's rules. {}. An outlined cell differs from the latest record.",
                rules_tag(self.rules)
            )))
            .into_any_element()
    }

    /// What the VUS sub-tiers mean, with their sources. Shown when a record reported a VUS.
    fn tier_reference(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_2().text_sm();
        for tier in &self.rules.vus_tiers {
            let citation = self.rules.source(&tier.source).map_or(tier.source.as_str(), |s| s.citation.as_str());
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(tier.name.clone()))
                    .child(div().child(tier.description.clone()))
                    .child(div().text_xs().text_color(muted).child(format!("Source: {citation}"))),
            );
        }
        list.child(div().text_xs().text_color(muted).child(
            "Laboratories use these tiers in their own way, and no official point ranges exist yet (they are expected with ACMG SVCv4.0). ClassiVAR records the tier a lab named, and does not assign one itself.",
        ))
        .into_any_element()
    }

    // ---------- whole right pane ----------

    pub(super) fn results_panel(
        &self,
        result: &Assessment,
        reevaluation: Option<&Reevaluation>,
        incomplete: &[String],
        variant: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let border = cx.theme().border;
        let mut pane = div()
            .id("result")
            .track_scroll(&self.result_scroll)
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .px_6()
            .py_5()
            .flex()
            .flex_col()
            .gap_6();

        // The result comes first on both paths; the verdict and history are commentary on it.
        pane = pane.child(self.hero(result, variant, cx));

        if self.mode == Mode::ReportedVariant {
            pane = pane.child(div().child(heading("Verdict")).child(self.verdict_card(reevaluation, incomplete, cx)));
            if let Some(re) = reevaluation {
                if re.verdict.is_some() {
                    pane = pane.child(
                        div().child(heading("What changed since the latest record")).child(self.reasons_list(re, cx)),
                    );
                }
                if !re.records.is_empty() {
                    pane = pane
                        .child(div().child(heading("History against today's rules")).child(self.history_table(re, cx)));
                }
                // Today's own notes stay visible while a record is being edited.
                if let (Editing::Record(_), Ok(today)) = (self.editing, &re.today) {
                    pane = pane
                        .child(div().child(heading("Notes on your evidence")).child(self.notes(today, variant, cx)));
                }
            }
            if self.learning && self.records.iter().any(|r| r.class == Some(Class::Vus)) {
                pane = pane.child(div().child(heading("What the VUS sub-tiers mean")).child(self.tier_reference(cx)));
            }
            pane = pane.child(div().pt_2().border_t_1().border_color(border));
        }

        pane.child(div().child(heading("What it is made of")).child(self.evidence_lanes(result, cx)))
            .when(result.class().is_some(), |pane| {
                pane.child(div().child(heading("What would change it")).child(self.what_changes(result, cx)))
            })
            .when(result.class().is_none() && result.has_evidence, |pane| {
                pane.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Resolve the withheld result above to see what would change the class."),
                )
            })
            .child(div().child(heading("Notes")).child(self.notes(result, variant, cx)))
            .into_any_element()
    }

    pub(super) fn error_screen(&self, message: String, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (background, foreground) = (theme.background, theme.foreground);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .bg(background)
            .text_color(foreground)
            .child(notice(message, warning_fill(), warning_ink()))
            .child(Button::new("error-clear").label("Clear this evidence").small().on_click(cx.listener(
                |this, _, _, cx| {
                    let scope = this.editing;
                    this.evidence_mut(scope).clear();
                    cx.notify();
                },
            )))
            .into_any_element()
    }
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rules = self.rules;
        // An earlier record is scored the way it was made, as in the history table.
        let counting = match self.editing {
            Editing::Today => classivar_core::Counting::Current,
            Editing::Record(_) => classivar_core::Counting::AsRecorded,
        };
        let result = match rules.assess_as(&to_applied(self.current()), counting) {
            Ok(result) => result,
            Err(error) => return self.error_screen(format!("The evidence on screen cannot be scored: {error}"), cx),
        };

        // Records without a class cannot be compared, so there is no verdict until each has one.
        let incomplete: Vec<String> =
            self.records.iter().filter(|r| r.class.is_none()).map(|r| format!("{} · {}", r.label, r.year)).collect();
        let reevaluation = (self.mode == Mode::ReportedVariant && incomplete.is_empty()).then(|| {
            let submissions: Vec<Submission> = self
                .records
                .iter()
                .filter_map(|r| {
                    Some(Submission {
                        label: r.label.clone(),
                        year: r.year,
                        reported_class: r.class?,
                        reported_tier: r.tier.clone(),
                        rule_set: None,
                        criteria: to_applied(&r.evidence),
                    })
                })
                .collect();
            rules.reevaluate(&submissions, &to_applied(&self.today))
        });

        let variant = self.variant_name(cx);
        let (background, foreground) = (cx.theme().background, cx.theme().foreground);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .child(self.top_bar(cx))
            .children(self.file_status_banner(cx))
            .child(self.case_panel(cx))
            .child(div().flex().flex_1().min_h_0().child(self.criteria_panel(&result, cx)).child(self.results_panel(
                &result,
                reevaluation.as_ref(),
                &incomplete,
                variant.as_deref(),
                cx,
            )))
            .children(self.confirm_modal(cx))
            .into_any_element()
    }
}
