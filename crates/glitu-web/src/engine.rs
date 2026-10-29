//! The presentation engine: slides (horizontal and vertical), steps, scaling, address,
//! keyboard.
//!
//! Same model as reveal.js: one slide `present`, the previous ones `past` (all their steps
//! visible), the next ones `future` (steps hidden); a stack of vertical slides is a
//! `section.stack`. The `.fragment`s of a slide are grouped by `data-fragment-index`, then
//! numbered from 0; the index written by the author stays readable in `data-sk-index`: it is the
//! "click" that diagrams follow.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use web_sys::{Element, KeyboardEvent};

use crate::dom::{self, add, data, has, qsa, remove, style, toggle};

/// Settings of the theme (glitu.json), passed by the page
#[derive(Clone)]
pub struct Config {
    pub width: f64,
    pub height: f64,
    pub margin: f64,
    pub center: bool,
    pub controls: bool,
    pub progress: bool,
    pub slide_number: bool,
    pub transition: String,
    /// After the last slide, go back to the first one
    pub looped: bool,
    /// The mouse wheel moves forward and back
    pub mouse_wheel: bool,
    /// Automatic advance: delay between two steps, in ms (0: none)
    pub auto_slide: f64,
    /// Bounds of the scaling in the window (as reveal.js: 0.2 and 2); a vector theme can go higher,
    /// to fill a large screen
    pub min_scale: f64,
    pub max_scale: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            width: 960.0,
            height: 700.0,
            margin: 0.04,
            center: true,
            controls: true,
            progress: true,
            slide_number: false,
            transition: "slide".into(),
            looped: false,
            mouse_wheel: false,
            auto_slide: 0.0,
            min_scale: 0.2,
            max_scale: 2.0,
        }
    }
}

/// Neighbouring slides kept displayed on each side of the current slide
pub const VIEW_DISTANCE: usize = 3;
/// Window width (px) under which the presentation is read by scrolling, as in reveal.js
const SCROLL_BELOW: f64 = 435.0;

pub enum Ev<'a> {
    Ready,
    SlideChanged { previous: Option<&'a Element> },
    FragmentShown(&'a [Element]),
    FragmentHidden(&'a [Element]),
    Resize,
}

type Listener = Rc<dyn Fn(&Ev)>;
type KeyHandler = Rc<dyn Fn()>;
/// Reads a key before the engine; `true`: the key is taken (overview, pause…)
type Interceptor = Rc<dyn Fn(&KeyboardEvent) -> bool>;

struct State {
    config: Config,
    /// Horizontal slides (a vertical stack is a single entry)
    slides: Vec<Element>,
    /// For each horizontal slide: its vertical slides (itself if it is not a stack)
    leaves: Vec<Vec<Element>>,
    /// For each slide (h, v): groups of fragments in click order
    groups: Vec<Vec<Vec<Vec<Element>>>>,
    h: usize,
    v: usize,
    /// Current group of the current slide (-1: none)
    f: i32,
    navigated_h: bool,
    navigated_v: bool,
    listeners: Vec<Listener>,
    keys: Vec<(String, KeyHandler)>,
    interceptors: Vec<Interceptor>,
    /// Transform added to the scaling of the slides (overview)
    extra_transform: String,
    /// `?print-pdf`: all the slides one under the other, for printing
    /// The slides as pages, all steps shown: printing, or scroll view
    printing: bool,
    /// … to be read by scrolling (see `print`)
    scrolling: bool,
    /// View chosen from the width of the window (neither `?view=scroll`, nor printing, nor
    /// remote): it changes when the window crosses `SCROLL_BELOW`
    by_width: bool,
    /// `?receiver`: preview in the speaker view, driven by messages, without keyboard
    receiver: bool,
    view_distance: usize,
    /// Overview: all the slides displayed, each one complete (steps included)
    overview: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(s.borrow_mut().as_mut().expect("engine not started")))
}

pub fn root() -> Element {
    dom::qs(&dom::document().document_element().unwrap(), ".reveal").expect(".reveal")
}

pub fn slides_element() -> Element {
    dom::qs(&root(), ".slides").expect(".slides")
}

pub fn config() -> Config {
    with(|s| s.config.clone())
}

/// Prepares the slides; the plugins subscribe next, then `ready()`.
pub fn init(config: Config) {
    // Attributes written in comments first: they can hide a slide
    for leaf in qsa(&slides_element(), ":scope > section").iter().flat_map(leaves_of) {
        crate::attributes::apply(&leaf);
    }
    // Hidden slides (`data-visibility="hidden"`): removed, as in reveal.js
    for hidden in qsa(&slides_element(), "section[data-visibility=\"hidden\"]") {
        hidden.remove();
    }
    let search = dom::window().location().search().unwrap_or_default();
    let receiver = search.contains("receiver");
    // Scroll view: asked for (`?view=scroll`), or by default on a phone screen, except for the
    // remote
    let by_width = !search.contains("view=scroll") && !receiver && !search.contains("remote") && !search.contains("print-pdf");
    let scrolling = search.contains("view=scroll") || (by_width && narrow());
    let printing = search.contains("print-pdf") || scrolling;
    let slides = qsa(&slides_element(), ":scope > section");
    let leaves: Vec<Vec<Element>> = slides.iter().map(leaves_of).collect();
    for (s, l) in slides.iter().zip(&leaves) {
        if l.first() != Some(s) {
            add(s, "stack");
        }
    }
    let groups = leaves.iter().map(|l| l.iter().map(sort_fragments).collect()).collect();
    let r = root();
    add(&r, &config.transition);
    if config.center {
        add(&r, "center");
    }
    add(&r, "has-horizontal-slides");
    if leaves.iter().any(|l| l.len() > 1) {
        add(&r, "has-vertical-slides");
    }
    let _ = r.set_attribute("data-transition-speed", "default");
    let _ = r.set_attribute("data-background-transition", "fade");
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            config,
            slides,
            leaves,
            groups,
            h: 0,
            v: 0,
            f: -1,
            navigated_h: false,
            navigated_v: false,
            listeners: Vec::new(),
            keys: Vec::new(),
            interceptors: Vec::new(),
            extra_transform: String::new(),
            printing,
            scrolling,
            by_width,
            receiver,
            view_distance: VIEW_DISTANCE,
            overview: false,
        });
    });
}

