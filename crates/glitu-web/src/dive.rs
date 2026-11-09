//! Zoom into an element of a diagram, from one slide to the next (`transition="dive:api"` on the
//! detailed slide): the diagram of the previous slide grows around its element `api` (a skemo
//! card, any `data-id`), or around a zone `x y w h` in % of its picture, until the element fills
//! the stage; the diagram of the new slide comes out of it, fading in. The text of the panel fades
//! out, then the new one in; the rest of the slide does not move. `surface:api` plays it the other
//! way round (back to the general diagram), and so does going back with ← over a `dive`.
//!
//! The previous slide is hidden at once: its panel and stage contents are copied, at their place,
//! into the new slide for the duration of the movement. Both diagrams zoom about the same fixed
//! point, by geometric steps: the new one stays exactly on the element of the old one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlMediaElement};

use crate::dom::{self, add, data, has, qs, remove, style, unstyle};
use crate::engine::{self, Ev};
use crate::focus;
use crate::motion::{self, Rect};
use crate::overview;

/// Duration of the zoom (ms)
const DURATION: f64 = 1100.0;
/// What fades instead of zooming: the captions of the stage
const TEXT: &str = ".caption, .visual-caption";
/// The properties set during the movement
const PROPS: [&str; 4] = ["transform", "transform-origin", "opacity", "clip-path"];

struct Anim {
    /// Zoomed in: the old diagram grows (`dive`); otherwise it shrinks back (`surface`)
    zoom_in: bool,
    /// The general diagram and the detailed one, each element with its box before any transform
    outer: Vec<(Element, Rect)>,
    inner: Vec<(Element, Rect)>,
    /// The texts that leave and those that come
    old_text: Vec<Element>,
    new_text: Vec<Element>,
    /// Copies of the previous slide, removed at the end
    ghosts: Vec<Element>,
    /// Magnification that brings the element to the stage, fixed point of the zoom (window coordinates)
    k: f64,
    fixed: (f64, f64),
    /// What the diagrams are cut to: the visible part of the stage, beside the panel and along its
    /// slanted edge; its corners clockwise from the top left (window coordinates)
    clip: [(f64, f64); 4],
    /// Window px per px of the slide
    scale: f64,
    start: f64,
    frame: Cell<i32>,
}

thread_local! {
    static RUN: RefCell<Option<Rc<Anim>>> = const { RefCell::new(None) };
}

