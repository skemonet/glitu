//! Shared template of the themes (`"template": true` in glitu.json, styles in glitu/template.css):
//! "SECTION / Topic" header, logos, the theme's backdrop (`"backdrop"`: animated on the cover and
//! the chapters, still behind the visuals), frieze that centres on the card of the step, footer
//! of progress by chapter.


use web_sys::Element;

use super::crystal::{self, Crystal, Motion};
use super::backdrops::{self, Animated};
use crate::dom::{self, add, closest, data, has, qs, qsa, style, toggle};
use crate::engine::{self, Ev};

fn header(slide: &Element, logo: &str) {
    let spec = data(slide, "header").unwrap_or_default();
    let mut parts = spec.split('|').map(str::trim);
    let (kicker, topic) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let el = dom::create("header");
    el.set_class_name("tpl-header");
    let mut html = format!("<span class=\"tpl-kicker\">{}</span>", dom::esc(kicker));
    if !topic.is_empty() {
        html += &format!("<span class=\"tpl-sep\">/</span><span class=\"tpl-topic\">{}</span>", dom::esc(topic));
    }
    if !logo.is_empty() {
        html += &format!("<img class=\"tpl-logo\" src=\"{}\" alt=\"\">", dom::esc(logo));
    }
    el.set_inner_html(&html);
    let _ = slide.prepend_with_node_1(&el);
}


/// Offset that centres the element `[data-id=id]` of the strip in the visible part (right of the panel)
fn center_on(track: &Element, id: &str) -> f64 {
    let Some(el) = qs(track, &format!("[data-id=\"{id}\"]")) else {
        dom::warn(&format!("[crystal] data-pan-to: element not found: {id}"));
        return 0.0;
    };
    let (t, r) = (track.get_bounding_client_rect(), el.get_bounding_client_rect());
    let ow = dom::html(track).offset_width() as f64;
    let k = if ow > 0.0 && t.width() > 0.0 { t.width() / ow } else { 1.0 };
    let view = track.parent_element().unwrap();
    let left = closest(&view, "section").and_then(|s| qs(&s, ".panel")).map_or(0.0, |p| dom::html(&p).offset_width() as f64);
    left + (dom::html(&view).offset_width() as f64 - left) / 2.0 - (r.left() + r.width() / 2.0 - t.left()) / k
}

fn update_pan(slide: &Element) {
    for track in qsa(slide, ".pan-track") {
        let section = closest(&track, "section").unwrap();
        let current = qsa(&section, ".pan-step").into_iter().filter(|s| has(s, "visible")).last().unwrap_or_else(|| track.clone());
        let x = match data(&current, "pan-to") {
            Some(id) => center_on(&track, &id),
            None => data(&current, "pan").and_then(|p| p.parse().ok()).unwrap_or(0.0),
        };
        style(&track, "transform", &format!("translateX({x}px)"));
    }
}

/// A chapter, as slide ranks in reading order (vertical slides included)
struct Chapter {
    start: usize,
    /// Rank of its interlude (where a click on its segment leads)
    at: usize,
    end: usize,
    num: String,
    title: String,
}

// Chapters: each `.chapter .num` opens one; the slides before join the first one
fn chapters(slides: &[Element]) -> Vec<Chapter> {
    let mut list: Vec<Chapter> = Vec::new();
    for (i, s) in slides.iter().enumerate() {
        if let Some(num) = qs(s, ".chapter .num") {
            let title = qs(s, ".chapter h2").and_then(|h| h.text_content()).unwrap_or_default().trim().to_string();
            list.push(Chapter { start: i, at: i, end: 0, num: num.text_content().unwrap_or_default().trim().to_string(), title });
        }
    }
    if list.is_empty() {
        list.push(Chapter { start: 0, at: 0, end: 0, num: String::new(), title: String::new() });
    }
    list[0].start = 0;
    for k in 0..list.len() {
        list[k].end = list.get(k + 1).map_or(slides.len(), |c| c.start);
    }
    list
}

struct Progress {
    parts: Vec<Chapter>,
    segs: Vec<Element>,
    label: Element,
    count: Element,
    total: usize,
}

fn progress() -> Progress {
    let leaves = engine::all_leaves();
    let slides: Vec<Element> = leaves.iter().map(|(_, _, s)| s.clone()).collect();
    let parts = chapters(&slides);
    let el = dom::create("footer");
    el.set_class_name("tpl-progress");
    el.set_inner_html("<span class=\"tpl-chapter\"></span><div class=\"tpl-segments\"></div><span class=\"tpl-count\"></span>");
    let label = qs(&el, ".tpl-chapter").unwrap();
    let count = qs(&el, ".tpl-count").unwrap();
    let box_ = qs(&el, ".tpl-segments").unwrap();
    let segs = parts.iter().map(|c| {
        let seg = dom::create("span");
        seg.set_class_name("tpl-seg");
        style(&seg, "flex", &(c.end - c.start).to_string());
        let _ = seg.set_attribute("title", &c.title);
        seg.set_inner_html("<i></i>");
        let (h, v, _) = leaves[c.at];
        dom::on::<web_sys::Event>(&seg, "click", move |_| engine::show(h, v, None, true));
        let _ = box_.append_child(&seg);
        seg
    }).collect();
    let _ = engine::slides_element().append_child(&el);
    Progress { parts, segs, label, count, total: slides.len() }
}

