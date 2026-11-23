//! A criterion's page, opened with "More": what it means, how strong it can be and
//! what each strength adds, what to check before using it, how it combines with
//! other criteria, the answer given, and the sources. The words are the rule
//! file's own; this page only gives them an order and room to be read.

use super::focus::{Page, criterion_heading, criterion_kicker, page_part, reading, tagged};
use super::point_axis::signed;
use super::*;

impl Workbench {
    pub(super) fn criterion_page(&self, c: &'static Criterion, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let strength = self.current().get(&c.code).map(|choice| choice.strength);
        let back = self.back_button(
            (format!("about-back-{}", c.code), format!("about-back-{}", c.code)),
            format!("Back to the criteria from {}", c.code),
            |this| this.about = None,
            cx,
        );
        let guided = self.editing == Editing::Today && !c.guide.is_empty();
        let body = div()
            .flex()
            .flex_col()
            .gap_6()
            .child(page_part("What it means", cx).child(reading(c.summary.clone())))
            .child(page_part("How strong it can be", cx).child(self.strength_ladder(c, strength, cx)))
            .when(guided && strength.is_none(), |body| {
                body.child(page_part("How the strength is set", cx).child(reading(format!(
                    "{} has no fixed strength here. A short guide asks what kind of variant it is{}, and the answers set the strength. Nothing is applied until you press Apply.",
                    c.code,
                    if c.gene_level.is_empty() { "" } else { ", after a question about the gene" }
                ))))
            })
            .when_some(c.note.clone(), |body, note| body.child(page_part("What to check", cx).child(reading(note))))
            .when_some(discouraged_row_note(c.discouraged, self.editing), |body, note| {
                body.child(reading(note).text_color(muted))
            })
            .children(self.interactions_panel(c, cx).map(|lines| page_part("Used with other criteria", cx).child(lines)))
            .children(self.given_answer(c, cx))
            .child(self.criterion_sources(c, cx));
        let page = Page {
            id: format!("about-page-{}", c.code),
            back,
            kicker: criterion_kicker(c, None),
            title: criterion_heading(c),
            intro: None,
            body,
            footer: Some(self.criterion_page_footer(c, strength, cx)),
        };
        self.focus_page(width, page, cx)
    }

    /// The guide answers recorded for an applied guided criterion.
    fn given_answer(&self, c: &Criterion, cx: &App) -> Option<Div> {
        let choice = self.current().get(&c.code)?;
        let option = c.guide_option(choice.situation.as_deref()?)?;
        let level = choice.gene_level.as_deref().and_then(|id| c.gene_level_option(id));
        Some(
            page_part("Your answer", cx)
                .when_some(level, |part, level| part.child(reading(format!("The gene: {}.", level.short))))
                .child(reading(format!("The variant: {}. {}", option.short, option.situation))),
        )
    }

    /// Every source the criterion's words and strengths come from, once each.
    fn criterion_sources(&self, c: &Criterion, cx: &App) -> Div {
        let muted = cx.theme().muted_foreground;
        let mut ids: Vec<&str> = vec![c.source.as_str()];
        for id in [c.strength_source.as_deref(), c.guide_source.as_deref(), c.gene_level_source.as_deref()]
            .into_iter()
            .flatten()
        {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let citations = ids.into_iter().filter_map(|id| self.rules.source(id)).map(|s| s.citation.clone());
        page_part(if c.guide_source.is_some() || c.strength_source.is_some() { "Sources" } else { "Source" }, cx)
            .children(citations.map(move |text| reading(text).text_color(muted)))
    }

    /// The bar at the foot: apply (or answer the guide), or, once applied, note, change and remove.
    fn criterion_page_footer(&self, c: &'static Criterion, strength: Option<Strength>, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let today = self.editing == Editing::Today;
        let guided = today && !c.guide.is_empty();
        let state = match strength {
            Some(s) => format!("Applied · {} · {} points", s.label(), signed(self.signed_points(c, s))),
            None => "Not applied".to_string(),
        };
        let bar = div().child(caption(muted).flex_1().min_w_0().child(state));
        let Some(_) = strength else {
            let label = if guided {
                "Answer the strength guide".to_string()
            } else {
                let default = if today { c.strength } else { c.original_strength.unwrap_or(c.strength) };
                format!("Apply {} · {} ({} points)", c.code, default.label(), signed(self.signed_points(c, default)))
            };
            let apply = Button::new(SharedString::from(format!("about-apply-{}", c.code)))
                .label(label.clone())
                .accessibility_label(label)
                .primary()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle(c);
                    cx.notify();
                }));
            return bar.child(tagged(format!("about-apply-{}", c.code), apply));
        };
        let remove = Button::new(SharedString::from(format!("about-remove-{}", c.code)))
            .icon(Icon::new(IconName::Delete))
            .label(format!("Remove {}", c.code))
            .accessibility_label(format!("Remove {}", c.code))
            .ghost()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.remove_criterion(&c.code);
                cx.notify();
            }));
        bar.when(today, |bar| bar.child(self.note_button(c, cx)))
            .when(guided, |bar| {
                bar.child(
                    Button::new(SharedString::from(format!("about-change-{}", c.code)))
                        .label("Change the answer")
                        .accessibility_label(format!("Answer the {} strength guide again", c.code))
                        .outline()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.next_strength(c, window, cx);
                            cx.notify();
                        })),
                )
            })
            .child(tagged(format!("about-remove-{}", c.code), remove))
    }

    /// Note what the evidence behind an applied criterion was. The note box opens in
    /// the criterion's row, so the page gives way to the list.
    fn note_button(&self, c: &'static Criterion, cx: &mut Context<Self>) -> Button {
        let has_note = self.today.get(&c.code).is_some_and(|choice| choice.reason.is_some());
        Button::new(SharedString::from(format!("note-{}", c.code)))
            .label(if has_note { "Edit note" } else { "Add a note" })
            .accessibility_label(format!("Note what you looked at for {}", c.code))
            .outline()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.about = None;
                this.category = Some(c.category);
                this.start_note(&c.code, window, cx);
                cx.notify();
            }))
    }
}
