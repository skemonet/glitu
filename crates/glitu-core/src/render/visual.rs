//! The visual of a slide: skemo diagram, code window, formula, emojis, terminal, image or video,
//! with its caption under it and, on an image or a video, its focus zones.

use std::collections::{BTreeMap, BTreeSet};

use super::Assets;
use super::steps::{Appear, appear, fx_attr, replacing, stepped_html};
use crate::highlight;
use crate::html::{MARKS, esc, inline, span_html};
use crate::markdown::{Block, Fence, Media, Stepped, marker, step_of};

/// What the previous slides have already shown
#[derive(Default)]
pub(super) struct Seen {
    /// Terminals of the presentation (each one has its id)
    casts: u32,
    /// Project files shown so far (`file=` of a code window), in order of appearance
    files: Vec<String>,
}

pub(super) fn visual_html(v: &Block, assets: &mut dyn Assets, seen: &mut Seen) -> Result<String, String> {
    Ok(match v {
        Block::Fence(f) if f.attrs.contains_key("caption") => {
            let mut bare = f.clone();
            let caption = bare.attrs.remove("caption");
            let visual = visual_html(&Block::Fence(bare), assets, seen)?;
            format!("{visual}{}", caption_html(caption.as_deref(), &f.appear, ""))
        }
        Block::Fence(f) if f.lang == "skemo" => skemo_html(f, assets)?,
        Block::Fence(f) if f.lang == "emoji" => glyphs_html(f),
        Block::Fence(f) if f.lang == "math" => formula_html(f),
        Block::Fence(f) => match assets.diagram(&f.lang, &f.content, &f.attrs) {
            Some(svg) => {
                let a = appear(&f.appear);
                format!("<div class=\"diagram{}\"{}{}>{}{}</div>", a.class, a.attrs, a.fx, svg?, a.end)
            }
            None => code_html(f, &mut seen.files),
        },
        Block::Image(m) if m.src.ends_with(".cast") => {
            seen.casts += 1;
            let cast = cast_html(&assets.cast(&m.src)?, &m.alt, seen.casts);
            format!("{cast}{}", caption_html(m.caption.as_deref(), &m.appear, ""))
        }
        Block::Image(m) => media_html(m, "visual")?,
        _ => String::new(),
    })
}

/// The line under a visual (`caption="…"`, or the title of an image), which appears with it
fn caption_html(caption: Option<&str>, appear_at: &Stepped, class: &str) -> String {
    let Some(text) = caption.filter(|c| !c.trim().is_empty()) else { return String::new() };
    let mut at = appear_at.clone();
    (at.until, at.marks) = (None, Vec::new());
    let a = appear(&at);
    format!("<p class=\"visual-caption{class}{}\"{}>{}</p>", a.class, a.attrs, inline(text))
}

