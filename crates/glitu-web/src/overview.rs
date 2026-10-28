//! Vue d'ensemble (Échap, O) : toutes les slides en mosaïque, la courante au centre ; les
//! flèches passent d'une slide à l'autre, Entrée ou un clic y va. Même DOM que reveal.js
//! (`.reveal.overview`, transformations des sections) : reveal.css l'habille.

use std::cell::Cell;

use web_sys::{Element, KeyboardEvent};

use crate::dom::{self, add, closest, data, qs, qsa, remove, style, unstyle};
use crate::engine::{self, Ev};

/// Écart entre deux slides de la mosaïque, en px de slide
const MARGIN: f64 = 70.0;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

fn active() -> bool {
    ACTIVE.with(Cell::get)
}

/// Pas de la mosaïque : une slide et sa marge
fn cell() -> (f64, f64) {
    let c = engine::config();
    (c.width + MARGIN, c.height + MARGIN)
}

/// Chaque slide (et son backdrop) à sa place dans la mosaïque : colonnes horizontales, piles verticales
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

/// Centre la mosaïque sur la slide courante, à une échelle lisible
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
    // reveal.js : une classe le temps d'une image, pour couper les transitions au retour
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

/// En vue d'ensemble, les flèches vont de slide en slide, sans les étapes ; la slide choisie
/// s'ouvre à son début, d'où qu'on vienne
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

/// Molette ou pavé tactile : un pas dans la mosaïque par geste
fn wheel(e: &web_sys::WheelEvent, travel: &Cell<(f64, f64)>) {
    e.prevent_default();
    // Maj + molette : la plupart des souris n'ont que l'axe vertical
    let (dx, dy) = if e.shift_key() { (e.delta_y(), 0.0) } else { (e.delta_x(), e.delta_y()) };
    let (x, y) = travel.get();
    let (x, y) = (x + dx, y + dy);
    let (h, v, _) = engine::position();
    let leaves = engine::leaves();
    let target = if x.abs() >= WHEEL_STEP {
        if x > 0.0 { (h + 1 < leaves.len()).then_some((h + 1, 0)) } else { h.checked_sub(1).map(|h| (h, 0)) }
    } else if y.abs() >= WHEEL_STEP {
        // En bas d'une colonne, on passe en haut de la suivante (et inversement)
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

/// Déplacement de molette qui vaut un pas (un cran de souris, un geste de pavé tactile)
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
    // Un clic sur une slide de la mosaïque y mène
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
