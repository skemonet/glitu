//! Dessin sur les slides (touche D) : au stylet, au doigt ou à la souris, par-dessus la slide
//! en cours. Les traits restent avec leur slide (un `svg.drawing` dans la section, aux
//! dimensions de la slide) et sont gardés dans le navigateur (localStorage), d'une séance à
//! l'autre. En mode dessin : 1 à 5 changent la couleur, Z annule le dernier trait, X efface la
//! slide, D ou Échap en sortent ; les flèches changent toujours de slide.

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use web_sys::{Element, PointerEvent};

use crate::dom::{self, has, qs, qsa, toggle};
use crate::engine;

const SVG: &str = "http://www.w3.org/2000/svg";
const COLORS: [&str; 5] = ["#f59e0b", "#ef4444", "#22d3ee", "#22c55e", "#ffffff"];

#[derive(Default)]
struct Pen {
    color: usize,
    /// Le trait en cours : son élément et son tracé
    stroke: Option<(Element, String)>,
}

thread_local! {
    static PEN: RefCell<Pen> = RefCell::new(Pen::default());
}

fn drawing() -> bool {
    has(&engine::root(), "drawing")
}

/// Clé de stockage : une par présentation (son adresse)
fn storage_key() -> String {
    format!("glitu-draw:{}", dom::window().location().pathname().unwrap_or_default())
}

fn storage() -> Option<web_sys::Storage> {
    dom::window().local_storage().ok().flatten()
}

/// Le calque de dessin d'une slide, créé au besoin
fn layer(slide: &Element) -> Element {
    if let Some(svg) = qs(slide, ":scope > svg.drawing") {
        return svg;
    }
    let c = engine::config();
    let svg = dom::document().create_element_ns(Some(SVG), "svg").expect("svg");
    let _ = svg.set_attribute("class", "drawing");
    let _ = svg.set_attribute("viewBox", &format!("0 0 {} {}", c.width, c.height));
    let _ = slide.append_child(&svg);
    svg
}

fn add_path(layer: &Element, color: &str, d: &str) -> Element {
    let path = dom::document().create_element_ns(Some(SVG), "path").expect("path");
    let _ = path.set_attribute("stroke", color);
    let _ = path.set_attribute("d", d);
    let _ = layer.append_child(&path);
    path
}

/// Tous les traits, par slide : `{"h/v": [[couleur, tracé], …]}`
fn save() {
    let all = js_sys::Object::new();
    for (h, v, slide) in engine::all_leaves() {
        let strokes = js_sys::Array::new();
        for p in qsa(&slide, ":scope > svg.drawing path") {
            let pair = js_sys::Array::of2(&p.get_attribute("stroke").unwrap_or_default().into(), &p.get_attribute("d").unwrap_or_default().into());
            strokes.push(&pair);
        }
        if strokes.length() > 0 {
            let _ = js_sys::Reflect::set(&all, &format!("{h}/{v}").into(), &strokes);
        }
    }
    if let (Some(store), Ok(json)) = (storage(), js_sys::JSON::stringify(&all)) {
        let _ = store.set_item(&storage_key(), &String::from(json));
    }
}

fn load() {
    let Some(text) = storage().and_then(|s| s.get_item(&storage_key()).ok().flatten()) else { return };
    let Ok(all) = js_sys::JSON::parse(&text) else { return };
    for (h, v, slide) in engine::all_leaves() {
        let Ok(strokes) = js_sys::Reflect::get(&all, &format!("{h}/{v}").into()) else { continue };
        let Ok(strokes) = strokes.dyn_into::<js_sys::Array>() else { continue };
        let svg = layer(&slide);
        for pair in strokes.iter() {
            let pair: js_sys::Array = pair.unchecked_into();
            add_path(&svg, &pair.get(0).as_string().unwrap_or_default(), &pair.get(1).as_string().unwrap_or_default());
        }
    }
}

/// Point de la fenêtre → coordonnées de la slide
fn point(svg: &Element, e: &PointerEvent) -> (f64, f64) {
    let r = svg.get_bounding_client_rect();
    let c = engine::config();
    let x = (e.client_x() as f64 - r.left()) / r.width() * c.width;
    let y = (e.client_y() as f64 - r.top()) / r.height() * c.height;
    ((x * 10.0).round() / 10.0, (y * 10.0).round() / 10.0)
}

fn set_mode(on: bool) {
    toggle(&engine::root(), "drawing", on);
    if on {
        layer(&engine::current_slide());
    }
}

fn undo() {
    if let Some(last) = qsa(&engine::current_slide(), ":scope > svg.drawing path").pop() {
        last.remove();
        save();
    }
}

fn clear() {
    for p in qsa(&engine::current_slide(), ":scope > svg.drawing path") {
        p.remove();
    }
    save();
}

pub fn install() {
    if engine::receiver() || engine::printing() {
        return;
    }
    load();
    engine::on_key("d", || set_mode(true));
    engine::intercept(|e| {
        if !drawing() {
            return false;
        }
        match e.key().as_str() {
            "d" | "D" | "Escape" => set_mode(false),
            "z" | "Z" => undo(),
            "x" | "X" => clear(),
            k if k.len() == 1 && ("1"..="5").contains(&k) => PEN.with(|p| p.borrow_mut().color = k.parse::<usize>().unwrap_or(1) - 1),
            _ => return false,
        }
        true
    });
    // Arrivée sur une slide en mode dessin : son calque
    engine::on(|_| {
        if drawing() {
            layer(&engine::current_slide());
        }
    });
    let doc = dom::document();
    dom::on::<PointerEvent>(&doc, "pointerdown", |e| {
        if !drawing() || e.button() > 0 {
            return;
        }
        e.prevent_default();
        let svg = layer(&engine::current_slide());
        let (x, y) = point(&svg, &e);
        let d = format!("M{x} {y}");
        let color = PEN.with(|p| COLORS[p.borrow().color]);
        let path = add_path(&svg, color, &d);
        PEN.with(|p| p.borrow_mut().stroke = Some((path, d)));
    });
    dom::on::<PointerEvent>(&doc, "pointermove", |e| {
        PEN.with(|p| {
            let mut p = p.borrow_mut();
            let Some((path, d)) = p.stroke.as_mut() else { return };
            let Some(svg) = path.parent_element() else { return };
            let (x, y) = point(&svg, &e);
            *d += &format!(" L{x} {y}");
            let _ = path.set_attribute("d", d);
        });
    });
    dom::on::<PointerEvent>(&doc, "pointerup", |_| {
        if PEN.with(|p| p.borrow_mut().stroke.take()).is_some() {
            save();
        }
    });
}