/// An image or a video (`class`: `visual`, or `layout-media` beside the text), then its focus
/// zones (`focus="…"`) and its caption; the zones are placed over the picture by the engine
pub(super) fn media_html(m: &Media, class: &str) -> Result<String, String> {
    let what = |e: String| format!("`{}`: {e}", m.src);
    // an image has no content to hide it after its last click (`@2-4`): it stays
    let mut at = m.appear.clone();
    at.until = None;
    let a = appear(&at);
    let extra: String = m.attrs.get("class").map(|c| format!(" {}", esc(c))).unwrap_or_default();
    let classes = format!("{class}{extra}{}", a.class);
    // after the zones, which follow the medium (the engine finds it before them)
    let mut steps = String::new();
    let el = if m.is_video() {
        let flag = |k: &str| m.attrs.contains_key(k);
        let pauses = match m.attrs.get("pauses") {
            Some(p) => pauses_of(p).map_err(what)?,
            None => Vec::new(),
        };
        // one invisible fragment per pause: the click that resumes the video
        steps = (1..=pauses.len() as u32)
            .map(|k| {
                format!("<span class=\"fragment video-step\" data-fragment-index=\"{}\"></span>", m.appear.step + k)
            })
            .collect();
        let mut attrs = String::new();
        if !pauses.is_empty() {
            let at: Vec<String> = pauses.iter().map(|t| t.to_string()).collect();
            attrs += &format!(" data-pauses=\"{}\" data-at=\"{}\"", at.join(" "), m.appear.step);
        } else if flag("autoplay") {
            attrs += " data-autoplay";
        }
        for (key, attr) in [("muted", " muted"), ("loop", " loop")] {
            if flag(key) {
                attrs += attr;
            }
        }
        if !flag("nocontrols") {
            attrs += " controls";
        }
        if let Some(poster) = m.attrs.get("poster") {
            attrs += &format!(" poster=\"{}\"", esc(poster));
        }
        if !m.alt.is_empty() {
            attrs += &format!(" aria-label=\"{}\"", esc(&m.alt));
        }
        format!(
            "<video class=\"{classes}\" data-src=\"{}\" preload=\"metadata\" playsinline{attrs}{}{}></video>",
            esc(&m.src),
            a.attrs,
            a.fx
        )
    } else {
        format!("<img class=\"{classes}\" src=\"{}\" alt=\"{}\"{}{}>", esc(&m.src), esc(&m.alt), a.attrs, a.fx)
    };
    let zones = match m.attrs.get("focus") {
        Some(focus) => zones_html(focus, m.is_video(), &m.src).map_err(what)?,
        None => String::new(),
    };
    let zones = if zones.is_empty() {
        zones
    } else {
        format!("<div class=\"zones{}\"{} aria-hidden=\"true\">{zones}</div>", a.class, a.attrs)
    };
    let on_media = if class == "visual" { "" } else { " on-media" };
    Ok(format!("{el}{zones}{}{steps}", caption_html(m.caption.as_deref(), &m.appear, on_media)))
}

/// `12`, `1:05`, `1:02:03.5` → seconds
fn seconds(t: &str) -> Option<f64> {
    let mut total = 0.0;
    for part in t.split(':') {
        let v: f64 = part.parse().ok().filter(|v: &f64| v.is_finite() && *v >= 0.0)?;
        total = total * 60.0 + v;
    }
    Some(total)
}

/// `pauses="0:12 1:05"`: the times where the video stops, in increasing order
fn pauses_of(p: &str) -> Result<Vec<f64>, String> {
    let mut out: Vec<f64> = Vec::new();
    for t in p.split([' ', ',']).filter(|t| !t.is_empty()) {
        let s = seconds(t).ok_or_else(|| format!("pauses: `{t}` is not a time (12, 1:05, 1:02:03)"))?;
        if out.last().is_some_and(|&last| s <= last) {
            return Err(format!("pauses: `{t}` comes before the previous pause"));
        }
        out.push(s);
    }
    Ok(out)
}

