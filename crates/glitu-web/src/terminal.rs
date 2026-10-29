//! Terminal replayed step by step: an asciicast recording (v2 or v3) is cut at its markers;
//! each click plays the next segment (typing included), the player stops on the marker and
//! waits for the click after. A click during the typing ends the current segment.
//!
//!   <div class="cast-stage" data-cast="#cast-1" data-idle-time-limit="1">
//!     <span class="fragment cast-step" data-fragment-index="1"></span>   …
//!   </div>
//!
//! The rendering takes the structure and the classes of asciinema-player 3 (`.ap-player`,
//! `.ap-term`, `.ap-line`…), dressed by the themes; the terminal emulation is done by `avt`,
//! asciinema's emulator.

use std::cell::RefCell;
use std::rc::Rc;

use avt::{Color, Vt};
use serde_json::Value;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};

use crate::dom::{self, data, has, qsa, style};
use crate::engine::{self, Ev};

const BLOCK_H: f64 = 8.0;
const BLOCK_V: f64 = 24.0;
const LINE_HEIGHT: f64 = 1.3333333333;

// ---------------------------------------------------------------- recording

enum Kind {
    Output(String),
    Marker,
    Resize(usize, usize),
}

struct Recording {
    cols: usize,
    rows: usize,
    events: Vec<(f64, Kind)>,
}

fn parse(text: &str, idle_limit: Option<f64>) -> Result<Recording, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: Value = serde_json::from_str(lines.next().ok_or("enregistrement vide")?).map_err(|e| e.to_string())?;
    let v3 = header["version"].as_u64() == Some(3);
    let (cols, rows) = if v3 {
        (header["term"]["cols"].as_u64().unwrap_or(80), header["term"]["rows"].as_u64().unwrap_or(24))
    } else {
        (header["width"].as_u64().unwrap_or(80), header["height"].as_u64().unwrap_or(24))
    };
    let limit = idle_limit.or_else(|| header["idle_time_limit"].as_f64());
    let (mut raw_prev, mut time) = (0.0, 0.0);
    let mut events = Vec::new();
    for line in lines {
        let ev: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let (t, code, payload) = (ev[0].as_f64().unwrap_or(0.0), ev[1].as_str().unwrap_or(""), ev[2].as_str().unwrap_or(""));
        // v2: absolute time; v3: interval since the previous event
        let mut delta = if v3 { t } else { t - raw_prev };
        raw_prev = if v3 { raw_prev + t } else { t };
        if let Some(l) = limit {
            delta = delta.min(l);
        }
        time += delta;
        let kind = match code {
            "o" => Kind::Output(payload.to_string()),
            "m" => Kind::Marker,
            "r" => {
                let (c, r) = payload.split_once('x').unwrap_or(("0", "0"));
                Kind::Resize(c.parse().unwrap_or(cols as usize), r.parse().unwrap_or(rows as usize))
            }
            _ => continue,
        };
        events.push((time, kind));
    }
    Ok(Recording { cols: cols as usize, rows: rows as usize, events })
}

// ---------------------------------------------------------------- player

struct Player {
    rec: Recording,
    vt: Vt,
    /// Next event to apply
    next: usize,
    /// Current time of the playback (s)
    time: f64,
    playing: bool,
    /// Instant (performance.now) matching time 0 during the playback
    origin: f64,
    timer: i32,
    view: View,
}

struct View {
    text: Element,
    ctx: CanvasRenderingContext2d,
    palette: Vec<String>,
    fg: String,
    bg: String,
}

type Shared = Rc<RefCell<Player>>;

impl Player {
    /// Playing or not; `data-playing` tells the page: captures (export, oracle) wait for the end of
    /// the typing, whose pauses look like a still screen
    fn set_playing(&mut self, on: bool) {
        self.playing = on;
        let _ = if on { self.view.text.set_attribute("data-playing", "") } else { self.view.text.remove_attribute("data-playing") };
    }

