//! The criteria list and the strength guide.

use super::*;
use classivar_core::rules::StrengthGuide;

impl Workbench {
    pub(super) fn criteria_panel(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let list = div()
            .id("criteria")
            .track_scroll(&self.criteria_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_4()
            .children(self.helper_section(cx))
            .child(self.criteria_group(Direction::Pathogenic, "For disease (pathogenic)", result.pathogenic_points, cx))
            .child(self.criteria_group(Direction::Benign, "Against disease (benign)", result.benign_points, cx));

        div()
            .flex()
            .flex_col()
            .w(px(480.))
            .h_full()
            .flex_none()
            .border_r_1()
            .border_color(border)
            .child(self.criteria_header(cx))
            .child(list)
            .into_any_element()
    }

    /// The title of the list (which evidence is being edited), how many criteria are applied, and Clear.
    fn criteria_header(&self, cx: &mut Context<Self>) -> Div {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let applied = self.current().len();
        let clear_label = match self.editing {
            Editing::Today => "Clear".to_string(),
            Editing::Record(i) => format!("Clear {}", self.records[i].label),
        };
        let count = if applied == 0 {
            "Click a code to apply it. You decide which criteria apply.".to_string()
        } else {
            format!("{applied} applied")
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_5()
            .py_3()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Criteria · {}", self.editing_label())),
                    )
                    .child(caption(muted).child(count)),
            )
            .child(Button::new("clear").label(clear_label).small().outline().on_click(cx.listener(|this, _, _, cx| {
                if !this.current().is_empty() {
                    this.ask(Confirm::Clear(this.editing));
                }
                cx.notify();
            })))
    }

    /// The criteria of one direction, grouped by category. `points` is the total of that side
    /// (positive for pathogenic, zero or negative for benign).
    fn criteria_group(&self, direction: Direction, title: &'static str, points: i32, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let sign = if direction == Direction::Pathogenic { "+" } else { "−" };
        let mut group = div().flex().flex_col().gap_0p5().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .pb_1()
                .child(div().w(px(10.)).h(px(10.)).rounded_sm().bg(direction_colour(direction)))
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(div().flex_1())
                .child(caption(muted).child(format!("{sign}{} points", points.abs()))),
        );
        let rules = self.rules;
        for category in Category::ALL {
            let in_category: Vec<&'static Criterion> =
                rules.criteria.iter().filter(|c| c.direction == direction && c.category == category).collect();
            if in_category.is_empty() {
                continue;
            }
            group = group.child(div().px_2().pt_2().text_xs().text_color(muted).child(category.label()));
            for c in in_category {
                group = group.child(self.criterion_row(c, cx));
            }
        }
        group
    }

    /// The questions that set a criterion's strength. Nothing is applied until one is answered.
    pub(super) fn strength_guide(&self, c: &Criterion, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let citation = c.guide_source.as_deref().and_then(|id| self.rules.source(id)).map(|s| s.citation.clone());
        let heading = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("Which fits this variant? The answer sets the strength of {}.", c.code)),
            )
            .child(
                Button::new(SharedString::from(format!("guide-cancel-{}", c.code)))
                    .label("Cancel")
                    .accessibility_label(format!("Close the {} strength guide", c.code))
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.guide_open = None;
                        cx.notify();
                    })),
            );
        let answers = c.guide.iter().enumerate().map(|(i, option)| self.guide_answer(c, i, option, cx));
        div()
            .mt_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(background)
            .flex()
            .flex_col()
            .gap_2()
            .child(heading)
            .children(answers)
            .when_some(citation, |d, citation| d.child(caption(muted).child(format!("Source: {citation}"))))
            .into_any_element()
    }

    /// One answer of a strength guide: the button that applies it, and the situation it describes.
    fn guide_answer(&self, c: &Criterion, i: usize, option: &StrengthGuide, cx: &mut Context<Self>) -> Div {
        let (code, strength, situation) = (c.code.clone(), option.strength, option.id.clone());
        let chosen =
            self.current().get(&c.code).and_then(|choice| choice.situation.as_deref()) == Some(option.id.as_str());
        div()
            .flex()
            .items_start()
            .gap_2()
            .child(
                div().w(px(112.)).flex_none().child(
                    Button::new(SharedString::from(format!("guide-{}-{i}", c.code)))
                        .label(strength.map_or("Does not apply", Strength::label))
                        .accessibility_label(format!(
                            "{} {}: {}",
                            c.code,
                            strength.map_or("does not apply", Strength::label),
                            option.short
                        ))
                        .small()
                        .outline()
                        .selected(chosen)
                        .toggled(chosen)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.answer_guide(&code, strength, situation.clone());
                            cx.notify();
                        })),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(option.short.clone()))
                    .child(div().text_xs().child(option.situation.clone())),
            )
    }
}
