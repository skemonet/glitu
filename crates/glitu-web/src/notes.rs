//! Speaker view (S key): a separate window with the current slide, the next step, the notes
//! (the passage of the current click highlighted, see `[click]`), the time and a timer, paused
//! with a click (it restarts when the slide changes) and corrected by hand (✎). The keys of the
//! presentation work there too.
//!
//! Timed slides (`duration="95"` after their title, see `glitu serve --rehearse`) give the lead
//! or the delay, next to the time: the time spent on the slides since the timer started, minus
//! their planned durations (the current slide counting at most its own).
//! In ghost mode, a button times a rehearsal: when it stops, the time spent on each slide left
//! goes to the server, which writes it in the markdown.
//!
//! The two previews are the presentation itself, loaded with `?receiver` in a frame: it goes
//! where the message `{glitu: "show", h, v, f, ahead}` tells it (see `engine::receive`).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use wasm_bindgen::JsValue;
use web_sys::{Document, Element, HtmlIFrameElement, KeyboardEvent, KeyboardEventInit, Window};

use crate::dom::{self, qs, qsa};
use crate::engine::{self, Ev};

/// The open window and its timer
struct Presenter {
    win: Window,
    /// Start of the timer (ms), shifted by the pauses
    start: f64,
    /// Paused since (ms)
    paused: Option<f64>,
    /// Since the timer started (or was reset)
    run: Run,
    /// Timed rehearsal in progress (ghost mode)
    record: Option<Run>,
    /// Current slide (reading rank) and time of the timer when it arrived
    at: (usize, f64),
}

/// Time spent (ms) on each slide (reading rank), and the slides left
#[derive(Default)]
struct Run {
    spent: BTreeMap<usize, f64>,
    left: BTreeSet<usize>,
}

impl Presenter {
    fn new(win: Window, now: f64) -> Presenter {
        Presenter { win, start: now, paused: None, run: Run::default(), record: None, at: (engine::leaf_index(), 0.0) }
    }

    /// Time of the timer (ms), pauses deducted
    fn elapsed(&self, now: f64) -> f64 {
        self.paused.unwrap_or(now) - self.start
    }

    /// Counts the time spent on the current slide; `leaf`: the slide we are on now
    fn account(&mut self, now: f64, leaf: usize) {
        let t = self.elapsed(now);
        let (cur, since) = self.at;
        for run in std::iter::once(&mut self.run).chain(self.record.as_mut()) {
            *run.spent.entry(cur).or_default() += t - since;
            if leaf != cur {
                run.left.insert(cur);
            }
        }
        self.at = (leaf, t);
    }

    /// Delay (positive) or lead (negative), in ms, on the planned durations; none if no slide is
    /// timed
    fn delta(&self, leaves: &[Element]) -> Option<f64> {
        let planned = |i: usize| leaves.get(i).and_then(|l| dom::data(l, "duration")).and_then(|d| d.parse::<f64>().ok()).map(|d| d * 1000.0);
        if !leaves.iter().any(|l| dom::data(l, "duration").is_some()) {
            return None;
        }
        Some(self.run.spent.iter().map(|(&i, &spent)| match planned(i) {
            Some(d) if i == self.at.0 => spent - spent.min(d),
            Some(d) => spent - d,
            None => 0.0,
        }).sum())
    }
}

/// The timer and its measures survive a reload of the page (localStorage, per presentation);
/// beyond 12 h, it starts again from zero
fn storage() -> Option<web_sys::Storage> {
    dom::window().local_storage().ok().flatten()
}

fn storage_key() -> String {
    format!("glitu-chrono:{}", dom::window().location().pathname().unwrap_or_default())
}

const KEEP: f64 = 12.0 * 3600.0 * 1000.0;

impl Run {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "spent": self.spent.iter().collect::<Vec<_>>(), "left": self.left })
    }

    fn from_json(v: &serde_json::Value) -> Option<Run> {
        let spent = v["spent"].as_array()?.iter().filter_map(|p| Some((p[0].as_u64()? as usize, p[1].as_f64()?))).collect();
        let left = v["left"].as_array()?.iter().filter_map(|i| Some(i.as_u64()? as usize)).collect();
        Some(Run { spent, left })
    }
}

