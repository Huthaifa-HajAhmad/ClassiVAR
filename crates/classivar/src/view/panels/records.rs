//! The bar of earlier records and the detail of the open record.

use super::*;

impl Workbench {
    pub(super) fn records_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let detail = match self.editing {
            Editing::Record(i) => self.record_detail(i, cx),
            Editing::Today => self.today_detail(cx),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_2()
            .border_t_1()
            .border_color(border)
            .child(self.record_chips(cx))
            .child(detail)
            .into_any_element()
    }

    /// One chip for each earlier record, the Add and Today buttons, and the example while there are no records.
    fn record_chips(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let mut chips = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("Earlier records"));
        for (i, r) in self.records.iter().enumerate() {
            let chosen = self.editing == Editing::Record(i);
            chips = chips.child(
                Button::new(SharedString::from(format!("record-{i}")))
                    .label(format!("{} · {}", r.label, r.year))
                    .small()
                    .selected(chosen)
                    .toggled(chosen)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.show_record(i, window, cx);
                        cx.notify();
                    })),
            );
        }
        let today_chosen = self.editing == Editing::Today;
        chips
            .child(Button::new("record-add").label("Add record").small().outline().on_click(cx.listener(
                |this, _, window, cx| {
                    this.add_record(window, cx);
                    cx.notify();
                },
            )))
            .child(
                Button::new("record-today")
                    .label("Today (you)")
                    .small()
                    .selected(today_chosen)
                    .toggled(today_chosen)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.inspected = None;
                        this.preview = false;
                        this.editing = Editing::Today;
                        cx.notify();
                    })),
            )
            .when(self.records.is_empty(), |d| d.child(example_button(cx)))
    }
    /// Under the chips while today's own evidence is edited: copy the latest record, and a hint.
    fn today_detail(&self, cx: &mut Context<Self>) -> AnyElement {
        let hint = if self.records.is_empty() {
            "Add what earlier labs reported and used, then apply your own evidence to compare."
        } else {
            "Editing your own evidence. The verdict compares it with the latest record."
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .when(!self.records.is_empty(), |d| {
                d.child(
                    Button::new("copy-latest")
                        .label("Start from the latest record's evidence")
                        .small()
                        .outline()
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.today.is_empty() {
                                this.start_from_latest();
                            } else {
                                this.ask(Confirm::StartFromLatest);
                            }
                            cx.notify();
                        })),
                )
            })
            .child(caption(cx.theme().muted_foreground).child(hint))
            .into_any_element()
    }

    /// The name, class, year and VUS tier one earlier record reported.
    pub(super) fn record_detail(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let name_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(caption(muted).child("Lab or source"))
            .child(div().w(px(260.)).child(Input::new(&self.record_name_input).aria_label("Lab or source name")));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(name_row)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(self.record_class_buttons(i, cx))
                    .when(self.records[i].class == Some(Class::Vus), |d| d.child(self.record_tier_buttons(i, cx)))
                    .child(self.record_year_buttons(i, cx)),
            )
            .into_any_element()
    }

    /// One button for each class the record may have reported.
    fn record_class_buttons(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let record = &self.records[i];
        let mut row =
            div().flex().flex_wrap().items_center().gap_2().child(caption(muted).child("This record reported"));
        for class in CLASSES {
            let chosen = record.class == Some(class);
            row = row.child(
                Button::new(SharedString::from(format!("class-{i}-{}", class.short())))
                    .label(class.short())
                    .accessibility_label(format!("This record reported {}", class.label()))
                    .small()
                    .selected(chosen)
                    .toggled(chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.records[i].class.is_none() {
                            this.baseline = None;
                        }
                        this.records[i].class = Some(class);
                        if class != Class::Vus {
                            // A sub-tier only makes sense for a VUS.
                            this.records[i].tier = None;
                        }
                        cx.notify();
                    })),
            );
        }
        row.when(record.class.is_none(), |d| d.child(caption(muted).child("(choose one to get a verdict)")))
    }

    /// The tier a VUS record named, if any. "None stated" is a choice of its own.
    fn record_tier_buttons(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let record = &self.records[i];
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().pl_2().child(caption(cx.theme().muted_foreground).child("Tier it named")))
            .child(
                Button::new(SharedString::from(format!("tier-{i}-none")))
                    .label("none stated")
                    .small()
                    .selected(record.tier.is_none())
                    .toggled(record.tier.is_none())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.records[i].tier = None;
                        cx.notify();
                    })),
            );
        for tier in &self.rules.vus_tiers {
            let name = tier.name.clone();
            let chosen = record.tier.as_deref() == Some(tier.name.as_str());
            row = row.child(
                Button::new(SharedString::from(format!("tier-{i}-{}", tier.name)))
                    .label(tier.name.clone())
                    .small()
                    .selected(chosen)
                    .toggled(chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.records[i].tier = Some(name.clone());
                        cx.notify();
                    })),
            );
        }
        row
    }

    /// The year, one step at a time within the range the case file accepts, and the remove button.
    fn record_year_buttons(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let remove_label = self.records[i].label.clone();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().pl_2().child(caption(cx.theme().muted_foreground).child("Year")))
            .child(
                Button::new(SharedString::from(format!("year-down-{i}")))
                    .label("−")
                    .accessibility_label("One year earlier")
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let year = &mut this.records[i].year;
                        *year = year.saturating_sub(1).max(classivar_core::MIN_YEAR);
                        cx.notify();
                    })),
            )
            .child(div().text_sm().child(self.records[i].year.to_string()))
            .child(
                Button::new(SharedString::from(format!("year-up-{i}")))
                    .label("+")
                    .accessibility_label("One year later")
                    .small()
                    .outline()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let year = &mut this.records[i].year;
                        *year = year.saturating_add(1).min(classivar_core::MAX_YEAR);
                        cx.notify();
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("record-remove-{i}")))
                    .label("Remove record")
                    .small()
                    .ghost()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ask(Confirm::RemoveRecord(i, remove_label.clone()));
                        cx.notify();
                    })),
            )
    }
}

/// Loads the fictional example. It replaces the whole case, including a typed variant
/// or gene, so it asks first whenever anything can be lost.
fn example_button(cx: &mut Context<Workbench>) -> Button {
    Button::new("record-example").label("Load example").small().ghost().on_click(cx.listener(|this, _, window, cx| {
        if this.has_case_data(cx) {
            this.ask(Confirm::LoadExample);
        } else {
            this.load_example(window, cx);
        }
        cx.notify();
    }))
}
