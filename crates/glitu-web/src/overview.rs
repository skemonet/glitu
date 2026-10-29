//! Overview (Esc, O): all the slides as a mosaic, the current one in the centre; the arrows go
//! from one slide to another, Enter or a click goes there. Same DOM as reveal.js
//! (`.reveal.overview`, transforms of the sections): reveal.css dresses it.

use std::cell::Cell;

use web_sys::{Element, KeyboardEvent};

use crate::dom::{self, add, closest, data, qs, qsa, remove, style, unstyle};
use crate::engine::{self, Ev};

/// Gap between two slides of the mosaic, in slide px
const MARGIN: f64 = 70.0;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

fn active() -> bool {
    ACTIVE.with(Cell::get)
}

/// Step of the mosaic: a slide and its margin
fn cell() -> (f64, f64) {
    let c = engine::config();
    (c.width + MARGIN, c.height + MARGIN)
}

/// Each slide (and its backdrop) in its place in the mosaic: horizontal columns, vertical stacks
fn place(elements: &[Element], nested: impl Fn(&Element) -> Vec<Element>) {
    let (w, h) = cell();
    for (x, el) in elements.iter().enumerate() {
        let _ = el.set_attribute("data-index-h", &x.to_string());
        style(el, "transform", &format!("translate3d({}px, 0, 0)", x as f64 * w));
        for (y, inner) in nested(el).iter().enumerate() {
            let _ = inner.set_attribute("data-index-h", &x.to_string());
            let _ = inner.set_attribute("data-index-v", &y.to_string());
            style(inner, "transform", &format!("translate3d(0, {}px, 0)", y as f64 * h));
        }
    }
}

fn backgrounds() -> Option<Element> {
    qs(&engine::root(), ".backgrounds")
}

/// Centres the mosaic on the current slide, at a readable scale
fn update() {
    let win = dom::window();
    let (iw, ih) = (win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1.0), win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1.0));
    let vmin = iw.min(ih).max(1.0);
    let scale = (vmin / 5.0).max(150.0) / vmin;
    let (h, v, _) = engine::position();
    let (w, hh) = cell();
    engine::set_extra_transform(&format!("scale({scale}) translateX({}px) translateY({}px)", -(h as f64) * w, -(v as f64) * hh));
}

fn activate() {
    ACTIVE.with(|a| a.set(true));
    add(&engine::root(), "overview");
    if let Some(bg) = backgrounds() {
        let _ = engine::slides_element().append_child(&bg);
        place(&qsa(&bg, ":scope > .slide-background"), |b| qsa(b, ":scope > .slide-background"));
    }
    place(&engine::slides(), |s| qsa(s, ":scope > section"));
    engine::set_overview(true);
    update();
}

fn deactivate() {
    ACTIVE.with(|a| a.set(false));
    let root = engine::root();
    remove(&root, "overview");
    // reveal.js: a class for one frame, to cut the transitions when coming back
    add(&root, "overview-deactivating");
    let r2 = root.clone();
    dom::timeout(1.0, move || remove(&r2, "overview-deactivating"));
    if let Some(bg) = backgrounds() {
        let _ = root.append_child(&bg);
        for b in qsa(&bg, ".slide-background") {
            unstyle(&b, "transform");
        }
    }
    for s in qsa(&engine::slides_element(), "section") {
        unstyle(&s, "transform");
    }
    engine::set_overview(false);
    engine::set_extra_transform("");
}

fn toggle_overview() {
    if active() { deactivate() } else { activate() }
}

/// In the overview, the arrows go from slide to slide, without the steps; the chosen slide
/// opens at its beginning, wherever we come from
fn navigate(e: &KeyboardEvent) -> bool {
    let (h, v, _) = engine::position();
    let leaves = engine::leaves();
    let target = match e.key().as_str() {
        "ArrowRight" | "l" | "L" if h + 1 < leaves.len() => (h + 1, 0),
        "ArrowLeft" | "h" | "H" if h > 0 => (h - 1, 0),
        "ArrowDown" | "j" | "J" if v + 1 < leaves[h].len() => (h, v + 1),
        "ArrowUp" | "k" | "K" if v > 0 => (h, v - 1),
        "Enter" | " " => {
            deactivate();
            return true;
        }
        "ArrowRight" | "ArrowLeft" | "ArrowDown" | "ArrowUp" | "PageDown" | "PageUp" => return true,
        _ => return false,
    };
    engine::show(target.0, target.1, Some(-1), true);
    true
}

/// Wheel or touchpad: one step in the mosaic per gesture
fn wheel(e: &web_sys::WheelEvent, travel: &Cell<(f64, f64)>) {
    e.prevent_default();
    // Shift + wheel: most mice only have the vertical axis
    let (dx, dy) = if e.shift_key() { (e.delta_y(), 0.0) } else { (e.delta_x(), e.delta_y()) };
    let (x, y) = travel.get();
    let (x, y) = (x + dx, y + dy);
    let (h, v, _) = engine::position();
    let leaves = engine::leaves();
    let target = if x.abs() >= WHEEL_STEP {
        if x > 0.0 { (h + 1 < leaves.len()).then_some((h + 1, 0)) } else { h.checked_sub(1).map(|h| (h, 0)) }
    } else if y.abs() >= WHEEL_STEP {
        // At the bottom of a column, go to the top of the next one (and conversely)
        if y > 0.0 {
            if v + 1 < leaves[h].len() { Some((h, v + 1)) } else { (h + 1 < leaves.len()).then_some((h + 1, 0)) }
        } else if v > 0 {
            Some((h, v - 1))
        } else {
            h.checked_sub(1).map(|h| (h, leaves[h].len() - 1))
        }
    } else {
        travel.set((x, y));
        return;
    };
    travel.set((0.0, 0.0));
    if let Some((h, v)) = target {
        engine::show(h, v, Some(-1), true);
    }
}

/// Wheel movement worth one step (a notch of a mouse, a touchpad gesture)
const WHEEL_STEP: f64 = 60.0;

pub fn install() {
    let travel = std::rc::Rc::new(Cell::new((0.0, 0.0)));
    dom::on::<web_sys::WheelEvent>(&dom::document(), "wheel", move |e| {
        if active() {
            wheel(&e, &travel);
        }
    });
    engine::intercept(|e| match e.key().as_str() {
        "Escape" | "o" | "O" => {
            toggle_overview();
            true
        }
        _ => active() && navigate(e),
    });
    // A click on a slide of the mosaic leads there
    dom::on::<web_sys::MouseEvent>(&engine::slides_element(), "click", |e| {
        if !active() {
            return;
        }
        let Some(section) = e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).and_then(|t| closest(&t, "section:not(.stack)")) else { return };
        e.prevent_default();
        let index = |k: &str| data(&section, k).and_then(|v| v.parse().ok()).unwrap_or(0);
        engine::show(index("index-h"), index("index-v"), Some(-1), true);
        deactivate();
    });
    engine::on(|ev| {
        if active() && matches!(ev, Ev::SlideChanged { .. } | Ev::Resize) {
            update();
        }
    });
}
