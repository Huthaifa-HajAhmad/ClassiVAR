//! One row of the criteria list.

use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

use super::*;

impl Workbench {
    pub(super) fn criterion_row(&self, c: &'static Criterion, cx: &mut Context<Self>) -> AnyElement {
        let applied_fill = cx.theme().muted;
        let choice = self.current().get(&c.code);
        let strength = choice.map(|choice| choice.strength);
        let reason = choice.and_then(|choice| choice.reason.clone());
        let guide_open = self.editing == Editing::Today && self.guide_open.as_deref() == Some(c.code.as_str());
        let raise_open =
            self.editing == Editing::Today && self.raising.as_ref().is_some_and(|raise| raise.code == c.code);
        let open = self.open_details.contains(&c.code);

        div()
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
                    .child(self.criterion_detail_button(c, open, cx)),
            )
            .when(open, |col| col.child(self.criterion_explanation(c, choice, cx)))
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
            .when(guide_open, |col| col.child(self.strength_guide(c, cx)))
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
        div().w(px(58.)).flex_none().child(
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

    fn criterion_detail_button(&self, c: &'static Criterion, open: bool, cx: &mut Context<Self>) -> Button {
        let word = if open { "Less" } else { "More" };
        let code = c.code.clone();
        Button::new(SharedString::from(format!("detail-{}", c.code)))
            .icon(Icon::new(if open { IconName::ChevronUp } else { IconName::ChevronDown }))
            .label(word)
            .tooltip(if open { "Hide the rule" } else { "Read the rule" })
            .accessibility_label(format!("{word} about {}: {}", c.code, c.title))
            .small()
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.open_details.remove(&code) {
                    this.open_details.insert(code.clone());
                }
                cx.notify();
            }))
    }

    /// The rule in plain words under the row, opened with "More": all of it,
    /// with the note, the source and (for a guided criterion) the way to remove it.
    fn criterion_explanation(&self, c: &'static Criterion, choice: Option<&Choice>, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let strength = choice.map(|choice| choice.strength);
        let guide_option = choice.and_then(|choice| choice.situation.as_deref()).and_then(|id| c.guide_option(id));
        let mut text = div().pl(px(66.)).flex().flex_col().gap_1().text_xs().text_color(muted);
        text = text.child(div().child(c.summary.clone()));
        if let Some(note) = c.note.clone() {
            text = text.child(format!("Note: {note}"));
        }
        text = text.child(self.strength_ladder(c, strength, cx));
        text = text.children(self.interactions_panel(c, cx));
        if let Some(option) = guide_option {
            text = text.child(format!("Guide answer: {} — {}", option.short, option.situation));
        }
        if !c.guide.is_empty() && strength.is_none() {
            text = text.child(
                div().child("Click the code to answer a short decision tree; nothing is applied until you choose."),
            );
        }
        if strength.is_some() && self.editing == Editing::Today {
            text = text.child(self.criterion_note_button(c, cx));
        }
        if strength.is_some() && !c.guide.is_empty() && self.editing == Editing::Today {
            text = text.child(self.criterion_remove_button(c, cx));
        }
        if let Some(citation) = self.rules.source(&c.source).map(|s| s.citation.clone()) {
            text = text.child(format!("Source: {citation}"));
        }
        text
    }

    /// Note what the evidence behind an applied criterion was (any criterion, not only a raised one).
    fn criterion_note_button(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Button {
        let code = c.code.clone();
        let has_note = self.today.get(&c.code).is_some_and(|choice| choice.reason.is_some());
        Button::new(SharedString::from(format!("note-{}", c.code)))
            .label(if has_note { "Edit note" } else { "Add a note" })
            .accessibility_label(format!("Note what you looked at for {}", c.code))
            .small()
            .ghost()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.start_note(&code, window, cx);
                cx.notify();
            }))
    }

    /// Removing a guided criterion has its own button: its code button only reopens the guide.
    fn criterion_remove_button(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Button {
        let code = c.code.clone();
        Button::new(SharedString::from(format!("remove-{}", c.code)))
            .label(format!("Remove {}", c.code))
            .accessibility_label(format!("Remove {}", c.code))
            .small()
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.remove_criterion(&code);
                cx.notify();
            }))
    }
}