    fn reset(&mut self) {
        self.vt = Vt::builder().size(self.rec.cols, self.rec.rows).build();
        self.next = 0;
        self.time = 0.0;
    }

    /// Applies the events up to index `end` excluded
    fn apply_until(&mut self, end: usize) {
        while self.next < end.min(self.rec.events.len()) {
            match &self.rec.events[self.next].1 {
                Kind::Output(s) => {
                    self.vt.feed_str(s);
                }
                Kind::Resize(c, r) => {
                    self.vt.resize(*c, *r);
                }
                Kind::Marker => {}
            }
            self.next += 1;
        }
    }

    fn markers(&self) -> Vec<usize> {
        self.rec.events.iter().enumerate().filter(|(_, (_, k))| matches!(k, Kind::Marker)).map(|(i, _)| i).collect()
    }

    /// Position: before everything (0), just after a marker, or at the end
    fn seek(&mut self, to: Seek) {
        let end = match to {
            Seek::Start => self.rec.events.iter().take_while(|(t, _)| *t <= 0.0).count(),
            Seek::Marker(i) => if let Some(&m) = self.markers().get(i) { m + 1 } else {
                dom::warn("[terminal] marker not found in the recording");
                self.rec.events.len()
            },
            Seek::End => self.rec.events.len(),
        };
        if end < self.next {
            self.reset();
        }
        self.apply_until(end);
        self.time = if end == 0 { 0.0 } else { self.rec.events[end - 1].0 };
        self.render();
    }

    fn render(&self) {
        let v = &self.view;
        let (cols, rows) = self.vt.size();
        let cursor = self.vt.cursor();
        v.ctx.clear_rect(0.0, 0.0, cols as f64 * BLOCK_H, rows as f64 * BLOCK_V);
        let mut html = String::new();
        for (r, line) in self.vt.view().enumerate() {
            html += &format!("<span class=\"ap-line\" style=\"--row: {r};\">");
            let cells = line.cells();
            let mut c = 0;
            while c < cells.len() {
                let at_cursor = |k: usize| cursor.visible && cursor.row == r && cursor.col == k;
                let pen = *cells[c].pen();
                let cur = at_cursor(c);
                let mut text = String::new();
                let start = c;
                while c < cells.len() && *cells[c].pen() == pen && at_cursor(c) == cur && (c == start || !cur) {
                    if cells[c].width() > 0 {
                        text.push(cells[c].char());
                    }
                    c += 1;
                }
                let (mut fg, mut bg) = (pen.foreground().map(|x| self.color(x)), pen.background().map(|x| self.color(x)));
                if pen.is_inverse() ^ cur {
                    let f = fg.take().unwrap_or_else(|| v.fg.clone());
                    fg = Some(bg.take().unwrap_or_else(|| v.bg.clone()));
                    bg = Some(f);
                }
                if let Some(b) = &bg {
                    v.ctx.set_fill_style_str(b);
                    v.ctx.fill_rect(start as f64 * BLOCK_H, r as f64 * BLOCK_V, (c - start) as f64 * BLOCK_H, BLOCK_V);
                }
                let mut class = String::new();
                if pen.is_bold() { class += "ap-bold " } else if pen.is_faint() { class += "ap-faint " }
                if pen.is_italic() { class += "ap-italic " }
                if pen.is_underline() { class += "ap-underline " }
                if pen.is_strikethrough() { class += "ap-strike " }
                let class = if class.is_empty() { String::new() } else { format!(" class=\"{class}\"") };
                let color = fg.map(|f| format!(" color: {f};")).unwrap_or_default();
                html += &format!("<span{class} style=\"--offset: {start};{color}\">{}</span>", dom::esc(&text));
            }
            html += "</span>";
        }
        v.text.set_inner_html(&html);
    }