impl Presenter {
    fn save(&self, now: f64) {
        let state = serde_json::json!({
            "saved": js_sys::Date::now(),
            "elapsed": self.elapsed(now),
            "paused": self.paused.is_some(),
            "at": [self.at.0, self.at.1],
            "run": self.run.to_json(),
            "record": self.record.as_ref().map(Run::to_json),
            "open": true,
        });
        if let Some(store) = storage() {
            let _ = store.set_item(&storage_key(), &state.to_string());
        }
    }

    /// The timer left by the page before its reload, if there is one
    fn restore(win: Window, now: f64) -> Option<Presenter> {
        let v = saved()?;
        let away = (js_sys::Date::now() - v["saved"].as_f64()?).max(0.0);
        let paused = v["paused"].as_bool()?;
        let elapsed = v["elapsed"].as_f64()? + if paused { 0.0 } else { away };
        Some(Presenter {
            win,
            start: now - elapsed,
            paused: paused.then_some(now),
            run: Run::from_json(&v["run"])?,
            record: Run::from_json(&v["record"]),
            at: (v["at"][0].as_u64()? as usize, v["at"][1].as_f64()?),
        })
    }
}

/// The kept timer, if it is recent
fn saved() -> Option<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(&storage()?.get_item(&storage_key()).ok()??).ok()?;
    (js_sys::Date::now() - v["saved"].as_f64()? < KEEP).then_some(v)
}

/// View closed: its timer stays kept (reopening resumes it), but the reloaded page no longer
/// reopens it by itself
fn closed() {
    if let (Some(mut v), Some(store)) = (saved(), storage()) {
        v["open"] = false.into();
        let _ = store.set_item(&storage_key(), &v.to_string());
    }
}

/// `1:02:03`, `12:30`, `45` (minutes) → ms
fn parse_time(s: &str) -> Option<f64> {
    let parts: Vec<f64> = s.trim().split(':').map(|p| p.trim().parse::<f64>().ok().filter(|v| *v >= 0.0)).collect::<Option<_>>()?;
    let secs = match parts[..] {
        [m] => m * 60.0,
        [m, s] => m * 60.0 + s,
        [h, m, s] => h * 3600.0 + m * 60.0 + s,
        _ => return None,
    };
    Some(secs * 1000.0)
}

/// `+1:20`, `-0:45`
fn signed(ms: f64) -> String {
    let secs = (ms.abs() / 1000.0).round() as u64;
    format!("{}{}:{:02}", if ms < 0.0 { "−" } else { "+" }, secs / 60, secs % 60)
}

/// Ghost mode: the page served by `glitu serve --rehearse` says so
fn rehearse() -> bool {
    dom::document().query_selector("meta[name=glitu-rehearse]").ok().flatten().is_some()
}

thread_local! {
    static PRESENTER: RefCell<Option<Presenter>> = const { RefCell::new(None) };
}

const STYLE: &str = "body{margin:0;background:#0b0f19;color:#e8edf5;font:18px/1.5 Lato,system-ui,sans-serif;height:100vh;overflow:hidden}\
.pv{display:grid;grid-template-columns:1.6fr 1fr;grid-template-rows:1fr auto;gap:14px;height:100vh;padding:14px;box-sizing:border-box}\
.cur,.next{position:relative;background:#000;border:1px solid #253045;border-radius:8px;overflow:hidden}\
.cur{align-self:center}.side{display:flex;flex-direction:column;gap:14px;min-height:0}\
iframe{display:block;width:100%;height:100%;border:0;pointer-events:none}\
h1{font-size:13px;letter-spacing:.14em;text-transform:uppercase;color:#22d3ee;margin:0 0 6px}\
.notes{flex:1;overflow:auto;font-size:22px;padding-right:6px;position:relative;scroll-behavior:smooth}.notes p{margin:0 0 .7em}.empty{color:#5d6b82;font-style:italic}\
.note-click{transition:opacity .2s,background .2s;border-radius:4px}.note-click.past{opacity:.45}\
.note-click.current{background:rgba(34,211,238,.16);color:#fff}\
.bar{grid-column:1/3;display:flex;gap:30px;align-items:center;padding:10px 4px 0;border-top:1px solid #253045;color:#93a0b4;font-size:17px}\
.bar b{color:#fff;font-size:22px}.bar button{background:#1b2333;color:#e8edf5;border:1px solid #2d3a52;border-radius:6px;padding:4px 10px;font:inherit;cursor:pointer}\
.bar .clock{margin-left:auto}.paused b{color:#f59e0b}.delta{margin-left:.5em;font-weight:700}.delta.late{color:#f87171}.delta.early{color:#4ade80}.bar button.rec{border-color:#f87171;color:#fecaca}";


