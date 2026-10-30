//! What reveal.js lays around the slides, with its markup (which reveal.css dresses): slide
//! backdrops (`data-background…`), navigation arrows, progress bar, slide number. Each element
//! is enabled by the theme's settings.

use web_sys::Element;

use crate::dom::{self, add, data, has, qs, qsa, style, toggle};
use crate::engine::{self, Ev};

// ---------------------------------------------------------------- backdrops

/// CSS colour → (r, g, b), for the forms #rgb, #rrggbb, rgb()/rgba()
fn rgb(color: &str) -> Option<(f64, f64, f64)> {
    let c = color.trim();
    if let Some(hex) = c.strip_prefix('#') {
        let hex: String = if hex.len() == 3 { hex.chars().flat_map(|ch| [ch, ch]).collect() } else { hex.chars().take(6).collect() };
        let n = u32::from_str_radix(&hex, 16).ok()?;
        return Some((((n >> 16) & 255) as f64, ((n >> 8) & 255) as f64, (n & 255) as f64));
    }
    let inner = c.strip_prefix("rgba(").or_else(|| c.strip_prefix("rgb("))?.trim_end_matches(')');
    let v: Vec<f64> = inner.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    (v.len() >= 3).then(|| (v[0], v[1], v[2]))
}

fn is_image(v: &str) -> bool {
    let l = v.to_lowercase();
    l.starts_with("url(") || [".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".avif"].iter().any(|e| l.contains(e))
}

fn background_for(slide: &Element) -> Element {
    let el = dom::create("div");
    el.set_class_name("slide-background");
    let content = dom::create("div");
    content.set_class_name("slide-background-content");
    let _ = el.append_child(&content);
    let mut color = None;
    if let Some(bg) = data(slide, "background") {
        if is_image(&bg) {
            let url = bg.trim_start_matches("url(").trim_end_matches(')');
            style(&content, "background-image", &format!("url(\"{url}\")"));
        } else {
            style(&el, "background", &bg);
            color = Some(bg);
        }
    }
    if let Some(c) = data(slide, "background-color") {
        style(&el, "background-color", &c);
        color = Some(c);
    }
    if let Some(img) = data(slide, "background-image") {
        style(&content, "background-image", &format!("url(\"{img}\")"));
    }
    for (attr, prop) in [("size", "background-size"), ("position", "background-position"), ("repeat", "background-repeat"), ("opacity", "opacity")] {
        if let Some(v) = data(slide, &format!("background-{attr}")) {
            style(&content, prop, &v);
        }
    }
    // Contrast: the theme adapts the colour of the text (and of the arrows) to light or dark backdrops
    if let Some((r, g, b)) = color.as_deref().and_then(rgb) {
        let brightness = (r * 299.0 + g * 587.0 + b * 114.0) / 1000.0;
        add(slide, if brightness < 128.0 { "has-dark-background" } else { "has-light-background" });
    }
    el
}

struct Backgrounds {
    /// For each horizontal slide: its backdrop, then those of its vertical slides
    horizontal: Vec<(Element, Vec<Element>)>,
}

fn backgrounds() -> Backgrounds {
    let container = dom::create("div");
    container.set_class_name("backgrounds");
    let mut horizontal = Vec::new();
    for (slide, leaves) in engine::slides().iter().zip(engine::leaves()) {
        let stack = leaves.len() > 1;
        let el = background_for(slide);
        let mut vertical = Vec::new();
        if stack {
            add(&el, "stack");
            for leaf in &leaves {
                let v = background_for(leaf);
                let _ = el.append_child(&v);
                vertical.push(v);
            }
        }
        let _ = container.append_child(&el);
        horizontal.push((el, vertical));
    }
    let _ = engine::root().insert_before(&container, engine::slides_element().next_sibling().as_ref());
    Backgrounds { horizontal }
}

fn set_state(el: &Element, state: &str) {
    for c in ["past", "present", "future"] {
        dom::remove(el, c);
    }
    add(el, state);
}

impl Backgrounds {
    fn update(&self) {
        let (h, v, _) = engine::position();
        for (x, (el, vertical)) in self.horizontal.iter().enumerate() {
            let st = if x < h { "past" } else if x > h { "future" } else { "present" };
            set_state(el, st);
            style(el, "display", if x.abs_diff(h) < 3 { "block" } else { "none" });
            for (y, ve) in vertical.iter().enumerate() {
                if x == h {
                    set_state(ve, if y < v { "past" } else if y > v { "future" } else { "present" });
                }
                style(ve, "display", if engine::distance(x, y, h, v) < 3 { "block" } else { "none" });
            }
        }
        // The contrast of the current slide goes up to .reveal
        let cur = engine::current_slide();
        let r = engine::root();
        for c in ["has-dark-background", "has-light-background"] {
            toggle(&r, c, has(&cur, c));
        }
    }
}

// ---------------------------------------------------------------- arrows

struct Controls {
    left: Element,
    right: Element,
    up: Element,
    down: Element,
}

fn controls() -> Controls {
    let aside = dom::create("aside");
    aside.set_class_name("controls");
    let _ = aside.set_attribute("data-controls-layout", "bottom-right");
    let _ = aside.set_attribute("data-controls-back-arrows", "faded");
    aside.set_inner_html(
        "<button class=\"navigate-left\" aria-label=\"previous slide\"><div class=\"controls-arrow\"></div></button>\n\
         \t\t\t<button class=\"navigate-right\" aria-label=\"next slide\"><div class=\"controls-arrow\"></div></button>\n\
         \t\t\t<button class=\"navigate-up\" aria-label=\"above slide\"><div class=\"controls-arrow\"></div></button>\n\
         \t\t\t<button class=\"navigate-down\" aria-label=\"below slide\"><div class=\"controls-arrow\"></div></button>",
    );
    style(&aside, "display", "block");
    let _ = engine::root().append_child(&aside);
    let button = |c: &str, f: fn()| {
        let b = qs(&aside, &format!(".navigate-{c}")).unwrap();
        dom::on::<web_sys::Event>(&b, "click", move |e| {
            e.prevent_default();
            f();
        });
        b
    };
    Controls { left: button("left", engine::left), right: button("right", engine::right), up: button("up", engine::up), down: button("down", engine::down) }
}

impl Controls {
    fn update(&self) {
        let (l, r, u, d) = engine::routes();
        for (el, on) in [(&self.left, l), (&self.right, r), (&self.up, u), (&self.down, d)] {
            dom::remove(el, "fragmented");
            toggle(el, "enabled", on);
            if on { let _ = el.remove_attribute("disabled"); } else { let _ = el.set_attribute("disabled", "disabled"); }
        }
        // Steps left: the arrow of the slide's axis carries the "fragmented" mark
        let (prev, next) = engine::fragment_routes();
        let cur = engine::current_slide();
        let vertical = cur.parent_element().is_some_and(|p| p.tag_name() == "SECTION" && qsa(&p, ":scope > section").len() > 1);
        let (back, forward) = if vertical { (&self.up, &self.down) } else { (&self.left, &self.right) };
        for (el, on) in [(back, prev), (forward, next)] {
            if on {
                add(el, "fragmented");
                add(el, "enabled");
                let _ = el.remove_attribute("disabled");
            }
        }
        // Tutorial: the arrow towards a direction not yet taken bounces
        let (nav_h, nav_v) = engine::navigated();
        let (_, v, _) = engine::position();
        toggle(&self.down, "highlight", !nav_v && d);
        toggle(&self.right, "highlight", !nav_h && r && v == 0);
    }
}

// ---------------------------------------------------------------- progress and number

fn progress_bar() -> Element {
    let el = dom::create("div");
    el.set_class_name("progress");
    el.set_inner_html("<span></span>");
    style(&el, "display", "block");
    let _ = engine::root().append_child(&el);
    qs(&el, "span").unwrap()
}

/// Progress as in reveal.js: slides past, plus 0.9 × the share of steps shown
fn progress_value() -> f64 {
    let leaves = engine::leaves();
    let total: usize = leaves.iter().map(Vec::len).sum();
    let (h, v, _) = engine::position();
    let past: usize = leaves.iter().take(h).map(Vec::len).sum::<usize>() + v;
    let cur = engine::current_slide();
    let all = qsa(&cur, ".fragment").len();
    let shown = qsa(&cur, ".fragment.visible").len();
    let frac = if all > 0 { shown as f64 / all as f64 * 0.9 } else { 0.0 };
    if total <= 1 { 1.0 } else { ((past as f64 + frac) / (total - 1) as f64).min(1.0) }
}

fn slide_number() -> Element {
    let el = dom::create("div");
    el.set_class_name("slide-number");
    style(&el, "display", "block");
    let _ = engine::root().append_child(&el);
    el
}

fn number_html() -> String {
    let (h, v, _) = engine::position();
    let url = if v > 0 { format!("#/{h}/{v}") } else { format!("#/{h}") };
    if v > 0 {
        format!(
            "<a href=\"{url}\">\n\t\t\t\t\t<span class=\"slide-number-a\">{}</span>\n\t\t\t\t\t<span class=\"slide-number-delimiter\">.</span>\n\t\t\t\t\t<span class=\"slide-number-b\">{}</span>\n\t\t\t\t\t</a>",
            h + 1,
            v + 1
        )
    } else {
        format!("<a href=\"{url}\">\n\t\t\t\t\t<span class=\"slide-number-a\">{}</span>\n\t\t\t\t\t</a>", h + 1)
    }
}

/// The presentation's logo (`logo:` in the front matter), in a corner of every slide, for the
/// themes without the template (which puts it in its header)
pub fn logo(src: &str) {
    let img = dom::create("img");
    img.set_class_name("glitu-logo");
    let _ = img.set_attribute("src", src);
    let _ = img.set_attribute("alt", "");
    let _ = engine::root().append_child(&img);
}

pub fn install() {
    let c = engine::config();
    let backgrounds = backgrounds();
    let number = c.slide_number.then(slide_number);
    let controls = c.controls.then(controls);
    let progress = c.progress.then(progress_bar);
    engine::on(move |ev| {
        if matches!(ev, Ev::Resize) {
            return;
        }
        backgrounds.update();
        if let Some(n) = &number {
            n.set_inner_html(&number_html());
        }
        if let Some(ctl) = &controls {
            ctl.update();
        }
        if let Some(p) = &progress {
            style(p, "transform", &format!("scaleX({})", progress_value()));
        }
    });
}
