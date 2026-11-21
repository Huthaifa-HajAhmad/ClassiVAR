use super::*;

impl Workbench {
    pub(super) fn report_focus(&self, re: &Reevaluation, cx: &mut Context<Self>) -> Option<Div> {
        let index = self.baseline.or(re.latest_index)?;
        let record = re.records.get(index)?;
        let mut rows = div().flex().flex_col().gap_1();
        let mut count = 0;
        for row in &re.rows {
            let earlier = row.cells[index];
            let today = row.cells[re.records.len()];
            if earlier != today {
                count += 1;
                rows = rows.child(change_row(&row.code, earlier.map(|s| s.label()), today.map(|s| s.label()), cx));
            }
        }
        let muted = cx.theme().muted_foreground;
        Some(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child("Compare a report with today"))
                        .child(self.report_picker(re, index, cx)),
                )
                .child(report_outcomes(record, re, cx))
                .child(div().flex().child(super::picture::status_pill(self.rules, cx)))
                .when(count > 0, |panel| {
                    panel.child(crate::look::eyebrow(format!("Changed criteria · {count}"), cx)).child(rows)
                })
                .when(count == 0, |panel| {
                    panel.child(caption(muted).child("The listed criteria and strengths match."))
                }),
        )
    }

    fn report_picker(&self, re: &Reevaluation, index: usize, cx: &mut Context<Self>) -> Div {
        let mut choices = Vec::new();
        for (i, entry) in re.records.iter().enumerate() {
            let label = format!("{} · {}", entry.label, entry.year);
            choices.push(div().debug_selector(|| format!("report-{i}")).child(
                crate::look::segment(format!("compare-record-{i}"), label, index == i, cx).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.baseline = Some(i);
                        cx.notify();
                    },
                )),
            ));
        }
        crate::look::segmented(choices, cx).flex_wrap()
    }
}

/// One changed criterion: its code, what the report used, and what today uses.
fn change_row(code: &str, earlier: Option<&str>, today: Option<&str>, cx: &App) -> Div {
    let theme = cx.theme();
    let (ink, muted) = (theme.foreground, theme.muted_foreground);
    let strength = |text: Option<&str>, colour: Hsla| {
        div()
            .flex_1()
            .text_sm()
            .text_color(if text.is_some() { colour } else { muted })
            .child(text.unwrap_or("Unlisted").to_owned())
    };
    crate::look::card(cx)
        .py_2()
        .flex()
        .items_center()
        .gap_3()
        .child(crate::look::figure(12.).w(px(56.)).child(code.to_owned()))
        .child(strength(earlier, muted))
        .child(crate::look::figure(13.).text_color(muted).child("→"))
        .child(strength(today, ink).font_weight(FontWeight::SEMIBOLD))
}

/// What the lab said, what its own evidence gives under the loaded rules, and what today gives.
fn report_outcomes(record: &classivar_core::Rescored, re: &Reevaluation, cx: &Context<Workbench>) -> Div {
    let mut outcomes = div().flex().flex_wrap().gap_3();
    for (label, value) in [
        ("Lab reported", record.reported_class.label().to_owned()),
        ("Its evidence, these rules", Workbench::total_text(&record.assessment)),
        ("Today's evidence", Workbench::total_text(&re.today)),
    ] {
        outcomes = outcomes.child(
            crate::look::card(cx)
                .flex_1()
                .min_w(px(150.))
                .flex()
                .flex_col()
                .gap_1()
                .child(crate::look::eyebrow(label, cx))
                .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(value)),
        );
    }
    outcomes
}
