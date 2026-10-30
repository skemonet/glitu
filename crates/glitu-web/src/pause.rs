//! Black screen (B, V, "."): the presentation fades behind a black veil, as in reveal.js
//! (`.reveal.paused` + `.pause-overlay`); the same keys bring it back.

use crate::dom::{self, has, toggle};
use crate::engine;

/// reveal.js keys: `:` `;` `b` `v` `.` `/`
const KEYS: [&str; 6] = [":", ";", "b", "v", ".", "/"];

fn toggle_pause() {
    let root = engine::root();
    let paused = !has(&root, "paused");
    toggle(&root, "paused", paused);
}

pub fn install() {
    let overlay = dom::create("div");
    overlay.set_class_name("pause-overlay");
    if engine::config().controls {
        overlay.set_inner_html("<button class=\"resume-button\">Reprendre</button>");
        dom::on::<web_sys::Event>(&overlay, "click", |_| toggle_pause());
    }
    let _ = engine::root().append_child(&overlay);
    // While paused, only the pause keys count: nothing moves behind the veil
    engine::intercept(|e| {
        let key = e.key().to_lowercase();
        if KEYS.contains(&key.as_str()) {
            toggle_pause();
            return true;
        }
        has(&engine::root(), "paused")
    });
}
