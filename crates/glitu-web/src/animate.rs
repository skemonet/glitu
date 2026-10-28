//! Slides animées, comme l'auto-animate de reveal.js : entre deux slides qui se suivent et
//! portent toutes deux `data-auto-animate` (`{.animate}` dans le titre), les éléments
//! communs glissent de leur place dans la première à leur place dans la seconde, en changeant
//! de taille ; les nouveaux apparaissent en fondu. Deux éléments sont communs s'ils ont le
//! même `data-id` (`[texte]{#nom}`), sinon la même nature et le même texte (même image pour
//! une image, même titre pour une fenêtre de code).

use std::collections::HashMap;

use web_sys::Element;

use crate::dom::{self, add, closest, has, qs, qsa, remove};
use crate::engine::{self, Ev};
use crate::motion::{self, Move, Rect};

/// Ce qui peut glisser d'une slide à l'autre
const CANDIDATES: &str = "[data-id], h1, h2, h3, h4, h5, h6, p, li, img, pre, blockquote, table, .window, .diagram, .formula, .glyphs";

fn animated(slide: &Element) -> bool {
    slide.has_attribute("data-auto-animate")
}

/// Ce qui change de taille en glissant ; le texte, lui, ne fait que glisser (étiré, il se
/// déformerait), calé sur son alignement
const BOXES: &str = "img, pre, table, .window, .diagram, .formula";

/// Point de départ d'un texte : sa nouvelle boîte, placée sur l'ancienne
fn text_start(el: &Element, old: Rect) -> Rect {
    let new = Rect::of(el);
    let align = dom::window().get_computed_style(el).ok().flatten().and_then(|c| c.get_property_value("text-align").ok()).unwrap_or_default();
    let shift = match align.as_str() {
        "center" => (old.w - new.w) / 2.0,
        "right" | "end" => old.w - new.w,
        _ => 0.0,
    };
    Rect { x: old.x + shift, y: old.y, ..new }
}

/// Ce qui rend deux éléments homologues
fn key(el: &Element) -> String {
    if let Some(id) = el.get_attribute("data-id") {
        return format!("#{id}");
    }
    let what = if el.tag_name() == "IMG" {
        el.get_attribute("src").unwrap_or_default()
    } else if has(el, "window") {
        qs(el, ".window-title").and_then(|t| t.text_content()).unwrap_or_default()
    } else {
        el.text_content().unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ")
    };
    format!("{} {}|{what}", el.tag_name(), el.get_attribute("class").unwrap_or_default().split_whitespace().filter(|c| !matches!(*c, "visible" | "current-fragment")).collect::<Vec<_>>().join("."))
}

/// Les éléments candidats de la slide, dans l'ordre du document, hors notes
fn candidates(slide: &Element) -> Vec<Element> {
    qsa(slide, CANDIDATES).into_iter().filter(|e| closest(e, "aside.notes").is_none()).collect()
}

fn transition(from: &Element, to: &Element) {
    let (from_box, to_box) = (Rect::of(from), Rect::of(to));
    let mut before: HashMap<String, Vec<Rect>> = HashMap::new();
    for el in candidates(from) {
        before.entry(key(&el)).or_default().push(Rect::of(&el).moved(from_box, to_box));
    }
    for list in before.values_mut() {
        list.reverse();
    }
    // Un élément traité emporte ses descendants : on ne les anime pas une seconde fois
    let mut done: Vec<Element> = Vec::new();
    let mut moves = Vec::new();
    for el in candidates(to) {
        if done.iter().any(|d| d.contains(Some(&el))) {
            continue;
        }
        let m = match before.get_mut(&key(&el)).and_then(Vec::pop) {
            Some(old) if el.matches(BOXES).unwrap_or(false) => Move::From(old),
            Some(old) => Move::From(text_start(&el, old)),
            None => Move::Appear,
        };
        done.push(el.clone());
        moves.push((el, m));
    }
    motion::play(moves);
}

pub fn install() {
    engine::on(|ev| {
        let Ev::SlideChanged { previous: Some(previous) } = ev else { return };
        let current = engine::current_slide();
        if !animated(previous) || !animated(&current) || motion::reduced() {
            return;
        }
        // Pas de transition de slide : la nouvelle prend aussitôt la place de l'ancienne,
        // ce sont ses éléments qui bougent
        let root = engine::root();
        add(&root, "auto-animating");
        transition(previous, &current);
        dom::raf(move |_| { dom::raf(move |_| remove(&root, "auto-animating")); });
    });
}