/// The focus zones of an image or a video: `x y w h` (in % of the picture, from its top-left
/// corner), then what happens there: a hand-drawn mark (`.circle` by default, `.box`,
/// `.underline`, `.highlight`, `.strike`, with its colour), `.spot` (the rest darkens), the
/// clicks (`@2`, `@2-3`, `@3:magnify` the view zooms on the zone), or on a video a time range
/// (`0:12-0:18`, with `magnify` to zoom); zones are separated by `;`
fn zones_html(focus: &str, video: bool, seed: &str) -> Result<String, String> {
    let mut out = String::new();
    for (n, zone) in focus.split(';').map(str::trim).filter(|z| !z.is_empty()).enumerate() {
        let toks: Vec<&str> = zone.split_whitespace().collect();
        let coords: Vec<f64> = toks
            .iter()
            .take(4)
            .filter_map(|t| t.trim_end_matches('%').parse().ok())
            .filter(|v: &f64| v.is_finite())
            .collect();
        if coords.len() < 4 {
            return Err(format!("focus zone `{zone}`: it starts with x y width height, in % of the picture"));
        }
        let (mut time, mut zoom, mut rest) = (None, false, Vec::new());
        for t in &toks[4..] {
            if t.starts_with(|c: char| c.is_ascii_digit()) {
                let (a, b) = t.split_once('-').unwrap_or((t, ""));
                let a =
                    seconds(a).ok_or_else(|| format!("focus zone `{zone}`: `{t}` is not a time range (0:12-0:18)"))?;
                let b = if b.is_empty() {
                    None
                } else {
                    Some(
                        seconds(b)
                            .filter(|b| *b >= a)
                            .ok_or_else(|| format!("focus zone `{zone}`: `{t}` is not a time range (0:12-0:18)"))?,
                    )
                };
                time = Some((a, b));
            } else if *t == "magnify" {
                zoom = true;
            } else {
                rest.push(*t);
            }
        }
        if time.is_some() && !video {
            return Err(format!("focus zone `{zone}`: a time range is only for a video; on an image, use clicks (@2)"));
        }
        if zoom && time.is_none() {
            return Err(format!("focus zone `{zone}`: `magnify` goes with a time range; at a click, write @3:magnify"));
        }
        if time.is_some() && rest.iter().any(|t| t.starts_with('@')) {
            return Err(format!("focus zone `{zone}`: clicks (@n) or a time range, not both"));
        }
        let markers: Vec<Stepped> = rest.iter().filter_map(|t| marker(t)).collect();
        let shown = rest.iter().any(|t| MARKS.contains(&t.trim_start_matches('.')) || *t == ".spot");
        // A zone that only zooms draws nothing; otherwise it gets a circle by default
        let only_zoom = !shown && (zoom || (!markers.is_empty() && markers.iter().all(Stepped::highlights)));
        let attrs = if shown || only_zoom { rest.join(" ") } else { format!(".circle {}", rest.join(" ")) };
        let mut inner = span_html("", &attrs, &format!("{seed}{n}{zone}"));
        // a mark keeps its text after its last click; a zone leaves with its mark
        if let Some(until) = markers.iter().find(|m| !m.highlights()).and_then(|m| m.until)
            && !inner.contains("fragment until")
        {
            inner += &format!("<span class=\"fragment until\" data-fragment-index=\"{}\"></span>", until + 1);
        }
        let timed = match time {
            Some((a, b)) => format!(
                " data-time=\"{a}{}\"{}",
                b.map(|b| format!(" {b}")).unwrap_or_default(),
                if zoom { " data-zoom" } else { "" }
            ),
            None => String::new(),
        };
        let [x, y, w, h] = [coords[0], coords[1], coords[2], coords[3]].map(|v| v / 100.0);
        out += &format!(
            "<span class=\"zone{}\" style=\"--x:{x};--y:{y};--w:{w};--h:{h}\"{timed}>{inner}</span>",
            if time.is_some() { " timed" } else { "" }
        );
    }
    Ok(out)
}

/// Highlights written in a diagram (`card#pg … @3-4:halo`): skemo does not know them, glitu
/// takes them out of the source then puts them on the SVG node (its `data-id`)
fn skemo_marks(src: &str) -> Result<(String, Vec<(String, String)>), String> {
    let mut clean = String::with_capacity(src.len());
    let mut marked = Vec::new();
    for (n, line) in src.lines().enumerate() {
        // the end-of-line markers: skemo's stay, the highlights go out
        let mut rest = line.trim_end();
        let (mut marks, mut kept) = (Vec::new(), Vec::new());
        while let Some((head, tok)) = rest.rsplit_once(char::is_whitespace) {
            match marker(tok) {
                Some(m) if m.highlights() => marks.insert(0, m),
                Some(_) => kept.insert(0, tok),
                None => break,
            }
            rest = head.trim_end();
        }
        clean += rest;
        for tok in &kept {
            clean += " ";
            clean += tok;
        }
        clean += "\n";
        if marks.is_empty() {
            continue;
        }
        let tag = rest.trim_start().split(['(', ' ']).next().unwrap_or("");
        let id = tag.split_once('#').map(|(_, id)| id.split('.').next().unwrap_or(id)).filter(|id| !id.is_empty());
        let id = id.ok_or_else(|| {
            format!("line {} of the diagram: a highlight (@n:halo…) needs a card or a zone with an #id", n + 1)
        })?;
        marked.push((id.to_string(), fx_attr(&marks)));
    }
    Ok((clean, marked))
}

