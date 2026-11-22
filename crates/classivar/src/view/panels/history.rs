//! The history table of a reported variant and the VUS tier reference.

use super::*;

impl Workbench {
    /// One line for the "Total" row of the history table.
    pub(super) fn total_text(result: &Result<Assessment, classivar_core::AssessError>) -> String {
        match result {
            Err(error) => format!("could not be scored: {error}"),
            Ok(a) => match a.class() {
                Some(class) => format!("{} pts → {}", a.total_points, class.short()),
                None if !a.has_evidence => "no counted criteria".to_string(),
                None => format!("{} pts, class withheld", a.total_points),
            },
        }
    }

    /// The history of a reported variant as three columns: the criterion codes, the
    /// reported records, and today's own evidence. Only the records scroll sideways,
    /// so the reader never loses the criterion or today's result while comparing a long history.
    ///
    /// The three columns are separate stacks, and a stack cannot see its neighbours'
    /// row heights. Every criterion row is therefore given the same height in all
    /// three, and the three header lines hold the same two one-line texts, so the
    /// columns cannot drift apart.
    pub(super) fn history_table(&self, re: &Reevaluation, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let (mut labels, mut records, mut today) = history_heads(re, muted);
        for row in &re.rows {
            let (label, line, cell) = self.history_row(re, row, border, cx);
            labels = labels.child(label);
            records = records.child(line);
            today = today.child(cell);
        }
        let (labels_total, records_total, today_total) = history_totals(re, border, muted);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .child(
                        div()
                            .w(px(HISTORY_LABEL_W))
                            .flex_none()
                            .border_r_1()
                            .border_color(border)
                            .child(labels.child(labels_total)),
                    )
                    .child(
                        div()
                            .id("history-scroll")
                            .flex_1()
                            .min_w_0()
                            .overflow_x_scroll()
                            .child(records.child(records_total)),
                    )
                    .child(
                        div()
                            .w(px(HISTORY_COL_W))
                            .flex_none()
                            .border_l_1()
                            .border_color(border)
                            .pl_1()
                            .child(today.child(today_total)),
                    ),
            )
            .child(caption(muted).child(
                "Records are scored the way they were made, so criteria that no longer count are still counted. Today uses today's rules. An outlined cell differs from the latest record.",
            ))
            .into_any_element()
    }

    /// One criterion across the three columns: its code, the strength each record used, and today's strength.
    /// Only today's cell is outlined, and only when it differs from the latest record.
    fn history_row(
        &self,
        re: &Reevaluation,
        row: &classivar_core::Row,
        border: Hsla,
        cx: &mut Context<Self>,
    ) -> (Div, Div, Div) {
        let cell_row = || div().h(px(HISTORY_ROW_H)).flex().items_center().border_t_1().border_color(border);
        let today_index = row.cells.len() - 1;
        let label = cell_row().child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.code.clone()));
        let mut line = cell_row();
        for strength in &row.cells[..today_index] {
            line = line.child(history_cell(self.evidence_chip(&row.code, *strength, false, cx)));
        }
        let changed = re.latest_index.is_some_and(|l| row.cells[l] != row.cells[today_index]);
        let today = cell_row().child(self.evidence_chip(&row.code, row.cells[today_index], changed, cx));
        (label, line, today)
    }

    /// What the VUS sub-tiers mean, with their sources. Shown when a record reported a VUS.
    pub(super) fn tier_reference(&self, cx: &mut Context<Self>) -> AnyElement {
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
                    .child(caption(muted).child(format!("Source: {citation}"))),
            );
        }
        let closing = if tiers_are_ranged(self.rules) {
            "Laboratories use these tiers in their own way. This rule file gives official point ranges to every tier, so a VUS result carries the tier the evidence falls in."
        } else {
            "Laboratories use these tiers in their own way, and no official point ranges exist yet (they are expected with ACMG SVCv4.0). ClassiVAR records the tier a lab named, and does not assign one itself."
        };
        list.child(caption(muted).child(closing)).into_any_element()
    }
}

const HISTORY_ROW_H: f32 = 36.;
const HISTORY_LABEL_W: f32 = 76.;
const HISTORY_COL_W: f32 = 132.;
/// The height of a column head and of the Total row. Every column uses the same
/// number, so the three stacks line up whatever the text inside wraps to.
const HISTORY_HEAD_H: f32 = 44.;
const HISTORY_TOTAL_H: f32 = 60.;

fn history_cell(content: AnyElement) -> Div {
    div().flex_1().min_w(px(HISTORY_COL_W)).px_1().child(content)
}

/// A two-line column head: a title and a muted line under it.
fn history_head(title: String, note: String, muted: Hsla) -> Div {
    div()
        .h(px(HISTORY_HEAD_H))
        .flex()
        .flex_col()
        .justify_center()
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).truncate().child(title))
        .child(caption(muted).truncate().child(note))
}

/// The three column heads: the codes, one for each record, and today.
fn history_heads(re: &Reevaluation, muted: Hsla) -> (Div, Div, Div) {
    let labels = div().flex().flex_col().child(history_head("Criterion".into(), "code".into(), muted));
    let mut heads = div().flex().items_start();
    for r in &re.records {
        let reported = match &r.reported_tier {
            Some(tier) => format!("reported {} · {tier}", r.reported_class.short()),
            None => format!("reported {}", r.reported_class.short()),
        };
        let head = history_head(format!("{} · {}", r.label, r.year), reported, muted);
        heads = heads.child(history_cell(head.into_any_element()));
    }
    let records = div().flex().flex_col().w_full().min_w(px(HISTORY_COL_W * re.records.len() as f32)).child(heads);
    let today = div().flex().flex_col().child(history_head("Today (you)".into(), "your evidence".into(), muted));
    (labels, records, today)
}

/// The "Total" row of the three columns.
fn history_totals(re: &Reevaluation, border: Hsla, muted: Hsla) -> (Div, Div, Div) {
    let total_row = || div().h(px(HISTORY_TOTAL_H)).pt_2().border_t_1().border_color(border);
    let labels = total_row().child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("Total"));
    let mut records = total_row().flex().items_start();
    for r in &re.records {
        let mut col = div().flex().flex_col().child(div().text_sm().child(Workbench::total_text(&r.assessment)));
        if r.consistency() == Consistency::Differs {
            col = col.child(
                caption(muted)
                    .child(format!("! it reported {}; its evidence scores differently", r.reported_class.short())),
            );
        }
        records = records.child(history_cell(col.into_any_element()));
    }
    let today =
        total_row().child(div().text_sm().font_weight(FontWeight::BOLD).child(Workbench::total_text(&re.today)));
    (labels, records, today)
}
