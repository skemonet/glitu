//! Speaker view (S key): a separate window with the current slide, the timer under it, the next
//! step, and the notes (the passage of the current click highlighted, see `[click]`) or the timing
//! of every slide. The keys of the presentation work there too.
//!
//! The timer (see `timing`) is started and stopped by hand; on timed slides (`## Title ⏳95🏃3`)
//! it gives the lead or the delay on the schedule. Under `glitu serve`, it times rehearsals: from
//! the start or from the current slide, a slide started again after a false start, and at the
//! end the measure of each slide is averaged with the former ones, replaces them or is skipped;
//! the server writes them in the markdown. The timer survives a reload of the page and the
//! closing of the view (localStorage, per presentation); the layout of the view (column widths,
//! size of the notes) is kept by the browser too.
//!
//! The two previews are the presentation itself, loaded with `?receiver` in a frame: it goes
//! where the message `{glitu: "show", h, v, f, ahead}` tells it (see `engine::receive`).

use std::cell::RefCell;
use std::collections::BTreeMap;

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Element, HtmlIFrameElement, KeyboardEvent, KeyboardEventInit, MouseEvent, PointerEvent, Window};

use crate::dom::{self, qs, qsa};
use crate::engine::{self, Ev};
use crate::timing::{self, Timer};

/// What to do with the measure of a slide, at the end of a rehearsal
#[derive(Clone, Copy, PartialEq)]
enum Keep {
    Average,
    Replace,
    Skip,
}

impl Keep {
    const ALL: [Keep; 3] = [Keep::Average, Keep::Replace, Keep::Skip];

    fn icon(self) -> &'static str {
        match self {
            Keep::Average => "📊",
            Keep::Replace => "✏️",
            Keep::Skip => "🗑️",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Keep::Average => "Average",
            Keep::Replace => "Replace",
            Keep::Skip => "Skip",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Keep::Average => "average",
            Keep::Replace => "replace",
            Keep::Skip => "skip",
        }
    }
}

/// The view: its window (if open), the timer, the choices of the end of a rehearsal
struct Presenter {
    win: Option<Window>,
    timer: Timer,
    /// End of a rehearsal: what to keep of each slide measured
    keep: BTreeMap<usize, Keep>,
}

thread_local! {
    static PRESENTER: RefCell<Option<Presenter>> = const { RefCell::new(None) };
}

/// A slide measured less than this was only gone through: skipped by default
const GONE_THROUGH: f64 = 2000.0;

/// Beyond 12 h, a kept timer starts again from zero
const KEEP: f64 = 12.0 * 3600.0 * 1000.0;

fn storage() -> Option<web_sys::Storage> {
    dom::window().local_storage().ok().flatten()
}

fn storage_key() -> String {
    format!("glitu-chrono:{}", dom::window().location().pathname().unwrap_or_default())
}

const LAYOUT_KEY: &str = "glitu-speaker-layout";

/// The kept state of the view, if it is recent: the timer and whether the view was open
fn saved() -> Option<serde_json::Value> {
    let v: serde_json::Value = serde_json::from_str(&storage()?.get_item(&storage_key()).ok()??).ok()?;
    (js_sys::Date::now() - v["saved"].as_f64()? < KEEP).then_some(v)
}

fn save(p: &Presenter, now: f64) {
    let state = serde_json::json!({
        "saved": js_sys::Date::now(),
        "timer": p.timer.to_json(now),
        "open": p.win.is_some(),
    });
    if let Some(store) = storage() {
        let _ = store.set_item(&storage_key(), &state.to_string());
    }
}

/// Rehearsals are recorded under `glitu serve` only (it writes the markdown)
fn served() -> bool {
    dom::document().query_selector("meta[name=glitu-rehearse]").ok().flatten().is_some()
}

/// The planned duration of each slide (ms), in reading order
fn planned() -> Vec<Option<f64>> {
    engine::all_leaves()
        .iter()
        .map(|(_, _, l)| dom::data(l, "duration").and_then(|d| d.parse::<f64>().ok()).map(|d| d * 1000.0))
        .collect()
}

/// The title of a slide: its topic, otherwise its first heading
fn title(leaf: &Element, i: usize) -> String {
    let topic = dom::data(leaf, "header").and_then(|h| h.split('|').nth(1).map(|t| t.trim().to_string()));
    topic
        .filter(|t| !t.is_empty())
        .or_else(|| qs(leaf, "h1, h2, h3").and_then(|h| h.text_content()).map(|t| t.trim().to_string()))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("Slide {}", i + 1))
}

fn runs(leaf: &Element) -> Option<u32> {
    dom::data(leaf, "runs").and_then(|r| r.parse().ok())
}