fn ease(t: f64) -> f64 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// 0 before `a`, 1 after `a + d`, smooth in between
fn ramp(x: f64, a: f64, d: f64) -> f64 {
    let t = ((x - a) / d).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A fragment still hidden keeps its own opacity
fn hidden(el: &Element) -> bool {
    has(el, "fragment") && !has(el, "visible")
}

/// Ends the movement at once: the copies go, the elements get their place back
fn finish() {
    let Some(run) = RUN.with(|r| r.borrow_mut().take()) else { return };
    dom::cancel_raf(run.frame.get());
    for g in &run.ghosts {
        g.remove();
    }
    let all = run.outer.iter().chain(&run.inner).map(|(e, _)| e).chain(&run.old_text).chain(&run.new_text);
    for el in all {
        for p in PROPS {
            unstyle(el, p);
        }
    }
}

impl Anim {
    /// An element zoomed by `s` about the fixed point, cut to the area of the stage
    fn zoom(&self, el: &Element, at: Rect, s: f64) {
        let (fx, fy) = ((self.fixed.0 - at.x) / self.scale, (self.fixed.1 - at.y) / self.scale);
        style(el, "transform-origin", &format!("{fx:.2}px {fy:.2}px"));
        style(el, "transform", &format!("scale({s:.5})"));
        // The corners of the area, in the coordinates of the element before its transform
        let local = |x: f64, y: f64| {
            let (lx, ly) = ((x - at.x) / self.scale, (y - at.y) / self.scale);
            (fx + (lx - fx) / s, fy + (ly - fy) / s)
        };
        let points: Vec<String> =
            self.clip.iter().map(|&(x, y)| local(x, y)).map(|(x, y)| format!("{x:.1}px {y:.1}px")).collect();
        style(el, "clip-path", &format!("polygon({})", points.join(", ")));
    }

    /// The state at time `t` (0 to 1)
    fn apply(&self, t: f64) {
        // e: how far the view is in the element, 0 on the general diagram, 1 on the detailed one
        let e = if self.zoom_in { ease(t) } else { 1.0 - ease(t) };
        let fade = |els: &[(Element, Rect)], s: f64, o: f64| {
            for (el, at) in els {
                self.zoom(el, *at, s);
                if !hidden(el) {
                    style(el, "opacity", &format!("{o:.3}"));
                }
            }
        };
        fade(&self.outer, self.k.powf(e), 1.0 - ramp(e, 0.55, 0.4));
        fade(&self.inner, self.k.powf(e - 1.0), ramp(e, 0.3, 0.45));
        for el in &self.old_text {
            style(el, "opacity", &format!("{:.3}", 1.0 - ramp(t, 0.0, 0.4)));
        }
        for el in self.new_text.iter().filter(|e| !hidden(e)) {
            style(el, "opacity", &format!("{:.3}", ramp(t, 0.6, 0.4)));
        }
    }
}

fn frame(run: Rc<Anim>) {
    let next = run.clone();
    let id = dom::raf(move |now| {
        if !RUN.with(|r| r.borrow().as_ref().is_some_and(|c| Rc::ptr_eq(c, &next))) {
            return;
        }
        let t = ((now - next.start) / DURATION).clamp(0.0, 1.0);
        next.apply(t);
        if t < 1.0 { frame(next) } else { finish() }
    });
    run.frame.set(id);
}

/// `(zoom in?, target)` of a slide entered forwards
fn way(slide: &Element) -> Option<(bool, String)> {
    data(slide, "dive").map(|t| (true, t)).or_else(|| data(slide, "surface").map(|t| (false, t)))
}

/// Window px per px of the slide (the slides are scaled, and a past slide may be transformed)
fn scale_of(el: &Element) -> f64 {
    let w = f64::from(dom::html(el).offset_width());
    if w > 0.0 { Rect::of(el).w / w } else { 1.0 }
}

fn px(el: &Element, prop: &str) -> f64 {
    dom::window()
        .get_computed_style(el)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value(prop).ok())
        .and_then(|v| v.trim_end_matches("px").parse().ok())
        .unwrap_or(0.0)
}

/// The horizontal offset of the foot of the panel's edge (`--slant`, a `calc()` the computed
/// style leaves unresolved): the width of a probe that takes it
fn slant(panel: &Element) -> f64 {
    let probe = dom::create("div");
    for (k, v) in [("position", "absolute"), ("visibility", "hidden"), ("height", "0"), ("width", "var(--slant, 0px)")]
    {
        style(&probe, k, v);
    }
    let _ = panel.append_child(&probe);
    let w = f64::from(dom::html(&probe).offset_width());
    probe.remove();
    w
}

/// The content box of the stage, inside its padding (window coordinates)
fn content_box(stage: &Element, scale: f64) -> Rect {
    let r = Rect::of(stage);
    let [t, rt, b, l] =
        ["padding-top", "padding-right", "padding-bottom", "padding-left"].map(|p| px(stage, p) * scale);
    Rect { x: r.x + l, y: r.y + t, w: (r.w - l - rt).max(1.0), h: (r.h - t - b).max(1.0) }
}

