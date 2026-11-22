//! The bar of earlier records and the detail of the open record.

use super::*;

impl Workbench {
    pub(super) fn records_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let detail = match self.editing {
            Editing::Record(i) => self.record_detail(i, cx),
            Editing::Today => self.today_detail(cx),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .when(!self.stage.rail_shown.get(), |d| d.child(self.record_chips(cx)))
            .child(detail)
            .into_any_element()
    }

    /// One chip for each earlier record, and the Add and Today buttons.
    fn record_chips(&self, cx: &mut Context<Self>) -> Div {
        let mut chips = Vec::new();
        for (i, r) in self.records.iter().enumerate() {
            let chosen = self.editing == Editing::Record(i);
            chips.push(
                crate::look::segment(format!("record-{i}"), format!("{} · {}", r.label, r.year), chosen, cx).on_click(
                    cx.listener(move |this, _, window, cx| {
                        this.show_record(i, window, cx);
                        cx.notify();
                    }),
                ),
            );
        }
        let today_chosen = self.editing == Editing::Today;
        chips.push(crate::look::segment("record-today", "Today (you)", today_chosen, cx).on_click(cx.listener(
            |this, _, _, cx| {
                this.inspected = None;
                this.preview = false;
                this.editing = Editing::Today;
                cx.notify();
            },
        )));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(crate::look::eyebrow("Reports", cx))
            .child(crate::look::segmented(chips, cx).flex_wrap())
            .child(div().flex().gap_2().child(
                Button::new("record-add").label("+ Add record").small().ghost().on_click(cx.listener(
                    |this, _, window, cx| {
                        this.add_record(window, cx);
                        cx.notify();
                    },
                )),
            ))
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
            .flex_wrap()
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
            .child(caption(cx.theme().muted_foreground).w_full().child(hint))
            .into_any_element()
    }

    /// The name, class, year and VUS tier one earlier record reported.
    pub(super) fn record_detail(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let record = &self.records[i];
        let remove_label = record.label.clone();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(form_row("Lab or source", self.lab_box(i, cx), cx))
            .child(form_row("It reported", self.record_class_buttons(i, cx), cx))
            .when(record.class.is_none(), |d| {
                d.child(form_row("", caption(muted).child("Choose the class it reported to get a verdict."), cx))
            })
            .when(record.class == Some(Class::Vus), |d| d.child(form_row("Tier it named", self.tier_menu(i, cx), cx)))
            .child(self.record_condition_rows(i, cx))
            .child(form_row(
                "Year",
                div().flex().items_center().justify_between().w_full().child(self.record_year_buttons(i, cx)).child(
                    Button::new(SharedString::from(format!("record-remove-{i}")))
                        .label("Remove record")
                        .small()
                        .ghost()
                        .text_color(muted)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.ask(Confirm::RemoveRecord(i, remove_label.clone()));
                            cx.notify();
                        })),
                ),
                cx,
            ))
            .into_any_element()
    }

    /// The class the record reported, as one segmented control.
    fn record_class_buttons(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let record = &self.records[i];
        let mut segments = Vec::new();
        for class in CLASSES {
            segments.push(
                crate::look::segment(
                    format!("class-{i}-{}", class.short()),
                    class.short(),
                    record.class == Some(class),
                    cx,
                )
                .accessibility_label(format!("This record reported {}", class.label()))
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
        crate::look::segmented(segments, cx)
    }

    /// The year: one step at a time on either side of the year menu, within the range the case file accepts.
    fn record_year_buttons(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let step = |id: String, glyph: &'static str, label: &'static str, change: i32, cx: &mut Context<Self>| {
            Button::new(SharedString::from(id)).label(glyph).accessibility_label(label).small().ghost().on_click(
                cx.listener(move |this, _, _, cx| {
                    let year = &mut this.records[i].year;
                    let next = (i32::from(*year) + change)
                        .clamp(i32::from(classivar_core::MIN_YEAR), i32::from(classivar_core::MAX_YEAR));
                    *year = next as u16;
                    cx.notify();
                }),
            )
        };
        let earlier = step(format!("year-down-{i}"), "−", "One year earlier", -1, cx);
        let later = step(format!("year-up-{i}"), "+", "One year later", 1, cx);
        crate::look::segmented([earlier.into_any_element(), self.year_menu(i, cx), later.into_any_element()], cx)
    }
}

/// One row of a small form: a label in a fixed column, and the control.
pub(super) fn form_row(label: &'static str, control: impl IntoElement, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(div().w(px(92.)).flex_none().child(caption(cx.theme().muted_foreground).child(label)))
        .child(div().flex_1().min_w_0().flex().child(control))
}
