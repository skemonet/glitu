//! Automatic advance, as in reveal.js: `autoSlide` (glitu.json, in ms) for the whole
//! presentation, `data-autoslide` on a slide or a step for its own delay (0: wait for the
//! click). A pauses and resumes; the black screen and the overview stop it.

use std::cell::Cell;

use crate::dom::{self, data, has, qsa};
use crate::engine::{self, Ev};

thread_local! {
    static TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static STOPPED: Cell<bool> = const { Cell::new(false) };
}

/// Delay of the current step: that of the step, otherwise of the slide, otherwise of the presentation
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

/// Schedules the next step; at the end, stop (or loop with `loop`)
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