/// Phone window (in portrait): the scroll view suits it better
fn narrow() -> bool {
    dom::window().inner_width().ok().and_then(|w| w.as_f64()).is_some_and(|w| w < SCROLL_BELOW)
}

/// Slides of a horizontal slide: its vertical slides, or itself
fn leaves_of(s: &Element) -> Vec<Element> {
    let inner = qsa(s, ":scope > section");
    if inner.is_empty() { vec![s.clone()] } else { inner }
}

/// Groups of fragments of a slide, as in reveal.js: by written index, then those without an
/// index in document order; then renumbered from 0.
fn sort_fragments(slide: &Element) -> Vec<Vec<Element>> {
    let mut ordered: Vec<(u32, Vec<Element>)> = Vec::new();
    let mut unordered: Vec<Vec<Element>> = Vec::new();
    for f in qsa(slide, ".fragment") {
        match data(&f, "fragment-index").and_then(|i| i.parse::<u32>().ok()) {
            Some(i) => {
                let _ = f.set_attribute("data-sk-index", &i.to_string());
                match ordered.iter_mut().find(|(k, _)| *k == i) {
                    Some((_, g)) => g.push(f),
                    None => ordered.push((i, vec![f])),
                }
            }
            None => unordered.push(vec![f]),
        }
    }
    ordered.sort_by_key(|(k, _)| *k);
    let groups: Vec<Vec<Element>> = ordered.into_iter().map(|(_, g)| g).chain(unordered).collect();
    for (i, g) in groups.iter().enumerate() {
        for f in g {
            let _ = f.set_attribute("data-fragment-index", &i.to_string());
        }
    }
    groups
}

pub fn on(f: impl Fn(&Ev) + 'static) {
    with(|s| s.listeners.push(Rc::new(f)));
}

/// Keyboard shortcut of a plugin (key as in `KeyboardEvent.key`, case-insensitive)
pub fn on_key(key: &str, f: impl Fn() + 'static) {
    with(|s| s.keys.push((key.to_lowercase(), Rc::new(f))));
}

/// A plugin reads the keys before the engine, the last registered first
pub fn intercept(f: impl Fn(&KeyboardEvent) -> bool + 'static) {
    with(|s| s.interceptors.push(Rc::new(f)));
}

/// Print mode (`?print-pdf`)
pub fn scrolling() -> bool {
    with(|s| s.scrolling)
}