/// Address of the presentation in a preview (without the position, given next by message)
fn receiver_url() -> String {
    let href = dom::window().location().href().unwrap_or_default();
    let base = href.split('#').next().unwrap_or_default();
    format!("{base}{}receiver", if base.contains('?') { '&' } else { '?' })
}

fn by_id(doc: &Document, id: &str) -> Option<Element> {
    doc.get_element_by_id(id)
}

/// Sends the current position to a preview, one step further with `ahead`
fn send(doc: &Document, id: &str, ahead: bool) {
    // The element comes from the speaker window, another "realm": `dyn_into` (instanceof) would
    // fail there
    let Some(frame) = by_id(doc, id).map(wasm_bindgen::JsCast::unchecked_into::<HtmlIFrameElement>) else { return };
    let Some(target) = frame.content_window() else { return };
    let (h, v, f) = engine::position();
    let msg = js_sys::Object::new();
    for (k, val) in [("glitu", JsValue::from("show")), ("h", h.into()), ("v", v.into()), ("f", f.into()), ("ahead", ahead.into())] {
        let _ = js_sys::Reflect::set(&msg, &k.into(), &val);
    }
    let _ = target.post_message(&msg, "*");
}

/// Notes of the slide, the passage of the current click highlighted
fn notes_html(slide: &Element) -> String {
    let Some(aside) = qs(slide, "aside.notes") else { return "<p class=\"empty\">No notes.</p>".into() };
    let step = engine::current_step(slide);
    for part in qsa(&aside, ".note-click") {
        let n: u32 = dom::data(&part, "click").and_then(|c| c.parse().ok()).unwrap_or(0);
        dom::toggle(&part, "past", n < step);
        dom::toggle(&part, "current", n == step);
    }
    aside.inner_html()
}

/// The notes scroll with the clicks, forward as backward: the current passage (failing that the
/// last one passed) in the middle of the area, the top of the notes before the first one
fn follow_notes(doc: &Document) {
    let Some(box_) = by_id(doc, "notes") else { return };
    let part = qs(&box_, ".note-click.current").or_else(|| qsa(&box_, ".note-click.past").pop());
    let top = part.map_or(0.0, |el| {
        let el = dom::html(&el);
        el.offset_top() as f64 - (box_.client_height() as f64 - el.offset_height() as f64) / 2.0
    });
    box_.set_scroll_top(top.max(0.0) as i32);
}

