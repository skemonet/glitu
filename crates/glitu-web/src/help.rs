//! Aide (?) : la liste des raccourcis, dans le calque de reveal.js (`.overlay.overlay-help`).
//! Échap, ? ou un clic sur la croix la referment.

use crate::dom::{self, add, qs, remove};
use crate::engine;

const SHORTCUTS: &[(&str, &str)] = &[
    ("→ Espace N", "Étape ou slide suivante"),
    ("← Maj+Espace P", "Étape ou slide précédente"),
    ("Page↓ / Page↑", "Chapitre suivant / précédent (en navigation 2D ; sinon slide suivante / précédente)"),
    ("↓ ↑", "Dans le chapitre : étape ou slide suivante / précédente"),
    ("H J K L", "Gauche, bas, haut, droite (slides verticales comprises)"),
    ("Début / Fin", "Première / dernière slide"),
    ("G", "Aller à une slide : numéro, 5.2 ou identifiant"),
    ("Échap / O", "Vue d'ensemble"),
    ("F", "Plein écran"),
    ("S", "Vue de l'orateur : slide, étape suivante, notes, chronomètre"),
    ("D", "Dessiner sur la slide (1 à 5 : couleur, Z : annuler, X : effacer, D : sortir)"),
    ("B / V / .", "Écran noir, puis reprise"),
    ("A", "Suspendre ou reprendre le défilement automatique"),
    ("Alt + clic (Ctrl + clic sous Linux)", "Zoomer sur un élément, puis revenir"),
    ("?", "Cette aide"),
];

fn overlay() -> Option<web_sys::Element> {
    qs(&engine::root(), ".overlay-help")
}

fn close() {
    if let Some(o) = overlay() {
        o.remove();
    }
    remove(&engine::root(), "has-overlay");
}

fn open() {
    if overlay().is_some() {
        return close();
    }
    let rows: String = SHORTCUTS.iter().map(|(k, a)| format!("<tr><td>{k}</td><td>{a}</td></tr>")).collect();
    let el = dom::create("div");
    el.set_class_name("overlay overlay-help");
    el.set_inner_html(&format!(
        "<header><a class=\"close\" href=\"#\"><span class=\"icon\"></span></a></header>\
         <div class=\"viewport\"><div class=\"viewport-inner\"><h1>Raccourcis clavier</h1>\
         <table><tr><th>Touche</th><th>Action</th></tr>{rows}</table></div></div>"
    ));
    if let Some(x) = qs(&el, ".close") {
        dom::on::<web_sys::Event>(&x, "click", |e| {
            e.prevent_default();
            close();
        });
    }
    let root = engine::root();
    let _ = root.append_child(&el);
    add(&root, "has-overlay");
    // reveal.js : la classe `visible` une image plus tard, pour la transition d'ouverture
    dom::timeout(1.0, move || add(&el, "visible"));
}

pub fn install() {
    engine::intercept(|e| match e.key().as_str() {
        "?" => {
            open();
            true
        }
        "Escape" if overlay().is_some() => {
            close();
            true
        }
        _ => false,
    });
}
