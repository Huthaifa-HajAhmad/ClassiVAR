//! The criteria, one tab per kind of evidence (as in the 2015 table): the open tab
//! shows its evidence helpers, then the codes for disease and against it. Each tab
//! counts the codes applied in it.

use super::*;

impl Workbench {
    /// Above the tabs: how many criteria are applied, and Clear criteria
    /// once there is something to clear. The points are in the readout, so they are not repeated here.
    pub(super) fn criteria_bar(&self, cx: &mut Context<Self>) -> Div {
        let applied = self.current().len();
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

    /// The kinds of evidence that have criteria in the loaded rules.
    fn kinds(&self) -> Vec<Category> {
        Category::ALL.into_iter().filter(|kind| self.rules.criteria.iter().any(|c| c.category == *kind)).collect()
    }

    /// The criteria behind tabs, one tab per kind of evidence (as in the 2015 table), in a
    /// row that wraps above them, so the criteria and helper cards keep the panel's full width.
    pub(super) fn criteria_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let kinds = self.kinds();
        let Some(open) = self.category.filter(|kind| kinds.contains(kind)).or(kinds.first().copied()) else {
            return div().into_any_element();
        };
        let tabs: Vec<Div> = kinds.iter().map(|kind| self.criteria_tab(*kind, *kind == open, cx)).collect();
        let border = cx.theme().border;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().flex().flex_wrap().gap_1().pb_2().border_b_1().border_color(border).children(tabs))
            .children(self.helper_cards(open, cx))
            .child(self.criteria_of(open, cx))
            .into_any_element()
    }

    /// The criteria of one kind: the codes for disease, then those against it.
    fn criteria_of(&self, kind: Category, cx: &mut Context<Self>) -> Div {
        let mut list = div().flex().flex_col().gap_0p5();
        for direction in [Direction::Pathogenic, Direction::Benign] {
            let codes: Vec<&'static Criterion> =
                self.rules.criteria.iter().filter(|c| c.direction == direction && c.category == kind).collect();
            if codes.is_empty() {
                continue;
            }
            list = list.child(direction_label(direction, cx));
            for c in codes {
                list = list.child(self.criterion_row(c, cx));
            }
        }
        list
    }

    /// One tab: the kind of evidence and, once some of its criteria apply, how many, in a small filled count.
    fn criteria_tab(&self, kind: Category, open: bool, cx: &mut Context<Self>) -> Div {
        let applied =
            self.rules.criteria.iter().filter(|c| c.category == kind && self.current().contains_key(&c.code)).count();
        let total = self.rules.criteria.iter().filter(|c| c.category == kind).count();
        let theme = cx.theme();
        let (ink, muted) = (theme.foreground, theme.muted_foreground);
        let count = (applied > 0).then(|| {
            crate::look::figure(crate::look::SMALL_TEXT)
                .px(px(6.))
                .rounded_full()
                .bg(theme.foreground)
                .text_color(theme.background)
                .child(applied.to_string())
        });
        let id = format!("criteria-tab-{}", kind.label().to_lowercase().replace(' ', "-"));
        let selector = id.clone();
        let tab = crate::look::tab(id, open, cx)
            .accessibility_label(format!("{} criteria, {applied} of {total} applied", kind.label()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_left()
                            .text_color(if open { ink } else { muted })
                            .whitespace_nowrap()
                            .when(open, |name| name.font_weight(FontWeight::SEMIBOLD))
                            .child(kind.label()),
                    )
                    .children(count),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.category = Some(kind);
                cx.notify();
            }));
        div().debug_selector(move || selector).child(tab)
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
