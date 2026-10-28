//! Zoom (Alt + clic, Ctrl + clic sous Linux) sur un élément de la slide, comme le plugin zoom
//! de reveal.js : la page entière s'agrandit autour de lui ; un second clic, ou un changement
//! de slide, revient à la normale.

use std::cell::Cell;

use wasm_bindgen::JsCast;
use web_sys::{Element, MouseEvent};

use crate::dom::{self, add, closest, remove, style, unstyle};
use crate::engine::{self, Ev};

/// Marge laissée autour de l'élément agrandi, en px d'écran
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
    // Retour à l'échelle 1 avec la même origine : la vue revient depuis l'élément agrandi (sans
    // origine, elle repartirait du centre) ; les styles partent une fois la transition finie
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

/// Comme `zoom_on`, l'agrandissement plafonné à `max` (la magnify d'un mot garde sa phrase)
pub fn zoom_to(el: &Element, max: f64) {
    let r = el.get_bounding_client_rect();
    let win = dom::window();
    let (iw, ih) = (win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1.0), win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1.0));
    let (w, h) = (r.width() + PADDING * 2.0, r.height() + PADDING * 2.0);
    let scale = (iw / w).min(ih / h).min(max).max(1.0);
    // L'élément, marge comprise, au centre de la fenêtre
    let x = (r.left() - PADDING) - (iw / scale - w) / 2.0;
    let y = (r.top() - PADDING) - (ih / scale - h) / 2.0;
    let b = body();
    style(&b, "transform-origin", "0 0");
    style(&b, "transform", &format!("scale({scale}) translate({}px, {}px)", -x, -y));
    add(&dom::document().document_element().unwrap(), "zoomed");
    ZOOMED.with(|z| z.set(true));
}

pub fn install() {
    // Linux : Alt + clic déplace les fenêtres, Ctrl + clic est libre
    let linux = dom::window().navigator().user_agent().unwrap_or_default().contains("Linux");
    // Sur tout le document : une fois agrandie, la slide peut déborder de la fenêtre
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