/// Copies the element children of `from` into `to`, at the same place (`from` and `to` are the
/// same container of two slides, laid out alike)
fn ghosts(from: &Element, to: &Element) -> Vec<Element> {
    let (fr, fs) = (Rect::of(from), scale_of(from));
    let children = from.children();
    let mut out = Vec::new();
    for i in 0..children.length() {
        let Some(c) = children.item(i) else { continue };
        let r = Rect::of(&c);
        let Ok(copy) = c.clone_node_with_deep(true) else { continue };
        let copy: Element = copy.unchecked_into();
        for (k, v) in
            [("left", (r.x - fr.x) / fs), ("top", (r.y - fr.y) / fs), ("width", r.w / fs), ("height", r.h / fs)]
        {
            style(&copy, k, &format!("{v:.2}px"));
        }
        for (k, v) in
            [("position", "absolute"), ("margin", "0"), ("box-sizing", "border-box"), ("pointer-events", "none")]
        {
            style(&copy, k, v);
        }
        let _ = copy.remove_attribute("id");
        for v in dom::qsa(&copy, "video") {
            let _ = v.remove_attribute("autoplay");
            if let Some(m) = v.dyn_ref::<HtmlMediaElement>() {
                let _ = m.pause();
            }
        }
        add(&copy, "dive-ghost");
        let _ = to.append_child(&copy);
        out.push(copy);
    }
    out
}

/// The picture of a visual (an image or a video less its bands, the drawing of an SVG), in window
/// coordinates: what a zone in % is measured on
fn picture(visual: &Element) -> Rect {
    let media = if visual.matches("img, video, svg").unwrap_or(false) {
        Some(visual.clone())
    } else {
        qs(visual, "img, video, svg")
    };
    let Some(media) = media else { return Rect::of(visual) };
    let r = Rect::of(&media);
    let size = focus::natural(&media).or_else(|| {
        let vb: Vec<f64> = media.get_attribute("viewBox")?.split([' ', ',']).filter_map(|v| v.parse().ok()).collect();
        (vb.len() == 4 && vb[2] > 0.0 && vb[3] > 0.0).then(|| (vb[2], vb[3]))
    });
    let Some((nw, nh)) = size else { return r };
    let k = (r.w / nw).min(r.h / nh);
    let (w, h) = (nw * k, nh * k);
    Rect { x: r.x + (r.w - w) / 2.0, y: r.y + (r.h - h) / 2.0, w, h }
}

/// The box of the target in the general diagram: a zone `x y w h` (% of its picture), or the
/// element of that `data-id`
fn target(visuals: &[(Element, Rect)], target: &str) -> Option<Rect> {
    let nums: Vec<f64> = target.split_whitespace().filter_map(|t| t.trim_end_matches('%').parse().ok()).collect();
    if nums.len() == 4 && target.split_whitespace().count() == 4 {
        let p = picture(&visuals.first()?.0);
        return Some(Rect {
            x: p.x + p.w * nums[0] / 100.0,
            y: p.y + p.h * nums[1] / 100.0,
            w: p.w * nums[2] / 100.0,
            h: p.h * nums[3] / 100.0,
        });
    }
    if target.contains(['"', '\\']) {
        return None;
    }
    let sel = format!("[data-id=\"{target}\"]");
    visuals
        .iter()
        .find_map(|(v, _)| if v.matches(&sel).unwrap_or(false) { Some(v.clone()) } else { qs(v, &sel) })
        .map(|e| Rect::of(&e))
}

