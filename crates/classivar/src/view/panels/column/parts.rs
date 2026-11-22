//! The pieces of the column picture that need no state: the class bands, the
//! earlier reports, the names under the columns, the total marker and the trail.

use super::*;

/// The class bands as stacked fills, the class shown (if any) in full colour.
/// The end bands carry the rounded corners, since a clip does not round its children.
pub(super) fn bands(scale: &Scale, class: Option<Class>, dark: bool, corner: f32) -> Vec<Div> {
    let last = scale.zones.len().saturating_sub(1);
    scale
        .zones
        .iter()
        .enumerate()
        .map(|(i, (zone, from, to))| {
            let (top, height) = span(*from, *to, scale);
            let alpha = if class == Some(*zone) {
                1.0
            } else if class.is_some() {
                0.6
            } else {
                0.4
            };
            div()
                .absolute()
                .left_0()
                .w_full()
                .top(relative(top))
                .h(relative(height))
                .bg(Rgba { a: alpha, ..class_fill(*zone, dark) })
                .when(i == last, |band| band.rounded_t(px(corner)))
                .when(i == 0, |band| band.rounded_b(px(corner)))
        })
        .collect()
}

pub(super) fn earlier_column(
    report: &Earlier,
    left: f32,
    scale: &Scale,
    dark: bool,
    (ink, muted): (Hsla, Hsla),
) -> Div {
    let at = level(report.total, scale) * HEIGHT;
    div()
        .child(
            div()
                .absolute()
                .left(px(left))
                .top_0()
                .w(px(EARLIER_WIDTH))
                .h(px(HEIGHT))
                .rounded(px(15.))
                .overflow_hidden()
                .children(bands(scale, None, dark, 15.)),
        )
        .child(
            div()
                .absolute()
                .left(px(left + EARLIER_WIDTH / 2. - 6.))
                .top(px(at - 6.))
                .size(px(12.))
                .rounded_full()
                .bg(ink),
        )
        .child(
            figure(11.)
                .absolute()
                .left(px(left - 10.))
                .w(px(EARLIER_WIDTH + 20.))
                .top(px(at - 24.))
                .flex()
                .justify_center()
                .text_color(ink)
                .child(points_text(report.total)),
        )
        .child(foot(
            &report.name,
            &report.year.to_string(),
            left - CAPTION_OVERHANG,
            EARLIER_WIDTH + 2. * CAPTION_OVERHANG,
            ink,
            muted,
        ))
}

/// The name under a column, cut short rather than wrapped.
pub(super) fn foot(name: &str, note: &str, left: f32, width: f32, ink: Hsla, muted: Hsla) -> Div {
    div()
        .absolute()
        .left(px(left))
        .w(px(width))
        .top(px(HEIGHT + 8.))
        .flex()
        .flex_col()
        .items_center()
        .text_xs()
        .child(
            div()
                .w_full()
                .truncate()
                .text_center()
                .text_color(ink)
                .font_weight(FontWeight::SEMIBOLD)
                .child(name.to_string()),
        )
        .child(div().w_full().truncate().text_center().text_color(muted).child(note.to_string()))
}

/// The total as a line across the capsule with its value at the right; outlined when `paper` is `None` (the preview).
pub(super) fn marker(points: i32, scale: &Scale, left: f32, ink: Hsla, paper: Option<Hsla>) -> Div {
    let at = level(points, scale) * HEIGHT;
    let sign = if points > 0 { "+" } else { "" };
    let tag = figure(13.)
        .absolute()
        .left(px(left + TODAY_WIDTH + 8.))
        .top(px(at - 11.))
        .px_2()
        .h(px(22.))
        .flex()
        .items_center()
        .rounded_full()
        .child(format!("{sign}{}", points_text(points)));
    let tag = match paper {
        Some(paper) => tag.bg(ink).text_color(paper),
        None => tag.border_1().border_dashed().border_color(ink).text_color(ink),
    };
    div()
        .child(
            div()
                .absolute()
                .left(px(left - 6.))
                .w(px(TODAY_WIDTH + 12.))
                .top(px(at - 1.5))
                .h(px(3.))
                .rounded_full()
                .bg(ink)
                .when(paper.is_none(), |line| line.opacity(0.5)),
        )
        .child(tag)
}

/// The band names beside the capsule, and the point value at each class limit.
pub(super) fn zone_names(scale: &Scale, class: Option<Class>, left: f32, (ink, muted): (Hsla, Hsla)) -> Div {
    let mut names = div();
    for (zone, from, to) in &scale.zones {
        let middle = (level(*from, scale) + level(*to, scale)) / 2. * HEIGHT;
        let active = Some(*zone) == class;
        names = names.child(
            figure(11.)
                .absolute()
                .left(px(left + 40.))
                .top(px(middle - 8.))
                .whitespace_nowrap()
                .text_color(if active { ink } else { muted })
                .when(!active, |d| d.font_weight(FontWeight::NORMAL))
                .child(zone.short()),
        );
    }
    names
}

/// A dotted line joining the earlier reports' dots to today's total.
pub(super) fn trail(dots: Vec<(f32, f32)>, ink: Hsla) -> impl IntoElement {
    let dots = Rc::new(dots);
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.5)).dash_array(&[px(2.), px(4.)]);
            for (i, (x, y)) in dots.iter().enumerate() {
                let at = point(bounds.origin.x + px(*x), bounds.origin.y + px(*y));
                if i == 0 { path.move_to(at) } else { path.line_to(at) }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, ink.opacity(0.6));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}
