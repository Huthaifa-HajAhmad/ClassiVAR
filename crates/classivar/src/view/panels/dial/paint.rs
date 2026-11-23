//! Painting the dial: the class bands, the evidence slices, the ticks and the needle.

use super::*;

pub(super) fn dial_canvas(face: Rc<Face>, frame: Frame, needle: f32) -> impl IntoElement {
    canvas(
        move |bounds, _, _| frame.set(Some(bounds.origin)),
        move |bounds, _, window, _| paint_face(&face, needle, bounds.origin, window),
    )
    .size_full()
}

/// Turn a position relative to the dial into a window position.
pub(super) fn to_window(origin: Point<Pixels>, (x, y): (f32, f32)) -> Point<Pixels> {
    point(origin.x + px(x), origin.y + px(y))
}

/// A slice of a ring between two angles (`from` is the larger), as a filled shape.
pub(super) fn ring_path(
    face: &Face,
    (inner, outer): (f32, f32),
    (from, to): (f32, f32),
    origin: Point<Pixels>,
) -> Option<Path<Pixels>> {
    let at = |radius: f32, angle: f32| to_window(origin, polar(face.centre(), radius, angle));
    let mut path = PathBuilder::fill();
    path.move_to(at(outer, from));
    path.arc_to(point(px(outer), px(outer)), px(0.), false, true, at(outer, to));
    path.line_to(at(inner, to));
    path.arc_to(point(px(inner), px(inner)), px(0.), false, false, at(inner, from));
    path.close();
    path.build().ok()
}

/// A straight line along `angle` from one radius to another.
pub(super) fn spoke(
    face: &Face,
    (from, to): (f32, f32),
    angle: f32,
    width: f32,
    origin: Point<Pixels>,
    dashed: bool,
) -> Option<Path<Pixels>> {
    let mut path = PathBuilder::stroke(px(width));
    if dashed {
        path = path.dash_array(&[px(4.), px(4.)]);
    }
    path.move_to(to_window(origin, polar(face.centre(), from, angle)));
    path.line_to(to_window(origin, polar(face.centre(), to, angle)));
    path.build().ok()
}

pub(super) fn paint_face(face: &Face, needle: f32, origin: Point<Pixels>, window: &mut Window) {
    for (zone, from, to) in &face.scale.zones {
        let active = face.class.or(face.pointed) == Some(*zone);
        let fill: Hsla = match face.class {
            None if active => class_fill(*zone, face.dark).into(),
            None => face.blank,
            Some(_) if active => class_fill(*zone, face.dark).into(),
            Some(_) => Rgba { a: 0.8, ..class_fill(*zone, face.dark) }.into(),
        };
        let angles = (angle_of(*from as f32, &face.scale) - GAP, angle_of(*to as f32, &face.scale) + GAP);
        // The band of the class shown stands out from the others.
        let (inner, outer) = face.bands();
        let radii = if active { (inner - 4., outer + 6.) } else { (inner, outer) };
        if let Some(path) = ring_path(face, radii, angles, origin) {
            window.paint_path(path, fill);
        }
        if active && let Some(rim) = ring_path(face, (radii.1 + 3., radii.1 + 4.5), angles, origin) {
            window.paint_path(rim, face.ink.opacity(crate::look::EDGE_STRENGTH));
        }
    }
    let inner = face.ring().0 - 6.;
    for value in face.scale.min..=face.scale.max {
        let limit = face.scale.zones.iter().any(|(_, from, _)| *from == value);
        let (reach, colour) = if limit { (12., face.ink) } else { (5., face.muted) };
        if let Some(path) = spoke(face, (inner - reach, inner), angle_of(value as f32, &face.scale), 1., origin, false)
        {
            window.paint_path(path, colour);
        }
    }
    for slice in &face.slices {
        paint_slice(face, slice, origin, window);
    }
    if let Some(ghost) = face.ghost {
        let angle = angle_of(ghost as f32, &face.scale);
        if let Some(path) = spoke(face, (face.ring().0 - 30., face.radius + 6.), angle, 2., origin, true) {
            window.paint_path(path, face.ink);
        }
    }
    if face.class.is_some() {
        paint_needle(face, angle_of(needle, &face.scale), origin, window);
    }
}

pub(super) fn paint_slice(face: &Face, slice: &Segment, origin: Point<Pixels>, window: &mut Window) {
    let colour = direction_colour(slice.direction);
    let chosen = face.selected.as_deref() == Some(slice.code.as_str());
    let angles = (slice.start - GAP, slice.end + GAP);
    let Some(path) = ring_path(face, face.ring(), angles, origin) else { return };
    if chosen && face.set_aside {
        // Set aside for the preview: only a faint trace of where it was.
        window.paint_path(path, Rgba { a: 0.22, ..colour });
        return;
    }
    window.paint_path(path, colour);
    let (inner, outer) = face.ring();
    if !chosen
        && face.hovered.as_deref() == Some(slice.code.as_str())
        && let Some(edge) = ring_path(face, (outer + 2., outer + 4.), angles, origin)
    {
        window.paint_path(edge, face.ink.opacity(crate::look::HOVER_STRENGTH));
    }
    if chosen {
        for edge in [(outer + 2., outer + 5.), (inner - 5., inner - 2.)] {
            if let Some(path) = ring_path(face, edge, angles, origin) {
                window.paint_path(path, face.ink.opacity(crate::look::EDGE_STRENGTH));
            }
        }
    }
}

/// The needle: a soft glow, a solid line from inside the ring to past the bands, and a hub.
pub(super) fn paint_needle(face: &Face, angle: f32, origin: Point<Pixels>, window: &mut Window) {
    let reach = (face.ring().0 - 34., face.radius + 8.);
    if let Some(glow) = spoke(face, reach, angle, 9., origin, false) {
        window.paint_path(glow, Hsla { a: if face.dark { 0.22 } else { 0.10 }, ..face.ink });
    }
    if let Some(line) = spoke(face, reach, angle, 3., origin, false) {
        window.paint_path(line, face.ink);
    }
    let hub = Bounds::centered_at(to_window(origin, polar(face.centre(), reach.0, angle)), size(px(9.), px(9.)));
    window.paint_quad(fill(hub, face.ink).corner_radii(px(4.5)));
}
