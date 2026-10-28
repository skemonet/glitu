//! Aller à une slide (G) : on tape son numéro (`12`), sa position (`5.2` ou `5/2`, comme
//! l'adresse) ou son identifiant, puis Entrée. Même DOM que reveal.js (`.jump-to-slide`).

use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::dom::{self, qs};
use crate::engine;

/// Slide visée par ce qu'on a tapé : (h, v)
fn target(text: &str) -> Option<(usize, usize)> {
    let text = text.trim();
    if let Ok(n) = text.parse::<usize>() {
        // Numéro affiché : on compte toutes les slides, verticales comprises, à partir de 1
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
    box_.set_inner_html("<input type=\"text\" class=\"jump-to-slide-input\" placeholder=\"Aller à la slide…\" aria-label=\"Aller à la slide\">");
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
    // Toute fermeture passe par la perte du focus : le champ n'est retiré qu'une fois
    let i3 = input.clone();
    dom::on::<web_sys::Event>(&input, "blur", move |_| close(&i3));
}

pub fn install() {
    engine::on_key("g", open);
}
