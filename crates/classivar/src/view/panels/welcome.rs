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
            .max_w(px(1480.))
            .mx_auto()
            .px_12()
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
            .child(
                Button::new("welcome-new")
                    .primary()
                    .icon(Icon::new(IconName::Plus))
                    .label("Start new variant")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_mode(Mode::NewVariant);
                        cx.notify();
                    })),
            )
            .child(
                Button::new("welcome-reported")
                    .outline()
                    .icon(Icon::new(IconName::FileText))
                    .label("Review reported variant")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_mode(Mode::ReportedVariant);
                        cx.notify();
                    })),
            )
            .child(
                Button::new("welcome-open")
                    .outline()
                    .icon(Icon::new(IconName::FolderOpen))
                    .label("Open saved case")
                    .on_click(cx.listener(|this, _, window, cx| this.open_case(window, cx))),
            )
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
    }
}
