//! The opening screen keeps the fictional example separate from a new case.
use super::*;

impl Workbench {
    pub(super) fn welcome_screen(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (paper, ink, muted, border) = (theme.background, theme.foreground, theme.muted_foreground, theme.border);
        let header = div()
            .flex()
            .items_center()
            .gap_4()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(border)
            .child(caption(muted).child("Research and learning only"))
            .child(div().flex_1())
            .child(self.theme_button(cx));
        let spread = div()
            .w_full()
            .max_w(px(1240.))
            .px_6()
            .py_6()
            .flex()
            .items_center()
            .gap_6()
            .child(self.welcome_intro(cx))
            .child(self.example_figure(cx));
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(paper)
            .text_color(ink)
            .child(header)
            .children(self.file_status_banner(cx))
            .child(
                div()
                    .id("welcome-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(div().min_h_full().flex().items_center().justify_center().child(spread)),
            )
            .into_any_element()
    }

    fn welcome_actions(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_wrap()
            .gap_3()
            .child(Button::new("welcome-new").primary().label("Start new variant").on_click(cx.listener(
                |this, _, _, cx| {
                    this.set_mode(Mode::NewVariant);
                    cx.notify();
                },
            )))
            .child(Button::new("welcome-reported").outline().label("Review reported variant").on_click(cx.listener(
                |this, _, _, cx| {
                    this.set_mode(Mode::ReportedVariant);
                    cx.notify();
                },
            )))
    }

    fn welcome_intro(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;

        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_5()
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("ACMG / AMP"))
            .child(
                div()
                    .text_size(px(44.))
                    .line_height(px(48.))
                    .font_weight(FontWeight::BOLD)
                    .child("What explains a variant’s classification?"),
            )
            .child(
                div().text_lg().text_color(muted).child(
                    "Trace the contribution of each criterion, or compare today’s evidence with an earlier report.",
                ),
            )
            .child(self.welcome_actions(cx))
            .child(
                div().child(
                    Button::new("welcome-open")
                        .ghost()
                        .label("Open saved case")
                        .on_click(cx.listener(|this, _, window, cx| this.open_case(window, cx))),
                ),
            )
    }

    fn example_figure(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let card = div()
            .flex_1()
            .min_w_0()
            .p_6()
            .rounded_xl()
            .flex()
            .flex_col()
            .gap_4()
            .bg(theme.primary)
            .text_color(theme.primary_foreground)
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("Fictional case"))
            .child(div().text_2xl().font_weight(FontWeight::BOLD).child("How the evidence adds up"));
        let applied = to_applied(&evidence_of(super::super::example::EXAMPLE_TODAY));
        let Ok(result) = self.rules.assess(&applied) else {
            return card.child("The fictional example is unavailable with the loaded rules.");
        };
        let label = match result.class() {
            Some(class) => format!("{} points · {}", result.total_points, class.label()),
            None => "Class withheld".into(),
        };
        card.child(example_steps(&result))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(label))
            .child(div().text_xs().child(rules_tag(self.rules)))
            .child(div().debug_selector(|| "welcome-example".into()).child(
                Button::new("explore-example").label("Explore fictional case").on_click(cx.listener(
                    |this, _, window, cx| {
                        this.load_example(window, cx);
                        if this.example {
                            this.set_mode(Mode::ReportedVariant);
                            this.reading = true;
                        }
                        cx.notify();
                    },
                )),
            ))
    }
}

fn example_steps(result: &Assessment) -> Div {
    let total: f32 = result.items.iter().filter(|i| i.counted).map(|i| i.points.unsigned_abs() as f32).sum();
    let mut figure = div().flex().flex_col().gap_3();
    let mut offset = 0.;
    for item in result.items.iter().filter(|i| i.counted) {
        let width = item.points.unsigned_abs() as f32 / total;
        let track = div()
            .flex_1()
            .flex()
            .items_center()
            .h(px(22.))
            .child(div().w(relative(offset)))
            .child(div().w(relative(width)).h(px(18.)).rounded_sm().bg(direction_colour(item.direction)));
        figure = figure.child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().w(px(46.)).font_weight(FontWeight::SEMIBOLD).child(item.code.clone()))
                .child(track)
                .child(div().w(px(32.)).child(format!("{:+}", item.points))),
        );
        offset += width;
    }
    figure
}
