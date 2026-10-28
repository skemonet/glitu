//! Petits raccourcis DOM.

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, Window};

/// Texte pour du HTML : `&`, `<`, `>` et `"` échappés
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn window() -> Window {
    web_sys::window().expect("window")
}

pub fn document() -> Document {
    window().document().expect("document")
}

pub fn qs(root: &Element, sel: &str) -> Option<Element> {
    root.query_selector(sel).ok().flatten()
}

pub fn qsa(root: &Element, sel: &str) -> Vec<Element> {
    let Ok(list) = root.query_selector_all(sel) else { return Vec::new() };
    (0..list.length()).filter_map(|i| list.item(i)?.dyn_into::<Element>().ok()).collect()
}

pub fn create(tag: &str) -> Element {
    document().create_element(tag).expect("create_element")
}

pub fn html(el: &Element) -> HtmlElement {
    el.clone().unchecked_into()
}

pub fn style(el: &Element, prop: &str, value: &str) {
    let _ = html(el).style().set_property(prop, value);
}

pub fn unstyle(el: &Element, prop: &str) {
    let _ = html(el).style().remove_property(prop);
}

pub fn data(el: &Element, name: &str) -> Option<String> {
    el.get_attribute(&format!("data-{name}"))
}

pub fn has(el: &Element, class: &str) -> bool {
    el.class_list().contains(class)
}

pub fn toggle(el: &Element, class: &str, on: bool) {
    let _ = el.class_list().toggle_with_force(class, on);
}

pub fn add(el: &Element, class: &str) {
    let _ = el.class_list().add_1(class);
}

pub fn remove(el: &Element, class: &str) {
    let _ = el.class_list().remove_1(class);
}

pub fn on<E: JsCast + 'static>(target: &web_sys::EventTarget, event: &str, f: impl FnMut(E) + 'static) {
    let mut f = f;
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| f(e.unchecked_into()));
    let _ = target.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Comme `on`, mais l'écouteur peut annuler l'action du navigateur (`prevent_default`) : les
/// navigateurs rendent passifs par défaut ceux du toucher posés sur le document
pub fn on_active<E: JsCast + 'static>(target: &web_sys::EventTarget, event: &str, f: impl FnMut(E) + 'static) {
    let mut f = f;
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| f(e.unchecked_into()));
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(false);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(event, cb.as_ref().unchecked_ref(), &options);
    cb.forget();
}

pub fn raf(f: impl FnOnce(f64) + 'static) -> i32 {
    let cb = Closure::once_into_js(f);
    window().request_animation_frame(cb.unchecked_ref()).unwrap_or(0)
}

pub fn cancel_raf(id: i32) {
    let _ = window().cancel_animation_frame(id);
}

pub fn timeout(ms: f64, f: impl FnOnce() + 'static) -> i32 {
    let cb = Closure::once_into_js(f);
    window().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms.max(0.0) as i32).unwrap_or(0)
}

pub fn clear_timeout(id: i32) {
    window().clear_timeout_with_handle(id);
}

pub fn now() -> f64 {
    window().performance().map_or(0.0, |p| p.now())
}

pub fn closest(el: &Element, sel: &str) -> Option<Element> {
    el.closest(sel).ok().flatten()
}

pub fn warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}
