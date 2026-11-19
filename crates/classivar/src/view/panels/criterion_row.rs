//! One row of the criteria list.

use super::*;

impl Workbench {
    pub(super) fn criterion_row(&self, c: &'static Criterion, cx: &mut Context<Self>) -> AnyElement {
        let accent = cx.theme().accent;
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
            .rounded_md()
            .when(strength.is_some(), |d| d.bg(accent))
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
            .when(self.learning || open, |col| col.child(self.criterion_explanation(c, choice, open, cx)))
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
        div().w(px(58.)).flex_none().child(
            Button::new(SharedString::from(format!("apply-{}", c.code)))
                .label(c.code.clone())
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

    /// The strength button: cycles the strength, or reopens the guide of a guided criterion.
    fn criterion_strength_button(&self, c: &'static Criterion, strength: Strength, cx: &mut Context<Self>) -> Button {
        let tooltip = if c.guide.is_empty() {
            let cap = c
                .max_strength
                .map(|limit| format!("Capped at {} by this program. ", limit.label()))
                .unwrap_or_default();
            format!("{cap}Click to change the strength")
        } else {
            "Click to answer the strength guide again".to_string()
        };
        Button::new(SharedString::from(format!("strength-{}", c.code)))
            .label(strength.label())
            .accessibility_label(format!("{} strength: {}. Click to change.", c.code, strength.label()))
            .small()
            .outline()
            .tooltip(tooltip)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.next_strength(c, window, cx);
                cx.notify();
            }))
    }

    fn criterion_detail_button(&self, c: &'static Criterion, open: bool, cx: &mut Context<Self>) -> Button {
        let word = if open { "Less" } else { "More" };
        let code = c.code.clone();
        Button::new(SharedString::from(format!("detail-{}", c.code)))
            .label(word)
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

    /// The rule in plain words under the row. Learning on: one or two lines of it. "More": all of it,
    /// with the note, the source and (for a guided criterion) the way to remove it.
    fn criterion_explanation(
        &self,
        c: &'static Criterion,
        choice: Option<&Choice>,
        open: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let muted = cx.theme().muted_foreground;
        let strength = choice.map(|choice| choice.strength);
        let guide_option = choice.and_then(|choice| choice.situation.as_deref()).and_then(|id| c.guide_option(id));
        let clamp = |text: Div| if open { text } else { text.line_clamp(2) };
        let mut text = div().pl(px(66.)).flex().flex_col().gap_1().text_xs().text_color(muted);
        text = text.child(clamp(div().child(c.summary.clone())));
        if open && let Some(note) = c.note.clone() {
            text = text.child(format!("Note: {note}"));
        }
        if open {
            text = text.child(self.strength_ladder(c, strength, cx));
        }
        if let Some(option) = guide_option {
            let answer = if open {
                format!("Guide answer: {} — {}", option.short, option.situation)
            } else {
                format!("Guide answer: {}", option.short)
            };
            text = text.child(clamp(div().child(answer)));
        }
        if !c.guide.is_empty() && strength.is_none() {
            text = text.child(
                div().child("Click the code to answer a short decision tree; nothing is applied until you choose."),
            );
        }
        if open && strength.is_some() && !c.guide.is_empty() && self.editing == Editing::Today {
            text = text.child(self.criterion_remove_button(c, cx));
        }
        if open && let Some(citation) = self.rules.source(&c.source).map(|s| s.citation.clone()) {
            text = text.child(format!("Source: {citation}"));
        }
        text
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
