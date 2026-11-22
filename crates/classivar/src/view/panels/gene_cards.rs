//! The gene sources: a summary card with one row per source. Their full answers
//! open in the drawer (`gene_drawer.rs`).

use super::*;

impl Workbench {
    /// One row per gene source: an icon for where it stands, its name, and its answer
    /// in one sentence, so the answer is visible without opening anything.
    pub(super) fn source_row<T>(
        label: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        cx: &App,
    ) -> Option<Div> {
        let muted = cx.theme().muted_foreground;
        let (icon, text, failed) = match lookup {
            Lookup::Idle => return None,
            Lookup::Loading => (IconName::Loader, "Checking…".to_string(), false),
            Lookup::Failed(message) => {
                (IconName::TriangleAlert, format!("Could not check. {message} This is not \"nothing found\"."), true)
            }
            Lookup::Done(found) => (IconName::CircleCheck, describe(found).0, false),
        };
        let row = div()
            .flex()
            .items_start()
            .gap_2()
            .px_2()
            .py(px(6.))
            .rounded(px(6.))
            .when(failed, |row| row.bg(warning_fill()).text_color(warning_ink()))
            .child(div().pt(px(1.)).child(crate::icons::glyph(icon, cx).when(failed, |i| i.text_color(warning_ink()))))
            .child(div().w(px(112.)).flex_none().font_weight(FontWeight::SEMIBOLD).child(label))
            .child(div().flex_1().min_w_0().when(!failed, |d| d.text_color(muted)).child(text));
        Some(row)
    }

    pub(super) fn gene_summary(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let g = &self.gene;
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .child(crate::look::eyebrow(format!("Gene sources · {}", self.case_gene), cx))
            .child(
                Button::new("gene-cards-toggle")
                    .icon(Icon::new(IconName::PanelRight))
                    .label("Details")
                    .accessibility_label("Open the gene sources")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.open_sources(Editing::Today, cx))),
            );
        crate::look::card(cx)
            .flex()
            .flex_col()
            .gap_1()
            .text_xs()
            .child(head)
            .children(Self::source_row("ClinGen validity", &g.clingen, describe_clingen, cx))
            .children(Self::source_row("Gene2Phenotype", &g.g2p, describe_g2p, cx))
            .children(Self::source_row("PanelApp England", &g.england, describe_panelapp, cx))
            .children(Self::source_row("PanelApp Australia", &g.australia, describe_panelapp, cx))
            .child(
                caption(muted)
                    .pt_1()
                    .child("Sources are expert opinion about the gene, not your variant, and they can disagree."),
            )
            .into_any_element()
    }
}
