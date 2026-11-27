//! The button under a helper's reading. A reading is only a proposal: it counts once
//! the curator presses Apply. When it contradicts one or more criteria applied today
//! (for example BS1 while PM2 is applied, or PM2 while BA1 and BS1 are applied), the
//! button names every replacement, and Keep sets the proposal aside instead.

use super::super::focus::tagged;
use super::*;
use crate::view::guided::{join_codes, rivals};
use classivar_core::EvidenceReading;

impl Workbench {
    /// Apply `reading`, or, against applied rivals, "Replace X with Y" and "Keep X".
    /// `id` is the plain Apply button's id; the others add `-replace`, `-keep`, `-reconsider`.
    pub(super) fn reading_apply(
        &self,
        id: impl Into<SharedString>,
        reading: EvidenceReading,
        reason: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id: SharedString = id.into();
        let what = format!("{} {}", reading.code, reading.strength.label());
        let conflicting = rivals(self.rules, |code| self.today.contains_key(code), reading.code);
        if conflicting.is_empty() {
            return self.plain_apply_button(id, what, reading, reason, cx);
        }
        let names = join_codes(&conflicting);
        if self.kept.contains(reading.code) {
            return self.kept_proposal_notice(id, &names, &what, reading.code, cx);
        }
        self.replace_or_keep_controls(id, conflicting, names, what, reading, reason, cx)
    }

    fn plain_apply_button(
        &self,
        id: SharedString,
        what: String,
        reading: EvidenceReading,
        reason: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = format!("Apply {what}");
        let button = Button::new(id.clone())
            .label(label.clone())
            .accessibility_label(format!("{label} to today's evidence"))
            .small()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.apply_reading(&reading, reason.clone());
                cx.notify();
            }));
        tagged(id.to_string(), button).into_any_element()
    }

    fn kept_proposal_notice(
        &self,
        id: SharedString,
        names: &str,
        what: &str,
        code: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let reconsider_id = format!("{id}-reconsider");
        let reconsider = Button::new(SharedString::from(reconsider_id.clone()))
            .label("Reconsider")
            .accessibility_label(format!("Show the {code} proposal again"))
            .xsmall()
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.reconsider_proposal(code);
                cx.notify();
            }));
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(caption(muted).child(format!("You kept {names}; {what} is not applied.")))
            .child(tagged(reconsider_id, reconsider))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn replace_or_keep_controls(
        &self,
        id: SharedString,
        conflicting: Vec<&'static str>,
        names: String,
        what: String,
        reading: EvidenceReading,
        reason: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let verb = if conflicting.len() == 1 { "is" } else { "are" };
        let label = format!("Replace {names} with {what}");
        let code = reading.code;
        let replace = Button::new(SharedString::from(format!("{id}-replace")))
            .label(label.clone())
            .accessibility_label(format!("{label} in today's evidence"))
            .small()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.replace_with_reading(&conflicting, &reading, reason.clone());
                cx.notify();
            }));
        let keep = Button::new(SharedString::from(format!("{id}-keep")))
            .label(format!("Keep {names}"))
            .accessibility_label(format!("Keep {names} and set the {code} proposal aside"))
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
            .child(caption(muted).child(format!("{names} {verb} applied and cannot be used with {code}.")))
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