const STYLE: &str = "\
*{box-sizing:border-box}body{margin:0;background:#0b0f19;color:#e8edf5;font:18px/1.5 Lato,system-ui,sans-serif;height:100vh;overflow:hidden}\
button{background:#1b2333;color:#e8edf5;border:1px solid #2d3a52;border-radius:6px;padding:4px 10px;font:inherit;font-size:15px;cursor:pointer}\
button:hover{border-color:#3b82f6}button.on{background:#123044;border-color:#22d3ee;color:#fff}\
.pv{display:flex;height:100vh;padding:12px;gap:0}\
.col{display:flex;flex-direction:column;min-width:0;gap:10px}.left{flex:0 0 var(--left,62%)}.right{flex:1}\
.split{flex:0 0 12px;cursor:col-resize;position:relative}.split::after{content:'';position:absolute;inset:0 5px;border-radius:2px;background:#1b2333}\
.split:hover::after,.split.drag::after{background:#22d3ee}\
.hsplit{flex:0 0 12px;cursor:row-resize;position:relative}.hsplit::after{content:'';position:absolute;inset:5px 0;border-radius:2px;background:#1b2333}\
.hsplit:hover::after,.hsplit.drag::after{background:#22d3ee}.left .hsplit{margin:-10px 0}\
.frame{position:relative;background:#000;border:1px solid #253045;border-radius:8px;overflow:hidden;aspect-ratio:var(--ratio);margin:0 auto}\
.curbox{flex:0 0 var(--slide,60%);min-height:80px;container-type:size}.cur{width:min(100cqw,calc(100cqh * var(--ratio)))}\
.nextbox{flex:0 0 var(--next,38%);min-height:60px;display:flex;flex-direction:column}.next{height:100%;max-width:100%}\
iframe{display:block;width:100%;height:100%;border:0;pointer-events:none}\
h1{font-size:13px;letter-spacing:.14em;text-transform:uppercase;color:#22d3ee;margin:0 0 6px}\
.dock{flex:0 0 auto;display:flex;flex-direction:column;gap:10px;padding:8px 6px 0;border-top:1px solid #1b2333}\
.row{display:flex;align-items:center;gap:18px;flex-wrap:wrap}\
.play{font-size:26px;width:52px;height:52px;border-radius:50%;padding:0}\
.elapsed{font-size:46px;font-weight:700;font-variant-numeric:tabular-nums;letter-spacing:.02em}.stopped .elapsed{color:#93a0b4}\
.delta{font-size:24px;font-weight:700;font-variant-numeric:tabular-nums}.delta small{font-size:15px;font-weight:400;color:#93a0b4}\
.late{color:#f87171}.early{color:#4ade80}.where{color:#93a0b4;font-size:17px}.where b{color:#fff}\
.spacer{flex:1}.clock{font-size:22px;font-weight:700;font-variant-numeric:tabular-nums}\
.reh{display:none;gap:12px;align-items:center;padding:8px 12px;border:1px solid #2d3a52;border-radius:8px;background:#111827}\
.rehearsing .reh{display:flex}.reh .label{color:#22d3ee;font-weight:700}.reh .slide{color:#93a0b4;font-variant-numeric:tabular-nums}\
.menu{position:relative}.menu .items{display:none;position:absolute;right:0;bottom:44px;background:#111827;border:1px solid #2d3a52;border-radius:8px;padding:6px;min-width:250px;z-index:5}\
.menu.open .items{display:flex;flex-direction:column;gap:4px}.menu .items button{text-align:left;border:0;background:none}.menu .items button:hover{background:#1b2333}\
.tabs{display:flex;gap:6px;align-items:center}.tabs h1{margin:0}.tabs .grow{flex:1}\
.pane{flex:1;overflow:auto;min-height:0;position:relative}\
.notes{font-size:var(--notes,22px);padding-right:6px;scroll-behavior:smooth}.notes p{margin:0 0 .7em}.empty{color:#5d6b82;font-style:italic}\
.note-click{transition:opacity .2s,background .2s;border-radius:4px}.note-click.past{opacity:.45}\
.note-click.current{background:rgba(34,211,238,.16);color:#fff}\
table{width:100%;border-collapse:collapse;font-size:16px;font-variant-numeric:tabular-nums}\
th{text-align:left;color:#93a0b4;font-weight:400;font-size:13px;letter-spacing:.06em;text-transform:uppercase;padding:4px 6px;border-bottom:1px solid #253045;position:sticky;top:0;background:#0b0f19}\
td{padding:5px 6px;border-bottom:1px solid #161d2b}td.n{color:#5d6b82;width:2.5em}.t{white-space:nowrap;text-align:right}\
tr.cur td{background:rgba(34,211,238,.1);color:#fff}tr.cur td.n{color:#22d3ee}\
.bar{height:3px;background:#1b2333;border-radius:2px;margin-top:3px}.bar i{display:block;height:100%;background:#22d3ee;border-radius:2px}.bar.over i{background:#f87171}\
.runs{color:#5d6b82;font-size:13px}\
.modal{position:fixed;inset:0;background:rgba(3,6,12,.72);display:none;align-items:center;justify-content:center;z-index:10}\
.modal.open{display:flex}.dialog{background:#111827;border:1px solid #2d3a52;border-radius:12px;padding:22px 26px;max-width:760px;width:90%;max-height:86vh;display:flex;flex-direction:column;gap:14px}\
.dialog h2{margin:0;font-size:22px}.dialog p{margin:0;color:#c3cad6}.dialog .warn{color:#fbbf24}\
.dialog .scroll{overflow:auto;min-height:0}.dialog .actions{display:flex;justify-content:flex-end;gap:10px}\
.dialog .primary{background:#0e7490;border-color:#22d3ee;color:#fff}\
.all{display:flex;gap:18px;align-items:center;color:#c3cad6}.all label{cursor:pointer;display:flex;gap:6px;align-items:center}\
.keep{display:flex;gap:4px}.keep button{padding:2px 8px;opacity:.45}.keep button.on{opacity:1}\
.toast{position:fixed;left:50%;bottom:20px;translate:-50% 0;background:#0e7490;color:#fff;padding:8px 16px;border-radius:8px;display:none}.toast.show{display:block}";

