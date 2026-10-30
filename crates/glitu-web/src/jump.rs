//! Go to a slide (G): type its number (`12`), its position (`5.2` or `5/2`, as in the address)
//! or its identifier, then Enter. Same DOM as reveal.js (`.jump-to-slide`).

use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::dom::{self, qs};
use crate::engine;

/// Slide aimed at by what was typed: (h, v)
fn target(text: &str) -> Option<(usize, usize)> {
    let text = text.trim();
    if let Ok(n) = text.parse::<usize>() {
        // Number shown: count all the slides, vertical ones included, from 1
        return engine::all_leaves().get(n.checked_sub(1)?).map(|(h, v, _)| (*h, *v));
    }
    if let Some((h, v)) = text.split_once(['.', '/']) {
        return Some((h.trim().parse().ok()?, v.trim().parse().ok()?));
    }
    let id = text.trim_start_matches('#');
    engine::all_leaves().into_iter().find(|(_, _, s)| s.id() == id).map(|(h, v, _)| (h, v))
}

fn close(input: &HtmlInputElement) {
    input.set_value("");
    if let Some(box_) = input.parent_element() {
        box_.remove();
    }
}

fn open() {
    let root = engine::root();
    if qs(&root, ".jump-to-slide").is_some() {
        return;
    }
    let box_ = dom::create("div");
    box_.set_class_name("jump-to-slide");
    box_.set_inner_html("<input type=\"text\" class=\"jump-to-slide-input\" placeholder=\"Go to slide…\" aria-label=\"Go to slide\">");
    let _ = root.append_child(&box_);
    let input: HtmlInputElement = qs(&box_, "input").unwrap().unchecked_into();
    let _ = input.focus();
    let i2 = input.clone();
    dom::on::<web_sys::KeyboardEvent>(&input, "keydown", move |e| {
        match e.key().as_str() {
            "Enter" => {
                if let Some((h, v)) = target(&i2.value()) {
                    engine::show(h, v, None, true);
                }
                let _ = i2.blur();
            }
            "Escape" => {
                let _ = i2.blur();
            }
            _ => return,
        }
        e.prevent_default();
        e.stop_propagation();
    });
    // Every closing goes through the loss of focus: the field is only removed once
    let i3 = input.clone();
    dom::on::<web_sys::Event>(&input, "blur", move |_| close(&i3));
}

pub fn install() {
    engine::on_key("g", open);
}