    fn color(&self, c: Color) -> String {
        match c {
            Color::Indexed(i) if (i as usize) < self.view.palette.len() => self.view.palette[i as usize].clone(),
            Color::Indexed(i) => xterm(i),
            Color::RGB(rgb) => format!("rgb({},{},{})", rgb.r, rgb.g, rgb.b),
        }
    }
}


/// Colours 16 to 255 of xterm
fn xterm(i: u8) -> String {
    if i >= 232 {
        let v = 8 + (i as u32 - 232) * 10;
        return format!("rgb({v},{v},{v})");
    }
    let i = i as u32 - 16;
    let level = |n: u32| if n == 0 { 0 } else { 55 + n * 40 };
    format!("rgb({},{},{})", level(i / 36), level((i / 6) % 6), level(i % 6))
}

#[derive(Clone, Copy)]
enum Seek {
    Start,
    Marker(usize),
    End,
}

fn pause(p: &Shared) {
    let mut pl = p.borrow_mut();
    pl.set_playing(false);
    dom::clear_timeout(pl.timer);
}

fn play(p: &Shared) {
    {
        let mut pl = p.borrow_mut();
        if pl.next >= pl.rec.events.len() {
            return;
        }
        pl.set_playing(true);
        pl.origin = dom::now() - pl.time * 1000.0;
    }
    tick(p);
}

/// Applies the events due, stops on a marker, schedules the rest
fn tick(p: &Shared) {
    let mut pl = p.borrow_mut();
    if !pl.playing {
        return;
    }
    let elapsed = (dom::now() - pl.origin) / 1000.0;
    let mut changed = false;
    while pl.next < pl.rec.events.len() && pl.rec.events[pl.next].0 <= elapsed {
        let i = pl.next;
        if matches!(pl.rec.events[i].1, Kind::Marker) {
            pl.next += 1;
            pl.time = pl.rec.events[i].0;
            pl.set_playing(false);
            pl.render();
            return;
        }
        pl.apply_until(i + 1);
        changed = true;
    }
    pl.time = elapsed;
    if changed {
        pl.render();
    }
    if pl.next >= pl.rec.events.len() {
        pl.set_playing(false);
        return;
    }
    let wait = (pl.rec.events[pl.next].0 - elapsed) * 1000.0;
    let me = Rc::clone(p);
    pl.timer = dom::timeout(wait, move || tick(&me));
}

// ---------------------------------------------------------------- steps of the slide

struct Stage {
    el: Element,
    player: Option<Shared>,
}

fn steps(stage: &Element) -> Vec<Element> {
    qsa(stage, ".cast-step")
}

fn shown(stage: &Element) -> usize {
    qsa(stage, ".cast-step.visible").len()
}

/// Expected screen when n steps are shown
fn end_of(stage: &Element, n: usize) -> Seek {
    let total = steps(stage).len();
    if n == 0 { Seek::Start } else if n < total { Seek::Marker(n - 1) } else { Seek::End }
}

fn start_of(i: usize) -> Seek {
    if i == 0 { Seek::Start } else { Seek::Marker(i - 1) }
}

fn sync(stage: &Element, p: &Shared) {
    pause(p);
    p.borrow_mut().seek(end_of(stage, shown(stage)));
}

/// asciinema-player's measure: width of a character and borders, at 15 px, outside the slide
fn measure(font: &str) -> (f64, f64, f64, f64) {
    let player = dom::create("div");
    player.set_class_name("ap-default-term-ff");
    style(&player, "height", "0px");
    style(&player, "overflow", "hidden");
    style(&player, "font-size", "15px");
    style(&player, "--term-font-family", font);
    let term = dom::create("div");
    term.set_class_name("ap-term");
    style(&term, "width", "80ch");
    style(&term, "height", &format!("{}em", 24.0 * LINE_HEIGHT));
    style(&term, "font-size", "100%");
    let _ = player.append_child(&term);
    let body = dom::document().body().unwrap();
    let _ = body.append_child(&player);
    let t = dom::html(&term);
    let m = (
        t.client_width() as f64 / 80.0,
        t.client_height() as f64 / 24.0,
        (t.offset_width() - t.client_width()) as f64,
        (t.offset_height() - t.client_height()) as f64,
    );
    let _ = body.remove_child(&player);
    m
}

