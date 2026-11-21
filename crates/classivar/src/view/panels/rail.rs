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
pub(super) const RAIL_WIDTH: f32 = 236.;
pub(super) const RAIL_FROM: f32 = 1180.;

/// What one stop says: the class in words and the total, if the engine gives a class.
struct Stop {
    title: String,
    class: String,
    total: Option<i32>,
}

impl Workbench {
    pub(super) fn rail(&self, variant: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let title = match variant {
            Some(name) => name.to_string(),
            None if self.example => "Fictional case".to_string(),
            None => "New assessment".to_string(),
        };
        let reported = self.mode == Mode::ReportedVariant;
        div()
            .id("rail")
            .w(px(RAIL_WIDTH))
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
                    .child(eyebrow("Case", cx))
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(title))
                    .when(!self.case_gene.is_empty(), |d| d.child(figure_line(self.case_gene.clone(), muted)))
                    .when(self.example, |d| {
                        d.child(caption(muted).child("No variant or gene is named: this case is made up."))
                    }),
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
        if self.mode == Mode::ReportedVariant && self.records.is_empty() {
            list = list.child(super::records::example_button(cx));
        }
        if self.mode == Mode::ReportedVariant {
            list = list.child(Button::new("record-add").label("+ Add record").small().ghost().on_click(cx.listener(
                |this, _, window, cx| {
                    this.add_record(window, cx);
                    cx.notify();
                },
            )));
        }
        list
    }

    fn record_stop(&self, i: usize) -> Stop {
        let record = &self.records[i];
        let scored = self.rules.assess_as(&to_applied(&record.evidence), Counting::AsRecorded).ok();
        Stop {
            title: format!("{} · {}", record.label, record.year),
            class: record.class.map_or("No class entered", |class| class.short()).to_string(),
            total: scored.filter(|s| s.class().is_some()).map(|s| s.total_points),
        }
    }

    fn today_stop(&self) -> Stop {
        let scored = self.rules.assess(&to_applied(&self.today)).ok();
        let class = scored.as_ref().and_then(Assessment::class);
        Stop {
            title: "Today (you)".to_string(),
            class: class.map_or("No class yet", |class| class.short()).to_string(),
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
            .accessibility_label(stop.title.clone())
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
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(ink).child(stop.title.clone()))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_between()
                            .text_xs()
                            .text_color(muted)
                            .child(stop.class.clone())
                            .child(div().font_family(MONO).child(points)),
                    )
                    .child(mini_scale(stop.total, scale, theme.is_dark(), ink)),
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

/// The class bands as one thin bar, with a tick at `total` when there is one.
fn mini_scale(total: Option<i32>, scale: &Scale, dark: bool, ink: Hsla) -> Div {
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
                .bg(class_fill(*zone, dark)),
        );
    }
    bar.when_some(total, |bar, total| {
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
