//! skemo diagrams step by step: the `@n` markers of the diagram (`data-step`, `data-until`,
//! `data-focus` attributes of the SVG) follow the current click of the slide.
//! The build adds an invisible fragment (`.sk-step`) per step of the diagram. A line replaced by
//! another at the same click (`| 172.30.0.3 @5-5` then `| 172.30.0.6 @6`) morphs into it, like
//! animated code: the characters kept slide, the others fade out or appear; the new ones stay
//! marked (`.sk-diff`) until the next click, for whoever missed the animation.

use wasm_bindgen::JsCast;
use web_sys::{Element, SvgGeometryElement, SvgTextContentElement};

use crate::dom::{self, data, has, qsa, toggle};
use crate::engine::{self, Ev};

const STEPPED: &str = "[data-step],[data-until],[data-focus]";

fn in_range(r: &str, s: u32) -> bool {
    let (a, b) = r.split_once('-').unwrap_or((r, r));
    match (a.parse::<u32>(), b.parse::<u32>()) {
        (Ok(a), Ok(b)) => a <= s && s <= b,
        _ => false,
    }
}

fn visible_at(el: &Element, s: u32) -> bool {
    let step = data(el, "step").and_then(|v| v.parse().ok()).unwrap_or(0);
    let until = data(el, "until").and_then(|v| v.parse().ok()).unwrap_or(u32::MAX);
    step <= s && s <= until
}

fn focused_at(el: &Element, s: u32) -> bool {
    data(el, "focus").unwrap_or_default().split_whitespace().any(|r| in_range(r, s))
}

/// Progressive drawing of an arrow (`draw` effect), like skemo's HTML player
fn draw(edge: &Element) {
    let Some(line) = dom::qs(edge, ".line") else { return };
    let Ok(geom) = line.clone().dyn_into::<SvgGeometryElement>() else { return };
    let len = geom.get_total_length();
    let dashes = dom::window().get_computed_style(&line).ok().flatten().and_then(|c| c.get_property_value("stroke-dasharray").ok());
    // a dashed or dotted line: a mask is drawn in its place, the pattern stays
    let target = match dashes.filter(|d| !d.is_empty() && d != "none") {
        Some(_) => match reveal_mask(&line) {
            Some(m) => m,
            None => return,
        },
        None => line.clone(),
    };
    let set = |p: &str, v: &str| dom::style(&target, p, v);
    set("transition", "none");
    set("stroke-dasharray", &format!("{len} {len}"));
    set("stroke-dashoffset", &len.to_string());
    let _ = line.get_bounding_client_rect();
    set("transition", &format!("stroke-dashoffset {}ms ease", DRAW_MS));
    set("stroke-dashoffset", "0");
    // the head (and the starting one, `.both`) only appears once the line has arrived
    let heads = qsa(edge, ".head, .tail");
    for h in &heads {
        dom::style(h, "transition", "none");
        dom::style(h, "opacity", "0");
    }
    let line2 = line.clone();
    dom::timeout(560.0, move || {
        for h in &heads {
            dom::style(h, "transition", "opacity .12s ease");
            dom::unstyle(h, "opacity");
        }
    });
    dom::timeout(650.0, move || {
        for p in ["stroke-dasharray", "stroke-dashoffset", "transition"] {
            dom::unstyle(&line2, p);
        }
        // the mask of a dashed line, if there is one
        if let Some(id) = line2.get_attribute("data-sk-mask") {
            let _ = line2.remove_attribute("mask");
            let _ = line2.remove_attribute("data-sk-mask");
            if let Some(svg) = line2.closest("svg").ok().flatten()
                && let Some(m) = dom::qs(&svg, &format!("#{id}"))
            {
                m.remove();
            }
        }
    });
}

