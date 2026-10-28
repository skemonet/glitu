//! The visual of a slide: skemo diagram, code window, formula, emojis, terminal or image.

use std::collections::{BTreeMap, BTreeSet};

use super::Assets;
use super::steps::{Appear, appear, fx_attr, replacing, stepped_html};
use crate::highlight;
use crate::html::esc;
use crate::markdown::{Block, Fence, Stepped, marker, step_of};

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
        Block::Fence(f) if f.lang == "skemo" => skemo_html(f, assets)?,
        Block::Fence(f) if f.lang == "emoji" => glyphs_html(f),
        Block::Fence(f) if f.lang == "math" => formula_html(f),
        Block::Fence(f) => code_html(f, &mut seen.files),
        Block::Image { src, alt } if src.ends_with(".cast") => {
            seen.casts += 1;
            cast_html(&assets.cast(src)?, alt, seen.casts)
        }
        Block::Image { src, alt } => format!("<img class=\"visual\" src=\"{}\" alt=\"{}\">", esc(src), esc(alt)),
        _ => String::new(),
    })
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
    let scale = f
        .attrs
        .get("font-scale")
        .map(|v| v.parse::<f64>().map_err(|_| format!("invalid font-scale: {v}")))
        .transpose()?;
    let (source, marked) = skemo_marks(&f.content)?;
    let mut svg = assets.skemo(&source, scale)?;
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
    format!("<div class=\"formula{}\"{}{}>{}{}</div>", a.class, a.attrs, a.fx, crate::math::mathml(&f.content, true), a.end)
}

/// A code window; with `file="folder/f.yml"`, an explorer on the left shows the project files
/// seen so far, this one highlighted
fn code_html(f: &Fence, files: &mut Vec<String>) -> String {
    let lines = f.attrs.get("lines").map(String::as_str);
    let first_step = (f.step() > 0).then_some(f.step() + 1);
    let Appear { class: cls, attrs: idx, end, fx } = appear(&f.appear);
    let idx = format!("{idx}{fx}");
    let file = f.attrs.get("file");
    let title =
        f.attrs.get("title").or(file).map(|t| format!("<div class=\"window-title\">{}</div>", esc(t))).unwrap_or_default();
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
    let title = if title.is_empty() { String::new() } else { format!("<div class=\"window-title\">{}</div>", esc(title)) };
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
        fn skemo(&mut self, _: &str, _: Option<f64>) -> Result<String, String> {
            Ok(self.0.into())
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
            skemo_marks("card#pg.pod(tag=\"5432\") postgresql @1:up @3-4:halo @6:focus\n  li ip @2\n").unwrap();
        assert_eq!(src, "card#pg.pod(tag=\"5432\") postgresql @1:up @6:focus\n  li ip @2\n");
        assert_eq!(marked, [("pg".to_string(), " data-fx=\"halo:3-4\"".to_string())]);
        assert!(skemo_marks("  to(b) @2:halo\n").unwrap_err().contains("#id"));
    }

    #[test]
    fn diagrams_carry_their_highlights_and_appearance() {
        let fence = Fence { lang: "skemo".into(), content: "card#a A @2:halo".into(), appear: marker("@1").unwrap(), ..Fence::default() };
        let html = skemo_html(&fence, &mut Svg("<svg><g class=\"card\" data-id=\"a\"></g></svg>")).unwrap();
        assert_eq!(
            html,
            "<div class=\"diagram fragment\" data-fragment-index=\"1\"><svg><g class=\"card\" data-id=\"a\" data-fx=\"halo:2\"></g></svg></div>"
        );
        let bad = Fence { lang: "skemo".into(), attrs: [("font-scale".to_string(), "big".to_string())].into(), ..Fence::default() };
        assert_eq!(skemo_html(&bad, &mut Svg("")).unwrap_err(), "invalid font-scale: big");
    }

    #[test]
    fn the_frieze_pans_to_the_event_rather_than_the_era() {
        // an era and an event at the same click: the era first, as in the source
        let svg = Svg(
            "<svg><g class=\"card era\" data-id=\"era\" data-step=\"3\"></g>\
             <g class=\"card\" data-id=\"evt\" data-step=\"3\"></g><path class=\"fr-stem\" data-for=\"evt\"/></svg>",
        );
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
    fn terminals_get_one_click_per_marker() {
        let mut seen = Seen::default();
        let block = Block::Image { src: "demo.cast".into(), alt: "Demo".into() };
        let html = visual_html(&block, &mut Svg(""), &mut seen).unwrap();
        assert_eq!(html.matches("cast-step").count(), 2);
        assert!(html.contains("data-cast=\"#cast-1\"") && html.contains("<\\/x>"));
        let image = Block::Image { src: "a.png".into(), alt: "A".into() };
        assert_eq!(visual_html(&image, &mut Svg(""), &mut seen).unwrap(), "<img class=\"visual\" src=\"a.png\" alt=\"A\">");
        let math = Block::Fence(Fence { lang: "math".into(), content: "x".into(), ..Fence::default() });
        assert!(visual_html(&math, &mut Svg(""), &mut seen).unwrap().starts_with("<div class=\"formula\"><math"));
    }
}
