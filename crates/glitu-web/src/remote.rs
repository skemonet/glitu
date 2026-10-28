//! Télécommande (`?remote` dans l'adresse, sur un téléphone) : la slide en haut, deux grands
//! boutons pour reculer et avancer, les notes dessous. Sous `glitu serve`, toutes les fenêtres
//! ouvertes suivent (voir le serveur) : le téléphone pilote le projecteur.

use crate::dom::{self, add, qs};
use crate::engine::{self, Ev};

fn update(notes: &web_sys::Element) {
    let cur = engine::current_slide();
    let html = qs(&cur, "aside.notes").map_or_else(|| "<p class=\"empty\">Pas de notes.</p>".into(), |n| n.inner_html());
    notes.set_inner_html(&format!(
        "<p class=\"where\">Slide {} / {}</p>{html}",
        engine::leaf_index() + 1,
        engine::all_leaves().len()
    ));
}

pub fn install() {
    let remote = dom::window().location().search().unwrap_or_default().contains("remote");
    if !remote || engine::receiver() {
        return;
    }
    add(&dom::document().document_element().unwrap(), "glitu-remote");
    let pad = dom::create("div");
    pad.set_class_name("remote-pad");
    pad.set_inner_html(
        "<div class=\"remote-buttons\"><button class=\"prev\" aria-label=\"Précédente\">◀</button>\
         <button class=\"next\" aria-label=\"Suivante\">▶</button></div><div class=\"remote-notes\"></div>",
    );
    if let Some(body) = dom::document().body() {
        let _ = body.append_child(&pad);
    }
    for (sel, go) in [(".prev", engine::prev as fn()), (".next", engine::next as fn())] {
        if let Some(b) = qs(&pad, sel) {
            dom::on::<web_sys::Event>(&b, "click", move |_| go());
        }
    }
    let Some(notes) = qs(&pad, ".remote-notes") else { return };
    update(&notes);
    engine::on(move |ev| {
        if !matches!(ev, Ev::Resize) {
            update(&notes);
        }
    });
    engine::layout();
}