fn clock(ms: f64) -> String {
    let secs = (ms / 1000.0) as u64;
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

/// Updates the text; `move_previews`: the previews too (the position changed)
fn update(move_previews: bool) {
    PRESENTER.with(|p| {
        let mut p = p.borrow_mut();
        if p.as_ref().is_some_and(|pr| pr.win.closed().unwrap_or(true)) {
            *p = None;
            closed();
        }
        let Some(pr) = p.as_mut() else { return };
        let Some(doc) = pr.win.document() else { return };
        let cur = engine::current_slide();
        let (total, shown) = engine::fragment_counts(&cur);
        let set = |id: &str, html: &str| {
            if let Some(e) = by_id(&doc, id) {
                e.set_inner_html(html);
            }
        };
        set("where", &format!("Slide <b>{} / {}</b>", engine::leaf_index() + 1, engine::all_leaves().len()));
        set("step", &format!("Step <b>{shown} / {total}</b>"));
        set("notes", &notes_html(&cur));
        let now = dom::now();
        let leaf = engine::leaf_index();
        let moved = leaf != pr.at.0;
        pr.account(now, leaf);
        // Changing slide resumes the paused timer
        if moved && let Some(since) = pr.paused.take() {
            pr.start += now - since;
        }
        let leaves: Vec<Element> = engine::all_leaves().into_iter().map(|(_, _, l)| l).collect();
        let delta = pr.delta(&leaves).map_or(String::new(), |d| {
            let class = if d > 30_000.0 { "late" } else if d < -30_000.0 { "early" } else { "" };
            format!("<span class=\"delta {class}\" title=\"Delay (+) or lead (−) on the timed durations\">{}</span>", signed(d))
        });
        set("timer", &format!("Time <b>{}</b>{delta}", clock(pr.elapsed(now))));
        pr.save(now);
        if let Some(ghost) = by_id(&doc, "ghost") {
            let label = match &pr.record {
                Some(run) => format!("■ Stop and save ({} slides)", run.left.len()),
                None => "● Time a rehearsal".into(),
            };
            ghost.set_text_content(Some(&label));
            dom::toggle(&ghost, "rec", pr.record.is_some());
        }
        if let Some(t) = by_id(&doc, "timer") {
            dom::toggle(&t, "paused", pr.paused.is_some());
        }
        let date = js_sys::Date::new_0();
        set("clock", &format!("<b>{:02}:{:02}</b>", date.get_hours(), date.get_minutes()));
        set("next-title", if engine::at_end() { "End" } else { "Next" });
        if move_previews {
            follow_notes(&doc);
            send(&doc, "current", false);
            send(&doc, "upcoming", true);
        }
    });
}

/// A key in the speaker window: replayed in the presentation
fn forward_keys(doc: &Document) {
    dom::on::<KeyboardEvent>(doc, "keydown", |e| {
        let init = KeyboardEventInit::new();
        init.set_key(&e.key());
        init.set_shift_key(e.shift_key());
        init.set_bubbles(true);
        if let Ok(copy) = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init) {
            let _ = dom::document().dispatch_event(&copy);
        }
        e.prevent_default();
    });
}

/// The window: two previews (the presentation with `?receiver`), the notes, the bottom bar
fn skeleton() -> String {
    let c = engine::config();
    let ratio = format!("{} / {}", c.width, c.height);
    let url = dom::esc(&receiver_url());
    let ghost = if rehearse() { "<button id=\"ghost\"></button>" } else { "" };
    format!(
        "<style>{STYLE}</style><div class=\"pv\">\
         <div class=\"cur\" style=\"aspect-ratio:{ratio}\"><iframe id=\"current\" src=\"{url}\"></iframe></div>\
         <div class=\"side\"><div><h1 id=\"next-title\">Next</h1><div class=\"next\" style=\"aspect-ratio:{ratio}\">\
         <iframe id=\"upcoming\" src=\"{url}&amp;upcoming\"></iframe></div></div>\
         <h1>Notes</h1><div class=\"notes\" id=\"notes\"></div></div>\
         <div class=\"bar\"><span id=\"where\"></span><span id=\"step\"></span>\
         <span id=\"timer\" title=\"Click: pause or resume\"></span><button id=\"edit-time\" title=\"Correct the time\">✎</button><button id=\"reset\">Reset</button>{ghost}\
         <span class=\"clock\" id=\"clock\"></span></div></div>"
    )
}

/// Acts on the timer (the time of the current slide counted first), then updates the window
fn with_timer(change: impl Fn(&mut Presenter, f64)) {
    PRESENTER.with(|p| {
        if let Some(pr) = p.borrow_mut().as_mut() {
            let now = dom::now();
            pr.account(now, pr.at.0);
            change(pr, now);
        }
    });
    update(false);
}

