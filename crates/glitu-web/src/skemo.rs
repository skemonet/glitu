//! Schémas skemo pas à pas : les marqueurs `@n` du schéma (attributs `data-step`,
//! `data-until`, `data-focus` du SVG) suivent le clic courant de la slide.
//! Le build ajoute un fragment invisible (`.sk-step`) par étape du schéma. Une ligne remplacée
//! par une autre au même clic (`li 172.30.0.3 @5-5` puis `li 172.30.0.6 @6`) se transforme en
//! elle, comme le code animé : les caractères gardés glissent, les autres s'effacent ou paraissent ;
//! les nouveaux restent marqués (`.sk-diff`) jusqu'au clic suivant, pour qui aurait raté l'animation.

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

/// Tracé progressif d'une flèche (effet `draw`), comme le lecteur HTML de skemo
fn draw(edge: &Element) {
    let Some(line) = dom::qs(edge, ".line") else { return };
    let Ok(geom) = line.clone().dyn_into::<SvgGeometryElement>() else { return };
    let len = geom.get_total_length();
    let dashes = dom::window().get_computed_style(&line).ok().flatten().and_then(|c| c.get_property_value("stroke-dasharray").ok());
    // un trait en tirets ou pointillés : un masque se trace à sa place, le motif reste
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
    set("transition", "stroke-dashoffset .6s ease");
    set("stroke-dashoffset", "0");
    // la pointe (et celle de départ, `.both`) n'apparaît qu'une fois le trait arrivé
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
        // le masque d'un trait en tirets, s'il y en a un
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

/// Masque qui révèle `line` le long de son tracé : une copie pleine, blanche et plus épaisse,
/// que `draw` trace à la place du trait (qui garde ainsi ses tirets)
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

/// Durée du passage d'une ligne à l'autre, en ms
const MORPH: f64 = 650.0;

/// Les lignes qui en remplacent une autre au clic `s` : (celle qui part, celle qui arrive),
/// sœurs dans la même carte
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

/// Mots d'un texte : suites de lettres et chiffres, ou un autre caractère seul ; (début, mot)
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

/// Paires de caractères gardés d'un texte à l'autre : plus longue sous-suite commune des mots,
/// un mot modifié change en entier (« NodePort » → « ClusterIP » ne garde pas « er »)
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

/// Un caractère du texte `t`, seul, à sa place : un clone du `<text>` (police, couleur)
fn glyph(t: &Element, i: usize, c: char) -> Option<(Element, f64, f64)> {
    let p = t.dyn_ref::<SvgTextContentElement>()?.get_start_position_of_char(i as u32).ok()?;
    let g: Element = t.clone_node().ok()?.dyn_into().ok()?;
    g.set_text_content(Some(&c.to_string()));
    let (x, y) = (f64::from(p.x()), f64::from(p.y()));
    let _ = g.set_attribute("x", &x.to_string());
    let _ = g.set_attribute("y", &y.to_string());
    Some((g, x, y))
}

/// La ligne `old` devient `new` : un calque de caractères posé dans la carte le temps du passage
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
            // gardé : part de sa place dans l'ancienne ligne
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
    // le calque reste, nouveaux caractères marqués, jusqu'au clic suivant (voir `unmorph`)
    dom::timeout(MORPH + 50.0, move || {
        for g in gone {
            g.remove();
        }
    });
}

/// Fin du passage : les vraies lignes reprennent la place du calque, sans fondu
fn unmorph(svg: &Element) {
    let layers = qsa(svg, ".sk-morph");
    if layers.is_empty() {
        return;
    }
    let lines = qsa(svg, "g.li[style]");
    for el in &lines {
        dom::unstyle(el, "opacity");
        // le style recalculé ici, transition coupée : l'opacité change d'un coup
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
        // en avançant d'un clic, les lignes remplacées se transforment
        let before = data(&svg, "sk-at").and_then(|v| v.parse::<u32>().ok());
        let _ = svg.set_attribute("data-sk-at", &s.to_string());
        if before != Some(s) {
            unmorph(&svg);
        }
        let morphing = if animate && before.is_some_and(|b| b < s) && !crate::motion::reduced() { replaced(&svg, s) } else { Vec::new() };
        let mut any = false;
        for el in qsa(&svg, STEPPED) {
            let on = visible_at(&el, s);
            if animate && on && has(&el, "sk-off") && data(&el, "fx").as_deref() == Some("draw") {
                draw(&el);
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

pub fn install() {
    for el in qsa(&engine::slides_element(), "svg.skemo [data-step]") {
        dom::add(&el, "sk-off");
    }
    engine::on(|ev| match ev {
        // À l'impression, chaque schéma montre son état final
        Ev::Ready if engine::printing() => engine::all_leaves().iter().for_each(|(_, _, s)| update(s, false)),
        Ev::Ready | Ev::SlideChanged { .. } => update(&engine::current_slide(), false),
        Ev::FragmentShown(_) | Ev::FragmentHidden(_) => update(&engine::current_slide(), true),
        Ev::Resize => {}
    });
}
