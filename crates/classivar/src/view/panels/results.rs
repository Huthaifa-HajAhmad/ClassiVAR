//! The whole right pane, and the screen shown when the evidence cannot be scored.

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
        let muted = cx.theme().muted_foreground;
        let reported = self.mode == Mode::ReportedVariant;
        div()
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
            .gap_6()
            // The result comes first on both paths; the verdict and history are commentary on it.
            .child(self.hero(result, variant, cx))
            .when(reported, |pane| pane.children(self.reported_sections(reevaluation, incomplete, variant, cx)))
            .child(div().child(heading("What it is made of")).child(self.evidence_lanes(result, cx)))
            .when(result.class().is_some(), |pane| {
                pane.child(div().child(heading("What would change it")).child(self.what_changes(result, cx)))
            })
            .when(result.class().is_none() && result.has_evidence, |pane| {
                pane.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("Resolve the withheld result above to see what would change the class."),
                )
            })
            .child(div().child(heading("Notes")).child(self.notes(result, variant, cx)))
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
        let mut sections = vec![div().child(heading("Verdict")).child(self.verdict_card(reevaluation, incomplete, cx))];
        if let Some(re) = reevaluation {
            if re.verdict.is_some() {
                sections.push(
                    div().child(heading("What changed since the latest record")).child(self.reasons_list(re, cx)),
                );
            }
            if !re.records.is_empty() {
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
