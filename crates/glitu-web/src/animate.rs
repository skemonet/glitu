//! Animated slides, like reveal.js's auto-animate: between two slides that follow each other and
//! both carry `data-auto-animate` (`{.animate}` in the title), the shared elements slide from
//! their place in the first one to their place in the second one, changing size; the new ones
//! fade in. Two elements are shared if they have the same `data-id` (`[text]{#name}`), otherwise
//! the same kind and the same text (same image for an image, same title for a code window).

use std::collections::HashMap;

use web_sys::Element;

use crate::dom::{self, add, closest, has, qs, qsa, remove};
use crate::engine::{self, Ev};
use crate::motion::{self, Move, Rect};

/// What can slide from one slide to the other
const CANDIDATES: &str = "[data-id], h1, h2, h3, h4, h5, h6, p, li, img, pre, blockquote, table, .window, .diagram, .formula, .glyphs";

fn animated(slide: &Element) -> bool {
    slide.has_attribute("data-auto-animate")
}

/// What changes size while sliding; text only slides (stretched, it would be distorted),
/// aligned on its alignment
const BOXES: &str = "img, pre, table, .window, .diagram, .formula";

/// Starting point of a text: its new box, placed on the old one
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

/// What makes two elements counterparts
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

/// The candidate elements of the slide, in document order, outside the notes
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
    // A processed element takes its descendants along: they are not animated a second time
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
        // No slide transition: the new one takes the place of the old one at once, its elements are
        // what moves
        let root = engine::root();
        add(&root, "auto-animating");
        transition(previous, &current);
        dom::raf(move |_| { dom::raf(move |_| remove(&root, "auto-animating")); });
    });
}
