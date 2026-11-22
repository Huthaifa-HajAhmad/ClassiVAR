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

use super::block_tip::{TIP_WIDTH, floating};
use super::dial_geometry::{Segment, angle_of, labelled, polar, radius_for, segment_at, segments};
use super::evidence_stack::{bars, steps};
use super::point_axis::{points_text, widened};
use super::*;
use crate::look::{MONO, figure};
use crate::view::describe::Scale;
mod paint;
use paint::dial_canvas;
#[cfg(test)]
use paint::to_window;

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
    /// The slice under the pointer: it gets a faint edge so the hover card is clearly about it.
    hovered: Option<String>,
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
            hovered: self.hovered.clone(),
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
        let (mut from, shown) = self.stage.needle.get();
        if shown != total {
            from = shown;
            self.stage.needle.set((shown, total));
        }
        let (painted, frame) = (face.clone(), self.stage.dial_frame.clone());
        let drawing = div().absolute().top_0().left_0().size_full().with_animation(
            SharedString::from(format!("dial-{from}-{total}")),
            Animation::new(Duration::from_millis(620)).with_easing(ease_out_quint()),
            move |el, t| {
                let needle = from as f32 + (total - from) as f32 * t;
                el.child(dial_canvas(painted.clone(), frame.clone(), needle))
            },
        );
        let tip = self.dial_tip(&face, result, cx).filter(|_| interactive);
        let dial = div().id("dial").relative().flex_none().w(px(w)).h(px(h));
        self.dial_pointer(dial, &face, interactive, cx)
            .child(drawing)
            .children(labels(&face))
            .children(tip)
            .into_any_element()
    }

    /// A click on a slice selects it (or previews without it); moving over a slice shows its card.
    fn dial_pointer(
        &self,
        dial: Stateful<Div>,
        face: &Rc<Face>,
        interactive: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        if !interactive {
            return dial;
        }
        let (clicks, moves) =
            ((face.clone(), self.stage.dial_frame.clone()), (face.clone(), self.stage.dial_frame.clone()));
        dial.cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    if let Some(code) = slice_under(&clicks.0, &clicks.1, event.position) {
                        this.choose_block(&code, cx);
                    }
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                this.hover_block(slice_under(&moves.0, &moves.1, event.position), cx);
            }))
            .on_hover(cx.listener(|this, over: &bool, _, cx| {
                if !*over {
                    this.hover_block(None, cx);
                }
            }))
    }

    /// The hover card for the slice under the pointer, just inside the ring and kept within the dial's width.
    fn dial_tip(&self, face: &Face, result: &Assessment, cx: &App) -> Option<AnyElement> {
        let code = self.hovered.as_deref()?;
        let slice = face.slices.iter().find(|slice| slice.code == code)?;
        let card = self.block_tip(result, code, cx)?;
        let (x, y) = polar(face.centre(), face.ring().0, (slice.start + slice.end) / 2.);
        let left = (x - TIP_WIDTH / 2.).clamp(0., (face.size().0 - TIP_WIDTH).max(0.));
        Some(floating(card, (left, y + 12.)))
    }
}

/// The code of the slice under a window position, if any.
fn slice_under(face: &Face, frame: &Frame, position: Point<Pixels>) -> Option<String> {
    let origin = frame.get()?;
    let at = (f32::from(position.x - origin.x), f32::from(position.y - origin.y));
    segment_at(&face.slices, face.centre(), face.ring(), at).map(|slice| slice.code.clone())
}

#[cfg(test)]
impl Workbench {
    /// Where the middle of a slice was last painted, in window coordinates, so a test can click it.
    pub(in crate::view) fn slice_centre(&self, result: &Assessment, code: &str, cx: &App) -> Option<Point<Pixels>> {
        let face = self.dial_face(result, result.class(), (self.stage.width.get(), self.stage.height.get()), cx);
        let slice = face.slices.iter().find(|slice| slice.code == code)?;
        let radius = (face.ring().0 + face.ring().1) / 2.;
        let origin = self.stage.dial_frame.get()?;
        Some(to_window(origin, polar(face.centre(), radius, (slice.start + slice.end) / 2.)))
    }
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
    for slice in labelled(&face.slices, middle, 34.) {
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
