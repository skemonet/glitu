//! Videos and focus zones. The zones of an image or a video (`focus="…"`, a `.zones` layer right
//! after it) are laid over the picture actually shown: the box of the element less the bands of
//! `object-fit: contain`, or cropped by `cover`. A video with `pauses="0:12 1:05"` plays when it
//! is shown, stops at each pause, and the next click resumes it; the zones of a video with a time
//! range (`0:12-0:18`) are shown while the video is within it, and zoom (`magnify`).

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement, HtmlImageElement, HtmlMediaElement, HtmlVideoElement};

use crate::dom::{self, data, has, qsa, style, toggle};
use crate::engine::{self, Ev};
use crate::zoom;

/// Maximum magnification of a zone of a video
const ZOOM_MAX: f64 = 4.0;
/// Margin of the times, in seconds: a video stopped at a pause is on the zones that end or start there
const EPSILON: f64 = 0.05;

thread_local! {
    /// Run of the frame loop: a change of slide ends the previous one
    static RUN: Cell<u32> = const { Cell::new(0) };
    /// The zone of a video the view is zoomed on
    static ZOOMED: RefCell<Option<Element>> = const { RefCell::new(None) };
}

/// The size of the picture of an image or a video, once it is known
pub fn natural(media: &Element) -> Option<(f64, f64)> {
    let (w, h) = if let Some(img) = media.dyn_ref::<HtmlImageElement>() {
        (img.natural_width(), img.natural_height())
    } else {
        let v = media.dyn_ref::<HtmlVideoElement>()?;
        (v.video_width(), v.video_height())
    };
    (w > 0 && h > 0).then_some((w as f64, h as f64))
}

/// Lays a `.zones` layer over the picture of the element before it
fn place(zones: &Element) {
    let Some(media) = zones.previous_element_sibling() else { return };
    let Some((nw, nh)) = natural(&media) else { return };
    let Some(el) = media.dyn_ref::<HtmlElement>() else { return };
    let (bw, bh) = (el.offset_width() as f64, el.offset_height() as f64);
    if bw == 0.0 || bh == 0.0 {
        return;
    }
    let fit = dom::window()
        .get_computed_style(&media)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value("object-fit").ok())
        .unwrap_or_default();
    let contain = (bw / nw).min(bh / nh);
    let (sx, sy) = match fit.as_str() {
        "cover" => ((bw / nw).max(bh / nh), (bw / nw).max(bh / nh)),
        // a video keeps the proportions of its picture
        "fill" if media.tag_name() != "VIDEO" => (bw / nw, bh / nh),
        "none" => (1.0, 1.0),
        "scale-down" => (contain.min(1.0), contain.min(1.0)),
        _ => (contain, contain),
    };
    let (w, h) = (nw * sx, nh * sy);
    let (x, y) = (el.offset_left() as f64 + (bw - w) / 2.0, el.offset_top() as f64 + (bh - h) / 2.0);
    for (k, v) in [("left", x), ("top", y), ("width", w), ("height", h)] {
        style(zones, k, &format!("{v:.1}px"));
    }
    // What `cover` crops stays hidden
    let (cx, cy) = (((w - bw) / 2.0).max(0.0), ((h - bh) / 2.0).max(0.0));
    if cx > 0.0 || cy > 0.0 {
        style(zones, "clip-path", &format!("inset({cy:.1}px {cx:.1}px)"));
    }
}

fn place_in(root: &Element) {
    for z in qsa(root, ".zones") {
        place(&z);
    }
}

/// The pauses of a video, in seconds
fn pauses(v: &Element) -> Vec<f64> {
    data(v, "pauses").unwrap_or_default().split_whitespace().filter_map(|t| t.parse().ok()).collect()
}

/// The part of the video that the click of the slide plays: its number, its start (the pause
/// before it, 0 for the first) and its end (the next pause; none: the end of the video). None
/// before the video appears.
fn segment(v: &Element, step: u32) -> Option<(usize, f64, Option<f64>)> {
    let at: u32 = data(v, "at").and_then(|a| a.parse().ok()).unwrap_or(0);
    let p = pauses(v);
    let s = (step.checked_sub(at)? as usize).min(p.len());
    Some((s, if s == 0 { 0.0 } else { p[s - 1] }, p.get(s).copied()))
}