impl Progress {
    fn update(&self) {
        let i = engine::leaf_index();
        let (steps, done) = engine::fragment_counts(&engine::current_slide());
        let pos = i as f64 + (done as f64 + 1.0) / (steps as f64 + 1.0); // the current slide counts in proportion to its steps
        for (k, c) in self.parts.iter().enumerate() {
            let fill = ((pos - c.start as f64) / (c.end - c.start) as f64).clamp(0.0, 1.0);
            style(&self.segs[k], "--fill", &format!("{}%", fill * 100.0));
            toggle(&self.segs[k], "current", i >= c.start && i < c.end);
        }
        let c = self.parts.iter().find(|c| i >= c.start && i < c.end);
        self.label.set_inner_html(&match c {
            Some(c) if i >= c.at && !c.num.is_empty() => format!("<b>{}</b>{}", dom::esc(&c.num), dom::esc(&c.title)),
            _ => String::new(),
        });
        self.count.set_inner_html(&format!("{:02} <span>/ {:02}</span>", i + 1, self.total));
    }
}

/// The theme's backdrop: crystal, or a scene of `backdrops`
enum Backdrop {
    Crystal(Crystal),
    Scene(Animated),
}

impl Backdrop {
    fn new(host: &Element, name: &str) -> Backdrop {
        match backdrops::scene(name) {
            Some(scene) => Backdrop::Scene(Animated::new(host, scene)),
            None => {
                if !name.is_empty() && name != "crystal" {
                    dom::warn(&format!("[template] unknown backdrop: {name} (crystal instead)"));
                }
                Backdrop::Crystal(Crystal::new(host, crystal::Options::default()))
            }
        }
    }

    fn redraw(&self) {
        match self {
            Backdrop::Crystal(c) => c.redraw(),
            Backdrop::Scene(s) => s.redraw(),
        }
    }

    fn snapshot(&self) -> Option<String> {
        match self {
            Backdrop::Crystal(c) => c.snapshot(),
            Backdrop::Scene(s) => s.snapshot(),
        }
    }

    fn set_motion(&self, motion: Motion, dim: f64) {
        match self {
            Backdrop::Crystal(c) => c.set_motion(motion),
            Backdrop::Scene(s) => s.set_motion(motion, dim),
        }
    }
}

/// Dimming of the backdrop for this slide: whole under the cover, erased on the text side under a
/// full-width text slide, in between next to a panel and under the chapters (framed like the
/// content: going from a chapter to its slides moves nothing)
fn dim(slide: &Element) -> f64 {
    if qs(slide, ".cover").is_some() {
        0.0
    } else if qs(slide, ".panel, .chapter").is_some() {
        0.6
    } else {
        1.0
    }
}

pub fn install(logo: &str, backdrop: &str) {
    let root = engine::slides_element();
    for s in qsa(&root, "section[data-header]") {
        header(&s, logo);
    }
    if !logo.is_empty() {
        for c in qsa(&root, ".chapter, .cover") {
            let _ = c.insert_adjacent_html("beforeend", &format!("<img class=\"tpl-logo\" src=\"{}\" alt=\"\">", dom::esc(logo)));
        }
    }
    for el in qsa(&root, "[data-bg]") {
        let base = dom::document().base_uri().ok().flatten().unwrap_or_default();
        let url = web_sys::Url::new_with_base(&data(&el, "bg").unwrap_or_default(), &base).map(|u| u.href()).unwrap_or_default();
        style(&el, "--bg", &format!("url(\"{url}\")"));
        if !has(&el, "blur") {
            style(&el, "background-image", &format!("url(\"{url}\")"));
            add(&el, "photo");
        }
    }

    // One backdrop under all the slides: it does not change from one slide to another, each slide
    // only says whether it runs (data-backdrop="animated") or stands still ("still")
    let host = dom::create("div");
    host.set_class_name("tpl-backdrop");
    let _ = root.prepend_with_node_1(&host);
    if engine::printing() {
        // Outside the page, the size of a slide: it only serves to take the image of the pages
        let c = engine::config();
        style(&host, "width", &format!("{}px", c.width));
        style(&host, "height", &format!("{}px", c.height));
    }
    let backdrop = Backdrop::new(&host, backdrop);
    let motion = || {
        let slide = engine::current_slide();
        let motion = match qs(&slide, "[data-backdrop=\"animated\"]") {
            Some(_) => Motion::Animated { chase: qs(&slide, ".chapter, .cover").is_some() },
            None => Motion::Still,
        };
        (motion, dim(&slide))
    };

    let progress = progress();
    engine::on(move |ev| {
        match ev {
            Ev::Ready | Ev::SlideChanged { .. } | Ev::FragmentShown(_) | Ev::FragmentHidden(_) => {
                update_pan(&engine::current_slide());
                progress.update();
            }
            Ev::Resize => {}
        }
        if matches!(ev, Ev::Ready | Ev::Resize) {
            backdrop.redraw();
        }
        if engine::printing() {
            // A printed page cannot share the backdrop: each one gets its image
            if let (Ev::Ready, Some(png)) = (ev, backdrop.snapshot()) {
                style(&engine::slides_element(), "--tpl-backdrop-image", &format!("url(\"{png}\")"));
            }
        } else if matches!(ev, Ev::Ready | Ev::SlideChanged { .. }) {
            let (m, d) = motion();
            backdrop.set_motion(m, d);
        }
    });
}