pub fn printing() -> bool {
    with(|s| s.printing)
}

/// Preview of the speaker view (`?receiver`)
pub fn receiver() -> bool {
    with(|s| s.receiver)
}

/// The "Next" preview of the speaker view (`?receiver&upcoming`): it shows the final state of
/// the next step, without its animations
pub fn upcoming() -> bool {
    receiver() && dom::window().location().search().unwrap_or_default().contains("upcoming")
}

/// Transform added to the scaling of the slides; empty: none
pub fn set_extra_transform(t: &str) {
    with(|s| s.extra_transform = t.to_string());
    layout();
}

/// Overview: all the slides displayed, whatever their distance (a neighbouring chapter can be
/// long), each one in its final state as when printing; on leaving, the current slide gets its
/// steps back
pub fn set_overview(on: bool) {
    with(|s| {
        s.overview = on;
        s.view_distance = if on { usize::MAX } else { VIEW_DISTANCE };
    });
    let (h, v, f) = position();
    show(h, v, Some(f), false);
    // On leaving, the chosen slide is announced as if arriving at it: diagrams, terminal and
    // effects take back its steps (in the overview, they showed it complete)
    if !on {
        emit(Ev::SlideChanged { previous: None });
    }
}

/// Rank of the current slide in reading order (vertical slides included)
pub fn leaf_index() -> usize {
    with(|s| s.leaves[..s.h].iter().map(Vec::len).sum::<usize>() + s.v)
}

/// Slides (h, v) in reading order
pub fn all_leaves() -> Vec<(usize, usize, Element)> {
    with(|s| {
        s.leaves.iter().enumerate().flat_map(|(h, l)| l.iter().enumerate().map(move |(v, e)| (h, v, e.clone()))).collect()
    })
}


fn emit(ev: Ev) {
    let listeners = with(|s| s.listeners.clone());
    for l in listeners {
        l(&ev);
    }
}

/// Horizontal slides
pub fn slides() -> Vec<Element> {
    with(|s| s.slides.clone())
}

/// Vertical slides of each horizontal slide
pub fn leaves() -> Vec<Vec<Element>> {
    with(|s| s.leaves.clone())
}

/// (h, v, f)
pub fn position() -> (usize, usize, i32) {
    with(|s| (s.h, s.v, s.f))
}

pub fn current_slide() -> Element {
    with(|s| s.leaves[s.h][s.v].clone())
}

/// Has the author already navigated horizontally, vertically?
pub fn navigated() -> (bool, bool) {
    with(|s| (s.navigated_h, s.navigated_v))
}

/// Current click of the slide, as written by the author (0: none)
pub fn current_step(slide: &Element) -> u32 {
    qsa(slide, ".fragment.visible[data-sk-index]")
        .iter()
        .filter_map(|f| data(f, "sk-index")?.parse().ok())
        .max()
        .unwrap_or(0)
}

/// Starts the presentation: position read from the address, then the `Ready` event.
pub fn ready() {
    layout();
    let (h, v, f) = read_hash();
    show(h, v, f, false);
    for c in data(&current_slide(), "state").unwrap_or_default().split_whitespace() {
        add(&dom::document().document_element().unwrap(), c);
    }
    add(&root(), "ready");
    emit(Ev::Ready);
    bind();
}

fn read_hash() -> (usize, usize, Option<i32>) {
    let hash = dom::window().location().hash().unwrap_or_default();
    let parts: Vec<&str> = hash.trim_start_matches('#').trim_start_matches('/').split('/').collect();
    let h = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
    let v = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let f = parts.get(2).and_then(|p| p.parse().ok());
    let n = with(|s| s.slides.len());
    (h.min(n.saturating_sub(1)), v, f)
}

fn write_hash() {
    let (h, v, f) = position();
    let hash = if f >= 0 {
        format!("#/{h}/{v}/{f}")
    } else if v > 0 {
        format!("#/{h}/{v}")
    } else if h > 0 {
        format!("#/{h}")
    } else {
        "#/".into()
    };
    let _ = dom::window().history().and_then(|hist| hist.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&hash)));
    // For the page: `glitu serve` keeps the open windows in sync this way; not during the
    // overview, the other windows only follow the chosen slide, on leaving
    if with(|s| s.overview) {
        return;
    }
    if let Ok(ev) = web_sys::Event::new("glitu:position") {
        let _ = dom::window().dispatch_event(&ev);
    }
}

