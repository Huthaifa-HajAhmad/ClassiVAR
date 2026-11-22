//! The rail at the left of a wide window: the case, and its reports over time.
//!
//! Each earlier report is a stop on a vertical timeline, oldest first, with the
//! class the lab stated, its total under the loaded rules and a small scale with
//! a tick where that total sits. Today's evidence is the last stop. Clicking a
//! stop opens that evidence in the editor and on the picture. A report whose
//! class the engine withholds gets no tick, so the rail never places a class
//! the engine does not give.

use classivar_core::Counting;

use super::point_axis::{fraction, points_text};
use super::*;
use crate::look::{MONO, eyebrow, lift};
use crate::view::describe::Scale;

/// The rail's width, and the window width below which it folds away (the record chips then show in the case card).
/// The narrowest the rail gets; it grows in a wide window (`panels::rail_width`).
pub(super) const RAIL_WIDTH: f32 = 236.;
pub(super) const RAIL_FROM: f32 = 1180.;

/// What one stop says: the class in words and the total, if the engine gives a class.
struct Stop {
    title: String,
    class: String,
    /// The class this report gave (a lab's stated class, or today's), marked on its scale.
    marked: Option<Class>,
    total: Option<i32>,
}

impl Workbench {
    pub(super) fn rail(&self, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let title = match variant {
            Some(name) => name.to_string(),
            None => "New assessment".to_string(),
        };
        let reported = self.mode == Mode::ReportedVariant;
        div()
            .id("rail")
            .w(px(self.stage.rail_width.get()))
            .flex_none()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_5()
            .py_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(eyebrow("Case", cx))
                            .child(self.clear_case_button(cx)),
                    )
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(title))
                    .when(!self.case_gene.is_empty(), |d| d.child(figure_line(self.case_gene.clone(), muted))),
            )
            .child(eyebrow(if reported { "Reports over time" } else { "Your evidence" }, cx))
            .child(self.timeline_stops(cx))
            .when(!reported, |d| {
                d.child(caption(muted).child(
                    "Choose Reported variant at the top to add what earlier labs reported and compare it with today.",
                ))
            })
            .into_any_element()
    }

    fn timeline_stops(&self, cx: &mut Context<Self>) -> Div {
        let border = cx.theme().border;
        let scale = scale_for(self.rules);
        let mut order: Vec<usize> = (0..self.records.len()).collect();
        order.sort_by_key(|&i| self.records[i].year);
        let mut list = div()
            .relative()
            .flex()
            .flex_col()
            .gap_1()
            .pl(px(18.))
            .child(div().absolute().left(px(5.)).top(px(20.)).bottom(px(40.)).w(px(1.)).bg(border));
        if self.mode == Mode::ReportedVariant {
            for i in order {
                let stop = self.record_stop(i);
                let chosen = self.editing == Editing::Record(i);
                let id = SharedString::from(format!("record-{i}"));
                let button =
                    self.stop(id, &stop, chosen, &scale, cx).on_click(cx.listener(move |this, _, window, cx| {
                        this.show_record(i, window, cx);
                        cx.notify();
                    }));
                list = list.child(with_dot(button, chosen, cx).debug_selector(move || format!("rail-record-{i}")));
            }
        }
        let today = self.today_stop();
        let chosen = self.editing == Editing::Today;
        let button =
            self.stop("record-today".into(), &today, chosen, &scale, cx).on_click(cx.listener(|this, _, _, cx| {
                this.inspected = None;
                this.preview = false;
                this.editing = Editing::Today;
                cx.notify();
            }));
        list = list.child(with_dot(button, chosen, cx).debug_selector(|| "rail-record-today".into()));
        if self.mode == Mode::ReportedVariant {
            list = list.child(
                div()
                    .pt_2()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(
                        Button::new("rail-record-add")
                            .icon(Icon::new(IconName::Plus))
                            .label("Add record")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_record(window, cx);
                                cx.notify();
                            })),
                    )
                    .children(self.clinvar_import_row(true, cx)),
            );
        }
        list
    }

    fn record_stop(&self, i: usize) -> Stop {
        let record = &self.records[i];
        let scored = self.rules.assess_as(&to_applied(&record.evidence), Counting::AsRecorded).ok();
        Stop {
            title: format!("{} · {}", record.label, record.year),
            class: record.class.map_or("No class entered", |class| class.short()).to_string(),
            marked: record.class,
            total: scored.filter(|s| s.class().is_some()).map(|s| s.total_points),
        }
    }

    fn today_stop(&self) -> Stop {
        let scored = self.rules.assess(&to_applied(&self.today)).ok();
        let class = scored.as_ref().and_then(Assessment::class);
        Stop {
            title: "Today (you)".to_string(),
            class: class.map_or("No class yet", |class| class.short()).to_string(),
            marked: class,
            total: class.and(scored.map(|s| s.total_points)),
        }
    }

    /// One stop: a dot on the line, the name, the class and total, and the small scale.
    fn stop(&self, id: SharedString, stop: &Stop, chosen: bool, scale: &Scale, cx: &App) -> Button {
        let theme = cx.theme();
        let (ink, muted, paper) = (theme.foreground, theme.muted_foreground, theme.popover);
        let points = stop.total.map(|total| format!("{} pts", points_text(total))).unwrap_or_default();
        Button::new(id)
            .ghost()
            .w_full()
            .h_auto()
            .rounded(px(10.))
            .accessibility_label(format!("{}: {}", stop.title, stop.class))
            .selected(chosen)
            .toggled(chosen)
            .when(chosen, |b| b.bg(paper).shadow(lift(theme.is_dark())))
            .child(
                div()
                    .relative()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .py_1()
                    .child(stop_title(&stop.title, ink))
                    .child(match stop.marked {
                        Some(class) => class_letter(class, scale, ink),
                        None => div().text_xs().text_color(muted).child(stop.class.clone()),
                    })
                    .child(mini_scale(stop, scale, theme.is_dark(), ink))
                    .when(stop.total.is_some(), |d| {
                        d.child(
                            div()
                                .w_full()
                                .flex()
                                .justify_end()
                                .text_xs()
                                .text_color(muted)
                                .font_family(MONO)
                                .child(points),
                        )
                    }),
            )
    }
}

