//! Mises en évidence au clic (`data-fx="halo:3-4 magnify:5"`, écrit par le build depuis
//! `@3-4:halo`) : `halo`, une lueur qui pulse ; `chase`, une lumière qui tourne autour de
//! l'élément ; `magnify`, la vue qui s'agrandit autour de lui (comme Alt + clic, voir `zoom`).
//! Un texte, une fenêtre de code, une formule ou un nœud de schéma skemo, le temps de ses clics.

use std::cell::{Cell, RefCell};

use web_sys::Element;

use crate::dom::{self, add, data, has, qsa, remove, style, toggle};
use crate::engine::{self, Ev};
use crate::zoom;

/// Écart entre l'élément et la lumière du chase, en px de slide
const RING_GAP: f64 = 8.0;
/// Agrandissement maximal de la magnify
const MAGNIFY_MAX: f64 = 2.5;
/// Arrondi du cadre du chase, en px de slide, et longueur de sa lumière : une part du tour
const RING_RADIUS: f64 = 14.0;
const LIGHT: f64 = 0.12;
/// La traîne : des segments qui suivent la lumière, plus longs et plus pâles (glitu.css)
const TRAIL: [(&str, f64); 3] = [(".chase-light", 1.0), (".chase-trail-1", 2.0), (".chase-trail-2", 3.5)];
/// Chase, en secondes : apparition, tour de la lumière, extinction (3 s en tout)
const RING_IN: f64 = 0.2;
const RING_TOUR: f64 = 2.5;
const RING_OUT: f64 = 0.5;

thread_local! {
    /// L'élément agrandi par `magnify`, s'il y en a un
    static MAGNIFY: RefCell<Option<Element>> = const { RefCell::new(None) };
    static RINGS: Cell<u32> = const { Cell::new(0) };
}

/// Effets actifs de l'élément au clic `s`
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

/// Le chase : un cadre posé sur la slide autour de l'élément (un nœud SVG n'a pas de
/// pseudo-élément), retrouvé par son numéro d'un clic à l'autre pour ne pas relancer sa ronde.
/// La lumière est un trait du contour qui en fait le tour, une fois, à vitesse uniforme ; le
/// moteur l'anime image par image (`run`), pareil dans tous les navigateurs
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

/// Le chase dans le temps : le cadre paraît, la lumière fait le tour, le cadre s'éteint.
/// Sans animation (préférence du système, captures) : le cadre reste allumé 3 s, immobile
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
        // la tête de la lumière, sur le contour ; chaque segment finit à la tête et ne
        // remonte pas avant le départ : rien ne s'allume avant le passage
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

/// Le cadre sur l'élément, en coordonnées de la slide (mise à l'échelle et zoom compris)
fn place(slide: &Element, el: &Element, r: &Element) {
    let (sr, er) = (slide.get_bounding_client_rect(), el.get_bounding_client_rect());
    let scale = if sr.width() > 0.0 { sr.width() / engine::config().width } else { 1.0 };
    style(r, "left", &format!("{}px", (er.left() - sr.left()) / scale - RING_GAP));
    style(r, "top", &format!("{}px", (er.top() - sr.top()) / scale - RING_GAP));
    let (w, h) = (er.width() / scale + 2.0 * RING_GAP, er.height() / scale + 2.0 * RING_GAP);
    style(r, "width", &format!("{w}px"));
    style(r, "height", &format!("{h}px"));
    // le contour, au trait près (1,5 px de marge) ; sa longueur règle la lumière
    let (w, h) = (w - 3.0, h - 3.0);
    let perimeter = 2.0 * (w + h) - (8.0 - 2.0 * std::f64::consts::PI) * RING_RADIUS;
    for rect in qsa(r, "rect") {
        for (k, v) in [("x", 1.5), ("y", 1.5), ("width", w), ("height", h), ("rx", RING_RADIUS)] {
            let _ = rect.set_attribute(k, &v.to_string());
        }
    }
    // la course de la lumière : un tour (voir `frame`)
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
            // la lueur s'éteint en fondu
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

/// La slide quittée perd ses effets
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
    // À l'impression et dans la vue défilante, les slides montrent leur état final, sans effet
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
        // les cadres suivent la nouvelle échelle ; la magnify, défaite par `zoom`, se refait
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
