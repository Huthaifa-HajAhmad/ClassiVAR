//! The criteria, one section per kind of evidence (as in the 2015 table): the
//! codes for disease and against it sit together under one heading, and each
//! section folds away. A folded section still names the codes applied in it.

use super::*;

impl Workbench {
    /// Above the sections: how many criteria are applied, a fold-all button, and Clear criteria
    /// once there is something to clear. The points are in the readout, so they are not repeated here.
    pub(super) fn criteria_bar(&self, cx: &mut Context<Self>) -> Div {
        let applied = self.current().len();
        let all_folded = self.folded.len() == Category::ALL.len();
        let fold = Button::new("criteria-fold")
            .icon(Icon::new(if all_folded { IconName::ChevronDown } else { IconName::ChevronUp }))
            .accessibility_label(if all_folded { "Open every kind of evidence" } else { "Fold every kind of evidence" })
            .tooltip(if all_folded { "Open all" } else { "Fold all" })
            .small()
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.folded = if all_folded { HashSet::new() } else { Category::ALL.into_iter().collect() };
                cx.notify();
            }));
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .min_h(px(28.))
            .child(crate::look::eyebrow(
                if applied == 0 { "Criteria".to_string() } else { format!("Criteria · {applied} applied") },
                cx,
            ))
            .child(div().flex_1())
            .child(fold)
            .when(applied > 0, |bar| {
                bar.child(
                    Button::new("clear")
                        .icon(Icon::new(IconName::Delete))
                        .label("Clear criteria")
                        .accessibility_label("Clear every criterion of this report")
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.ask(Confirm::Clear(this.editing));
                            cx.notify();
                        })),
                )
            })
    }

    /// Every kind of evidence that has criteria in the loaded rules, as a section.
    pub(super) fn criteria_sections(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        Category::ALL
            .into_iter()
            .filter(|category| self.rules.criteria.iter().any(|c| c.category == *category))
            .map(|category| self.criteria_section(category, cx))
            .collect()
    }

    fn criteria_section(&self, category: Category, cx: &mut Context<Self>) -> AnyElement {
        let folded = self.folded.contains(&category);
        let mut section = div()
            .flex()
            .flex_col()
            .gap_0p5()
            .pb_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(self.section_head(category, folded, cx));
        if !folded {
            for direction in [Direction::Pathogenic, Direction::Benign] {
                let codes: Vec<&'static Criterion> =
                    self.rules.criteria.iter().filter(|c| c.direction == direction && c.category == category).collect();
                if codes.is_empty() {
                    continue;
                }
                section = section.child(direction_label(direction, cx));
                for c in codes {
                    section = section.child(self.criterion_row(c, cx));
                }
            }
        }
        section.into_any_element()
    }

    /// The section's heading: a chevron, the kind of evidence, the applied codes as small
    /// chips in their direction's colour, and how many criteria it holds. A click folds it.
    fn section_head(&self, category: Category, folded: bool, cx: &mut Context<Self>) -> Button {
        let muted = cx.theme().muted_foreground;
        let in_section: Vec<&'static Criterion> =
            self.rules.criteria.iter().filter(|c| c.category == category).collect();
        let applied: Vec<&'static Criterion> =
            in_section.iter().copied().filter(|c| self.current().contains_key(&c.code)).collect();
        let chips = applied.iter().map(|c| {
            crate::look::figure(10.5)
                .px(px(6.))
                .py(px(1.))
                .rounded(px(5.))
                .bg(direction_colour(c.direction))
                .text_color(direction_ink())
                .child(c.code.clone())
        });
        let action = if folded { "Show" } else { "Fold" };
        Button::new(SharedString::from(format!("section-{}", category.label())))
            .ghost()
            .w_full()
            .h_auto()
            .rounded(px(crate::look::CARD_RADIUS))
            .accessibility_label(format!("{action} {} criteria", category.label()))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .child(
                        Icon::new(if folded { IconName::ChevronRight } else { IconName::ChevronDown })
                            .w(px(14.))
                            .h(px(14.))
                            .text_color(muted),
                    )
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(category.label()))
                    .child(div().flex().flex_wrap().gap_1().children(chips))
                    .child(div().flex_1())
                    .child(crate::look::figure(11.).text_color(muted).child(format!("{}", in_section.len()))),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.folded.remove(&category) {
                    this.folded.insert(category);
                }
                cx.notify();
            }))
    }
}

/// "For disease" or "Against disease" with the direction's dot, over its codes in a section.
fn direction_label(direction: Direction, cx: &App) -> Div {
    let words = match direction {
        Direction::Pathogenic => "For disease",
        Direction::Benign => "Against disease",
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .pt_2()
        .pb_0p5()
        .child(div().size(px(6.)).rounded_full().bg(direction_colour(direction)))
        .child(crate::look::eyebrow(words, cx))
}