/// Address of the presentation in a preview (without the position, given next by message)
fn receiver_url() -> String {
    let href = dom::window().location().href().unwrap_or_default();
    let base = href.split('#').next().unwrap_or_default();
    format!("{base}{}receiver", if base.contains('?') { '&' } else { '?' })
}

/// The window: on the left the current slide, the timing of the slides under it and the timer at
/// the bottom; on the right the next step and the notes; separators to drag between them
fn skeleton() -> String {
    let c = engine::config();
    let ratio = format!("{} / {}", c.width, c.height);
    let url = dom::esc(&receiver_url());
    let rehearse = if served() {
        "<button data-act=\"from-start\">● Rehearse from the start</button>\
         <button data-act=\"from-here\">↳ Rehearse from here</button>"
    } else {
        ""
    };
    format!(
        "<style>{STYLE}</style><div class=\"pv\" id=\"pv\" style=\"--ratio:{ratio}\">\
         <div class=\"col left\" id=\"left\">\
           <div class=\"curbox\"><div class=\"frame cur\"><iframe id=\"current\" src=\"{url}\"></iframe></div></div>\
           <div class=\"hsplit\" id=\"vsplit\" title=\"Drag to resize\"></div>\
           <div class=\"pane\" id=\"timing\"></div>\
           <div class=\"dock\" id=\"dock\">\
             <div class=\"row\">\
               <button class=\"play\" id=\"play\" title=\"Start or stop the timer\"></button>\
               <span class=\"elapsed\" id=\"elapsed\"></span><span class=\"delta\" id=\"delta\"></span>\
               <span class=\"spacer\"></span><span class=\"where\" id=\"where\"></span><span class=\"clock\" id=\"clock\"></span>\
               <div class=\"menu\" id=\"menu\"><button id=\"menu-btn\" title=\"Timer and layout\">⋯</button><div class=\"items\">\
                 {rehearse}<button data-act=\"reset\">⟲ Reset the timer</button><button data-act=\"layout\">▦ Reset the layout</button>\
               </div></div>\
             </div>\
             <div class=\"reh\" id=\"reh\"><span class=\"label\" id=\"reh-label\"></span><span class=\"slide\" id=\"reh-slide\"></span>\
               <span class=\"spacer\"></span>\
               <button data-act=\"restart\" title=\"A false start: the time of this slide goes, it starts again\">↺ Restart this slide</button>\
               <button data-act=\"finish\" class=\"primary\">■ Finish…</button><button data-act=\"discard\">✕ Discard</button></div>\
           </div>\
         </div>\
         <div class=\"split\" id=\"split\" title=\"Drag to resize\"></div>\
         <div class=\"col right\" id=\"right\">\
           <div class=\"nextbox\"><h1 id=\"next-title\">Next</h1><div class=\"frame next\"><iframe id=\"upcoming\" src=\"{url}&amp;upcoming\"></iframe></div></div>\
           <div class=\"hsplit\" id=\"hsplit\" title=\"Drag to resize\"></div>\
           <div class=\"tabs\"><h1>Notes</h1>\
             <span class=\"grow\"></span><button data-act=\"smaller\" title=\"Smaller notes\">A−</button><button data-act=\"larger\" title=\"Larger notes\">A+</button></div>\
           <div class=\"pane notes\" id=\"notes\"></div>\
         </div>\
         </div><div class=\"modal\" id=\"modal\"><div class=\"dialog\" id=\"dialog\"></div></div><div class=\"toast\" id=\"toast\"></div>"
    )
}

fn by_id(doc: &Document, id: &str) -> Option<Element> {
    doc.get_element_by_id(id)
}

