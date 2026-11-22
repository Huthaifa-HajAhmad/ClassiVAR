//! The window frame: the title bar drawn by the app, on the same canvas colour
//! as the page so the window reads as one surface. It carries the logo mark (a
//! small dial in ink, the app's signature picture) and the name. It holds no
//! controls and no case text, so the whole bar stays a place to drag the window.

use gpui_kit::component::TitleBar;

use super::*;

/// The logo mark: a half ring with a needle, drawn in the text colour. It uses
/// no class colour, so it never reads as a class.
fn logo(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let centre = point(bounds.origin.x + px(w / 2.), bounds.origin.y + px(h - 1.));
            let radius = w / 2. - 1.5;
            let at = |r: f32, angle: f32| point(centre.x + px(r * angle.cos()), centre.y - px(r * angle.sin()));
            let mut ring = PathBuilder::stroke(px(2.4));
            ring.move_to(at(radius, std::f32::consts::PI));
            ring.arc_to(point(px(radius), px(radius)), px(0.), false, true, at(radius, 0.));
            if let Ok(path) = ring.build() {
                window.paint_path(path, ink);
            }
            let mut needle = PathBuilder::stroke(px(2.));
            needle.move_to(at(0., 0.));
            needle.line_to(at(radius - 3., 1.0));
            if let Ok(path) = needle.build() {
                window.paint_path(path, ink);
            }
        },
    )
    .w(px(18.))
    .h(px(11.))
}

impl Workbench {
    /// The title bar holds only the mark and the name. Where you are (the case and
    /// the report) is on the rail, so it is not said twice.
    pub(in crate::view) fn frame(&self, cx: &App) -> TitleBar {
        let theme = cx.theme();
        TitleBar::new().bg(theme.background).border_color(theme.background).child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .min_w_0()
                .child(logo(theme.foreground))
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("ClassiVAR")),
        )
    }
}
