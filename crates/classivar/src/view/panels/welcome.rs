//! The opening screen: what the app does, how to start, and how the class is drawn.
use super::*;

impl Workbench {
    pub(super) fn welcome_screen(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (paper, ink) = (theme.background, theme.foreground);
        let header =
            div().flex().items_center().gap_4().px_6().py_3().child(div().flex_1()).child(self.theme_button(cx));
        let spread =
            div().w_full().flex().items_center().gap_8().child(self.welcome_intro(cx)).child(self.empty_figure(cx));
        let page = div()
            .w_full()
            .max_w(px(1180.))
            .mx_auto()
            .px_8()
            .pt_6()
            .pb_8()
            .flex()
            .flex_col()
            .gap_12()
            .child(spread)
            .child(self.picture_choices(cx))
            .child(self.promises(cx))
            .child(self.welcome_footer(cx));
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(paper)
            .text_color(ink)
            .child(header)
            .children(self.file_status_banner(self.has_unsaved_changes(cx), cx))
            .child(div().id("welcome-scroll").flex_1().min_h_0().overflow_y_scroll().child(page))
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
            .child(Button::new("welcome-reported").ghost().label("Review reported variant").on_click(cx.listener(
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
            .child(crate::look::eyebrow("ACMG / AMP · variant evidence", cx))
            .child(
                div()
                    .text_size(px(46.))
                    .line_height(px(52.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("What explains a variant’s classification?"),
            )
            .child(div().text_lg().text_color(muted).child(
                "See how each criterion you choose moves the class, and how today’s evidence compares with earlier lab reports.",
            ))
            .child(self.welcome_actions(cx))
            .child(
                div().flex().child(
                    Button::new("welcome-open")
                        .ghost()
                        .label("Open saved case")
                        .on_click(cx.listener(|this, _, window, cx| this.open_case(window, cx))),
                ),
            )
    }

    /// The dial with no evidence on it: grey bands, no needle and no class, so the
    /// opening screen shows the instrument without suggesting any class.
    fn empty_figure(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let card = crate::look::panel(cx)
            .flex_1()
            .min_w_0()
            .p_6()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(crate::look::eyebrow("How the evidence adds up", cx));
        let Ok(result) = self.rules.assess(&[]) else {
            return card;
        };
        card.child(div().debug_selector(|| "welcome-dial".into()).child(self.dial(
            &result,
            None,
            (460., 330.),
            false,
            cx,
        )))
        .child(
            caption(muted).child("Each criterion you choose moves the needle. No class is shown until you choose one."),
        )
        .child(super::picture::status_pill(self.rules, cx))
    }
}