fn set(doc: &Document, id: &str, html: &str) {
    if let Some(e) = by_id(doc, id) {
        e.set_inner_html(html);
    }
}

/// Sends the current position to a preview, one step further with `ahead`
fn send(doc: &Document, id: &str, ahead: bool) {
    // The element comes from the speaker window, another "realm": `dyn_into` (instanceof) would
    // fail there
    let Some(frame) = by_id(doc, id).map(JsCast::unchecked_into::<HtmlIFrameElement>) else { return };
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

/// The timing table keeps the current slide in sight
fn follow_timing(doc: &Document) {
    let Some(pane) = by_id(doc, "timing") else { return };
    let Some(row) = qs(&pane, "tr.cur") else { return };
    let row = dom::html(&row);
    let top = row.offset_top() as f64 - (pane.client_height() as f64 - row.offset_height() as f64) / 2.0;
    pane.set_scroll_top(top.max(0.0) as i32);
}

/// `+0:07 behind`, `−0:12 ahead`
fn delta_html(d: f64) -> String {
    if d.abs() < 1000.0 {
        return "<small>on time</small>".into();
    }
    let (class, sign, word) = if d > 0.0 { ("late", "+", "behind") } else { ("early", "−", "ahead") };
    format!("<span class=\"{class}\">{sign}{}</span> <small>{word}</small>", timing::short(d.abs()))
}

/// The timing of every slide: planned (⏳, 🏃 rehearsals) and this run, the current one marked
fn timing_html(p: &Presenter, now: f64) -> String {
    let planned = planned();
    let cur = engine::leaf_index();
    let mut rows = String::new();
    let (mut total_planned, mut total_now) = (0.0, 0.0);
    for (i, (_, _, leaf)) in engine::all_leaves().iter().enumerate() {
        let spent = p.timer.spent_on(now, i);
        total_now += spent;
        let plan = planned[i];
        total_planned += plan.unwrap_or(0.0);
        let plan_html = plan.map_or("–".into(), |d| {
            let r = runs(leaf).map_or(String::new(), |r| format!(" <span class=\"runs\">🏃{r}</span>"));
            format!("{}{r}", timing::short(d))
        });
        let over = plan.is_some_and(|d| spent > d * 1.1);
        let now_html = if spent > 0.0 { timing::short(spent) } else { "–".into() };
        let bar = match plan {
            Some(d) if i == cur && d > 0.0 => format!(
                "<div class=\"bar{}\"><i style=\"width:{:.0}%\"></i></div>",
                if spent > d { " over" } else { "" },
                (spent / d * 100.0).min(100.0)
            ),
            _ => String::new(),
        };
        rows += &format!(
            "<tr{}><td class=\"n\">{}</td><td>{}{bar}</td><td class=\"t\">{plan_html}</td><td class=\"t{}\">{now_html}</td></tr>",
            if i == cur { " class=\"cur\"" } else { "" },
            i + 1,
            dom::esc(&title(leaf, i)),
            if over { " late" } else { "" }
        );
    }
    format!(
        "<table><tr><th>#</th><th>Slide</th><th class=\"t\">⏳ Planned</th><th class=\"t\">This run</th></tr>{rows}\
         <tr><td></td><td><b>Total</b></td><td class=\"t\"><b>{}</b></td><td class=\"t\"><b>{}</b></td></tr></table>",
        timing::short(total_planned),
        timing::short(total_now)
    )
}

/// Updates the window; `move_previews`: the previews and the notes too (the position changed)
fn update(move_previews: bool) {
    PRESENTER.with(|p| {
        let mut p = p.borrow_mut();
        let Some(pr) = p.as_mut() else { return };
        if pr.win.as_ref().is_some_and(|w| w.closed().unwrap_or(true)) {
            pr.win = None;
        }
        let now = dom::now();
        pr.timer.account(now, engine::leaf_index());
        save(pr, now);
        let Some(doc) = pr.win.as_ref().and_then(Window::document) else { return };
        let cur = engine::current_slide();
        let (total, shown) = engine::fragment_counts(&cur);
        let leaf = engine::leaf_index();
        let step = if total > 0 { format!(" · step <b>{shown}/{total}</b>") } else { String::new() };
        set(&doc, "where", &format!("slide <b>{}/{}</b>{step}", leaf + 1, engine::all_leaves().len()));
        set(&doc, "elapsed", &timing::clock(pr.timer.elapsed(now)));
        set(&doc, "play", if pr.timer.running() { "⏸" } else { "▶" });
        if let Some(dock) = by_id(&doc, "dock") {
            dom::toggle(&dock, "stopped", !pr.timer.running());
            dom::toggle(&dock, "rehearsing", pr.timer.rehearsal.is_some());
        }
        let planned = planned();
        set(&doc, "delta", &pr.timer.delta(now, &planned).map_or(String::new(), delta_html));
        if pr.timer.rehearsal.is_some() {
            set(&doc, "reh-label", if pr.timer.running() { "⏱ Timing a rehearsal" } else { "⏸ Rehearsal paused" });
            let spent = timing::short(pr.timer.spent_on(now, leaf));
            let plan = planned.get(leaf).copied().flatten().map_or(String::new(), |d| format!(" / ⏳{}", timing::short(d)));
            set(&doc, "reh-slide", &format!("this slide {spent}{plan}"));
        }
        let date = js_sys::Date::new_0();
        set(&doc, "clock", &format!("{:02}:{:02}", date.get_hours(), date.get_minutes()));
        set(&doc, "timing", &timing_html(pr, now));
        if move_previews {
            follow_timing(&doc);
            set(&doc, "notes", &notes_html(&cur));
            set(&doc, "next-title", if engine::at_end() { "End" } else { "Next" });
            follow_notes(&doc);
            send(&doc, "current", false);
            send(&doc, "upcoming", true);
        }
    });
}

/// Acts on the presenter, then updates the window
fn with(change: impl FnOnce(&mut Presenter, f64)) {
    PRESENTER.with(|p| {
        if let Some(pr) = p.borrow_mut().as_mut() {
            let now = dom::now();
            pr.timer.account(now, engine::leaf_index());
            change(pr, now);
        }
    });
    update(false);
}

fn window() -> Option<Window> {
    PRESENTER.with(|p| p.borrow().as_ref().and_then(|pr| pr.win.clone()))
}

/// A key in the speaker window: replayed in the presentation, unless a dialog is open
fn forward_keys(doc: &Document) {
    let doc2 = doc.clone();
    dom::on::<KeyboardEvent>(doc, "keydown", move |e| {
        if by_id(&doc2, "modal").is_some_and(|m| dom::has(&m, "open")) {
            if e.key() == "Escape" {
                close_dialog();
            }
            return;
        }
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

// ---------------------------------------------------------------- layout

/// The layout of the view, kept by the browser: width of the left column, height of the current
/// slide and of the next step (percent), size of the notes (px)
#[derive(Clone, Copy)]
struct Layout {
    left: f64,
    slide: f64,
    next: f64,
    notes: f64,
}

const LAYOUT: Layout = Layout { left: 62.0, slide: 60.0, next: 38.0, notes: 22.0 };

fn layout() -> Layout {
    let v: Option<serde_json::Value> = storage().and_then(|s| s.get_item(LAYOUT_KEY).ok().flatten()).and_then(|t| serde_json::from_str(&t).ok());
    let get = |k: &str, d: f64| v.as_ref().and_then(|v| v[k].as_f64()).unwrap_or(d);
    Layout {
        left: get("left", LAYOUT.left),
        slide: get("slide", LAYOUT.slide),
        next: get("next", LAYOUT.next),
        notes: get("notes", LAYOUT.notes),
    }
}

fn apply_layout(doc: &Document, l: Layout) {
    if let Some(pv) = by_id(doc, "pv") {
        dom::style(&pv, "--left", &format!("{:.1}%", l.left));
        dom::style(&pv, "--slide", &format!("{:.1}%", l.slide));
        dom::style(&pv, "--next", &format!("{:.1}%", l.next));
        dom::style(&pv, "--notes", &format!("{:.0}px", l.notes));
    }
    if let Some(s) = storage() {
        let _ = s.set_item(LAYOUT_KEY, &serde_json::json!({ "left": l.left, "slide": l.slide, "next": l.next, "notes": l.notes }).to_string());
    }
}

/// Which separator is dragged
#[derive(Clone, Copy)]
enum Split {
    /// Between the columns
    Columns,
    /// Under the current slide
    Slide,
    /// Under the next step
    Next,
}

/// A separator dragged: the left column's width, or the current slide's or the next step's
/// height, follows the pointer
fn drag(doc: &Document, id: &str, split_of: Split) {
    let Some(split) = by_id(doc, id) else { return };
    let dragging = std::rc::Rc::new(std::cell::Cell::new(false));
    let (d1, d2, d3) = (dragging.clone(), dragging.clone(), dragging);
    let s1 = split.clone();
    dom::on::<PointerEvent>(&split, "pointerdown", move |e| {
        d1.set(true);
        dom::add(&s1, "drag");
        e.prevent_default();
    });
    let doc2 = doc.clone();
    dom::on::<PointerEvent>(doc, "pointermove", move |e| {
        if !d2.get() {
            return;
        }
        let mut l = layout();
        let rect = |id: &str| by_id(&doc2, id).map(|el| el.get_bounding_client_rect());
        let (x, y) = (e.client_x() as f64, e.client_y() as f64);
        match split_of {
            Split::Columns => {
                let Some(r) = rect("pv") else { return };
                l.left = ((x - r.left()) / r.width() * 100.0).clamp(25.0, 80.0);
            }
            Split::Slide => {
                let Some(r) = rect("left") else { return };
                l.slide = ((y - r.top()) / r.height() * 100.0).clamp(20.0, 85.0);
            }
            Split::Next => {
                let Some(r) = rect("right") else { return };
                l.next = ((y - r.top()) / r.height() * 100.0).clamp(10.0, 80.0);
            }
        }
        apply_layout(&doc2, l);
    });
    let s2 = split;
    dom::on::<PointerEvent>(doc, "pointerup", move |_| {
        d3.set(false);
        dom::remove(&s2, "drag");
    });
}

// ---------------------------------------------------------------- dialogs

fn dialog(html: &str) {
    let Some(doc) = window().and_then(|w| w.document()) else { return };
    set(&doc, "dialog", html);
    if let Some(m) = by_id(&doc, "modal") {
        dom::add(&m, "open");
    }
}

fn close_dialog() {
    let Some(doc) = window().and_then(|w| w.document()) else { return };
    if let Some(m) = by_id(&doc, "modal") {
        dom::remove(&m, "open");
    }
}

fn toast(text: &str) {
    let Some(doc) = window().and_then(|w| w.document()) else { return };
    let Some(t) = by_id(&doc, "toast") else { return };
    t.set_text_content(Some(text));
    dom::add(&t, "show");
    dom::timeout(3000.0, move || dom::remove(&t, "show"));
}

/// Before a rehearsal: what it does, and the markdown to save first
fn ask_start(from_here: bool) {
    let what = if from_here {
        "The timer starts at the time planned for this slide, from its first step."
    } else {
        "The presentation goes back to slide 1, the timer to 0."
    };
    dialog(&format!(
        "<h2>{}</h2><p>{what} At the end, you choose for each slide what to keep of its time; it is \
         written in the markdown, in its title: <code>## Title ⏳95🏃3</code>.</p>\
         <p class=\"warn\">⚠ Save the markdown first, and do not edit it during the rehearsal.</p>\
         <div class=\"actions\"><button data-act=\"cancel\">Cancel</button><button class=\"primary\" data-act=\"{}\">● Start</button></div>",
        if from_here { "↳ Rehearse from here" } else { "● Rehearse from the start" },
        if from_here { "start-here" } else { "start" }
    ));
}

fn start_rehearsal(from_here: bool) {
    close_dialog();
    let (h, v, _) = engine::position();
    let leaf = if from_here { engine::leaf_index() } else { 0 };
    if from_here {
        engine::show(h, v, Some(-1), true);
    } else {
        engine::show(0, 0, Some(-1), true);
    }
    let planned = planned();
    with(move |pr, now| {
        let at = if from_here { Timer::planned_before(leaf, &planned) } else { 0.0 };
        pr.timer.reset(now, leaf, at, true);
        pr.timer.rehearsal = Some(leaf);
    });
    update(true);
}

/// The slides measured by the rehearsal, with what to keep by default
fn measured(pr: &Presenter, now: f64) -> Vec<(usize, f64)> {
    let from = pr.timer.rehearsal.unwrap_or(0);
    (from..engine::all_leaves().len()).map(|i| (i, pr.timer.spent_on(now, i))).filter(|(_, s)| *s > 0.0).collect()
}

/// End of a rehearsal: for each slide measured, average, replace or skip
fn ask_finish() {
    let Some(html) = PRESENTER.with(|p| {
        let mut p = p.borrow_mut();
        let pr = p.as_mut()?;
        let now = dom::now();
        pr.timer.stop(now);
        let rows = measured(pr, now);
        pr.keep = rows.iter().map(|&(i, s)| (i, if s < GONE_THROUGH { Keep::Skip } else { Keep::Average })).collect();
        Some(finish_html(pr, now))
    }) else {
        return;
    };
    dialog(&html);
    update(false);
}

fn finish_html(pr: &Presenter, now: f64) -> String {
    let leaves = engine::all_leaves();
    let planned = planned();
    let rows = measured(pr, now);
    // the choice of "All": the one of every slide but those only gone through
    let kept = |k: Keep| rows.iter().filter(|(_, s)| *s >= GONE_THROUGH).all(|(i, _)| pr.keep.get(i) == Some(&k));
    let all = |k: Keep| {
        let checked = if kept(k) && (k != Keep::Skip || pr.keep.values().all(|v| *v == Keep::Skip)) { " checked" } else { "" };
        format!("<label><input type=\"radio\" name=\"all\" data-all=\"{}\"{checked}> {} {}</label>", k.key(), k.name(), k.icon())
    };
    let mut html_rows = String::new();
    for &(i, spent) in &rows {
        let Some((_, _, leaf)) = leaves.get(i) else { continue };
        let plan = planned[i].map_or("–".into(), |d| {
            format!("{}{}", timing::short(d), runs(leaf).map_or(String::new(), |r| format!(" <span class=\"runs\">🏃{r}</span>")))
        });
        let keep = pr.keep.get(&i).copied().unwrap_or(Keep::Skip);
        let buttons: String = Keep::ALL
            .iter()
            .map(|k| {
                format!(
                    "<button data-keep=\"{i}:{}\" class=\"{}\" title=\"{}\">{}</button>",
                    k.key(),
                    if *k == keep { "on" } else { "" },
                    k.name(),
                    k.icon()
                )
            })
            .collect();
        let note = if spent < GONE_THROUGH { " <span class=\"runs\">&lt; 2 s</span>" } else { "" };
        html_rows += &format!(
            "<tr><td class=\"n\">{}</td><td>{}{note}</td><td class=\"t\">{plan}</td><td class=\"t\">{}</td><td><div class=\"keep\">{buttons}</div></td></tr>",
            i + 1,
            dom::esc(&title(leaf, i)),
            timing::short(spent)
        );
    }
    format!(
        "<h2>■ End of the rehearsal: what to keep?</h2>\
         <div class=\"all\">All: {} {} {}</div>\
         <div class=\"scroll\"><table><tr><th>#</th><th>Slide</th><th class=\"t\">⏳ Planned</th><th class=\"t\">This run</th><th>Keep</th></tr>{html_rows}</table></div>\
         <p>📊 averages this run with the former ones (🏃 + 1), ✏️ replaces them, 🗑️ keeps nothing. Written in the markdown; the presentation does not reload.</p>\
         <div class=\"actions\"><button data-act=\"back\">Back to the rehearsal</button><button class=\"primary\" data-act=\"save\">Save</button></div>",
        all(Keep::Average),
        all(Keep::Replace),
        all(Keep::Skip)
    )
}

fn refresh_finish() {
    let Some(html) = PRESENTER.with(|p| p.borrow().as_ref().map(|pr| finish_html(pr, dom::now()))) else { return };
    dialog(&html);
}

/// Sends what to keep to the server (`glitu serve` receives it through the page, see `build`)
fn save_rehearsal() {
    let Some(entries) = PRESENTER.with(|p| {
        let p = p.borrow();
        let pr = p.as_ref()?;
        let now = dom::now();
        let leaves = engine::all_leaves();
        let out = js_sys::Object::new();
        let mut n = 0;
        for (i, spent) in measured(pr, now) {
            let keep = pr.keep.get(&i).copied().unwrap_or(Keep::Skip);
            let Some(slide) = leaves.get(i).and_then(|(_, _, l)| dom::data(l, "slide")) else { continue };
            if keep != Keep::Skip {
                let entry = js_sys::Array::of2(&(spent / 1000.0).into(), &keep.key().into());
                let _ = js_sys::Reflect::set(&out, &slide.into(), &entry);
                n += 1;
            }
        }
        Some((out, n))
    }) else {
        return;
    };
    let (out, n) = entries;
    let win = dom::window();
    if let Ok(json) = js_sys::JSON::stringify(&out) {
        let _ = js_sys::Reflect::set(&win, &"glituChrono".into(), &json);
    }
    if let Ok(ev) = web_sys::Event::new("glitu:chrono") {
        let _ = win.dispatch_event(&ev);
    }
    end_rehearsal();
    toast(&format!("Saved: {n} slide{} timed in the markdown", if n == 1 { "" } else { "s" }));
}

fn end_rehearsal() {
    close_dialog();
    with(|pr, _| {
        pr.timer.rehearsal = None;
        pr.keep.clear();
    });
}

// ---------------------------------------------------------------- the window

/// The clicks of the window: timer, menu, rehearsal, dialogs, size of the notes
fn bind(doc: &Document) {
    let doc2 = doc.clone();
    dom::on::<MouseEvent>(doc, "click", move |e| {
        let Some(target) = e.target().map(JsCast::unchecked_into::<Element>) else { return };
        let menu = by_id(&doc2, "menu");
        // the menu closes on any click but its own button
        if let Some(m) = &menu {
            let on_button = target.closest("#menu-btn").ok().flatten().is_some();
            dom::toggle(m, "open", on_button && !dom::has(m, "open"));
        }
        if target.closest("#play").ok().flatten().is_some() {
            with(|pr, now| if pr.timer.running() { pr.timer.stop(now) } else { pr.timer.start(now) });
        }
        if let Some(b) = target.closest("[data-keep]").ok().flatten() {
            let spec = dom::data(&b, "keep").unwrap_or_default();
            if let Some((i, k)) = spec.split_once(':') {
                let keep = Keep::ALL.into_iter().find(|x| x.key() == k).unwrap_or(Keep::Skip);
                let i: usize = i.parse().unwrap_or(usize::MAX);
                PRESENTER.with(|p| {
                    if let Some(pr) = p.borrow_mut().as_mut() {
                        pr.keep.insert(i, keep);
                    }
                });
                refresh_finish();
            }
        }
        if let Some(b) = target.closest("[data-all]").ok().flatten() {
            let k = dom::data(&b, "all").unwrap_or_default();
            let keep = Keep::ALL.into_iter().find(|x| x.key() == k).unwrap_or(Keep::Average);
            PRESENTER.with(|p| {
                if let Some(pr) = p.borrow_mut().as_mut() {
                    pr.keep.values_mut().for_each(|v| *v = keep);
                }
            });
            refresh_finish();
        }
        let Some(act) = target.closest("[data-act]").ok().flatten().and_then(|b| dom::data(&b, "act")) else { return };
        match act.as_str() {
            "from-start" => ask_start(false),
            "from-here" => ask_start(true),
            "start" => start_rehearsal(false),
            "start-here" => start_rehearsal(true),
            "cancel" => close_dialog(),
            "reset" => with(|pr, now| {
                let running = pr.timer.running();
                pr.timer.reset(now, engine::leaf_index(), 0.0, running);
                pr.timer.rehearsal = None;
            }),
            "layout" => apply_layout(&doc2, LAYOUT),
            "smaller" | "larger" => {
                let mut l = layout();
                l.notes = (l.notes + if act == "larger" { 2.0 } else { -2.0 }).clamp(12.0, 48.0);
                apply_layout(&doc2, l);
            }
            "restart" => {
                let (h, v, _) = engine::position();
                with(|pr, now| pr.timer.restart_slide(now));
                engine::show(h, v, Some(-1), true);
            }
            "finish" => ask_finish(),
            "back" => {
                close_dialog();
                with(|pr, now| pr.timer.start(now));
            }
            "save" => save_rehearsal(),
            "discard" => dialog(
                "<h2>✕ Discard this rehearsal?</h2><p>Nothing is written; the timer goes on.</p>\
                 <div class=\"actions\"><button data-act=\"cancel\">Cancel</button><button class=\"primary\" data-act=\"discarded\">Discard</button></div>",
            ),
            "discarded" => end_rehearsal(),
            _ => {}
        }
        // a button keeps no focus: Space and the arrows go to the presentation
        if let Some(b) = target.closest("button").ok().flatten() {
            let _ = dom::html(&b).blur();
        }
    });
}

fn open() {
    if let Some(w) = window().filter(|w| !w.closed().unwrap_or(true)) {
        let _ = w.focus();
        return;
    }
    let Ok(Some(win)) = dom::window().open_with_url_and_target_and_features("", "glitu-notes", "width=1280,height=800") else { return };
    let Some(doc) = win.document() else { return };
    doc.set_title("Speaker view");
    if let Some(body) = doc.body() {
        body.set_inner_html(&skeleton());
    }
    apply_layout(&doc, layout());
    // A ready preview receives the current position
    dom::on::<web_sys::MessageEvent>(&win, "message", |e| {
        let ready = js_sys::Reflect::get(&e.data(), &"glitu".into()).ok().and_then(|v| v.as_string());
        if ready.as_deref() == Some("ready") {
            update(true);
        }
    });
    bind(&doc);
    forward_keys(&doc);
    drag(&doc, "split", Split::Columns);
    drag(&doc, "vsplit", Split::Slide);
    drag(&doc, "hsplit", Split::Next);
    PRESENTER.with(|p| {
        let mut p = p.borrow_mut();
        match p.as_mut() {
            Some(pr) => pr.win = Some(win),
            None => *p = Some(Presenter { win: Some(win), timer: Timer::new(engine::leaf_index()), keep: BTreeMap::new() }),
        }
    });
    update(true);
}

/// The time moves on even without a change of slide
fn tick() {
    update(false);
    dom::timeout(1000.0, tick);
}

pub fn install() {
    if engine::receiver() {
        return;
    }
    // The timer kept before a reload, whether the view was open or not
    let kept = saved();
    let now = dom::now();
    let away = kept.as_ref().and_then(|v| v["saved"].as_f64()).map_or(0.0, |s| js_sys::Date::now() - s);
    if let Some(timer) = kept.as_ref().and_then(|v| Timer::from_json(&v["timer"], now, away)) {
        PRESENTER.with(|p| *p.borrow_mut() = Some(Presenter { win: None, timer, keep: BTreeMap::new() }));
    }
    let reopen = kept.is_some_and(|v| v["open"] == true);
    engine::on_key("s", open);
    // The new timings written by the server: the planned durations of the page change in place
    dom::on::<web_sys::Event>(&dom::window(), "glitu:timings", |_| update(false));
    engine::on(move |ev| {
        // Page reloaded while the view was open: it reopens (the same window), with its timer
        if matches!(ev, Ev::Ready) && reopen {
            open();
        }
        if !matches!(ev, Ev::Resize) {
            update(true);
        }
    });
    tick();
}