thread_local! {
    static MASKS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Mask that reveals `line` along its path: a solid copy, white and thicker, that `draw` draws
/// in place of the line (which keeps its dashes this way)
fn reveal_mask(line: &Element) -> Option<Element> {
    let svg = line.closest("svg").ok().flatten()?;
    let n = MASKS.with(|c| { c.set(c.get() + 1); c.get() });
    let id = format!("sk-reveal-{n}");
    let ns = Some("http://www.w3.org/2000/svg");
    let mask = dom::document().create_element_ns(ns, "mask").ok()?;
    let _ = mask.set_attribute("id", &id);
    let _ = mask.set_attribute("maskUnits", "userSpaceOnUse");
    let path = dom::document().create_element_ns(ns, "path").ok()?;
    let _ = path.set_attribute("d", &line.get_attribute("d")?);
    for (k, v) in [("fill", "none"), ("stroke", "#fff"), ("stroke-width", "12"), ("stroke-linecap", "round")] {
        let _ = path.set_attribute(k, v);
    }
    let _ = mask.append_child(&path);
    let _ = svg.append_child(&mask);
    let _ = line.set_attribute("mask", &format!("url(#{id})"));
    let _ = line.set_attribute("data-sk-mask", &id);
    Some(path)
}

/// Duration of the change from one line to the other, in ms
const MORPH: f64 = 650.0;

/// The lines that replace another one at click `s`: (the one leaving, the one arriving),
/// siblings in the same card
fn replaced(svg: &Element, s: u32) -> Vec<(Element, Element)> {
    let mut pairs = Vec::new();
    for old in qsa(svg, "g.li[data-until]") {
        if data(&old, "until").and_then(|u| u.parse::<u32>().ok()) != Some(s.wrapping_sub(1)) {
            continue;
        }
        let mut next = old.next_element_sibling();
        while let Some(n) = next {
            if has(&n, "li") && data(&n, "step").and_then(|v| v.parse::<u32>().ok()) == Some(s) {
                pairs.push((old.clone(), n));
                break;
            }
            next = n.next_element_sibling();
        }
    }
    pairs
}

/// Words of a text: runs of letters and digits, or another character alone; (start, word)
fn words(a: &[char]) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for (i, &c) in a.iter().enumerate() {
        match out.last_mut() {
            Some((_, w)) if c.is_alphanumeric() && w.chars().all(char::is_alphanumeric) => w.push(c),
            _ => out.push((i, c.to_string())),
        }
    }
    out
}

