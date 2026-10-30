//! glitu-web: the presentation engine, compiled to WebAssembly.
//!
//! The page (produced by `glitu build`) holds the slides in HTML; `start` animates them:
//! navigation and steps (engine), theme (themes), skemo diagrams (skemo), replayed terminal
//! (terminal), animated code (magic), animated slides (animate), speaker view (notes), remote (remote), drawing (draw), and what
//! reveal.js offers when presenting: overview, black screen, go to a slide, help, media,
//! automatic advance, zoom, PDF export.

mod animate;
mod attributes;
mod autoslide;
mod chrome;
mod code;
mod dom;
mod draw;
mod engine;
mod fx;
mod help;
mod jump;
mod magic;
mod media;
mod motion;
mod notes;
mod overview;
mod pause;
mod print;
mod remote;
mod skemo;
mod terminal;
mod themes;
mod zoom;

use wasm_bindgen::prelude::*;

/// Options passed by the page: settings of the theme (glitu.json), its name and its logo
#[wasm_bindgen]
pub fn start(options: JsValue) {
    let get = |k: &str| js_sys::Reflect::get(&options, &JsValue::from_str(k)).ok().filter(|v| !v.is_undefined());
    let text = |k: &str| get(k).and_then(|v| v.as_string()).unwrap_or_default();
    let num = |k: &str, d: f64| get(k).and_then(|v| v.as_f64()).unwrap_or(d);
    let flag = |k: &str, d: bool| get(k).and_then(|v| v.as_bool()).unwrap_or(d);
    let d = engine::Config::default();
    let config = engine::Config {
        width: num("width", d.width),
        height: num("height", d.height),
        margin: num("margin", d.margin),
        center: flag("center", d.center),
        controls: flag("controls", d.controls),
        progress: flag("progress", d.progress),
        slide_number: flag("slideNumber", d.slide_number),
        transition: get("transition").and_then(|v| v.as_string()).unwrap_or(d.transition),
        looped: flag("loop", d.looped),
        mouse_wheel: flag("mouseWheel", d.mouse_wheel),
        auto_slide: num("autoSlide", d.auto_slide),
        min_scale: num("minScale", d.min_scale),
        max_scale: num("maxScale", d.max_scale),
    };
    let theme = text("theme");

    engine::init(config);
    // The theme first: it lays the template before the components read the slides
    if flag("template", false) {
        themes::template::install(&text("logo"), &text("backdrop"));
    } else if !text("logo").is_empty() {
        chrome::logo(&text("logo"));
    }
    chrome::install();
    print::install();
    code::install();
    magic::install();
    animate::install();
    skemo::install();
    notes::install();
    terminal::install(&theme);
    media::install();
    autoslide::install();
    jump::install();
    zoom::install();
    fx::install();
    // Keys go through the last registered first: black screen, then help, then mosaic
    overview::install();
    help::install();
    pause::install();
    // Drawing reads the keys before all the others: Esc leaves it without opening the mosaic
    draw::install();
    remote::install();
    engine::ready();
}