fn skemo_html(f: &Fence, assets: &mut dyn Assets) -> Result<String, String> {
    // highlights: skemo's own when it knows them, otherwise taken out and written by glitu
    let (source, marked) = if assets.diagram_can("skemo", "highlights") {
        (f.content.clone(), Vec::new())
    } else {
        skemo_marks(&f.content)?
    };
    let mut svg =
        assets.diagram("skemo", &source, &f.attrs).unwrap_or_else(|| Err("no engine renders ```skemo".into()))?;
    for (id, fx) in marked {
        let at = format!("data-id=\"{}\"", esc(&id));
        if let Some(i) = svg.find(&at) {
            svg.insert_str(i + at.len(), &fx);
        }
    }
    let a = appear(&f.appear);
    // `frieze` (skemo v0.8) or `frise` (its former spelling, still read)
    if ["layout=\"frieze\"", "layout=\"frise\""].iter().any(|l| source.contains(l)) {
        return Ok(frieze_html(&svg, &a));
    }
    Ok(format!("<div class=\"diagram{}\"{}{}>{svg}{}</div>", a.class, a.attrs, a.fx, a.end))
}

/// A frieze: the strip centres on the card that appears at each step; an event (its stem points
/// at it: `data-for`) rather than an era, whose centre is far from its beginnings
fn frieze_html(svg: &str, a: &Appear) -> String {
    let event = |id: &str| svg.contains(&format!("data-for=\"{id}\""));
    let better = |current: &str, id: &str| !event(current) && event(id);
    let mut first: Option<String> = None;
    let mut steps: Vec<(u32, String)> = Vec::new();
    for part in svg.split("<g class=\"card").skip(1) {
        let Some(id) = attr(part, "data-id") else { continue };
        // the attribute must belong to the card's tag, not to a descendant
        let tag_end = part.find('>').unwrap_or(part.len());
        let step = if part[..tag_end].contains("data-step=") {
            attr(part, "data-step").and_then(|s| s.parse().ok()).unwrap_or(0)
        } else {
            0
        };
        if step == 0 {
            if first.as_deref().is_none_or(|f| better(f, &id)) {
                first = Some(id);
            }
            continue;
        }
        match steps.iter_mut().find(|(s, _)| *s == step) {
            Some((_, current)) if better(current, &id) => *current = id,
            Some(_) => {}
            None => steps.push((step, id)),
        }
    }
    steps.sort();
    // without a card visible from the start, the strip starts from the first one to come
    let start = first.or_else(|| steps.first().map(|(_, id)| id.clone()));
    let pan_to = start.map(|id| format!(" data-pan-to=\"{}\"", esc(&id))).unwrap_or_default();
    // ` ```skemo @1 `: the whole diagram (axis included) appears at click 1
    let mut s = format!(
        "<div class=\"pan-viewport{}\"{}><div class=\"pan-track\"{pan_to}>{svg}</div>{}</div>",
        a.class, a.attrs, a.end
    );
    for (n, id) in steps {
        s += &format!(
            "<span class=\"fragment pan-step\" data-fragment-index=\"{n}\" data-pan-to=\"{}\"></span>",
            esc(&id)
        );
    }
    s
}

/// Value of an attribute in the first tag of the text
fn attr(tag: &str, name: &str) -> Option<String> {
    let end = tag.find('>')?;
    let start = tag[..end].find(&format!("{name}=\""))? + name.len() + 2;
    let len = tag[start..].find('"')?;
    Some(tag[start..start + len].to_string())
}

/// Big emojis, one per line: they replace one another at the click (`@n`), like `###` blocks
fn glyphs_html(f: &Fence) -> String {
    let items: Vec<Stepped> = f.content.lines().filter(|l| !l.trim().is_empty()).map(step_of).collect();
    let inner = match items.as_slice() {
        [one] => stepped_html("span", one),
        _ => replacing(&items, "span", |it| esc(&it.text)),
    };
    format!("<div class=\"glyphs\">{inner}</div>")
}

