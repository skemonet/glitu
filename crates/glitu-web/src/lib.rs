//! glitu-web : le moteur de présentation, compilé en WebAssembly.
//!
//! La page (produite par `glitu build`) contient les slides en HTML ; `start` les anime :
//! navigation et étapes (engine), thème (themes), schémas skemo (skemo), terminal rejoué
//! (terminal), code animé (magic), slides animées (animate), vue de l'orateur (notes), télécommande (remote), dessin (draw), et ce que
//! reveal.js offre en présentant : vue d'ensemble, écran noir, aller à une slide, aide,
//! médias, défilement automatique, zoom, export PDF.

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

/// Options passées par la page : réglages du thème (glitu.json), son nom et son logo
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
    // Le thème d'abord : il pose le template avant que les composants ne lisent les slides
    if flag("template", false) {
        themes::template::install(&text("logo"), &text("backdrop"));
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
    // Les touches passent d'abord par le dernier inscrit : écran noir, puis aide, puis mosaïque
    overview::install();
    help::install();
    pause::install();
    // Le dessin lit les touches avant tous les autres : Échap le quitte sans ouvrir la mosaïque
    draw::install();
    remote::install();
    engine::ready();
}