/// Pairs of characters kept from one text to the other: longest common subsequence of the
/// words, a modified word changes entirely ("NodePort" → "ClusterIP" does not keep "er")
fn kept(a: &[char], b: &[char]) -> Vec<(usize, usize)> {
    let (a, b) = (words(a), words(b));
    let mut t = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            t[i][j] = if a[i].1 == b[j].1 { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        if a[i].1 == b[j].1 {
            out.extend((0..a[i].1.chars().count()).map(|k| (a[i].0 + k, b[j].0 + k)));
            i += 1;
            j += 1;
        } else if t[i + 1][j] >= t[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// One character of the text `t`, alone, in its place: a clone of the `<text>` (font, colour)
fn glyph(t: &Element, i: usize, c: char) -> Option<(Element, f64, f64)> {
    let p = t.dyn_ref::<SvgTextContentElement>()?.get_start_position_of_char(i as u32).ok()?;
    let g: Element = t.clone_node().ok()?.dyn_into().ok()?;
    g.set_text_content(Some(&c.to_string()));
    let (x, y) = (f64::from(p.x()), f64::from(p.y()));
    let _ = g.set_attribute("x", &x.to_string());
    let _ = g.set_attribute("y", &y.to_string());
    Some((g, x, y))
}

/// The line `old` becomes `new`: a layer of characters laid in the card during the change
fn morph(old: &Element, new: &Element) {
    let (Some(ta), Some(tb), Some(card)) = (dom::qs(old, "text"), dom::qs(new, "text"), old.parent_element()) else { return };
    let a: Vec<char> = ta.text_content().unwrap_or_default().chars().collect();
    let b: Vec<char> = tb.text_content().unwrap_or_default().chars().collect();
    let pairs = kept(&a, &b);
    let Ok(layer) = dom::document().create_element_ns(Some("http://www.w3.org/2000/svg"), "g") else { return };
    let _ = layer.set_attribute("class", "sk-morph");
    let _ = card.append_child(&layer);
    for el in [old, new] {
        dom::style(el, "transition", "none");
        dom::style(el, "opacity", "0");
    }
    let mut moves = Vec::new();
    for (j, &c) in b.iter().enumerate() {
        let Some((g, bx, by)) = glyph(&tb, j, c) else { continue };
        match pairs.iter().find(|(_, pj)| *pj == j).and_then(|(i, _)| glyph(&ta, *i, a[*i])) {
            // kept: leaves from its place in the old line
            Some((_, ax, ay)) => dom::style(&g, "transform", &format!("translate({}px, {}px)", ax - bx, ay - by)),
            None => {
                dom::style(&g, "opacity", "0");
                let _ = g.set_attribute("class", "sk-diff");
            }
        }
        let _ = layer.append_child(&g);
        moves.push(g);
    }
    let mut gone = Vec::new();
    for (i, &c) in a.iter().enumerate() {
        if pairs.iter().any(|(pi, _)| *pi == i) {
            continue;
        }
        if let Some((g, _, _)) = glyph(&ta, i, c) {
            let _ = layer.append_child(&g);
            gone.push(g);
        }
    }
    let _ = layer.get_bounding_client_rect();
    let fading = gone.clone();
    dom::raf(move |_| {
        for g in &moves {
            dom::style(g, "transition", &format!("transform {}ms ease, opacity {}ms ease {}ms", MORPH * 0.8, MORPH * 0.5, MORPH * 0.4));
            dom::style(g, "transform", "none");
            dom::style(g, "opacity", "1");
        }
        for g in &fading {
            dom::style(g, "transition", &format!("opacity {}ms ease", MORPH * 0.4));
            dom::style(g, "opacity", "0");
        }
    });
    // the layer stays, new characters marked, until the next click (see `unmorph`)
    dom::timeout(MORPH + 50.0, move || {
        for g in gone {
            g.remove();
        }
    });
}

/// End of the change: the real lines take back the place of the layer, without fading
fn unmorph(svg: &Element) {
    let layers = qsa(svg, ".sk-morph");
    if layers.is_empty() {
        return;
    }
    let lines = qsa(svg, "g.li[style]");
    for el in &lines {
        dom::unstyle(el, "opacity");
        // the style computed here, transition cut: the opacity changes at once
        let _ = el.get_bounding_client_rect();
    }
    for l in layers {
        l.remove();
    }
    dom::raf(move |_| {
        dom::raf(move |_| {
            for el in &lines {
                dom::unstyle(el, "transition");
            }
        });
    });
}

fn update(slide: &Element, animate: bool) {
    let s = engine::current_step(slide);
    for svg in qsa(slide, "svg.skemo") {
        // moving forward by one click, the replaced lines morph
        let before = data(&svg, "sk-at").and_then(|v| v.parse::<u32>().ok());
        let _ = svg.set_attribute("data-sk-at", &s.to_string());
        if before != Some(s) {
            unmorph(&svg);
        }
        let morphing = if animate && before.is_some_and(|b| b < s) && !crate::motion::reduced() { replaced(&svg, s) } else { Vec::new() };
        let after_arrow = if animate && !crate::motion::reduced() { cards_after_arrows(&svg, s) } else { Vec::new() };
        let mut any = false;
        for el in qsa(&svg, STEPPED) {
            let on = visible_at(&el, s);
            if animate && on && has(&el, "sk-off") && data(&el, "fx").as_deref() == Some("draw") {
                draw(&el);
            }
            if on && after_arrow.contains(&el) {
                // the arrow is drawn first, the card appears once it has arrived
                let (svg, el) = (svg.clone(), el.clone());
                dom::timeout(DRAW_MS, move || {
                    if data(&svg, "sk-at").as_deref() == Some(&s.to_string()) {
                        dom::remove(&el, "sk-off");
                    }
                });
                continue;
            }
            toggle(&el, "sk-off", !on);
            let f = on && focused_at(&el, s);
            toggle(&el, "sk-focus-on", f);
            any |= f;
        }
        toggle(&svg, "sk-focus", any);
        for (old, new) in morphing {
            morph(&old, &new);
        }
    }
}

/// Duration of the drawing of an arrow (`draw`), in ms
const DRAW_MS: f64 = 600.0;

/// The cards appearing at step `s` with an arrow drawn between them and a card already shown:
/// they wait for the arrow
fn cards_after_arrows(svg: &Element, s: u32) -> Vec<Element> {
    let shown = |id: &str| {
        dom::qs(svg, &format!(".card[data-id=\"{id}\"]")).is_some_and(|c| !has(&c, "sk-off"))
    };
    let mut ids = Vec::new();
    for edge in qsa(svg, ".edge.sk-off[data-fx=\"draw\"]").into_iter().filter(|e| visible_at(e, s)) {
        let (Some(from), Some(to)) = (data(&edge, "from"), data(&edge, "to")) else { continue };
        match (shown(&from), shown(&to)) {
            (true, false) => ids.push(to),
            (false, true) => ids.push(from),
            _ => {}
        }
    }
    ids.iter()
        .filter_map(|id| dom::qs(svg, &format!(".card.sk-off[data-id=\"{id}\"]")))
        .filter(|c| visible_at(c, s))
        .collect()
}

pub fn install() {
    for el in qsa(&engine::slides_element(), "svg.skemo [data-step]") {
        dom::add(&el, "sk-off");
    }
    engine::on(|ev| match ev {
        // When printing, each diagram shows its final state
        Ev::Ready if engine::printing() => engine::all_leaves().iter().for_each(|(_, _, s)| update(s, false)),
        Ev::Ready | Ev::SlideChanged { .. } => update(&engine::current_slide(), false),
        Ev::FragmentShown(_) | Ev::FragmentHidden(_) => update(&engine::current_slide(), true),
        Ev::Resize => {}
    });
}
