//! The dial: one of the three pictures of the class (see `settings::Picture`).
//!
//! A half circle of class bands. The counted criteria are slices of an inner
//! ring, laid end to end from zero the way the track lays its blocks, and a
//! needle points at the total. The class shown is in full colour; the other
//! bands are quieter. Clicking a slice selects it (its explanation opens under
//! the picture); clicking it again previews the class without it, and a dashed
//! needle shows where the total would be. The buttons under the picture do the
//! same from the keyboard.
//!
//! The dial is drawn only when a class is shown. Its geometry is in `dial_geometry.rs`.

use std::rc::Rc;
use std::time::Duration;

use classivar_core::Assessment;

use super::dial_geometry::{Segment, angle_of, polar, radius_for, segment_at, segments};
use super::evidence_stack::{bars, steps};
use super::point_axis::{points_text, widened};
use super::*;
use crate::look::{MONO, figure};
use crate::view::describe::Scale;

/// Room around the half circle for the band labels, in pixels.
const SIDE_ROOM: f32 = 44.;
const TOP_ROOM: f32 = 34.;
/// Room under the centre line.
const BOTTOM_ROOM: f32 = 16.;
/// A small gap between neighbouring bands and slices, in radians.
const GAP: f32 = 0.006;

/// Everything the painter needs, shared by every frame of the needle's motion.
struct Face {
    scale: Scale,
    /// None draws an empty dial: grey bands, no needle, no class.
    class: Option<Class>,
    slices: Vec<Segment>,
    selected: Option<String>,
    set_aside: bool,
    ghost: Option<i32>,
    radius: f32,
    dark: bool,
    ink: Hsla,
    muted: Hsla,
    blank: Hsla,
}

impl Face {
    fn centre(&self) -> (f32, f32) {
        (self.radius + SIDE_ROOM, self.radius + TOP_ROOM)
    }

    /// The class bands (outer), as inner and outer radii.
    fn bands(&self) -> (f32, f32) {
        (self.radius - 26., self.radius)
    }

    /// The evidence ring (inner), as inner and outer radii.
    fn ring(&self) -> (f32, f32) {
        (self.radius - 76., self.radius - 34.)
    }

    fn size(&self) -> (f32, f32) {
        (2. * (self.radius + SIDE_ROOM), self.radius + TOP_ROOM + BOTTOM_ROOM)
    }
}

type Frame = Rc<Cell<Option<Point<Pixels>>>>;

impl Workbench {
    fn dial_face(&self, result: &Assessment, class: Option<Class>, (width, height): (f32, f32), cx: &App) -> Face {
        let theme = cx.theme();
        let steps = steps(&result.items);
        let scale = widened(&scale_for(self.rules), &bars(&steps));
        let set_aside = self.preview && self.editing == Editing::Today;
        let ghost = self
            .inspected
            .as_deref()
            .filter(|_| set_aside)
            .and_then(|code| self.preview_without(code).ok())
            .filter(|outcome| outcome.class().is_some())
            .map(|outcome| outcome.total_points);
        Face {
            slices: segments(&steps, &scale),
            scale,
            class,
            selected: self.inspected.clone(),
            set_aside,
            ghost,
            radius: radius_for(width, height),
            dark: theme.is_dark(),
            ink: theme.foreground,
            muted: theme.muted_foreground,
            blank: theme.muted,
        }
    }

    pub(super) fn dial(
        &self,
        result: &Assessment,
        class: Option<Class>,
        room: (f32, f32),
        interactive: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let face = Rc::new(self.dial_face(result, class, room, cx));
        let (w, h) = face.size();
        let total = result.total_points;
        let (mut from, shown) = self.needle.get();
        if shown != total {
            from = shown;
            self.needle.set((shown, total));
        }
        let (painted, frame) = (face.clone(), self.dial_frame.clone());
        let drawing = div().absolute().top_0().left_0().size_full().with_animation(
            SharedString::from(format!("dial-{from}-{total}")),
            Animation::new(Duration::from_millis(620)).with_easing(ease_out_quint()),
            move |el, t| {
                let needle = from as f32 + (total - from) as f32 * t;
                el.child(dial_canvas(painted.clone(), frame.clone(), needle))
            },
        );
        let (hits, frame) = (face.clone(), self.dial_frame.clone());
        div()
            .id("dial")
            .relative()
            .flex_none()
            .w(px(w))
            .h(px(h))
            .when(interactive, |dial| dial.cursor_pointer())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    if !interactive {
                        return;
                    }
                    let Some(origin) = frame.get() else { return };
                    let at = (f32::from(event.position.x - origin.x), f32::from(event.position.y - origin.y));
                    if let Some(slice) = segment_at(&hits.slices, hits.centre(), hits.ring(), at) {
                        let code = slice.code.clone();
                        this.choose_block(&code, cx);
                    }
                }),
            )
            .child(drawing)
            .children(labels(&face))
            .into_any_element()
    }
}

