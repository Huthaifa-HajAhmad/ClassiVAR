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

impl Workbench {
    pub(super) fn toggle_predictor(&mut self) {
        self.predictor_open = !self.predictor_open;
        if !self.predictor_open {
            self.predictor_eligible = false;
        }
    }

    /// The helpers that apply to what is on screen. The evidence helpers fill in
    /// today's evidence only; the BA1 list is a dated reference and also shows
    /// for earlier records. It cannot insert evidence.
    fn helper_entries(&self) -> Vec<Entry> {
        let rules = self.rules;
        let today = self.editing == Editing::Today;
        let mut entries = Vec::new();
        if today && !rules.predictors.is_empty() {
            entries.push(Entry {
                key: "predictor",
                label: "Predictor score",
                what: "predictor score helper",
                open: self.predictor_open,
                toggle: Self::toggle_predictor,
                panel: Self::predictor_helper,
            });
        }
        if today && rules.functional.is_some() {
            entries.push(Entry {
                key: "functional",
                label: "Functional assay",
                what: "functional evidence helper",
                open: self.functional_open,
                toggle: |this| this.functional_open = !this.functional_open,
                panel: Self::functional_helper,
            });
        }
        if today && rules.locus_evidence.is_some() {
            entries.push(Entry {
                key: "locus",
                label: "Family and phenotype",
                what: "co-segregation and phenotype helper",
                open: self.locus_open,
                toggle: |this| this.locus_open = !this.locus_open,
                panel: Self::locus_helper,
            });
        }
        if today && rules.non_segregation.is_some() {
            entries.push(Entry {
                key: "segregation",
                label: "Non-segregation",
                what: "non-segregation helper",
                open: self.segregation_open,
                toggle: |this| this.segregation_open = !this.segregation_open,
                panel: Self::non_segregation_helper,
            });
        }
        if today && rules.pm3.is_some() {
            entries.push(Entry {
                key: "pm3",
                label: "PM3 patients",
                what: "PM3 patients helper",
                open: self.pm3_open,
                toggle: |this| this.pm3_open = !this.pm3_open,
                panel: Self::pm3_helper,
            });
        }
        if !rules.ba1_exceptions.is_empty() {
            entries.push(Entry {
                key: "ba1-reference",
                label: "BA1 exceptions",
                what: "BA1 exceptions reference",
                open: self.ba1_open,
                toggle: |this| this.ba1_open = !this.ba1_open,
                panel: Self::ba1_reference,
            });
        }
        if today && rules.de_novo.is_some() {
            entries.push(Entry {
                key: "de-novo",
                label: "De novo patients",
                what: "de novo helper",
                open: self.de_novo_open,
                toggle: |this| this.de_novo_open = !this.de_novo_open,
                panel: Self::de_novo_helper,
            });
        }
        entries
    }

    /// The bar, then the panel of each open helper. Empty when no helper applies.
    pub(super) fn helper_section(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let entries = self.helper_entries();
        if entries.is_empty() {
            return Vec::new();
        }
        let muted = cx.theme().muted_foreground;
        let mut buttons = div().flex().flex_wrap().gap_2();
        for entry in &entries {
            let (toggle, open) = (entry.toggle, entry.open);
            buttons = buttons.child(
                Button::new(SharedString::from(format!("{}-toggle", entry.key)))
                    .label(entry.label)
                    .accessibility_label(format!("Open the {}", entry.what))
                    .small()
                    .outline()
                    .selected(open)
                    .toggled(open)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        toggle(this);
                        cx.notify();
                    })),
            );
        }
        let bar = div()
            .flex()
            .flex_col()
            .gap_2()
            .px_2()
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("Evidence helpers"))
            .child(
                caption(muted)
                    .child("They read published tables and suggest a strength. You still choose what to apply."),
            )
            .child(buttons)
            .into_any_element();
        let mut out = vec![bar];
        out.extend(entries.iter().filter(|entry| entry.open).map(|entry| (entry.panel)(self, cx)));
        out
    }
}
