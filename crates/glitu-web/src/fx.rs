//! Highlights on click (`data-fx="halo:3-4 magnify:5"`, written by the build from `@3-4:halo`):
//! `halo`, a glow that pulses; `chase`, a light that runs around the element; `magnify`, the
//! view that grows around it (like Alt + click, see `zoom`).
//! A text, a code window, a formula or a skemo diagram node, for the time of its clicks.

use std::cell::{Cell, RefCell};

use web_sys::Element;

use crate::dom::{self, add, data, has, qsa, remove, style, toggle};
use crate::engine::{self, Ev};
use crate::zoom;

/// Gap between the element and the light of the chase, in slide px
const RING_GAP: f64 = 8.0;
/// Maximum magnification of the magnify
const MAGNIFY_MAX: f64 = 2.5;
/// Rounding of the chase's frame, in slide px, and length of its light: a share of the lap
const RING_RADIUS: f64 = 14.0;
const LIGHT: f64 = 0.12;
/// The trail: segments that follow the light, longer and paler (glitu.css)
const TRAIL: [(&str, f64); 3] = [(".chase-light", 1.0), (".chase-trail-1", 2.0), (".chase-trail-2", 3.5)];
/// Chase, in seconds: appearance, lap of the light, extinction (3 s in all)
const RING_IN: f64 = 0.2;
const RING_TOUR: f64 = 2.5;
const RING_OUT: f64 = 0.5;

thread_local! {
    /// The element magnified by `magnify`, if there is one
    static MAGNIFY: RefCell<Option<Element>> = const { RefCell::new(None) };
    static RINGS: Cell<u32> = const { Cell::new(0) };
}

/// Active effects of the element at click `s`
fn active(el: &Element, s: u32) -> Vec<String> {
    data(el, "fx")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|fx| {
            let (name, range) = fx.split_once(':')?;
            let (a, b) = range.split_once('-').map_or((range, None), |(a, b)| (a, Some(b)));
            let a: u32 = a.parse().ok()?;
            let b: u32 = b.map_or(Some(u32::MAX), |b| b.parse().ok())?;
            (a <= s && s <= b).then(|| name.to_string())
        })
        .collect()
}

/// The chase: a frame laid on the slide around the element (an SVG node has no
/// pseudo-element), found again by its number from one click to the next so as not to restart.
/// The light is a stroke of the outline that runs around it, once, at uniform speed; the engine
/// animates it frame by frame (`run`), the same in all browsers
fn ring(slide: &Element, el: &Element, on: bool) {
    let key = data(el, "fx-ring");
    let existing = key.as_ref().and_then(|k| dom::qs(slide, &format!(".fx-ring[data-for=\"{k}\"]")));
    if !on {
        if let Some(r) = existing {
            add(&r, "off");
            dom::timeout(250.0, move || r.remove());
        }
        return;
    }
    if let Some(r) = existing {
        place(slide, el, &r);
        return;
    }
    let r = {
        let k = key.unwrap_or_else(|| {
            let n = RINGS.with(|c| { c.set(c.get() + 1); c.get() });
            let _ = el.set_attribute("data-fx-ring", &n.to_string());
            n.to_string()
        });
        let r = dom::create("div");
        r.set_class_name("fx-ring");
        r.set_inner_html("<svg><rect class=\"chase-trail-2\"/><rect class=\"chase-trail-1\"/><rect class=\"chase-light\"/></svg>");
        let _ = r.set_attribute("data-for", &k);
        let _ = slide.append_child(&r);
        r
    };
    place(slide, el, &r);
    run(r);
}

/// The chase over time: the frame appears, the light runs around, the frame goes out.
/// Without animation (system preference, captures): the frame stays lit 3 s, still
fn run(r: Element) {
    if crate::motion::reduced() {
        style(&r, "opacity", "1");
        dom::timeout((RING_IN + RING_TOUR + RING_OUT) * 1000.0, move || style(&r, "opacity", "0"));
        return;
    }
    frame(r, None);
}

fn frame(r: Element, start: Option<f64>) {
    dom::raf(move |now| {
        if !r.is_connected() || has(&r, "off") {
            return;
        }
        let start = start.unwrap_or(now);
        let t = (now - start) / 1000.0;
        let opacity = if t < RING_IN {
            t / RING_IN
        } else if t < RING_IN + RING_TOUR {
            1.0
        } else {
            (1.0 - (t - RING_IN - RING_TOUR) / RING_OUT).max(0.0)
        };
        style(&r, "opacity", &format!("{opacity:.3}"));
        let perimeter: f64 = data(&r, "perimeter").and_then(|p| p.parse().ok()).unwrap_or(0.0);
        let run = ((t - RING_IN) / RING_TOUR).clamp(0.0, 1.0);
        // the head of the light, on the outline; each segment ends at the head and does not go back
        // before the start: nothing lights up before the light passes
        let light = (perimeter * LIGHT).max(40.0);
        let head = perimeter * run + light;
        for (sel, k) in TRAIL {
            let Some(seg) = dom::qs(&r, sel) else { continue };
            let len = (light * k).min(head).min(perimeter);
            let _ = seg.set_attribute("stroke-dasharray", &format!("{len:.2} {:.2}", (perimeter - len).max(0.0)));
            let _ = seg.set_attribute("stroke-dashoffset", &format!("{:.2}", len - head));
        }
        if t < RING_IN + RING_TOUR + RING_OUT {
            frame(r, Some(start));
        }
    });
}

