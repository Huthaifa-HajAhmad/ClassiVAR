//! One row of the criteria list.

use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

use super::*;

impl Workbench {
    pub(super) fn criterion_row(&self, c: &'static Criterion, cx: &mut Context<Self>) -> AnyElement {
        let applied_fill = cx.theme().muted;
        let choice = self.current().get(&c.code);
        let strength = choice.map(|choice| choice.strength);
        let reason = choice.and_then(|choice| choice.reason.clone());
        let raise_open =
            self.editing == Editing::Today && self.raising.as_ref().is_some_and(|raise| raise.code == c.code);

        let selector = format!("criterion-{}", c.code);
        div()
            .debug_selector(move || selector)
            .flex()
            .flex_col()
            .gap_1()
            .px_2()
            .py_1p5()
            .rounded(px(crate::look::CARD_RADIUS))
            .when(strength.is_some(), |d| d.bg(applied_fill))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(self.criterion_apply_button(c, strength.is_some(), cx))
                    .child(self.criterion_title(c, cx))
                    .when_some(strength, |row, strength| row.child(self.criterion_strength_button(c, strength, cx)))
                    .child(self.criterion_detail_button(c, cx)),
            )
            .when_some(reason, |col, reason| {
                col.child(
                    div()
                        .pl(px(66.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Reason: {reason}")),
                )
            })
            .when(raise_open, |col| col.child(self.raise_prompt(c, cx)))
            .into_any_element()
    }

    /// The criterion's code: applies it, opens its guide, or removes it. It is the keyboard path.
    fn criterion_apply_button(&self, c: &'static Criterion, applied: bool, cx: &mut Context<Self>) -> Div {
        let state = if applied { "applied" } else { "not applied" };
        let theme = cx.theme();
        let style = if applied {
            let fill: Hsla = direction_colour(c.direction).into();
            ButtonCustomVariant::new(cx)
                .color(fill)
                .foreground(direction_ink().into())
                .hover(fill.opacity(0.88))
                .active(fill)
        } else {
            ButtonCustomVariant::new(cx)
                .color(theme.transparent)
                .foreground(theme.foreground)
                .hover(theme.accent)
                .active(theme.accent)
        };
        let selector = format!("criterion-code-{}", c.code);
        div().debug_selector(move || selector).w(px(58.)).flex_none().child(
            Button::new(SharedString::from(format!("apply-{}", c.code)))
                .custom(style)
                .w_full()
                .rounded(px(7.))
                .when(!applied, |b| b.border_1().border_color(theme.border))
                .child(crate::look::figure(11.5).child(c.code.clone()))
                .accessibility_label(format!("{}: {} ({state})", c.code, c.title))
                .small()
                .selected(applied)
                .toggled(applied)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle(c);
                    cx.notify();
                })),
        )
    }

    /// The rest of the row does the same as the code button for the pointer, so a
    /// criterion is applied by clicking its name as well as its code. The buttons
    /// beside it keep their own clicks: a click on them never reaches this container.
    fn criterion_title(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Stateful<Div> {
        let muted = cx.theme().muted_foreground;
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .id(SharedString::from(format!("criterion-title-{}", c.code)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle(c);
                cx.notify();
            }))
            .child(div().text_sm().min_w_0().child(c.title.clone()))
            .when_some(discouraged_row_note(c.discouraged, self.editing), |row, note| {
                row.child(caption(muted).child(note))
            })
    }

    /// The strength button. A guided criterion reopens its guide; any other opens a
    /// menu of the strengths it may take here, with the current one ticked.
    fn criterion_strength_button(
        &self,
        c: &'static Criterion,
        strength: Strength,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let guided = self.editing == Editing::Today && !c.guide.is_empty();
        let button = Button::new(SharedString::from(format!("strength-{}", c.code)))
            .label(if guided { strength.label().to_string() } else { format!("{} ▾", strength.label()) })
            .accessibility_label(format!("{} strength: {}. Click to change.", c.code, strength.label()))
            .small()
            .outline()
            .rounded(px(999.));
        if guided {
            return button
                .tooltip("Click to answer the strength guide again")
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.next_strength(c, window, cx);
                    cx.notify();
                }))
                .into_any_element();
        }
        let cap = c.max_strength.map(|limit| format!("Highest level in these rules: {}.", limit.label()));
        let options = self.strength_options(c);
        let this = cx.entity().downgrade();
        button
            .dropdown_menu(move |mut menu, _, _| {
                if let Some(cap) = &cap {
                    menu = menu.label(cap.clone());
                }
                for option in &options {
                    let (this, option) = (this.clone(), *option);
                    menu = menu.item(PopupMenuItem::new(option.label()).checked(option == strength).on_click(
                        move |_, window, cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.choose_strength(c, option, window, cx);
                                cx.notify();
                            });
                        },
                    ));
                }
                menu
            })
            .into_any_element()
    }

    /// "More": the criterion's page, with what it means, its strengths and its sources.
    fn criterion_detail_button(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Div {
        let code = c.code.clone();
        let button = Button::new(SharedString::from(format!("detail-{}", c.code)))
            .icon(Icon::new(IconName::ChevronRight))
            .label("More")
            .tooltip("What it means, its strengths and what to check")
            .accessibility_label(format!("More about {}: {}", c.code, c.title))
            .small()
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.about = Some(code.clone());
                cx.notify();
            }));
        super::focus::tagged(format!("criterion-more-{}", c.code), button)
    }
}
