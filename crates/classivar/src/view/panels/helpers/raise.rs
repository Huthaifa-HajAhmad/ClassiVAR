//! The prompt that asks why a strength is raised above its default.

use super::*;

impl Workbench {
    /// Asks why a strength is raised above its default. The raise is only made
    /// once a reason is written; Cancel (or Escape) leaves the strength as it was.
    pub(in crate::view::panels) fn raise_prompt(&self, c: &Criterion, cx: &mut Context<Self>) -> AnyElement {
        let (border, muted, background) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().background);
        let to = self.raising.as_ref().map_or(c.strength, |raise| raise.to);
        let empty = self.boxes.reason.read(cx).value().trim().is_empty();
        let note = self.raising.as_ref().is_some_and(|raise| raise.note);
        let (title, help) = if note {
            (
                format!("What did you look at for {}?", c.code),
                "Name the source and what it showed, with its date (for example \"gnomAD v4.1: absent, read 2026-11-21\"). It is saved with the case and shown next to the criterion.".to_string(),
            )
        } else {
            (
                format!("Why raise {} from {} (the default) to {}?", c.code, c.strength.label(), to.label()),
                "A stronger strength than the guideline default needs a reason. It is saved with the case and shown next to the criterion.".to_string(),
            )
        };
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
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
            .child(caption(muted).child(help))
            .child(Input::new(&self.boxes.reason).aria_label(if note {
                "Note on the evidence"
            } else {
                "Reason for raising the strength"
            }))
            .child(raise_buttons(c, to, empty, note, cx))
            .into_any_element()
    }
}

/// Save (only once a reason is written) and Cancel.
fn raise_buttons(c: &Criterion, to: Strength, empty: bool, note: bool, cx: &mut Context<Workbench>) -> Div {
    let (label, accessible) = if note {
        ("Save note".to_string(), format!("Save the note on {}", c.code))
    } else {
        ("Save reason and raise".to_string(), format!("Save the reason and raise {} to {}", c.code, to.label()))
    };
    div()
        .flex()
        .gap_2()
        .child(
            Button::new(SharedString::from(format!("raise-save-{}", c.code)))
                .label(label)
                .accessibility_label(accessible)
                .small()
                .disabled(empty)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.confirm_raise(cx);
                    cx.notify();
                })),
        )
        .child(
            Button::new(SharedString::from(format!("raise-cancel-{}", c.code)))
                .label("Cancel")
                .accessibility_label(if note {
                    format!("Do not change the note on {}", c.code)
                } else {
                    format!("Do not raise {}", c.code)
                })
                .small()
                .ghost()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.raising = None;
                    cx.notify();
                })),
        )
}
