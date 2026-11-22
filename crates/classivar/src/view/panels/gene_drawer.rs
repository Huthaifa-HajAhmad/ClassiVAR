//! The gene sources in the right panel: while open they take the place of the
//! evidence form, one source under the other. Every record is a button; a click
//! takes it as the condition of what is being edited (today's case, or the open
//! earlier report), and Back returns to the form.
//! The inheritance read from PanelApp's and G2P's wording is a plain tested
//! function; the curator can change it afterwards.

use classivar_core::Condition;
use classivar_sources::clingen::Curation;
use classivar_sources::panelapp::PanelAppGene;

use super::source_inheritance::{inheritance_from_g2p, inheritance_from_panelapp, named};
use super::*;
use crate::view::variant::curation_is_chosen;

/// What a click on a record takes.
#[derive(Clone)]
pub(super) enum Picked {
    /// A ClinGen rating: on today's case it also suggests the gene answer.
    ClinGen(Curation),
    /// A condition from G2P or PanelApp, and the source's name for the banner.
    Named(Condition, &'static str),
}

/// One record as a button: its name, a line about it, an optional rating, and the condition it gives.
struct Row {
    id: String,
    name: String,
    meta: String,
    rating: Option<(String, Rgba, Rgba)>,
    chosen: bool,
    picked: Picked,
}

impl Workbench {
    /// The condition the sources are choosing.
    fn scope_condition(&self) -> Option<&Condition> {
        match self.sources_for {
            Editing::Today => self.variant.condition.as_ref(),
            Editing::Record(i) => self.records.get(i).and_then(|r| r.condition.as_ref()),
        }
    }

    pub(super) fn gene_drawer(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.gene_cards_open || matches!(self.gene.clingen, Lookup::Idle) {
            return None;
        }
        let g = &self.gene;
        let columns = div()
            .flex()
            .flex_col()
            .gap_6()
            .child(self.drawer_column("ClinGen validity", &g.clingen, describe_clingen, self.clingen_rows(), cx))
            .child(self.drawer_column("Gene2Phenotype", &g.g2p, describe_g2p, self.g2p_rows(), cx))
            .child(self.drawer_column(
                "PanelApp England",
                &g.england,
                describe_panelapp,
                panel_rows("england", &g.england, self.scope_condition()),
                cx,
            ))
            .child(self.drawer_column(
                "PanelApp Australia",
                &g.australia,
                describe_panelapp,
                panel_rows("australia", &g.australia, self.scope_condition()),
                cx,
            ));
        let body = div().id("gene-drawer-body").flex_1().min_h_0().overflow_y_scroll().px_5().py_4().child(columns);
        Some(div().flex_1().min_h_0().flex().flex_col().child(self.drawer_head(cx)).child(body).into_any_element())
    }

    fn drawer_head(&self, cx: &mut Context<Self>) -> Div {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let target = match self.sources_for {
            Editing::Today => "today's case".to_string(),
            Editing::Record(i) => self.records.get(i).map_or(String::new(), |r| r.label.clone()),
        };
        div()
            .flex()
            .items_start()
            .gap_2()
            .px_4()
            .pt_4()
            .pb_3()
            .border_b_1()
            .border_color(border)
            .child(
                Button::new("gene-drawer-close")
                    .icon(Icon::new(IconName::ChevronLeft))
                    .accessibility_label("Back to the evidence")
                    .tooltip("Back to the evidence")
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.gene_cards_open = false;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(crate::look::eyebrow(format!("Gene sources · {}", self.case_gene), cx))
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Choose the condition"))
                    .child(caption(muted).child(format!(
                        "Click a record to use it as the condition of {target}. Sources are expert opinion about the gene, not your variant."
                    ))),
            )
    }

