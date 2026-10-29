//! Zoom (Alt + click, Ctrl + click on Linux) on an element of the slide, like reveal.js's zoom
//! plugin: the whole page grows around it; a second click, or a change of slide, goes back to
//! normal.

use std::cell::Cell;

use wasm_bindgen::JsCast;
use web_sys::{Element, MouseEvent};

use crate::dom::{self, add, closest, remove, style, unstyle};
use crate::engine::{self, Ev};

/// Margin left around the magnified element, in screen px
const PADDING: f64 = 20.0;

thread_local! {
    static ZOOMED: Cell<bool> = const { Cell::new(false) };
}

fn body() -> Element {
    dom::document().body().unwrap().into()
}

pub fn zoom_out() {
    if !ZOOMED.with(|z| z.replace(false)) {
        return;
    }
    // Back to scale 1 with the same origin: the view comes back from the magnified element
    // (without an origin, it would start from the centre); the styles go once the transition is over
    let b = body();
    style(&b, "transform", "scale(1) translate(0px, 0px)");
    dom::timeout(900.0, move || {
        if !ZOOMED.with(Cell::get) {
            unstyle(&b, "transform");
            unstyle(&b, "transform-origin");
        }
    });
    remove(&dom::document().document_element().unwrap(), "zoomed");
}

pub fn zoom_on(el: &Element) {
    zoom_to(el, f64::INFINITY);
}

/// Like `zoom_on`, the magnification capped at `max` (the magnify of a word keeps its sentence)
pub fn zoom_to(el: &Element, max: f64) {
    let r = el.get_bounding_client_rect();
    let win = dom::window();
    let (iw, ih) = (win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1.0), win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1.0));
    let (w, h) = (r.width() + PADDING * 2.0, r.height() + PADDING * 2.0);
    let scale = (iw / w).min(ih / h).min(max).max(1.0);
    // The element, margin included, in the centre of the window
    let x = (r.left() - PADDING) - (iw / scale - w) / 2.0;
    let y = (r.top() - PADDING) - (ih / scale - h) / 2.0;
    let b = body();
    style(&b, "transform-origin", "0 0");
    style(&b, "transform", &format!("scale({scale}) translate({}px, {}px)", -x, -y));
    add(&dom::document().document_element().unwrap(), "zoomed");
    ZOOMED.with(|z| z.set(true));
}

pub fn install() {
    // Linux: Alt + click moves the windows, Ctrl + click is free
    let linux = dom::window().navigator().user_agent().unwrap_or_default().contains("Linux");
    // On the whole document: once magnified, the slide can overflow the window
    dom::on::<MouseEvent>(&dom::document(), "mousedown", move |e| {
        if !(if linux { e.ctrl_key() } else { e.alt_key() }) {
            return;
        }
        e.prevent_default();
        if ZOOMED.with(Cell::get) {
            zoom_out();
        } else if let Some(el) = e.target().and_then(|t| t.dyn_into::<Element>().ok()).filter(|t| closest(t, ".slides").is_some()) {
            zoom_on(&el);
        }
    });
    engine::on(|ev| {
        if matches!(ev, Ev::SlideChanged { .. } | Ev::Resize) {
            zoom_out();
        }
    });
}
