//! Help (?): the list of shortcuts, in reveal.js's overlay (`.overlay.overlay-help`).
//! Esc, ? or a click on the cross close it.

use crate::dom::{self, add, qs, remove};
use crate::engine;

const SHORTCUTS: &[(&str, &str)] = &[
    ("→ Space N", "Next step or slide"),
    ("← Shift+Space P", "Previous step or slide"),
    ("Page↓ / Page↑", "Next / previous chapter (in 2D navigation; otherwise next / previous slide)"),
    ("↓ ↑", "Within the chapter: next / previous step or slide"),
    ("H J K L", "Left, down, up, right (vertical slides included)"),
    ("Home / End", "First / last slide"),
    ("G", "Go to a slide: number, 5.2 or identifier"),
    ("Esc / O", "Overview"),
    ("F", "Full screen"),
    ("S", "Speaker view: slide, next step, notes, timer"),
    ("D", "Draw on the slide (1 to 5: colour, Z: undo, X: clear, D: leave)"),
    ("B / V / .", "Black screen, then resume"),
    ("A", "Pause or resume the automatic advance"),
    ("Alt + click (Ctrl + click on Linux)", "Zoom on an element, then back"),
    ("?", "This help"),
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
         <div class=\"viewport\"><div class=\"viewport-inner\"><h1>Keyboard shortcuts</h1>\
         <table><tr><th>Key</th><th>Action</th></tr>{rows}</table></div></div>"
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
    // reveal.js: the `visible` class one frame later, for the opening transition
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
