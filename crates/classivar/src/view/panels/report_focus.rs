use super::*;

impl Workbench {
    pub(super) fn report_focus(&self, re: &Reevaluation, cx: &mut Context<Self>) -> Option<Div> {
        let index = self.baseline.or(re.latest_index)?;
        let record = re.records.get(index)?;
        let theme = cx.theme();
        let (border, muted) = (theme.border, theme.muted_foreground);
        let mut rows = div()
            .flex()
            .flex_col()
            .child(comparison_row("Criterion", "Earlier report", "Today", border).text_color(muted));
        let mut count = 0;
        for row in &re.rows {
            let earlier = row.cells[index];
            let today = row.cells[re.records.len()];
            if earlier != today {
                count += 1;
                rows = rows.child(comparison_row(
                    &row.code,
                    earlier.map_or("Unlisted", |s| s.label()),
                    today.map_or("Unlisted", |s| s.label()),
                    border,
                ));
            }
        }
        Some(
            explanation_section(cx)
                .flex()
                .flex_col()
                .gap_3()
                .child(heading("Compare a report with today"))
                .child(self.report_picker(re, index, cx))
                .child(report_outcomes(record, re, cx))
                .child(caption(muted).child(rules_tag(self.rules)))
                .when(count > 0, |panel| panel.child(heading(format!("Changed criteria ({count})"))).child(rows))
                .when(count == 0, |panel| panel.child("The listed criteria and strengths match.")),
        )
    }

    fn report_picker(&self, re: &Reevaluation, index: usize, cx: &mut Context<Self>) -> Div {
        let mut picker = div().flex().flex_wrap().gap_1();
        for (i, entry) in re.records.iter().enumerate() {
            picker = picker.child(
                div().debug_selector(|| format!("report-{i}")).child(
                    Button::new(SharedString::from(format!("compare-record-{i}")))
                        .small()
                        .label(format!("{} · {}", entry.label, entry.year))
                        .selected(index == i)
                        .toggled(index == i)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.baseline = Some(i);
                            cx.notify();
                        })),
                ),
            );
        }
        picker
    }
}

fn comparison_row(code: &str, earlier: &str, today: &str, border: Hsla) -> Div {
    div()
        .flex()
        .gap_3()
        .px_2()
        .py_2()
        .text_sm()
        .border_b_1()
        .border_color(border)
        .child(div().w(px(85.)).font_weight(FontWeight::SEMIBOLD).child(code.to_owned()))
        .child(div().flex_1().child(earlier.to_owned()))
        .child(div().flex_1().child(today.to_owned()))
}

fn report_outcomes(record: &classivar_core::Rescored, re: &Reevaluation, cx: &Context<Workbench>) -> Div {
    let theme = cx.theme();
    let muted = theme.muted_foreground;
    let mut outcomes = div().flex().flex_wrap().gap_2();
    for (label, value) in [
        ("Lab reported", record.reported_class.label().to_owned()),
        ("Recorded evidence under these rules", Workbench::total_text(&record.assessment)),
        ("Today's evidence", Workbench::total_text(&re.today)),
    ] {
        outcomes = outcomes.child(
            div()
                .flex_1()
                .min_w(px(120.))
                .p_3()
                .rounded_md()
                .bg(theme.secondary)
                .child(caption(muted).child(label))
                .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(value)),
        );
    }
    outcomes
}
