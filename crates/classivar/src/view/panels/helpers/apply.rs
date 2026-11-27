//! The button under a helper's reading. A reading is only a proposal: it counts once
//! the curator presses Apply. When it contradicts a criterion applied today (for
//! example BS1 while PM2 is applied), the button names the replacement, and Keep
//! sets the proposal aside instead.

use super::super::focus::tagged;
use super::*;
use classivar_core::EvidenceReading;

impl Workbench {
    /// Apply `reading`, or, against an applied rival, "Replace X with Y" and "Keep X".
    /// `id` is the plain Apply button's id; the other two add `-replace` and `-keep`.
    pub(super) fn reading_apply(
        &self,
        id: impl Into<SharedString>,
        reading: EvidenceReading,
        reason: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id: SharedString = id.into();
        let what = format!("{} {}", reading.code, reading.strength.label());
        let Some(rival) = crate::view::guided::rival(self.rules, |code| self.today.contains_key(code), reading.code)
        else {
            let label = format!("Apply {what}");
            let button = Button::new(id.clone())
                .label(label.clone())
                .accessibility_label(format!("{label} to today's evidence"))
                .small()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.apply_reading(&reading, reason.clone());
                    cx.notify();
                }));
            return tagged(id.to_string(), button).into_any_element();
        };
        let muted = cx.theme().muted_foreground;
        if self.kept.contains(reading.code) {
            return caption(muted).child(format!("You kept {rival}; {what} is not applied.")).into_any_element();
        }
        let label = format!("Replace {rival} with {what}");
        let code = reading.code;
        let replace = Button::new(SharedString::from(format!("{id}-replace")))
            .label(label.clone())
            .accessibility_label(format!("{label} in today's evidence"))
            .small()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.replace_with_reading(rival, &reading, reason.clone());
                cx.notify();
            }));
        let keep = Button::new(SharedString::from(format!("{id}-keep")))
            .label(format!("Keep {rival}"))
            .accessibility_label(format!("Keep {rival} and set the {code} proposal aside"))
            .small()
            .outline()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.keep_rival(code);
                cx.notify();
            }));
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(caption(muted).child(format!("{rival} is applied and cannot be used with {code}.")))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(tagged(format!("{id}-replace"), replace))
                    .child(tagged(format!("{id}-keep"), keep)),
            )
            .into_any_element()
    }
}