fn mount(stage: &Element, theme: &str, font: &str) -> Result<Shared, String> {
    let src = data(stage, "cast").unwrap_or_default();
    let text = match src.strip_prefix('#') {
        Some(id) => dom::document().get_element_by_id(id).and_then(|e| e.text_content()).ok_or(format!("enregistrement introuvable : {src}"))?,
        None => return Err(format!("{src}: only embedded recordings (#id) are read")),
    };
    let idle = data(stage, "idle-time-limit").and_then(|v| v.parse().ok());
    let rec = parse(text.trim(), idle)?;
    let (cols, rows) = (rec.cols, rec.rows);

    let screen = dom::create("div");
    screen.set_class_name("cast-screen");
    let _ = stage.prepend_with_node_1(&screen);
    let wrapper = dom::create("div");
    wrapper.set_class_name("ap-wrapper");
    let _ = wrapper.set_attribute("tabindex", "-1");
    let player = dom::create("div");
    player.set_class_name(&format!("ap-player ap-default-term-ff asciinema-player-theme-{theme}"));
    style(&player, "--term-font-family", font);
    let term = dom::create("div");
    term.set_class_name("ap-term");
    let canvas: HtmlCanvasElement = dom::create("canvas").unchecked_into();
    canvas.set_width((cols as f64 * BLOCK_H) as u32);
    canvas.set_height((rows as f64 * BLOCK_V) as u32);
    style(&canvas, "image-rendering", "pixelated");
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten().ok_or("canvas 2d")?.unchecked_into();
    ctx.set_image_smoothing_enabled(false);
    let pre = dom::create("pre");
    pre.set_class_name("ap-term-text");
    let _ = pre.set_attribute("aria-live", "off");
    let _ = term.append_child(&canvas);
    let _ = term.append_child(&pre);
    let _ = player.append_child(&term);
    let _ = wrapper.append_child(&player);
    let _ = screen.append_child(&wrapper);

    // "fit: width" scaling of asciinema-player
    let (char_w, char_h, borders_w, borders_h) = measure(font);
    let fit = {
        let (term, player, wrapper) = (term.clone(), player.clone(), wrapper.clone());
        move || {
            let tw = char_w * cols as f64 + borders_w;
            let th = char_h * rows as f64 + borders_h;
            let container = dom::html(&wrapper).offset_width() as f64;
            if container == 0.0 || tw == 0.0 {
                return;
            }
            let scale = container / tw;
            style(&player, "width", &format!("{container}px"));
            style(&player, "height", &format!("{}px", th * scale));
            style(&term, "font-size", &format!("{}%", scale * 100.0));
        }
    };
    style(&term, "width", &format!("{cols}ch"));
    style(&term, "height", &format!("{}em", LINE_HEIGHT * rows as f64));
    style(&term, "--term-line-height", &format!("{LINE_HEIGHT}em"));
    style(&term, "--term-cols", &cols.to_string());
    style(&term, "--term-rows", &rows.to_string());
    fit();

    let css = dom::window().get_computed_style(&player).ok().flatten();
    let var = |name: &str| css.as_ref().and_then(|c| c.get_property_value(name).ok()).map(|v| v.trim().to_string()).unwrap_or_default();
    let fg = var("--term-color-foreground");
    let bg = var("--term-color-background");
    let palette = (0..16).map(|i| {
        let v = var(&format!("--term-color-{i}"));
        if v.is_empty() { fg.clone() } else { v }
    }).collect();

    let view = View { text: pre, ctx, palette, fg, bg };
    let vt = Vt::builder().size(cols, rows).build();
    let p = Rc::new(RefCell::new(Player { rec, vt, next: 0, time: 0.0, playing: false, origin: 0.0, timer: 0, view }));
    let observer = {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(fit);
        let ro = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()).ok();
        cb.forget();
        ro
    };
    if let Some(ro) = observer {
        ro.observe(&wrapper);
    }
    Ok(p)
}