fn set_state(el: &Element, state: &str) {
    for c in ["past", "present", "future"] {
        remove(el, c);
    }
    add(el, state);
}

fn state_of(i: usize, current: usize) -> &'static str {
    if i < current {
        "past"
    } else if i > current {
        "future"
    } else {
        "present"
    }
}

/// Distance from a slide (x, y) to the current slide, as in reveal.js
pub fn distance(x: usize, y: usize, h: usize, v: usize) -> usize {
    x.abs_diff(h) + if x == h { y.abs_diff(v) } else { y }
}

/// Shows the slide (h, v), its steps up to group `f` (None: depending on the direction of arrival)
pub fn show(h: usize, v: usize, f: Option<i32>, events: bool) {
    let (previous, slides, leaves, prev, prev_f) = with(|s| (s.leaves[s.h][s.v].clone(), s.slides.clone(), s.leaves.clone(), (s.h, s.v), s.f));
    let view = with(|s| s.view_distance);
    let h = h.min(slides.len().saturating_sub(1));
    let v = v.min(leaves[h].len() - 1);
    for (x, slide) in slides.iter().enumerate() {
        set_state(slide, state_of(x, h));
        let _ = slide.set_attribute("aria-hidden", if x == h { "false" } else { "true" });
        style(slide, "display", if x.abs_diff(h) < view { "block" } else { "none" });
        let stack = leaves[x].len() > 1;
        for (y, leaf) in leaves[x].iter().enumerate() {
            if stack {
                if x == h {
                    set_state(leaf, state_of(y, v));
                }
                style(leaf, "display", if distance(x, y, h, v) < view { "block" } else { "none" });
            }
            // Steps: all visible before the current slide, hidden after
            if (x, y) < (h, v) {
                set_fragments(x, y, i32::MAX);
            } else if (x, y) > (h, v) {
                set_fragments(x, y, -1);
            }
        }
    }
    let count = with(|s| s.groups[h][v].len() as i32);
    let f = match f {
        Some(f) => f.clamp(-1, count - 1),
        None if (h, v) < prev && events => count - 1,
        None => -1,
    };
    with(|s| {
        s.h = h;
        s.v = v;
        s.f = f;
    });
    set_fragments(h, v, f);
    toggle(&root(), "is-vertical-slide", v > 0);
    layout_centers();
    write_hash();
    if (h, v) != prev {
        mark_entry(&leaves, &previous, &leaves[h][v]);
        apply_state(&previous, &leaves[h][v]);
    }
    if events && (h, v) != prev {
        emit(Ev::SlideChanged { previous: Some(&previous) });
    } else if events && f != prev_f {
        // same slide, another step (synchronised window, address): as so many clicks, one group at a
        // time (terminal, animated code, diagrams)
        let groups = with(|s| s.groups[h][v].clone());
        if f > prev_f {
            ((prev_f + 1)..=f).for_each(|i| emit(Ev::FragmentShown(&groups[i as usize])));
        } else {
            ((f + 1)..=prev_f).rev().for_each(|i| emit(Ev::FragmentHidden(&groups[i as usize])));
        }
    }
}

/// `data-state="a b"` of the current slide: classes of the page, as in reveal.js
fn apply_state(previous: &Element, current: &Element) {
    let html = dom::document().document_element().unwrap();
    for c in data(previous, "state").unwrap_or_default().split_whitespace() {
        remove(&html, c);
    }
    for c in data(current, "state").unwrap_or_default().split_whitespace() {
        add(&html, c);
    }
}

/// The incoming slide keeps the layout of the one it replaces (`data-layout` set by the
/// compiler): the theme chooses its transition from the pair, in CSS
fn mark_entry(leaves: &[Vec<Element>], previous: &Element, current: &Element) {
    for leaf in leaves.iter().flatten() {
        let _ = leaf.remove_attribute("data-from-layout");
    }
    if let Some(layout) = data(previous, "layout") {
        let _ = current.set_attribute("data-from-layout", &layout);
    }
}

