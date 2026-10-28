//! Défilement automatique, comme reveal.js : `autoSlide` (glitu.json, en ms) pour toute la
//! présentation, `data-autoslide` sur une slide ou une étape pour son propre délai (0 : on
//! attend le clic). A suspend et reprend ; l'écran noir et la vue d'ensemble l'arrêtent.

use std::cell::Cell;

use crate::dom::{self, data, has, qsa};
use crate::engine::{self, Ev};

thread_local! {
    static TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static STOPPED: Cell<bool> = const { Cell::new(false) };
}

/// Délai de l'étape courante : celui de l'étape, sinon de la slide, sinon de la présentation
fn delay() -> f64 {
    let slide = engine::current_slide();
    let step = qsa(&slide, ".fragment.current-fragment[data-autoslide]").into_iter().next();
    step.or(Some(slide))
        .and_then(|el| data(&el, "autoslide"))
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| engine::config().auto_slide)
}

fn cancel() {
    if let Some(id) = TIMER.with(std::cell::Cell::take) {
        dom::clear_timeout(id);
    }
}

fn halted() -> bool {
    let root = engine::root();
    STOPPED.with(Cell::get) || has(&root, "paused") || has(&root, "overview")
}

/// Programme l'étape suivante ; à la fin, on s'arrête (ou on reboucle avec `loop`)
fn cue() {
    cancel();
    let ms = delay();
    if ms <= 0.0 || halted() || (engine::at_end() && !engine::config().looped) {
        return;
    }
    let id = dom::timeout(ms, || {
        TIMER.with(|t| t.set(None));
        if !halted() {
            engine::next();
        }
    });
    TIMER.with(|t| t.set(Some(id)));
}

pub fn install() {
    engine::on_key("a", || {
        STOPPED.with(|s| s.set(!s.get()));
        cue();
    });
    engine::on(|ev| {
        if !matches!(ev, Ev::Resize) {
            cue();
        }
    });
}