thread_local! {
    static STAGES: RefCell<Vec<Stage>> = const { RefCell::new(Vec::new()) };
}

fn player_of(stage: &Element) -> Option<Shared> {
    STAGES.with(|s| s.borrow().iter().find(|x| &x.el == stage).and_then(|x| x.player.clone()))
}

fn enter(slide: &Element, theme: &str, font: &str) {
    for el in qsa(slide, ".cast-stage") {
        if let Some(p) = player_of(&el) {
            sync(&el, &p);
            continue;
        }
        let known = STAGES.with(|s| s.borrow().iter().any(|x| x.el == el));
        if known {
            continue;
        }
        STAGES.with(|s| s.borrow_mut().push(Stage { el: el.clone(), player: None }));
        // The terminal mounts once the font is loaded (its width is used for the scaling)
        let font_ref = font.to_string();
        let (theme, font, el2) = (theme.to_string(), font.to_string(), el.clone());
        let ready = move || match mount(&el2, &theme, &font) {
            Ok(p) => {
                sync(&el2, &p);
                STAGES.with(|s| {
                    if let Some(x) = s.borrow_mut().iter_mut().find(|x| x.el == el2) {
                        x.player = Some(p);
                    }
                });
            }
            Err(e) => dom::warn(&format!("[terminal] {e}")),
        };
        let promise = dom::document().fonts().load(&format!("16px {font_ref}"));
        let mut ready = Some(ready);
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(wasm_bindgen::JsValue)>::new(move |_| {
            if let Some(f) = ready.take() {
                f();
            }
        });
        let _ = promise.then2(&cb, &cb);
        cb.forget();
    }
}

fn on_steps(frags: &[Element], f: impl Fn(&Element, &Shared, usize)) {
    let Some(frag) = frags.iter().rfind(|x| has(x, "cast-step")) else { return };
    let Some(stage) = dom::closest(frag, ".cast-stage") else { return };
    let Some(p) = player_of(&stage) else { return };
    let i = steps(&stage).iter().position(|s| s == frag).unwrap_or(0);
    f(&stage, &p, i);
}

pub fn install(theme: &str) {
    let font = dom::window()
        .get_computed_style(&dom::document().document_element().unwrap())
        .ok()
        .flatten()
        .and_then(|c| c.get_property_value("--mono").ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "monospace".into());
    let theme = theme.to_string();
    engine::on(move |ev| match ev {
        // When printing, each terminal shows its final screen
        Ev::Ready if engine::printing() => engine::all_leaves().iter().for_each(|(_, _, s)| enter(s, &theme, &font)),
        Ev::Ready => enter(&engine::current_slide(), &theme, &font),
        Ev::SlideChanged { previous } => {
            if let Some(prev) = previous {
                for el in qsa(prev, ".cast-stage") {
                    if let Some(p) = player_of(&el) {
                        pause(&p);
                    }
                }
            }
            enter(&engine::current_slide(), &theme, &font);
        }
        // Plays segment i; if some of the previous segment was left, it is skipped. In the speaker's
        // "Next" preview, directly the screen at the end of the segment
        Ev::FragmentShown(frags) => on_steps(frags, |stage, p, i| {
            if engine::upcoming() {
                sync(stage, p);
                return;
            }
            pause(p);
            p.borrow_mut().seek(start_of(i));
            play(p);
        }),
        Ev::FragmentHidden(frags) => on_steps(frags, |_, p, i| {
            pause(p);
            p.borrow_mut().seek(start_of(i));
        }),
        Ev::Resize => {}
    });
}