/// A big formula (` ```math `), which appears at click `@n`
fn formula_html(f: &Fence) -> String {
    let a = appear(&f.appear);
    format!(
        "<div class=\"formula{}\"{}{}>{}{}</div>",
        a.class,
        a.attrs,
        a.fx,
        crate::math::mathml(&f.content, true),
        a.end
    )
}

/// A code window; with `file="folder/f.yml"`, an explorer on the left shows the project files
/// seen so far, this one highlighted
fn code_html(f: &Fence, files: &mut Vec<String>) -> String {
    let lines = f.attrs.get("lines").map(String::as_str);
    let first_step = (f.step() > 0).then_some(f.step() + 1);
    let Appear { class: cls, attrs: idx, end, fx } = appear(&f.appear);
    let idx = format!("{idx}{fx}");
    let file = f.attrs.get("file");
    let title = f
        .attrs
        .get("title")
        .or(file)
        .map(|t| format!("<div class=\"window-title\">{}</div>", esc(t)))
        .unwrap_or_default();
    let code = match f.versions() {
        Some(versions) => {
            let versions: Vec<(&str, &str)> = versions.iter().map(|v| (v.lang.as_str(), v.content.as_str())).collect();
            highlight::magic_move(&versions, f.next_click)
        }
        None => highlight::block(&highlight::Code {
            lang: &f.lang,
            code: &f.content,
            lines,
            first_step,
            start_from: 1,
            reveal_md: false,
        }),
    };
    let Some(file) = file else {
        return format!("<div class=\"window{cls}\"{idx}>{title}{code}{end}</div>");
    };
    if !files.contains(file) {
        files.push(file.clone());
    }
    format!(
        "<div class=\"window with-explorer{cls}\"{idx}>{title}<div class=\"window-body\">{}{code}</div>{end}</div>",
        explorer_html(files, file)
    )
}

/// The file tree, folders first then by name, like an editor
fn explorer_html(files: &[String], current: &str) -> String {
    let mut root = Folder::default();
    for path in files {
        root.add(path);
    }
    format!("<nav class=\"explorer\">{}</nav>", root.html(current))
}

/// A folder of the explorer: its subfolders and its files (name, full path), sorted
#[derive(Default)]
struct Folder<'a> {
    folders: BTreeMap<&'a str, Folder<'a>>,
    files: BTreeSet<(&'a str, &'a str)>,
}

impl<'a> Folder<'a> {
    fn add(&mut self, path: &'a str) {
        let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let Some(name) = parts.pop() else { return };
        let folder = parts.into_iter().fold(self, |f, p| f.folders.entry(p).or_default());
        folder.files.insert((name, path));
    }

    fn html(&self, current: &str) -> String {
        let mut s = String::from("<ul>");
        for (name, sub) in &self.folders {
            s += &format!("<li class=\"dir\"><span>{}</span>{}</li>", esc(name), sub.html(current));
        }
        for (name, path) in &self.files {
            let on = if *path == current { " class=\"current\"" } else { "" };
            s += &format!("<li{on}><span>{}</span></li>", esc(name));
        }
        s + "</ul>"
    }
}