#[cfg(test)]
impl Workbench {
    /// Where the middle of a slice was last painted, in window coordinates, so a test can click it.
    pub(in crate::view) fn slice_centre(&self, result: &Assessment, code: &str, cx: &App) -> Option<Point<Pixels>> {
        let face = self.dial_face(result, result.class(), (self.stage_width.get(), self.stage_height.get()), cx);
        let slice = face.slices.iter().find(|slice| slice.code == code)?;
        let radius = (face.ring().0 + face.ring().1) / 2.;
        let origin = self.dial_frame.get()?;
        Some(to_window(origin, polar(face.centre(), radius, (slice.start + slice.end) / 2.)))
    }
}

fn dial_canvas(face: Rc<Face>, frame: Frame, needle: f32) -> impl IntoElement {
    canvas(
        move |bounds, _, _| frame.set(Some(bounds.origin)),
        move |bounds, _, window, _| paint_face(&face, needle, bounds.origin, window),
    )
    .size_full()
}

/// Turn a position relative to the dial into a window position.
fn to_window(origin: Point<Pixels>, (x, y): (f32, f32)) -> Point<Pixels> {
    point(origin.x + px(x), origin.y + px(y))
}

/// A slice of a ring between two angles (`from` is the larger), as a filled shape.
fn ring_path(
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
fn spoke(
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

fn paint_face(face: &Face, needle: f32, origin: Point<Pixels>, window: &mut Window) {
    for (zone, from, to) in &face.scale.zones {
        let active = face.class == Some(*zone);
        let fill: Hsla = match face.class {
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
            window.paint_path(rim, face.ink);
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

fn paint_slice(face: &Face, slice: &Segment, origin: Point<Pixels>, window: &mut Window) {
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
    if chosen {
        let (inner, outer) = face.ring();
        for edge in [(outer + 2., outer + 5.), (inner - 5., inner - 2.)] {
            if let Some(path) = ring_path(face, edge, angles, origin) {
                window.paint_path(path, face.ink);
            }
        }
    }
}

/// The needle: a soft glow, a solid line from inside the ring to past the bands, and a hub.
fn paint_needle(face: &Face, angle: f32, origin: Point<Pixels>, window: &mut Window) {
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

/// A short text centred on a point of the dial.
fn place(face: &Face, text: String, (radius, angle): (f32, f32), colour: Hsla, bold: bool) -> AnyElement {
    let (x, y) = polar(face.centre(), radius, angle);
    div()
        .absolute()
        .left(px(x - 40.))
        .top(px(y - 8.))
        .w(px(80.))
        .flex()
        .justify_center()
        .font_family(MONO)
        .text_size(px(11.))
        .line_height(px(16.))
        .text_color(colour)
        .when(bold, |d| d.font_weight(FontWeight::BOLD))
        .child(text)
        .into_any_element()
}

/// The band names outside the arc, the point value at each class limit, and the code on each slice wide enough for it.
fn labels(face: &Face) -> Vec<AnyElement> {
    let mut out = Vec::new();
    for (zone, from, to) in &face.scale.zones {
        let middle = angle_of((*from + *to) as f32 / 2., &face.scale);
        let active = face.class == Some(*zone);
        let colour = if active { face.ink } else { face.muted };
        out.push(place(face, zone.short().to_string(), (face.radius + 22., middle), colour, active));
    }
    for (_, from, _) in face.scale.zones.iter().skip(1) {
        let at = (face.ring().0 - 30., angle_of(*from as f32, &face.scale));
        out.push(place(face, points_text(*from), at, face.muted, false));
    }
    let middle = (face.ring().0 + face.ring().1) / 2.;
    for slice in face.slices.iter().filter(|slice| (slice.start - slice.end) * middle >= 34.) {
        let (x, y) = polar(face.centre(), middle, (slice.start + slice.end) / 2.);
        out.push(
            figure(11.)
                .absolute()
                .left(px(x - 30.))
                .top(px(y - 8.))
                .w(px(60.))
                .flex()
                .justify_center()
                .text_color(direction_ink())
                .child(slice.code.clone())
                .into_any_element(),
        );
    }
    out
}
