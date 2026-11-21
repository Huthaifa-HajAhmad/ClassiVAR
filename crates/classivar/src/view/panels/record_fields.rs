//! The record's name and year fields: the lab name with its dropdowns, and the year menu.

use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

use super::*;

impl Workbench {
    /// The record's name box, with lab names in a floating list while typing and a menu of every name.
    pub(super) fn lab_box(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let typed = self.boxes.record_name.read(cx).value();
        let in_case: Vec<&str> = self
            .records
            .iter()
            .enumerate()
            .filter(|(j, r)| *j != i && r.label != UNNAMED_RECORD && !r.label.starts_with("Record "))
            .map(|(_, r)| r.label.as_str())
            .collect();
        let typing = crate::labs::suggestions_for(&typed, &in_case, 8);
        let typing = if typed.trim().is_empty() || typed.starts_with("Record ") { Vec::new() } else { typing };
        let groups = vec![
            ("In this case", in_case.iter().map(|name| name.to_string()).collect()),
            ("Often in ClinVar", crate::labs::suggestions_for("", &[], usize::MAX)),
        ];
        let field = Input::new(&self.boxes.record_name).aria_label("Lab or source name");
        div()
            .flex_1()
            .flex()
            .items_center()
            .gap_1()
            .child(super::suggest::with_suggestions(
                "lab-suggestion",
                field,
                typing,
                "For typing only: any name can be used.",
                &self.boxes.record_name,
                cx,
            ))
            .child(super::suggest::choice_menu("lab-menu", "Choose a lab name", groups, &self.boxes.record_name, cx))
    }

    /// The record's year as a menu of years, newest first. The − and + beside it reach the rest of the range.
    pub(super) fn year_menu(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let year = self.records[i].year;
        let this = cx.entity().downgrade();
        let newest = classivar_sources::current_year().clamp(classivar_core::MIN_YEAR, classivar_core::MAX_YEAR);
        Button::new(SharedString::from(format!("year-{i}")))
            .child(crate::look::figure(12.).child(format!("{year} ▾")))
            .accessibility_label(format!("Year: {year}. Choose a year"))
            .small()
            .ghost()
            .dropdown_menu(move |mut menu, _, _| {
                for option in (classivar_core::MIN_YEAR..=newest).rev() {
                    let this = this.clone();
                    menu = menu.item(PopupMenuItem::new(option.to_string()).checked(option == year).on_click(
                        move |_, _, cx| {
                            let _ = this.update(cx, |this, cx| {
                                if let Some(record) = this.records.get_mut(i) {
                                    record.year = option;
                                }
                                cx.notify();
                            });
                        },
                    ));
                }
                menu.max_h(px(320.)).scrollable(true)
            })
            .into_any_element()
    }

    /// The VUS sub-tier the record named, as a menu. "None stated" is a choice of its own.
    pub(super) fn tier_menu(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let current = self.records[i].tier.clone();
        let tiers: Vec<String> = self.rules.vus_tiers.iter().map(|tier| tier.name.clone()).collect();
        let this = cx.entity().downgrade();
        Button::new(SharedString::from(format!("tier-{i}")))
            .label(format!("{} ▾", current.as_deref().unwrap_or("None stated")))
            .accessibility_label(format!(
                "Tier it named: {}. Choose a tier",
                current.as_deref().unwrap_or("none stated")
            ))
            .small()
            .outline()
            .rounded(px(999.))
            .dropdown_menu(move |mut menu, _, _| {
                let choices = std::iter::once(None).chain(tiers.iter().cloned().map(Some));
                for choice in choices {
                    let this = this.clone();
                    let label = choice.clone().unwrap_or_else(|| "None stated".to_string());
                    let checked = choice == current;
                    menu = menu.item(PopupMenuItem::new(label).checked(checked).on_click(move |_, _, cx| {
                        let choice = choice.clone();
                        let _ = this.update(cx, |this, cx| {
                            if let Some(record) = this.records.get_mut(i) {
                                record.tier = choice;
                            }
                            cx.notify();
                        });
                    }));
                }
                menu
            })
            .into_any_element()
    }
}
