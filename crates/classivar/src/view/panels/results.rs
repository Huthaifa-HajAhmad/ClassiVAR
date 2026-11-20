//! The explanation pane, and the screen shown when the evidence cannot be scored.

use super::*;

impl Workbench {
    pub(super) fn results_panel(
        &self,
        result: &Assessment,
        reevaluation: Option<&Reevaluation>,
        incomplete: &[String],
        variant: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let reported = self.mode == Mode::ReportedVariant;
        let body = div()
            .id("result")
            .track_scroll(&self.result_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_8()
            .pt_2()
            .pb_8()
            .flex()
            .flex_col()
            .gap_6();
        let details = if reported && self.comparison {
            body.children(self.reported_sections(reevaluation, incomplete, variant, cx)).into_any_element()
        } else {
            self.evidence_body(body, result, variant, cx)
        };
        crate::look::panel(cx)
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.result_toolbar(reported, cx))
            .child(details)
            .into_any_element()
    }

    fn result_toolbar(&self, reported: bool, cx: &mut Context<Self>) -> Div {
        let picture_shown = !(reported && self.comparison);
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .pt_4()
            .pb_2()
            .when(reported, |pane| pane.child(self.result_tabs(cx)))
            .when(!reported, |pane| pane.child(crate::look::eyebrow("Explanation", cx)))
            .child(div().flex_1())
            .when(picture_shown, |pane| pane.child(self.picture_switch(cx)))
            .child(
                div().debug_selector(|| "reading-view".into()).child(
                    Button::new("reading-view")
                        .small()
                        .ghost()
                        .toggled(self.reading)
                        .label(if self.reading { "Show editor" } else { "Expand" })
                        .accessibility_label(if self.reading { "Show editor" } else { "Expand explanation" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.reading = !this.reading;
                            cx.notify();
                        })),
                ),
            )
    }

    fn result_tabs(&self, cx: &mut Context<Self>) -> Div {
        let mut tabs = Vec::new();
        for (id, label, comparison) in [("view-evidence", "Evidence", false), ("view-compare", "Compare reports", true)]
        {
            tabs.push(div().debug_selector(|| id.into()).child(
                crate::look::segment(id, label, self.comparison == comparison, cx).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.comparison = comparison;
                        this.result_scroll.set_offset(point(px(0.), px(0.)));
                        this.preview = false;
                        cx.notify();
                    },
                )),
            ));
        }
        crate::look::segmented(tabs, cx)
    }

    fn evidence_body(
        &self,
        body: Stateful<Div>,
        result: &Assessment,
        variant: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        body.child(self.hero(result, variant, cx))
            .when(!result.items.is_empty(), |pane| {
                pane.child(
                    explanation_section(cx).child(heading("What it is made of")).child(self.evidence_lanes(result, cx)),
                )
            })
            .when(combining_table::is_shown(result), |pane| {
                pane.child(
                    explanation_section(cx)
                        .child(heading("What the 2015 table says"))
                        .child(self.combining_table(result, cx)),
                )
            })
            .when(result.class().is_some(), |pane| {
                pane.child(
                    explanation_section(cx).child(heading("What would change it")).child(self.what_changes(result, cx)),
                )
            })
            .when(result.class().is_none() && result.has_evidence, |pane| {
                pane.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("Resolve the withheld result above to see what would change the class."),
                )
            })
            .when(self.has_case_data(cx), |pane| {
                pane.child(explanation_section(cx).child(heading("Notes")).child(self.notes(result, variant, cx)))
            })
            .when(result.class().is_some(), |pane| {
                pane.child(
                    div().child(heading("About these rules")).child(caption(muted).child(rules_about_text(self.rules))),
                )
            })
            .into_any_element()
    }

    /// What a reported variant adds under the result: the verdict, what changed, the
    /// history table, today's notes while a record is edited, and the VUS tier reference.
    fn reported_sections(
        &self,
        reevaluation: Option<&Reevaluation>,
        incomplete: &[String],
        variant: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Vec<Div> {
        let border = cx.theme().border;
        let mut sections = vec![div().child(heading("Verdict against the latest report")).child(self.verdict_card(
            reevaluation.filter(|_| incomplete.is_empty()),
            incomplete,
            cx,
        ))];
        if let Some(re) = reevaluation {
            if let Some(focus) = self.report_focus(re, cx) {
                sections.insert(0, focus);
            }
            if incomplete.is_empty() && re.verdict.is_some() {
                sections.push(
                    div().child(heading("What changed since the latest record")).child(self.reasons_list(re, cx)),
                );
            }
            if !re.records.is_empty() {
                if re.today.as_ref().is_ok_and(|today| today.class().is_some()) {
                    sections
                        .push(div().child(heading("Records against today, in points")).child(self.timeline(re, cx)));
                }
                sections.push(div().child(heading("History against today's rules")).child(self.history_table(re, cx)));
            }
            // Today's own notes stay visible while a record is being edited.
            if let (Editing::Record(_), Ok(today)) = (self.editing, &re.today) {
                sections.push(div().child(heading("Notes on your evidence")).child(self.notes(today, variant, cx)));
            }
        }
        if self.learning && self.records.iter().any(|r| r.class == Some(Class::Vus)) {
            sections.push(div().child(heading("What the VUS sub-tiers mean")).child(self.tier_reference(cx)));
        }
        sections.push(div().pt_2().border_t_1().border_color(border));
        sections
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