/// Click on the time: pause or resume; "Reset" starts again from 0
fn bind_timer(doc: &Document) {
    if let Some(timer) = by_id(doc, "timer") {
        dom::on::<web_sys::MouseEvent>(&timer, "click", |_| {
            with_timer(|pr, now| match pr.paused.take() {
                Some(since) => pr.start += now - since,
                None => pr.paused = Some(now),
            });
        });
    }
    if let Some(reset) = by_id(doc, "reset") {
        dom::on::<web_sys::MouseEvent>(&reset, "click", |_| {
            with_timer(|pr, now| {
                pr.start = now;
                pr.paused = pr.paused.map(|_| now);
                pr.run = Run::default();
                pr.at.1 = 0.0;
            });
        });
    }
    // ✎: the time corrected by hand (a page reloaded without its timer); the time added counts for
    // no slide
    if let Some(edit) = by_id(doc, "edit-time") {
        dom::on::<web_sys::MouseEvent>(&edit, "click", |_| {
            let Some((win, shown)) = PRESENTER.with(|p| p.borrow().as_ref().map(|pr| (pr.win.clone(), clock(pr.elapsed(dom::now()))))) else { return };
            let Ok(Some(answer)) = win.prompt_with_message_and_default("Elapsed time (h:mm:ss, mm:ss or minutes)", &shown) else { return };
            let Some(t) = parse_time(&answer) else { return };
            with_timer(move |pr, now| {
                let old = pr.elapsed(now);
                pr.start += old - t;
                pr.at.1 += t - old;
            });
        });
    }
    if let Some(ghost) = by_id(doc, "ghost") {
        dom::on::<web_sys::MouseEvent>(&ghost, "click", |_| {
            with_timer(|pr, _| match pr.record.take() {
                None => pr.record = Some(Run::default()),
                Some(run) => send_record(&run),
            });
        });
    }
}

/// End of a rehearsal: the time spent on each slide left (2 s at least: not those crossed to go
/// further), by rank in the markdown, to the server (`glitu serve` receives it through the page,
/// see `build`)
fn send_record(run: &Run) {
    let leaves = engine::all_leaves();
    let measured = js_sys::Object::new();
    for i in &run.left {
        let (Some(&spent), Some((_, _, leaf))) = (run.spent.get(i), leaves.get(*i)) else { continue };
        let Some(slide) = dom::data(leaf, "slide") else { continue };
        if spent >= 2000.0 {
            let _ = js_sys::Reflect::set(&measured, &slide.into(), &(spent / 1000.0).into());
        }
    }
    let win = dom::window();
    if let Ok(json) = js_sys::JSON::stringify(&measured) {
        let _ = js_sys::Reflect::set(&win, &"glituChrono".into(), &json);
    }
    if let Ok(ev) = web_sys::Event::new("glitu:chrono") {
        let _ = win.dispatch_event(&ev);
    }
}

fn is_open() -> bool {
    PRESENTER.with(|p| p.borrow().as_ref().is_some_and(|pr| !pr.win.closed().unwrap_or(true)))
}

fn open() {
    if is_open() {
        PRESENTER.with(|p| p.borrow().as_ref().map(|pr| pr.win.focus()));
        return;
    }
    let Ok(Some(win)) = dom::window().open_with_url_and_target_and_features("", "glitu-notes", "width=1280,height=760") else { return };
    let Some(doc) = win.document() else { return };
    doc.set_title("Speaker view");
    if let Some(body) = doc.body() {
        body.set_inner_html(&skeleton());
    }
    // A ready preview receives the current position
    dom::on::<web_sys::MessageEvent>(&win, "message", |e| {
        let ready = js_sys::Reflect::get(&e.data(), &"glitu".into()).ok().and_then(|v| v.as_string());
        if ready.as_deref() == Some("ready") {
            update(true);
        }
    });
    bind_timer(&doc);
    forward_keys(&doc);
    let now = dom::now();
    let pr = Presenter::restore(win.clone(), now).unwrap_or_else(|| Presenter::new(win, now));
    PRESENTER.with(|p| *p.borrow_mut() = Some(pr));
    update(true);
    tick();
}

/// The time and the timer move on even without a change of slide
fn tick() {
    if is_open() {
        update(false);
        dom::timeout(1000.0, tick);
    }
}

pub fn install() {
    if engine::receiver() {
        return;
    }
    engine::on_key("s", open);
    engine::on(|ev| {
        // Page reloaded while the view was open (in ghost mode, after the durations were written): it
        // reopens, with its timer
        if matches!(ev, Ev::Ready) && saved().is_some_and(|v| v["open"] == true) {
            open();
        }
        if !matches!(ev, Ev::Resize) {
            update(true);
        }
    });
}