    /// One source: its name, its state or its answer in one line, and its records.
    fn drawer_column<T>(
        &self,
        title: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        rows: Vec<Row>,
        cx: &mut Context<Self>,
    ) -> Div {
        let muted = cx.theme().muted_foreground;
        let column =
            div().flex().flex_col().gap_2().child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title));
        match lookup {
            Lookup::Idle => column,
            Lookup::Loading => column.child(caption(muted).child("Checking…")),
            Lookup::Failed(message) => column.child(notice(
                format!("Could not check: {message}. This is not \"nothing found\"."),
                warning_fill(),
                warning_ink(),
            )),
            Lookup::Done(found) => column
                .child(caption(muted).child(format!("{} Fetched {}.", describe(found).0, found.fetched_on)))
                .children(rows.into_iter().map(|row| self.pick_row(row, cx))),
        }
    }

    fn clingen_rows(&self) -> Vec<Row> {
        let Lookup::Done(found) = &self.gene.clingen else { return Vec::new() };
        found
            .value
            .curations
            .iter()
            .enumerate()
            .map(|(i, c)| Row {
                id: format!("pick-clingen-{i}"),
                name: c.disease.clone(),
                meta: format!("{} · {} · {} · {}", c.inheritance, c.classification, c.expert_panel, c.classified_on),
                rating: None,
                chosen: curation_is_chosen(self.scope_condition(), c),
                picked: Picked::ClinGen(c.clone()),
            })
            .collect()
    }

    fn g2p_rows(&self) -> Vec<Row> {
        let Lookup::Done(found) = &self.gene.g2p else { return Vec::new() };
        found
            .value
            .records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let condition = named(&r.disease, inheritance_from_g2p(&r.genotype));
                Row {
                    id: format!("pick-g2p-{i}"),
                    name: r.disease.clone(),
                    meta: format!("{} · {} · {}", r.confidence, r.genotype.replace('_', " "), r.molecular_mechanism),
                    rating: None,
                    chosen: self.scope_condition() == Some(&condition),
                    picked: Picked::Named(condition, "Gene2Phenotype"),
                }
            })
            .collect()
    }

    /// A record as a button. The chosen one is marked by its edge, fill and tick, not by words.
    fn pick_row(&self, row: Row, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (ink, muted, border, fill) = (theme.foreground, theme.muted_foreground, theme.border, theme.muted);
        let edge = crate::look::edge(cx);
        let rating = row.rating.map(|(word, back, text)| {
            div().flex_none().px(px(7.)).py(px(1.)).rounded(px(5.)).text_xs().bg(back).text_color(text).child(word)
        });
        let tick = Icon::new(if row.chosen { IconName::CircleCheck } else { IconName::Plus })
            .w(px(16.))
            .h(px(16.))
            .flex_none()
            .text_color(if row.chosen { ink } else { muted });
        let picked = row.picked;
        let selector = row.id.clone();
        let button = Button::new(SharedString::from(row.id))
            .ghost()
            .w_full()
            .h_auto()
            .rounded(px(crate::look::CARD_RADIUS))
            .border_1()
            .border_color(if row.chosen { edge } else { border })
            .when(row.chosen, |b| b.bg(fill))
            .accessibility_label(format!("Use {} as the condition", row.name))
            .tooltip("Use as the condition")
            .child(row_body(row.name, row.meta, rating, tick, (ink, muted)))
            .on_click(cx.listener(move |this, _, window, cx| this.take_pick(picked.clone(), window, cx)));
        div().w_full().debug_selector(move || selector).child(button).into_any_element()
    }
}

/// Inside a record's button: the name (wrapping), the rating pill and facts under it, and the tick.
fn row_body(name: String, meta: String, rating: Option<Div>, tick: Icon, (ink, muted): (Hsla, Hsla)) -> Div {
    let words = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_start()
        .gap_0p5()
        .child(
            div()
                .w_full()
                .text_left()
                .whitespace_normal()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink)
                .child(name),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .children(rating)
                .child(caption(muted).text_left().whitespace_normal().child(meta)),
        );
    div().w_full().flex().items_start().gap_3().py_2().child(words).child(tick)
}

/// PanelApp's panels as rows, each with its traffic-light rating in words and colour.
fn panel_rows(key: &str, lookup: &Lookup<PanelAppGene>, chosen: Option<&Condition>) -> Vec<Row> {
    let Lookup::Done(found) = lookup else { return Vec::new() };
    found
        .value
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let condition = named(&e.panel_name, inheritance_from_panelapp(&e.mode_of_inheritance));
            let (back, text) = crate::design::panel_rating(e.confidence);
            let mode = if e.mode_of_inheritance.is_empty() { "inheritance not given" } else { &e.mode_of_inheritance };
            Row {
                id: format!("pick-{key}-{i}"),
                name: e.panel_name.clone(),
                meta: format!("v{} · {mode}", e.panel_version),
                rating: Some((e.confidence.label().to_string(), back, text)),
                chosen: chosen == Some(&condition),
                picked: Picked::Named(condition, "PanelApp"),
            }
        })
        .collect()
}