/// A stop with its dot on the timeline line.
fn with_dot(button: Button, chosen: bool, cx: &App) -> Div {
    let theme = cx.theme();
    let edge = if chosen { theme.foreground } else { theme.muted_foreground };
    let dot = div()
        .absolute()
        .left(px(-18.))
        .top(px(14.))
        .size(px(11.))
        .rounded_full()
        .border_2()
        .border_color(edge)
        .bg(if chosen { theme.foreground } else { theme.background });
    div().relative().child(dot).child(button)
}

fn figure_line(text: String, muted: Hsla) -> Div {
    div().font_family(MONO).text_sm().text_color(muted).child(text)
}

/// How strongly a class band is drawn on a report's scale: the report's own class
/// in full colour, the others faint, so reports with different classes look different.
fn zone_strength(zone: Class, marked: Option<Class>) -> f32 {
    if marked == Some(zone) { 1.0 } else { 0.28 }
}

/// The lab and year. A long lab name wraps instead of being cut mid-word.
fn stop_title(title: &str, ink: Hsla) -> Div {
    div()
        .w_full()
        .whitespace_normal()
        .text_left()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink)
        .child(title.to_string())
}

/// The report's class letter, set over its own band of the scale just below.
fn class_letter(class: Class, scale: &Scale, ink: Hsla) -> Div {
    let row = div().relative().w_full().h(px(14.));
    let Some((_, from, to)) = scale.zones.iter().find(|(zone, _, _)| *zone == class) else { return row };
    let (left, right) = (fraction(*from, scale), fraction(*to, scale));
    row.child(
        div()
            .absolute()
            .left(relative(left))
            .w(relative(right - left))
            .flex()
            .justify_center()
            .font_family(MONO)
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .text_color(ink)
            .child(class.short()),
    )
}

/// The class bands as one thin bar, with a tick at the report's total when there is one.
fn mini_scale(stop: &Stop, scale: &Scale, dark: bool, ink: Hsla) -> Div {
    let mut bar = div().relative().w_full().h(px(5.)).mt_1().rounded_full();
    for (zone, from, to) in &scale.zones {
        let (left, right) = (fraction(*from, scale), fraction(*to, scale));
        bar = bar.child(
            div()
                .absolute()
                .top_0()
                .h_full()
                .left(relative(left))
                .w(relative(right - left))
                .bg(class_fill(*zone, dark).opacity(zone_strength(*zone, stop.marked))),
        );
    }
    bar.when_some(stop.total, |bar, total| {
        bar.child(
            div()
                .absolute()
                .top(px(-3.))
                .left(relative(fraction(total, scale)))
                .ml(px(-1.))
                .w(px(2.))
                .h(px(11.))
                .rounded_full()
                .bg(ink),
        )
    })
}

#[cfg(test)]
mod tests {
    use classivar_core::Class;

    use super::zone_strength;

    #[test]
    fn each_report_marks_its_own_class_on_its_scale() {
        assert_eq!(zone_strength(Class::Pathogenic, Some(Class::Pathogenic)), 1.0);
        assert!(zone_strength(Class::LikelyPathogenic, Some(Class::Pathogenic)) < 0.5, "other classes are faint");
        assert_ne!(
            zone_strength(Class::Pathogenic, Some(Class::Pathogenic)),
            zone_strength(Class::Pathogenic, Some(Class::LikelyPathogenic)),
            "a P report and an LP report do not draw the same scale"
        );
        assert!(zone_strength(Class::Vus, None) < 0.5, "no class: nothing is marked");
    }
}
