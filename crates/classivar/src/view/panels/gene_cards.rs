//! The gene sources: a summary card with one row per source, and the four detail cards.

use super::*;

impl Workbench {
    /// One row per gene source: a mark for where it stands, its name, and its answer
    /// in one sentence, so the answer is visible without opening anything.
    pub(super) fn source_row<T>(
        label: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        cx: &App,
    ) -> Option<Div> {
        let muted = cx.theme().muted_foreground;
        let (mark, text, failed) = match lookup {
            Lookup::Idle => return None,
            Lookup::Loading => ("…", "Checking…".to_string(), false),
            Lookup::Failed(message) => {
                ("!", format!("Could not check. {message} This is not \"nothing found\"."), true)
            }
            Lookup::Done(found) => ("✓", describe(found).0, false),
        };
        let row = div()
            .flex()
            .gap_2()
            .px_2()
            .py(px(6.))
            .rounded(px(6.))
            .when(failed, |row| row.bg(warning_fill()).text_color(warning_ink()))
            .child(div().w(px(12.)).flex_none().font_family(crate::look::MONO).child(mark))
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
                    .label(if self.gene_cards_open { "Hide details" } else { "Details" })
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.gene_cards_open = !this.gene_cards_open;
                        cx.notify();
                    })),
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

    pub(super) fn gene_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        let g = &self.gene;
        div()
            .id("gene-cards")
            .flex()
            .flex_wrap()
            .gap_2()
            // Four cards can be taller than the window; keep them in a short
            // scrolling strip so the criteria list and the result stay reachable.
            .max_h(px(360.))
            .overflow_y_scroll()
            .child(self.source_card("ClinGen gene-disease validity", &g.clingen, describe_clingen, cx))
            .child(self.source_card("Gene2Phenotype (G2P)", &g.g2p, describe_g2p, cx))
            .child(self.source_card("PanelApp England", &g.england, describe_panelapp, cx))
            .child(self.source_card("PanelApp Australia", &g.australia, describe_panelapp, cx))
            .into_any_element()
    }

    /// One source's full answer. A failure is shown as a failure, never as "nothing found".
    pub(super) fn source_card<T>(
        &self,
        title: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (border, background) = (cx.theme().border, cx.theme().background);
        div()
            .flex_1()
            // Narrow enough that all four cards share one row, even at the 960 px minimum window.
            .min_w(px(220.))
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
            .child(self.source_card_content(title, lookup, describe, cx))
            .into_any_element()
    }

    /// What is under the card's title: waiting, the failure, or the answer with its detail lines.
    fn source_card_content<T>(
        &self,
        title: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        match lookup {
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
                let expanded = self.expanded_cards.contains(title);
                let total = lines.len();
                let shown = if expanded { total } else { CARD_LINES };
                let more = (total > CARD_LINES).then(|| {
                    let label = if expanded { "Show fewer".to_string() } else { format!("Show all {total}") };
                    div().child(
                        Button::new(SharedString::from(format!("card-more-{title}")))
                            .label(label.clone())
                            .accessibility_label(format!("{label} {title}"))
                            .small()
                            .outline()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.expanded_cards.remove(title) {
                                    this.expanded_cards.insert(title);
                                }
                                cx.notify();
                            })),
                    )
                });
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(caption(muted).child(format!("fetched {}", found.fetched_on)))
                    .child(div().text_sm().child(headline))
                    .children(lines.into_iter().take(shown).map(|line| caption(muted).child(line)))
                    .children(more)
                    .into_any_element()
            }
        }
    }
}