/// Visible steps of a slide: groups 0..=f; group f is the "current" one
fn set_fragments(h: usize, v: usize, f: i32) {
    // When printing and in the overview, each slide shows itself complete
    let f = if printing() || with(|s| s.overview) { i32::MAX } else { f };
    let groups = with(|s| s.groups[h][v].clone());
    for (i, g) in groups.iter().enumerate() {
        for el in g {
            toggle(el, "visible", (i as i32) <= f);
            toggle(el, "current-fragment", i as i32 == f && f != i32::MAX);
        }
    }
}

/// Next fragment of the current slide; false if there is none left
fn fragment_next() -> bool {
    step_fragments(1)
}

/// Previous fragment; false if the slide is at its beginning
fn fragment_prev() -> bool {
    step_fragments(-1)
}

/// Moves forward (`by` = 1) or back (-1) by one group of fragments in the current slide, and
/// announces the group shown or hidden; false at the end
fn step_fragments(by: i32) -> bool {
    let (h, v, f) = position();
    let count = with(|s| s.groups[h][v].len() as i32);
    let target = f + by;
    if target < -1 || target >= count {
        return false;
    }
    // The group that changes: the one that appears going forward, the one that leaves going back
    let changed = with(|s| s.groups[h][v][f.max(target) as usize].clone());
    with(|s| s.f = target);
    set_fragments(h, v, target);
    write_hash();
    emit(if by > 0 { Ev::FragmentShown(&changed) } else { Ev::FragmentHidden(&changed) });
    true
}

/// Available routes: left, right, up, down
pub fn routes() -> (bool, bool, bool, bool) {
    with(|s| (s.h > 0, s.h + 1 < s.slides.len(), s.v > 0, s.v + 1 < s.leaves[s.h].len()))
}

/// Steps left on each side in the current slide: previous, next
pub fn fragment_routes() -> (bool, bool) {
    with(|s| (s.f >= 0, s.f + 1 < s.groups[s.h][s.v].len() as i32))
}

pub fn right() {
    with(|s| s.navigated_h = true);
    if fragment_next() {
        return;
    }
    if routes().1 {
        show(position().0 + 1, 0, None, true);
    } else if config().looped {
        goto(0);
    }
}

pub fn left() {
    with(|s| s.navigated_h = true);
    if !fragment_prev() && routes().0 {
        show(position().0 - 1, 0, None, true);
    }
}

pub fn down() {
    with(|s| s.navigated_v = true);
    if !fragment_next() && routes().3 {
        let (h, v, _) = position();
        show(h, v + 1, None, true);
    }
}

pub fn up() {
    with(|s| s.navigated_v = true);
    if !fragment_prev() && routes().2 {
        let (h, v, _) = position();
        show(h, v - 1, None, true);
    }
}

/// Top of the next column (the next chapter in 2D navigation), without its steps
pub fn next_column() {
    with(|s| s.navigated_h = true);
    if routes().1 {
        show(position().0 + 1, 0, None, true);
    }
}

/// Top of the previous column (the previous chapter in 2D navigation)
pub fn previous_column() {
    with(|s| s.navigated_h = true);
    if routes().0 {
        show(position().0 - 1, 0, None, true);
    }
}

/// Next step, vertical slides included
pub fn next() {
    if fragment_next() {
        return;
    }
    if routes().3 { down() } else { right() }
}

/// Previous step; coming back to a stack, arrive at the bottom of the stack
pub fn prev() {
    if fragment_prev() {
        return;
    }
    let (left_ok, _, up_ok, _) = routes();
    if up_ok {
        up();
    } else if left_ok {
        with(|s| s.navigated_h = true);
        let h = position().0 - 1;
        let bottom = with(|s| s.leaves[h].len() - 1);
        show(h, bottom, None, true);
    }
}

pub fn goto(h: usize) {
    show(h, 0, None, true);
}

/// Last step of the last slide?
pub fn at_end() -> bool {
    let (_, right, _, down) = routes();
    !right && !down && !fragment_routes().1
}