/// A terminal that replays an asciicast recording: one click per marker (`"m"` event), plus one
fn cast_html(text: &str, title: &str, n: u32) -> String {
    let markers = text
        .lines()
        .skip(1)
        .filter(|l| {
            let l = l.trim_start_matches('[');
            l.split_once(',').is_some_and(|(_, rest)| rest.trim_start().starts_with("\"m\""))
        })
        .count();
    let steps: String = (1..=markers + 1)
        .map(|k| format!("<span class=\"fragment cast-step\" data-fragment-index=\"{k}\"></span>"))
        .collect();
    let title =
        if title.is_empty() { String::new() } else { format!("<div class=\"window-title\">{}</div>", esc(title)) };
    format!(
        "<div class=\"window\">{title}<div class=\"cast-stage\" data-cast=\"#cast-{n}\" data-idle-time-limit=\"1\">{steps}</div></div>\
         <script type=\"text/plain\" id=\"cast-{n}\">{}</script>",
        text.trim().replace("</", "<\\/")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Svg(&'static str);

    impl Assets for Svg {
        fn diagram(&mut self, lang: &str, _: &str, _: &BTreeMap<String, String>) -> Option<Result<String, String>> {
            ["skemo", "mermaid"].contains(&lang).then(|| Ok(self.0.into()))
        }
        fn cast(&mut self, _: &str) -> Result<String, String> {
            Ok("{\"version\": 2}\n[0.5, \"o\", \"$ \"]\n[1.0, \"m\", \"\"]\n[2.0, \"o\", \"</x>\"]".into())
        }
    }

    #[test]
    fn explorer_lists_folders_first_and_marks_the_current_file() {
        let files = ["main.yml", "roles/lb/tasks/main.yml", "inventory/pokedex"].map(String::from);
        assert_eq!(
            explorer_html(&files, "inventory/pokedex"),
            "<nav class=\"explorer\"><ul>\
             <li class=\"dir\"><span>inventory</span><ul><li class=\"current\"><span>pokedex</span></li></ul></li>\
             <li class=\"dir\"><span>roles</span><ul><li class=\"dir\"><span>lb</span><ul>\
             <li class=\"dir\"><span>tasks</span><ul><li><span>main.yml</span></li></ul></li></ul></li></ul></li>\
             <li><span>main.yml</span></li></ul></nav>"
        );
    }

    #[test]
    fn code_windows_remember_the_files_already_shown() {
        let mut files = Vec::new();
        let fence = |file: &str| Fence {
            lang: "yaml".into(),
            content: "a: b".into(),
            attrs: [("file".to_string(), file.to_string())].into(),
            ..Fence::default()
        };
        code_html(&fence("b.yml"), &mut files);
        let html = code_html(&fence("a.yml"), &mut files);
        assert_eq!(files, ["b.yml", "a.yml"]);
        assert!(html.starts_with("<div class=\"window with-explorer\"><div class=\"window-title\">a.yml</div>"));
        let plain = code_html(&Fence { lang: "sh".into(), content: "ls".into(), ..Fence::default() }, &mut files);
        assert!(plain.starts_with("<div class=\"window\"><pre class=\"code-wrapper\">"), "{plain}");
    }

    #[test]
    fn highlights_leave_the_diagram_source_for_its_nodes() {
        let (src, marked) =
            skemo_marks("card#pg.pod(tag=\"5432\") postgresql @1:up @3-4:halo @6:focus\n  | ip @2\n").unwrap();
        assert_eq!(src, "card#pg.pod(tag=\"5432\") postgresql @1:up @6:focus\n  | ip @2\n");
        assert_eq!(marked, [("pg".to_string(), " data-highlight=\"halo:3-4\"".to_string())]);
        assert!(skemo_marks("  to(b) @2:halo\n").unwrap_err().contains("#id"));
    }

    #[test]
    fn diagrams_carry_their_highlights_and_appearance() {
        let fence = Fence {
            lang: "skemo".into(),
            content: "card#a A @2:halo".into(),
            appear: marker("@1").unwrap(),
            ..Fence::default()
        };
        let html = skemo_html(&fence, &mut Svg("<svg><g class=\"card\" data-id=\"a\"></g></svg>")).unwrap();
        assert_eq!(
            html,
            "<div class=\"diagram fragment\" data-fragment-index=\"1\"><svg><g class=\"card\" data-id=\"a\" data-highlight=\"halo:2\"></g></svg></div>"
        );
        // the appearance effect skemo writes stays: one attribute each (`@1:up @3:halo`)
        let up = Fence { lang: "skemo".into(), content: "card#a A @1:up @3:halo".into(), ..Fence::default() };
        let html =
            skemo_html(&up, &mut Svg("<svg><g class=\"card\" data-id=\"a\" data-step=\"1\" data-fx=\"up\"></g></svg>"))
                .unwrap();
        assert!(html.contains("data-id=\"a\" data-highlight=\"halo:3\" data-step=\"1\" data-fx=\"up\""), "{html}");
        assert_eq!(html.matches("data-fx=").count(), 1);
        // another engine's diagram: the block, without skemo's treatments
        let mermaid = Block::Fence(Fence { lang: "mermaid".into(), appear: marker("@2").unwrap(), ..Fence::default() });
        let html = visual_html(&mermaid, &mut Svg("<svg></svg>"), &mut Seen::default()).unwrap();
        assert_eq!(html, "<div class=\"diagram fragment\" data-fragment-index=\"2\"><svg></svg></div>");
        let rust = Block::Fence(Fence { lang: "rust".into(), content: "fn f() {}".into(), ..Fence::default() });
        assert!(visual_html(&rust, &mut Svg(""), &mut Seen::default()).unwrap().contains("code"));
    }

    #[test]
    fn the_frieze_pans_to_the_event_rather_than_the_era() {
        // an era and an event at the same click: the era first, as in the source
        let svg = Svg("<svg><g class=\"card era\" data-id=\"era\" data-step=\"3\"></g>\
             <g class=\"card\" data-id=\"evt\" data-step=\"3\"></g><path class=\"fr-stem\" data-for=\"evt\"/></svg>");
        let fence = Fence { lang: "skemo".into(), content: "diagram(layout=\"frieze\")".into(), ..Fence::default() };
        let html = skemo_html(&fence, &mut { svg }).unwrap();
        assert!(html.contains("<div class=\"pan-track\" data-pan-to=\"evt\">"), "{html}");
        assert!(html.contains("data-fragment-index=\"3\" data-pan-to=\"evt\""), "{html}");
    }

    #[test]
    fn emoji_block_replaces_its_lines_click_after_click() {
        let fence = |content: &str| Fence { lang: "emoji".into(), content: content.into(), ..Fence::default() };
        assert_eq!(glyphs_html(&fence("🤔")), "<div class=\"glyphs\"><span>🤔</span></div>");
        assert_eq!(
            glyphs_html(&fence("🚪\n😰 @1")),
            "<div class=\"glyphs\"><span class=\"fragment fade-out\" data-fragment-index=\"1\">🚪</span>\
             <span class=\"fragment\" data-fragment-index=\"1\">😰</span></div>"
        );
    }

    #[test]
    fn captions_go_under_the_visual_and_appear_with_it() {
        let mut seen = Seen::default();
        let mut attrs = BTreeMap::new();
        attrs.insert("caption".to_string(), "Source: [docs](https://x.org)".to_string());
        let fence = Fence {
            lang: "rust".into(),
            content: "fn f() {}".into(),
            attrs,
            appear: Stepped::at("", 2),
            ..Fence::default()
        };
        let html = visual_html(&Block::Fence(fence), &mut Svg(""), &mut seen).unwrap();
        assert!(html.ends_with(
            "<p class=\"visual-caption fragment\" data-fragment-index=\"2\">Source: <a href=\"https://x.org\">docs</a></p>"
        ));
        assert!(!html.contains("caption=\""));
        let image = Media { caption: Some("Photo: me".into()), ..Media::at("a.png", "A") };
        assert_eq!(
            visual_html(&Block::Image(image), &mut Svg(""), &mut seen).unwrap(),
            "<img class=\"visual\" src=\"a.png\" alt=\"A\"><p class=\"visual-caption\">Photo: me</p>"
        );
    }

    #[test]
    fn videos_pause_at_their_times_one_click_each() {
        let mut m = Media::at("img/demo.mp4", "Demo");
        m.attrs.insert("pauses".into(), "0:12 1:05.5".into());
        m.attrs.insert("muted".into(), String::new());
        m.appear = Stepped::at("", 1);
        let html = media_html(&m, "visual").unwrap();
        assert!(html.starts_with(
            "<video class=\"visual fragment\" data-src=\"img/demo.mp4\" preload=\"metadata\" playsinline"
        ));
        assert!(html.contains(" data-pauses=\"12 65.5\" data-at=\"1\" muted controls aria-label=\"Demo\""));
        assert!(!html.contains("data-autoplay"));
        assert!(html.ends_with(
            "<span class=\"fragment video-step\" data-fragment-index=\"2\"></span><span class=\"fragment video-step\" data-fragment-index=\"3\"></span>"
        ));
        m.attrs.insert("pauses".into(), "1:00 0:30".into());
        assert!(media_html(&m, "visual").unwrap_err().contains("before the previous pause"));
        let mut plain = Media::at("clip.webm", "");
        plain.attrs.insert("autoplay".into(), String::new());
        plain.attrs.insert("nocontrols".into(), String::new());
        assert_eq!(
            media_html(&plain, "visual").unwrap(),
            "<video class=\"visual\" data-src=\"clip.webm\" preload=\"metadata\" playsinline data-autoplay></video>"
        );
    }

    #[test]
    fn focus_zones_lie_over_the_picture() {
        let mut m = Media::at("a.png", "");
        m.attrs.insert(
            "focus".into(),
            "10 20 30% 40 @2; 50 50 10 10 .box .red @3-4; 0 0 5 5 @5:magnify; 1 1 2 2 .spot @6".into(),
        );
        let html = media_html(&m, "visual").unwrap();
        let zones = html.split("<div class=\"zones\" aria-hidden=\"true\">").nth(1).unwrap();
        let zone: Vec<&str> = zones.split("<span class=\"zone\"").skip(1).collect();
        assert_eq!(zone.len(), 4);
        // a circle by default, drawn at its click
        assert!(zone[0].starts_with(" style=\"--x:0.1;--y:0.2;--w:0.3;--h:0.4\"><span class=\"mark mark-circle\"><span class=\"fragment mark-on\" data-fragment-index=\"2\">"));
        assert!(
            zone[1].contains("mark mark-box red") && zone[1].contains("fragment until\" data-fragment-index=\"5\"")
        );
        // only a zoom: nothing drawn
        assert!(
            zone[2].starts_with(" style=\"--x:0;--y:0;--w:0.05;--h:0.05\"><span data-highlight=\"magnify:5\"></span>")
        );
        assert!(zone[3].contains("<span class=\"spot fragment\" data-fragment-index=\"6\">"));
        // on a video, a time range instead of clicks
        let mut v = Media::at("v.mp4", "");
        v.attrs.insert("focus".into(), "10 10 20 20 .spot 0:12-0:18; 0 0 50 50 1:00 magnify".into());
        let html = media_html(&v, "visual").unwrap();
        assert!(html.contains("<span class=\"zone timed\" style=\"--x:0.1;--y:0.1;--w:0.2;--h:0.2\" data-time=\"12 18\"><span class=\"spot\"></span></span>"));
        assert!(html.contains("data-time=\"60\" data-zoom><span></span></span>"));
        for (focus, error) in [
            ("10 20 30", "x y width height"),
            ("1 1 2 2 0:12-0:18", "only for a video"),
            ("1 1 2 2 magnify", "goes with a time range"),
        ] {
            m.attrs.insert("focus".into(), focus.into());
            assert!(media_html(&m, "visual").unwrap_err().contains(error), "{focus}");
        }
        v.attrs.insert("focus".into(), "1 1 2 2 @2 0:01".into());
        assert!(media_html(&v, "visual").unwrap_err().contains("not both"));
    }

    #[test]
    fn terminals_get_one_click_per_marker() {
        let mut seen = Seen::default();
        let block = Block::Image(Media::at("demo.cast", "Demo"));
        let html = visual_html(&block, &mut Svg(""), &mut seen).unwrap();
        assert_eq!(html.matches("cast-step").count(), 2);
        assert!(html.contains("data-cast=\"#cast-1\"") && html.contains("<\\/x>"));
        let image = Block::Image(Media::at("a.png", "A"));
        assert_eq!(
            visual_html(&image, &mut Svg(""), &mut seen).unwrap(),
            "<img class=\"visual\" src=\"a.png\" alt=\"A\">"
        );
        let math = Block::Fence(Fence { lang: "math".into(), content: "x".into(), ..Fence::default() });
        assert!(visual_html(&math, &mut Svg(""), &mut seen).unwrap().starts_with("<div class=\"formula\"><math"));
    }
}