fn start(previous: &Element, current: &Element, zoom_in: bool, aim: &str) {
    let (Some(old_stage), Some(new_stage)) = (qs(previous, ".stage"), qs(current, ".stage")) else { return };
    // No slide transition: the new slide takes the place of the old one at once
    let root = engine::root();
    add(&root, "auto-animating");
    dom::raf(move |_| {
        dom::raf(move |_| remove(&root, "auto-animating"));
    });
    let mut copies = ghosts(&old_stage, &new_stage);
    let (mut old_text, mut new_text) = (Vec::new(), Vec::new());
    if let (Some(old_panel), Some(new_panel)) = (qs(previous, ".panel"), qs(current, ".panel")) {
        let panel = ghosts(&old_panel, &new_panel);
        old_text.extend(panel.iter().cloned());
        let children = new_panel.children();
        new_text.extend((0..children.length()).filter_map(|i| children.item(i)).filter(|c| !has(c, "dive-ghost")));
        copies.extend(panel);
    }
    // The visuals of each slide, measured once the copies are in place
    let (mut old_visuals, mut new_visuals) = (Vec::new(), Vec::new());
    let children = new_stage.children();
    for el in (0..children.length()).filter_map(|i| children.item(i)) {
        let text = el.matches(TEXT).unwrap_or(false);
        match (has(&el, "dive-ghost"), text) {
            (true, true) => old_text.push(el),
            (false, true) => new_text.push(el),
            (true, false) => old_visuals.push(el),
            (false, false) => new_visuals.push(el),
        }
    }
    let measured = |els: Vec<Element>| {
        els.into_iter()
            .map(|e| {
                let r = Rect::of(&e);
                (e, r)
            })
            .collect::<Vec<_>>()
    };
    let (old_visuals, new_visuals) = (measured(old_visuals), measured(new_visuals));
    let (outer, inner) = if zoom_in { (old_visuals, new_visuals) } else { (new_visuals, old_visuals) };
    let scale = scale_of(current);
    let view = content_box(&new_stage, scale);
    let st = Rect::of(&new_stage);
    let (right, bottom) = (st.x + st.w, st.y + st.h);
    let mut clip = [(st.x, st.y), (right, st.y), (right, bottom), (st.x, bottom)];
    if let Some(panel) = qs(current, ".panel") {
        // The edge of the panel leans by `--slant` from its top to its bottom (see template.css)
        let p = Rect::of(&panel);
        let slant = slant(&panel) * scale;
        let (top, low) = ((p.x + p.w).clamp(st.x, right), (p.x + p.w - slant).clamp(st.x, right));
        let edge = |y: f64| top + (low - top) * ((y - p.y) / p.h.max(1.0));
        clip[0].0 = edge(st.y);
        clip[3].0 = edge(bottom);
    }
    let way = if zoom_in { "dive" } else { "surface" };
    let Some(t) = target(&outer, aim).filter(|t| t.w > 0.0 && t.h > 0.0) else {
        dom::warn(&format!("transition=\"{way}:{aim}\": no element `{aim}` in the general diagram"));
        for c in copies {
            c.remove();
        }
        return;
    };
    let k = (view.w / t.w).min(view.h / t.h);
    if (k - 1.0).abs() < 1e-3 {
        for c in copies {
            c.remove();
        }
        return;
    }
    // The fixed point of the zoom by k that brings the centre of the target to that of the view
    let fix = |c: f64, tc: f64| (c - k * tc) / (1.0 - k);
    let fixed = (fix(view.x + view.w / 2.0, t.x + t.w / 2.0), fix(view.y + view.h / 2.0, t.y + t.h / 2.0));
    let run = Rc::new(Anim {
        zoom_in,
        outer,
        inner,
        old_text,
        new_text,
        ghosts: copies,
        k,
        fixed,
        clip,
        scale,
        start: dom::now(),
        frame: Cell::new(0),
    });
    // The first state at once: nothing of the new slide shows before its place
    run.apply(0.0);
    RUN.with(|r| *r.borrow_mut() = Some(run.clone()));
    frame(run);
}

pub fn install() {
    engine::on(|ev| {
        let Ev::SlideChanged { previous: Some(previous) } = ev else { return };
        finish();
        // Not in the mosaic of the slides nor in the scroll view, where the slides are side by side
        if motion::reduced() || overview::active() || engine::scrolling() {
            return;
        }
        let current = engine::current_slide();
        let plan = if previous.next_element_sibling().as_ref() == Some(&current) {
            way(&current)
        } else if current.next_element_sibling().as_ref() == Some(previous) {
            // Back over the slide: the movement that brought it, reversed
            way(previous).map(|(zoom_in, t)| (!zoom_in, t))
        } else {
            None
        };
        if let Some((zoom_in, aim)) = plan {
            start(previous, &current, zoom_in, &aim);
        }
    });
}