/// Scaling in the window, centred (margin included)
pub fn layout() {
    let c = config();
    let reveal = dom::html(&root());
    let (w, h) = (reveal.offset_width() as f64, reveal.offset_height() as f64);
    let slides = slides_element();
    style(&slides, "width", &format!("{}px", c.width));
    style(&slides, "height", &format!("{}px", c.height));
    let (pw, ph) = (w - w * c.margin, h - h * c.margin);
    let scale = (pw / c.width).min(ph / c.height).clamp(c.min_scale, c.max_scale.max(c.min_scale));
    let extra = with(|s| s.extra_transform.clone());
    if printing() {
        return;
    }
    if extra.is_empty() && ((scale - 1.0).abs() < f64::EPSILON || w == 0.0) {
        for p in ["left", "top", "bottom", "right", "transform"] {
            dom::unstyle(&slides, p);
        }
    } else {
        style(&slides, "left", "50%");
        style(&slides, "top", "50%");
        style(&slides, "bottom", "auto");
        style(&slides, "right", "auto");
        style(&slides, "transform", format!("translate(-50%, -50%) scale({scale}) {extra}").trim_end());
    }
    if let Some(body) = dom::document().body() {
        let b: Element = body.into();
        style(&b, "--slide-width", &format!("{}px", c.width));
        style(&b, "--slide-height", &format!("{}px", c.height));
        style(&b, "--slide-scale", &scale.to_string());
        style(&b, "--viewport-width", &format!("{w}px"));
        style(&b, "--viewport-height", &format!("{h}px"));
    }
    layout_centers();
}

/// Slides centred vertically (stacks stay at the top)
fn layout_centers() {
    let c = config();
    for s in qsa(&slides_element(), "section") {
        if !c.center {
            dom::unstyle(&s, "top");
        } else if has(&s, "stack") {
            style(&s, "top", "0px");
        } else {
            let sh = dom::html(&s).scroll_height() as f64;
            style(&s, "top", &format!("{}px", ((c.height - sh) / 2.0).max(0.0)));
        }
    }
}

fn bind() {
    bind_window();
    if receiver() {
        receive();
        return;
    }
    bind_keys();
    // The scroll view is read by scrolling the page: the finger and the wheel stay with it
    if scrolling() {
        return;
    }
    if config().mouse_wheel {
        bind_wheel();
    }
    bind_touch();
}

/// Window size, fonts loaded, address changed by hand
fn bind_window() {
    let win = dom::window();
    dom::on::<web_sys::Event>(&win, "resize", |_| {
        // A phone that is turned goes from one view to the other; the scroll view lays the slides
        // out as pages: reload, the position is in the address
        if with(|s| s.by_width && s.scrolling != narrow()) {
            let _ = dom::window().location().reload();
            return;
        }
        layout();
        emit(Ev::Resize);
    });
    // Fonts change the height of the slides: centre again once everything is loaded
    dom::on::<web_sys::Event>(&win, "load", |_| layout());
    if let Ok(ready) = dom::document().fonts().ready() {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(wasm_bindgen::JsValue)>::new(|_| layout());
        let _ = ready.then(&cb);
        cb.forget();
    }
    dom::on::<web_sys::Event>(&win, "hashchange", |_| {
        let (h, v, f) = read_hash();
        if (h, v, f.unwrap_or(-1)) != position() {
            show(h, v, f, true);
        }
    });
}

/// Keyboard: plugins first (interceptions, shortcuts), then navigation
fn bind_keys() {
    dom::on::<KeyboardEvent>(&dom::document(), "keydown", |e| {
        if e.alt_key() || e.ctrl_key() || e.meta_key() {
            return;
        }
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok())
            && (matches!(t.tag_name().as_str(), "INPUT" | "TEXTAREA") || t.has_attribute("contenteditable"))
        {
            return;
        }
        let interceptors = with(|s| s.interceptors.clone());
        if interceptors.iter().rev().any(|f| f(&e)) {
            e.prevent_default();
            return;
        }
        let key = e.key();
        // Scroll view: the arrows, Space and the page keys scroll the page
        if scrolling() && matches!(key.as_str(), " " | "PageDown" | "PageUp" | "ArrowRight" | "ArrowLeft" | "ArrowDown" | "ArrowUp" | "Home" | "End") {
            return;
        }
        match key.as_str() {
            // the arrows follow reading order; Page↓ Page↑ jump from one column to another
            // (a chapter in 2D navigation); H J K L: left, down, up, right
            " " if e.shift_key() => prev(),
            "ArrowRight" | " " | "n" | "N" => next(),
            "ArrowLeft" | "p" | "P" => prev(),
            "PageDown" => next_column(),
            "PageUp" => previous_column(),
            "l" | "L" => right(),
            "h" | "H" => left(),
            "ArrowDown" | "j" | "J" => down(),
            "ArrowUp" | "k" | "K" => up(),
            "Home" => goto(0),
            "End" => goto(with(|s| s.slides.len()) - 1),
            "f" | "F" => fullscreen(),
            _ => {
                let handlers: Vec<KeyHandler> =
                    with(|s| s.keys.iter().filter(|(k, _)| *k == key.to_lowercase()).map(|(_, f)| f.clone()).collect());
                if handlers.is_empty() {
                    return;
                }
                for f in handlers {
                    f();
                }
            }
        }
        e.prevent_default();
    });
}

