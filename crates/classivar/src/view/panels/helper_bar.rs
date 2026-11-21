//! The "Evidence helpers" bar at the top of the criteria list: one row of small
//! buttons instead of seven tall cards, so the criteria themselves start high on
//! the screen. A helper's panel shows below the bar only while it is open.

use super::*;

/// One helper: how it is named on screen and how it opens, closes and draws.
struct Entry {
    /// Stem of the button ids: `{key}-toggle` in the bar, `{key}-close` in the panel.
    key: &'static str,
    label: &'static str,
    /// Used in the accessible names: "Open the {what}".
    what: &'static str,
    open: bool,
    toggle: fn(&mut Workbench),
    panel: fn(&Workbench, &mut Context<Workbench>) -> AnyElement,
}

impl Entry {
    fn new(
        names: (&'static str, &'static str, &'static str),
        open: bool,
        toggle: fn(&mut Workbench),
        panel: fn(&Workbench, &mut Context<Workbench>) -> AnyElement,
    ) -> Self {
        let (key, label, what) = names;
        Self { key, label, what, open, toggle, panel }
    }
}

impl Workbench {
    pub(super) fn toggle_predictor(&mut self) {
        self.helpers.predictor.open = !self.helpers.predictor.open;
        if !self.helpers.predictor.open {
            self.helpers.predictor.eligible = false;
        }
    }

    /// The helpers that apply to what is on screen. The evidence helpers fill in
    /// today's evidence only; the BA1 list is a dated reference and also shows
    /// for earlier records. It cannot insert evidence.
    fn evidence_entries(&self) -> Vec<Entry> {
        let (rules, helpers) = (self.rules, &self.helpers);
        let today = self.editing == Editing::Today;
        let table = [
            (
                today && !rules.predictors.is_empty(),
                Entry::new(
                    ("predictor", "Predictor score", "predictor score helper"),
                    helpers.predictor.open,
                    Self::toggle_predictor,
                    Self::predictor_helper,
                ),
            ),
            (
                today && rules.functional.is_some(),
                Entry::new(
                    ("functional", "Functional assay", "functional evidence helper"),
                    helpers.functional.open,
                    |this| this.helpers.functional.open = !this.helpers.functional.open,
                    Self::functional_helper,
                ),
            ),
            (
                today && rules.locus_evidence.is_some(),
                Entry::new(
                    ("locus", "Family and phenotype", "co-segregation and phenotype helper"),
                    helpers.locus.open,
                    |this| this.helpers.locus.open = !this.helpers.locus.open,
                    Self::locus_helper,
                ),
            ),
            (
                today && rules.non_segregation.is_some(),
                Entry::new(
                    ("segregation", "Non-segregation", "non-segregation helper"),
                    helpers.segregation.open,
                    |this| this.helpers.segregation.open = !this.helpers.segregation.open,
                    Self::non_segregation_helper,
                ),
            ),
        ];
        table.into_iter().filter_map(|(applies, entry)| applies.then_some(entry)).collect()
    }

    /// The helpers about patients and the dated BA1 list (the list also shows for earlier records).
    fn patient_entries(&self) -> Vec<Entry> {
        let (rules, helpers) = (self.rules, &self.helpers);
        let today = self.editing == Editing::Today;
        let table = [
            (
                today && rules.pm3.is_some(),
                Entry::new(
                    ("pm3", "PM3 patients", "PM3 patients helper"),
                    helpers.pm3.open,
                    |this| this.helpers.pm3.open = !this.helpers.pm3.open,
                    Self::pm3_helper,
                ),
            ),
            (
                !rules.ba1_exceptions.is_empty(),
                Entry::new(
                    ("ba1-reference", "BA1 exceptions", "BA1 exceptions reference"),
                    helpers.ba1_open,
                    |this| this.helpers.ba1_open = !this.helpers.ba1_open,
                    Self::ba1_reference,
                ),
            ),
            (
                today && rules.de_novo.is_some(),
                Entry::new(
                    ("de-novo", "De novo patients", "de novo helper"),
                    helpers.de_novo.open,
                    |this| this.helpers.de_novo.open = !this.helpers.de_novo.open,
                    Self::de_novo_helper,
                ),
            ),
        ];
        table.into_iter().filter_map(|(applies, entry)| applies.then_some(entry)).collect()
    }

    fn helper_entries(&self) -> Vec<Entry> {
        let mut entries = self.evidence_entries();
        entries.extend(self.patient_entries());
        entries
    }

    /// The bar, then the panel of each open helper. Empty when no helper applies.
    pub(super) fn helper_section(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let entries = self.helper_entries();
        if entries.is_empty() {
            return Vec::new();
        }
        let toggle = self.helpers_toggle(entries.len(), cx);
        if !self.show_helpers {
            return vec![div().child(toggle).into_any_element()];
        }
        let muted = cx.theme().muted_foreground;
        let bar = div()
            .flex()
            .flex_col()
            .gap_2()
            .px_2()
            .child(div().child(toggle))
            .child(
                caption(muted)
                    .child("They read published tables and suggest a strength. You still choose what to apply."),
            )
            .child(self.helper_buttons(&entries, cx))
            .into_any_element();
        let mut out = vec![bar];
        out.extend(entries.iter().filter(|entry| entry.open).map(|entry| (entry.panel)(self, cx)));
        out
    }

    /// The button that opens and closes the bar of helpers; `count` is how many apply.
    fn helpers_toggle(&self, count: usize, cx: &mut Context<Self>) -> Button {
        let label = if self.show_helpers { "Hide evidence helpers" } else { "Evidence helpers" };
        let count = format!("{} {}", count, if self.show_helpers { "▴" } else { "▾" });
        Button::new("evidence-tools")
            .small()
            .ghost()
            .w_full()
            .rounded(px(crate::look::CARD_RADIUS))
            .bg(cx.theme().muted)
            .accessibility_label(label)
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(label))
                    .child(crate::look::figure(11.).text_color(cx.theme().muted_foreground).child(count)),
            )
            .toggled(self.show_helpers)
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_helpers = !this.show_helpers;
                cx.notify();
            }))
    }

    /// One toggle button per helper.
    fn helper_buttons(&self, entries: &[Entry], cx: &mut Context<Self>) -> Div {
        let mut buttons = div().flex().flex_wrap().gap_2();
        for entry in entries {
            let (toggle, open) = (entry.toggle, entry.open);
            buttons = buttons.child(
                Button::new(SharedString::from(format!("{}-toggle", entry.key)))
                    .label(entry.label)
                    .accessibility_label(format!("Open the {}", entry.what))
                    .small()
                    .outline()
                    .rounded(px(999.))
                    .selected(open)
                    .toggled(open)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        toggle(this);
                        cx.notify();
                    })),
            );
        }
        buttons
    }
}