/// The frame on the element, in slide coordinates (scaling and zoom included)
fn place(slide: &Element, el: &Element, r: &Element) {
    let (sr, er) = (slide.get_bounding_client_rect(), el.get_bounding_client_rect());
    let scale = if sr.width() > 0.0 { sr.width() / engine::config().width } else { 1.0 };
    style(r, "left", &format!("{}px", (er.left() - sr.left()) / scale - RING_GAP));
    style(r, "top", &format!("{}px", (er.top() - sr.top()) / scale - RING_GAP));
    let (w, h) = (er.width() / scale + 2.0 * RING_GAP, er.height() / scale + 2.0 * RING_GAP);
    style(r, "width", &format!("{w}px"));
    style(r, "height", &format!("{h}px"));
    // the outline, to the stroke (1.5 px of margin); its length sets the light
    let (w, h) = (w - 3.0, h - 3.0);
    let perimeter = 2.0 * (w + h) - (8.0 - 2.0 * std::f64::consts::PI) * RING_RADIUS;
    for rect in qsa(r, "rect") {
        for (k, v) in [("x", 1.5), ("y", 1.5), ("width", w), ("height", h), ("rx", RING_RADIUS)] {
            let _ = rect.set_attribute(k, &v.to_string());
        }
    }
    // the course of the light: one lap (see `frame`)
    let _ = r.set_attribute("data-perimeter", &format!("{perimeter:.2}"));
}

fn update(slide: &Element) {
    let s = engine::current_step(slide);
    let mut magnify = None;
    for el in qsa(slide, "[data-fx]") {
        let fx = active(&el, s);
        let on = |name: &str| fx.iter().any(|f| f == name);
        toggle(&el, "fx-on", !fx.is_empty());
        if on("halo") {
            remove(&el, "fx-halo-off");
            add(&el, "fx-halo");
        } else if has(&el, "fx-halo") {
            // the glow fades out
            remove(&el, "fx-halo");
            add(&el, "fx-halo-off");
        }
        ring(slide, &el, on("chase"));
        if on("magnify") {
            magnify = Some(el);
        }
    }
    let previous = MAGNIFY.with(|l| l.borrow().clone());
    match magnify {
        Some(el) if previous.as_ref() != Some(&el) => {
            zoom::zoom_to(&el, MAGNIFY_MAX);
            MAGNIFY.with(|l| *l.borrow_mut() = Some(el));
        }
        None if previous.is_some() => {
            zoom::zoom_out();
            MAGNIFY.with(|l| *l.borrow_mut() = None);
        }
        _ => {}
    }
}

/// The slide left loses its effects
fn clear(slide: &Element) {
    for el in qsa(slide, ".fx-on, .fx-halo, .fx-halo-off") {
        for c in ["fx-on", "fx-halo", "fx-halo-off"] {
            remove(&el, c);
        }
    }
    for r in qsa(slide, ".fx-ring") {
        r.remove();
    }
    MAGNIFY.with(|l| *l.borrow_mut() = None);
}

pub fn install() {
    // When printing and in the scroll view, the slides show their final state, without effect
    if engine::printing() {
        return;
    }
    engine::on(|ev| match ev {
        Ev::Ready => update(&engine::current_slide()),
        Ev::SlideChanged { previous } => {
            if let Some(p) = previous {
                clear(p);
            }
            update(&engine::current_slide());
        }
        Ev::FragmentShown(_) | Ev::FragmentHidden(_) => update(&engine::current_slide()),
        // the frames follow the new scale; the magnify, undone by `zoom`, is done again
        Ev::Resize => {
            let slide = engine::current_slide();
            if let Some(el) = MAGNIFY.with(|l| l.borrow().clone()) {
                zoom::zoom_to(&el, MAGNIFY_MAX);
            }
            for el in qsa(&slide, "[data-fx-ring]") {
                let k = data(&el, "fx-ring").unwrap_or_default();
                if let Some(r) = dom::qs(&slide, &format!(".fx-ring[data-for=\"{k}\"]")) {
                    place(&slide, &el, &r);
                }
            }
        }
    });
}