/// The wheel moves forward and back (`"mouseWheel": true`)
fn bind_wheel() {
    let last = Rc::new(RefCell::new(0.0));
    dom::on::<web_sys::WheelEvent>(&dom::document(), "wheel", move |e| {
        // A wheel sends a burst of events: one step every 600 ms at most
        let now = dom::now();
        // The overview reads the wheel in its own way
        if has(&root(), "overview") || now - *last.borrow() < 600.0 || e.delta_y() == 0.0 {
            return;
        }
        *last.borrow_mut() = now;
        if e.delta_y() > 0.0 { next() } else { prev() }
    });
}

/// Swipe with one finger: left / right, up / down. With two fingers, the gesture stays with the
/// browser (`touch-action: pinch-zoom` on `.reveal`): zoom, then move the view
fn bind_touch() {
    let start = Rc::new(RefCell::new(None::<(i32, i32)>));
    let s2 = Rc::clone(&start);
    dom::on::<web_sys::TouchEvent>(&dom::document(), "touchstart", move |e| {
        // In drawing mode, the finger draws (see `draw`)
        if has(&root(), "drawing") {
            return;
        }
        // A second finger cancels the swipe in progress, until the next gesture
        *s2.borrow_mut() = match (e.touches().length(), e.touches().get(0)) {
            (1, Some(t)) => Some((t.client_x(), t.client_y())),
            _ => None,
        };
    });
    // One finger does not move the view: touch-action (glitu.css) is not enough everywhere
    // (Firefox on Android, over diagrams)
    dom::on_active::<web_sys::TouchEvent>(&dom::document(), "touchmove", |e| {
        if e.touches().length() == 1 && !has(&root(), "drawing") {
            e.prevent_default();
        }
    });
    dom::on::<web_sys::TouchEvent>(&dom::document(), "touchend", move |e| {
        let Some((x0, y0)) = start.borrow_mut().take() else { return };
        let Some(t) = e.changed_touches().get(0) else { return };
        let (dx, dy) = (t.client_x() - x0, t.client_y() - y0);
        if dx.abs() > 40 && dx.abs() > dy.abs() {
            if dx < 0 { next() } else { prev() }
        } else if dy.abs() > 40 {
            if dy < 0 { down() } else { up() }
        }
    });
}

/// Preview of the speaker view: it goes where it is told (`{glitu: "show", h, v, f, ahead}`,
/// `ahead`: one step further), and tells its window that it is ready
fn receive() {
    let win = dom::window();
    dom::on::<web_sys::MessageEvent>(&win, "message", |e| {
        let msg = e.data();
        let get = |k: &str| js_sys::Reflect::get(&msg, &k.into()).ok();
        if get("glitu").and_then(|v| v.as_string()).as_deref() != Some("show") {
            return;
        }
        let num = |k: &str| get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
        show(num("h") as usize, num("v") as usize, Some(num("f") as i32), true);
        if get("ahead").and_then(|v| v.as_bool()) == Some(true) {
            next();
        }
    });
    if let Ok(Some(parent)) = win.parent() {
        let ready = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&ready, &"glitu".into(), &"ready".into());
        let _ = parent.post_message(&ready, "*");
    }
}

fn fullscreen() {
    let doc = dom::document();
    if doc.fullscreen_element().is_some() {
        doc.exit_fullscreen();
    } else if let Some(el) = doc.body() {
        let _ = el.request_fullscreen();
    }
}

/// Number of groups of steps of a slide, and how many are shown
pub fn fragment_counts(slide: &Element) -> (usize, usize) {
    let groups = with(|s| {
        s.leaves
            .iter()
            .enumerate()
            .find_map(|(h, l)| l.iter().position(|x| x == slide).map(|v| s.groups[h][v].clone()))
            .unwrap_or_default()
    });
    let shown = groups.iter().filter(|g| g.first().is_some_and(|f| has(f, "visible"))).count();
    (groups.len(), shown)
}