/// Plays the part of each video of the slide that its click calls for; `arrived`: the slide has
/// just been shown, each video starts again from its part
fn steer(slide: &Element, arrived: bool) {
    let step = engine::current_step(slide);
    for el in qsa(slide, "video[data-pauses]") {
        let Some(v) = el.dyn_ref::<HtmlMediaElement>() else { continue };
        let seg = segment(&el, step);
        let key = seg.map_or("-".to_string(), |(s, ..)| s.to_string());
        if !arrived && data(&el, "segment").as_deref() == Some(key.as_str()) {
            continue;
        }
        let _ = el.set_attribute("data-segment", &key);
        match seg {
            None => {
                let _ = v.pause();
                v.set_current_time(0.0);
            }
            Some((_, start, end)) => {
                let t = v.current_time();
                if arrived || t < start - EPSILON || end.is_some_and(|e| t >= e - EPSILON) {
                    v.set_current_time(start);
                }
                let _ = el.set_attribute("data-t", &v.current_time().to_string());
                // The previews of the speaker view follow without sound
                if engine::receiver() {
                    v.set_muted(true);
                }
                // The browser may refuse to play without a gesture of the user: do not insist
                let _ = v.play();
            }
        }
    }
}

/// At each frame: the videos stop at their pause, the zones of a video follow its time
fn tick(run: u32) {
    if RUN.with(Cell::get) != run {
        return;
    }
    let slide = engine::current_slide();
    let step = engine::current_step(&slide);
    for el in qsa(&slide, "video[data-pauses]") {
        let Some(v) = el.dyn_ref::<HtmlMediaElement>() else { continue };
        let t = v.current_time();
        let before: f64 = data(&el, "t").and_then(|p| p.parse().ok()).unwrap_or(t);
        let _ = el.set_attribute("data-t", &t.to_string());
        // stops when it reaches the pause, not when the viewer moves past it
        if let Some((_, _, Some(end))) = segment(&el, step)
            && !v.paused()
            && before < end
            && t >= end
        {
            let _ = v.pause();
            v.set_current_time(end);
        }
    }
    let mut zoom_on = None;
    for zone in qsa(&slide, ".zone[data-time]") {
        let video = dom::closest(&zone, ".zones").and_then(|z| z.previous_element_sibling());
        let Some(t) = video.as_ref().and_then(|v| v.dyn_ref::<HtmlMediaElement>()).map(|v| v.current_time()) else {
            continue;
        };
        let range: Vec<f64> =
            data(&zone, "time").unwrap_or_default().split_whitespace().filter_map(|t| t.parse().ok()).collect();
        let on = range.first().is_some_and(|a| t >= a - EPSILON) && range.get(1).is_none_or(|b| t <= b + EPSILON);
        toggle(&zone, "on", on);
        if on && zone.has_attribute("data-zoom") {
            zoom_on = Some(zone);
        }
    }
    let current = ZOOMED.with(|z| z.borrow().clone());
    match zoom_on {
        Some(zone) if current.as_ref() != Some(&zone) => {
            zoom::zoom_to(&zone, ZOOM_MAX);
            ZOOMED.with(|z| *z.borrow_mut() = Some(zone));
        }
        None if current.is_some() => {
            zoom::zoom_out();
            ZOOMED.with(|z| *z.borrow_mut() = None);
        }
        _ => {}
    }
    dom::raf(move |_| tick(run));
}

/// A new slide: its zones placed, its videos started, the frame loop if it needs one
fn arrive(slide: &Element) {
    let run = RUN.with(|r| r.get() + 1);
    RUN.with(|r| r.set(run));
    ZOOMED.with(|z| *z.borrow_mut() = None);
    place_in(slide);
    if engine::upcoming() {
        return;
    }
    steer(slide, true);
    if dom::qs(slide, "video[data-pauses], .zone[data-time]").is_some() {
        tick(run);
    }
}

pub fn install() {
    // The zones are placed again once the picture is loaded (its size is then known)
    for zones in qsa(&engine::slides_element(), ".zones") {
        if let Some(media) = zones.previous_element_sibling() {
            let event = if media.tag_name() == "VIDEO" { "loadedmetadata" } else { "load" };
            dom::on::<web_sys::Event>(&media, event, move |_| place(&zones));
        }
    }
    // When printing and in the scroll view, every slide is shown: their zones, without the videos
    if engine::printing() || engine::scrolling() {
        engine::on(|ev| {
            if matches!(ev, Ev::Ready | Ev::Resize) {
                place_in(&engine::slides_element());
            }
        });
        return;
    }
    engine::on(|ev| match ev {
        Ev::Ready | Ev::SlideChanged { .. } => arrive(&engine::current_slide()),
        Ev::FragmentShown(_) | Ev::FragmentHidden(_) => {
            if !engine::upcoming() {
                steer(&engine::current_slide(), false);
            }
        }
        Ev::Resize => {
            place_in(&engine::current_slide());
            // the zoom, undone by the resizing, is done again at the next frame
            ZOOMED.with(|z| *z.borrow_mut() = None);
        }
    });
}

/// A zone gets more magnification than a word (`magnify` of `fx`): it is a small part of a picture
pub fn is_zone(el: &Element) -> bool {
    has(el, "zone") || dom::closest(el, ".zone").is_some()
}

pub const ZONE_MAX: f64 = ZOOM_MAX;
