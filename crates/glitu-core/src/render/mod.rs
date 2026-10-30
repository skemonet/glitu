//! Slides → HTML sections, with the markup that themes dress
//! (.cover, .chapter, .panel, .stage, .window, .diagram, .caption, .cast-stage).
//!
//! `steps` handles the clicks (fragments, highlights, notes) and `visual` the visual of a slide
//! (diagram, code, formula, terminal, image).

mod steps;
mod visual;

use std::collections::BTreeMap;

pub use steps::{Appear, appear, effect_class};

use crate::html::{esc, inline};
use crate::markdown::{Block, Deck, Head, Slide, Stepped};
use steps::{fx_steps, notes_html, replacing, skemo_steps, stepped_html};
use visual::{Seen, visual_html};

/// What rendering asks of the rest of the pipeline
pub trait Assets {
    /// skemo source → SVG; `font_scale`: the diagram's own size factor
    fn skemo(&mut self, source: &str, font_scale: Option<f64>) -> Result<String, String>;
    /// asciicast recording (path relative to the markdown) → text
    fn cast(&mut self, src: &str) -> Result<String, String>;
}

#[derive(Debug)]
pub struct Rendered {
    pub title: String,
    pub sections: String,
}

/// Layout of a slide, read by the engine to pick the transition (`data-layout`)
const FULL: &str = "full";
const PANEL: &str = "panel";
/// The theme's backdrop: animated (cover, chapters), still (behind visuals)
const BACKDROP_ANIMATED: &str = "animated";
const BACKDROP_STILL: &str = "still";

pub fn render(deck: &Deck, assets: &mut dyn Assets) -> Result<Rendered, String> {
    let two_d = deck.meta.get("navigation").map(String::as_str) == Some("2d");
    let cover = deck.meta.get("title").map(|title| cover_html(&deck.meta, title));
    let toc = contents(deck, cover.is_some(), two_d);
    let mut r = Renderer { assets, chapter: String::new(), chapters: 0, seen: Seen::default(), toc };
    let slides = deck
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| Ok((slide.level == 1, with_layers(&timed(&r.slide(slide)?, i, &slide.head), &deck.meta))))
        .collect::<Result<Vec<_>, String>>()?;
    let cover = cover.map(|c| with_layers(&c, &deck.meta));
    let sections: Vec<String> = arrange(cover, slides, two_d)
        .into_iter()
        .map(|c| if c.len() == 1 { c.concat() } else { format!("<section>\n{}\n</section>", c.join("\n")) })
        .collect();
    Ok(Rendered { title: deck.meta.get("title").cloned().unwrap_or_default(), sections: sections.join("\n") })
}

/// Timing marks on the `<section>` of a slide: its rank in the markdown (`data-slide`, for the
/// rehearsals of `glitu serve`), its planned duration in seconds and the number of rehearsals it
/// averages, if it was timed (`## Title ⏳95🏃3`)
fn timed(html: &str, i: usize, head: &Head) -> String {
    let mut attrs = format!(" data-slide=\"{i}\"");
    if let Some(d) = head.attrs.get("duration").and_then(|d| d.parse::<u32>().ok()) {
        attrs += &format!(" data-duration=\"{d}\"");
        if let Some(r) = head.attrs.get("runs").and_then(|r| r.parse::<u32>().ok()) {
            attrs += &format!(" data-runs=\"{r}\"");
        }
    }
    html.replacen("<section", &format!("<section{attrs}"), 1)
}

/// Slides arranged in columns (horizontal slides, each one with its vertical slides).
/// `navigation: 2d`: one chapter per column, its divider on top and its slides below; what
/// precedes the first chapter goes under the cover. Otherwise: one slide per column.
fn arrange<T>(cover: Option<T>, slides: Vec<(bool, T)>, two_d: bool) -> Vec<Vec<T>> {
    let mut columns: Vec<Vec<T>> = cover.into_iter().map(|c| vec![c]).collect();
    for (chapter, slide) in slides {
        match columns.last_mut() {
            Some(column) if two_d && !chapter => column.push(slide),
            _ => columns.push(vec![slide]),
        }
    }
    columns
}

/// An entry of the table of contents: chapter number (none: `{-}`), title, horizontal slide
struct Entry {
    num: Option<u32>,
    title: String,
    h: usize,
}

/// The chapters and their place in the navigation, for `{.toc}`
fn contents(deck: &Deck, cover: bool, two_d: bool) -> Vec<Entry> {
    let slides: Vec<(bool, usize)> = deck.slides.iter().enumerate().map(|(i, s)| (s.level == 1, i)).collect();
    let columns = arrange(cover.then_some(usize::MAX), slides, two_d);
    let mut num = 0;
    let mut out = Vec::new();
    for (h, column) in columns.iter().enumerate() {
        for &i in column.iter().filter(|&&i| i != usize::MAX) {
            let s = &deck.slides[i];
            if s.level == 1 {
                let n = (!s.head.unnumbered).then(|| {
                    num += 1;
                    num
                });
                out.push(Entry { num: n, title: s.head.text.clone(), h });
            }
        }
    }
    out
}

/// What passes from one slide to the next during rendering
struct Renderer<'a> {
    assets: &'a mut dyn Assets,
    /// Current chapter: the topic in the header of the following slides
    chapter: String,
    /// Chapters numbered so far
    chapters: u32,
    seen: Seen,
    /// Chapters, for the `{.toc}` slides
    toc: Vec<Entry>,
}

impl Renderer<'_> {
    fn slide(&mut self, slide: &Slide) -> Result<String, String> {
        let (blocks, notes) = split_notes(slide);
        let aside = notes_html(&notes);
        let html =
            if slide.level == 1 { self.chapter(slide, &blocks, &aside) } else { self.content(slide, &blocks, aside)? };
        let steps = fx_steps(&html);
        let html = match html.rfind("</section>") {
            Some(end) if !steps.is_empty() => format!("{}{steps}{}", &html[..end], &html[end..]),
            _ => html,
        };
        Ok(with_settings(&html, &slide.head))
    }

    /// A content slide, according to its layout (`{.cols}`, `{.quote}`… see `LAYOUTS`) or, by
    /// default, text on the left and visual on the right
    fn content(&mut self, slide: &Slide, blocks: &[&Block], aside: String) -> Result<String, String> {
        let kicker = slide.head.attrs.get("kicker").unwrap_or(&self.chapter);
        let header = esc(&format!("{kicker}|{}", slide.head.text));
        let has = |c: &str| slide.head.classes.iter().any(|x| x == c);
        let layout = LAYOUTS.iter().copied().find(|l| has(l)).or_else(|| has(IFRAME).then_some(IFRAME));
        match layout {
            Some("cols") => return self.cols(&header, blocks, &aside),
            Some(l @ ("cover" | "intro" | "section" | "end")) => {
                return Ok(title_slide_html(l, &slide.head.text, blocks, &aside));
            }
            Some(l) if l.starts_with("image-") || l.starts_with("iframe") => {
                return self.beside(l, &header, slide, blocks, &aside);
            }
            _ => {}
        }
        let (mut groups, visual) = text_and_visual(blocks);
        if has("toc") {
            group_of(&mut groups).items.extend(self.toc_items());
        }
        let stage = self.visual(visual)?;
        // One invisible fragment per step of the skemo diagrams: they move on with the slide's clicks
        let aside = format!("{}{aside}", skemo_steps(&stage));
        if let Some(layout) = layout {
            let text: String = groups.iter().map(group_html).collect();
            return Ok(format!(
                "<section data-header=\"{header}\" data-layout=\"{FULL}\" class=\"layout-{layout}\">\
                 <div class=\"layout-body\" data-backdrop=\"{BACKDROP_STILL}\">{text}{stage}</div>{aside}</section>"
            ));
        }
        Ok(match groups.first() {
            Some(g) if has("large") => captioned_html(&header, g, &stage, &aside),
            Some(_) => panel_slide_html(&header, &slide.head.classes, &groups, &stage, &aside),
            None => full_slide_html(&header, &stage, &aside),
        })
    }

    /// The chapters as links, for `{.toc}`
    fn toc_items(&self) -> impl Iterator<Item = Stepped> + '_ {
        self.toc.iter().map(|e| {
            let num = e.num.map(|n| format!("{n:02} ")).unwrap_or_default();
            Stepped::at(format!("[{num}{}](#/{})", e.title, e.h), 0)
        })
    }

    fn visual(&mut self, visual: Option<&Block>) -> Result<String, String> {
        match visual {
            Some(v) => visual_html(v, self.assets, &mut self.seen),
            None => Ok(String::new()),
        }
    }

    /// `{.cols}`: two columns separated by a `::right::` line; with a `::left::` line, what
    /// precedes it is a full-width header
    fn cols(&mut self, header: &str, blocks: &[&Block], aside: &str) -> Result<String, String> {
        let at = |m: &str| blocks.iter().position(|b| matches!(b, Block::Para(p) if p.text.trim() == m));
        let right = at("::right::").unwrap_or(blocks.len());
        let (top, left) = match at("::left::") {
            Some(l) if l < right => (&blocks[..l], &blocks[l + 1..right]),
            _ => (&blocks[..0], &blocks[..right]),
        };
        let top =
            if top.is_empty() { String::new() } else { format!("<div class=\"cols-header\">{}</div>", self.column(top)?) };
        let left = self.column(left)?;
        let right = self.column(blocks.get(right + 1..).unwrap_or(&[]))?;
        let steps = skemo_steps(&format!("{left}{right}"));
        Ok(format!(
            "<section data-header=\"{header}\" data-layout=\"{FULL}\" class=\"layout-cols\"><div class=\"cols\" data-backdrop=\"{BACKDROP_STILL}\">\
             {top}<div class=\"col\">{left}</div><div class=\"col\">{right}</div></div>{steps}{aside}</section>"
        ))
    }

    /// `{.image-left}`, `{.image-right}`: an image on one half of the slide (`image="…"`,
    /// otherwise the first one of the slide), the text on the other; `{.iframe url="…"}`: a web
    /// page, over the whole slide, or beside the text (`{.iframe-left}`, `{.iframe-right}`)
    fn beside(&mut self, layout: &str, header: &str, slide: &Slide, blocks: &[&Block], aside: &str) -> Result<String, String> {
        let attr = |k: &str| slide.head.attrs.get(k).cloned();
        let iframe = layout.starts_with("iframe");
        let media = if iframe {
            let url = attr("url").ok_or_else(|| format!("slide \"{}\": {{.{layout}}} needs url=\"…\"", slide.head.text))?;
            format!("<iframe class=\"layout-media\" data-src=\"{}\" allowfullscreen></iframe>", esc(&url))
        } else {
            let image = || {
                blocks.iter().find_map(|b| match b {
                    Block::Image { src, alt } => Some((src.clone(), alt.clone())),
                    _ => None,
                })
            };
            let (src, alt) = attr("image")
                .map(|s| (s, String::new()))
                .or_else(image)
                .ok_or_else(|| format!("slide \"{}\": {{.{layout}}} needs an image", slide.head.text))?;
            format!("<img class=\"layout-media\" src=\"{}\" alt=\"{}\">", esc(&src), esc(&alt))
        };
        // The image taken from the slide is not repeated in the text
        let from_slide = !iframe && attr("image").is_none();
        let rest: Vec<&Block> =
            blocks.iter().copied().filter(|b| !(from_slide && matches!(b, Block::Image { .. }))).collect();
        let text = self.column(&rest)?;
        let side = if layout.ends_with("left") {
            "left"
        } else if layout.ends_with("right") || !text.is_empty() {
            "right"
        } else {
            "full"
        };
        let body = if side == "full" {
            String::new()
        } else {
            format!("<div class=\"layout-body\" data-backdrop=\"{BACKDROP_STILL}\">{text}</div>")
        };
        Ok(format!(
            "<section data-header=\"{header}\" data-layout=\"{FULL}\" class=\"layout-beside media-{side}\">{body}{media}{}{aside}</section>",
            skemo_steps(&text)
        ))
    }

    /// A column of `{.cols}`: its texts, then its visual
    fn column(&mut self, blocks: &[&Block]) -> Result<String, String> {
        let (groups, visual) = text_and_visual(blocks);
        let text: String = groups.iter().map(group_html).collect();
        let visual = self.visual(visual)?;
        Ok(format!("{text}{visual}"))
    }

    fn chapter(&mut self, slide: &Slide, blocks: &[&Block], aside: &str) -> String {
        self.chapter = slide.head.text.clone();
        let num = if slide.head.unnumbered {
            String::new()
        } else {
            self.chapters += 1;
            format!("<span class=\"num\">{:02}</span>", self.chapters)
        };
        let sub = blocks.iter().find_map(|b| match b {
            Block::Para(p) => Some(format!("<p>{}</p>", inline(&p.text))),
            _ => None,
        });
        format!(
            "<section data-layout=\"{FULL}\"><div class=\"chapter\" data-backdrop=\"{BACKDROP_ANIMATED}\">{num}<h2>{}</h2>{}</div>{aside}</section>",
            inline(&slide.head.text),
            sub.unwrap_or_default()
        )
    }
}

/// Layouts of a slide, chosen by its class (`## Topic {.quote}`): two columns (separated by a
/// `::right::` line), quote, key figure, statement, centred, table of contents (the chapters, as
/// links, generated), full (no margins), title (`cover`, `intro`), divider (`section`, `end`),
/// image or web page beside the text
const LAYOUTS: [&str; 15] = [
    "cols",
    "quote",
    "fact",
    "statement",
    "center",
    "toc",
    "full",
    "cover",
    "intro",
    "section",
    "end",
    "image-left",
    "image-right",
    "iframe-left",
    "iframe-right",
];
/// `{.iframe}`: looked for after `iframe-left` and `iframe-right`
const IFRAME: &str = "iframe";

/// Settings of a slide, in the attributes of its heading:
/// `{transition="zoom" background="#123" class="dark" zoom="0.8" .animate}`; `background`: a
/// colour, otherwise an image
fn with_settings(section: &str, head: &Head) -> String {
    let mut attrs = String::new();
    let mut style = Vec::new();
    if let Some(t) = head.attrs.get("transition") {
        attrs += &format!(" data-transition=\"{}\"", esc(t));
    }
    // `{.animate}`: the elements shared with the neighbouring animated slide slide over (see the engine)
    if head.classes.iter().any(|c| c == "animate") {
        attrs += " data-auto-animate";
    }
    // The slide's background goes in front of the theme's
    if let Some(bg) = head.attrs.get("background") {
        let color = bg.starts_with('#')
            || bg.starts_with("rgb")
            || bg.starts_with("hsl")
            || bg.chars().all(|c| c.is_ascii_alphabetic());
        style.push(if color { format!("background:{bg}") } else { format!("background:url('{bg}') center/cover no-repeat") });
    }
    if let Some(z) = head.attrs.get("zoom").and_then(|z| z.parse::<f64>().ok()) {
        attrs += " data-zoom";
        style.push(format!("--zoom:{z}"));
    }
    if !style.is_empty() {
        attrs += &format!(" style=\"{}\"", esc(&style.join(";")));
    }
    let Some(tag_end) = section.find('>') else { return section.to_string() };
    let mut tag = format!("{}{attrs}", &section[..tag_end]);
    if let Some(class) = head.attrs.get("class") {
        tag = match tag.find(" class=\"") {
            Some(at) => format!("{}{} {}", &tag[..at + 8], esc(class), &tag[at + 8..]),
            None => format!("{tag} class=\"{}\"", esc(class)),
        };
    }
    tag + &section[tag_end..]
}

/// Title slide (`{.cover}`, `{.intro}`): the title in large type, the first paragraph as a
/// subtitle, the following ones as a byline; divider (`{.section}`, `{.end}`): the title and its
/// paragraphs, without a number or a change of chapter
fn title_slide_html(layout: &str, title: &str, blocks: &[&Block], aside: &str) -> String {
    let paras: Vec<String> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::Para(p) => Some(inline(&p.text)),
            _ => None,
        })
        .collect();
    let inner = match layout {
        "cover" | "intro" => {
            let mut s = format!("<div class=\"cover\" data-backdrop=\"{BACKDROP_ANIMATED}\"><h1>{}</h1>", inline(title));
            for (i, p) in paras.iter().enumerate() {
                s += &format!("<p class=\"{}\">{p}</p>", if i == 0 { "subtitle" } else { "byline" });
            }
            s + "</div>"
        }
        _ => format!(
            "<div class=\"chapter\" data-backdrop=\"{BACKDROP_ANIMATED}\"><h2>{}</h2>{}</div>",
            inline(title),
            paras.iter().map(|p| format!("<p>{p}</p>")).collect::<String>()
        ),
    };
    format!("<section data-layout=\"{FULL}\" class=\"layout-{layout}\">{inner}{aside}</section>")
}

/// Layers shared by every slide, written in the front matter: `global-top` (in front of the
/// content) and `global-bottom` (behind), in one line of markdown: a logo, a mention
fn with_layers(section: &str, meta: &BTreeMap<String, String>) -> String {
    let layers: String = [("global-bottom", "bottom"), ("global-top", "top")]
        .iter()
        .filter_map(|(k, class)| {
            meta.get(*k)
                .map(|md| format!("<div class=\"global-layer global-{class}\">{}</div>", inline(md.trim_matches('"'))))
        })
        .collect();
    match section.find('>') {
        Some(end) if !layers.is_empty() => format!("{}{layers}{}", &section[..=end], &section[end + 1..]),
        _ => section.to_string(),
    }
}

fn cover_html(meta: &BTreeMap<String, String>, title: &str) -> String {
    let mut s = format!("<section data-layout=\"{FULL}\"><div class=\"cover\" data-backdrop=\"{BACKDROP_ANIMATED}\">");
    if let Some(e) = meta.get("eyebrow") {
        s += &format!("<span class=\"eyebrow\">{}</span>", inline(e))
    }
    s += &format!("<h1>{}</h1>", inline(title));
    if let Some(sub) = meta.get("subtitle") {
        s += &format!("<p class=\"subtitle\">{}</p>", inline(sub))
    }
    let byline: Vec<&str> = ["author", "date"].iter().filter_map(|k| meta.get(*k).map(String::as_str)).collect();
    if !byline.is_empty() {
        s += &format!("<p class=\"byline\">{}</p>", inline(&byline.join(" · ")))
    }
    s + "</div></section>"
}

/// Text: `### heading + bullets` groups; visual: the first diagram, code, terminal or image
fn text_and_visual<'a>(blocks: &[&'a Block]) -> (Vec<Group>, Option<&'a Block>) {
    let mut groups: Vec<Group> = Vec::new();
    let mut visual = None;
    for b in blocks {
        match b {
            Block::Title(t) => groups.push(Group { title: t.text.clone(), step: t.step, ..Group::default() }),
            Block::List(items) => group_of(&mut groups).items.extend(items.iter().cloned()),
            Block::Para(p) => group_of(&mut groups).paras.push(p.clone()),
            v => {
                visual.get_or_insert(*v);
            }
        }
    }
    (groups, visual)
}

/// `{.large}`: the visual at full width, the bullets as a caption above it
fn captioned_html(header: &str, g: &Group, stage: &str, aside: &str) -> String {
    let title = if g.title.is_empty() { String::new() } else { format!("<h2>{}</h2>", inline(&g.title)) };
    let steps = replacing(&g.items, "p", |it| inline(&it.text));
    format!(
        "<section data-header=\"{header}\" data-layout=\"{FULL}\"><div class=\"stage full\" data-backdrop=\"{BACKDROP_STILL}\">\
         <div class=\"caption\">{title}<div class=\"caption-steps\">{steps}</div></div>{stage}</div>{aside}</section>"
    )
}

/// The text in the left panel, the visual on the right
fn panel_slide_html(header: &str, classes: &[String], groups: &[Group], stage: &str, aside: &str) -> String {
    let classes: Vec<&str> =
        std::iter::once("panel").chain(classes.iter().map(String::as_str).filter(|c| *c != "large")).collect();
    let inner = if groups.len() > 1 {
        let steps: Vec<Stepped> = groups.iter().map(|g| Stepped::at("", g.step)).collect();
        let mut k = 0;
        let replaced = replacing(&steps, "div", |_| {
            k += 1;
            group_html(&groups[k - 1])
        });
        format!("<div class=\"panel-steps\">{replaced}</div>")
    } else {
        group_html(&groups[0])
    };
    let stage = stage_html("stage", stage);
    format!(
        "<section data-header=\"{header}\" data-layout=\"{PANEL}\"><div class=\"{}\">{inner}</div>{stage}{aside}</section>",
        classes.join(" ")
    )
}

/// Without text: the visual takes the full width
fn full_slide_html(header: &str, stage: &str, aside: &str) -> String {
    let stage = stage_html("stage full", stage);
    format!("<section data-header=\"{header}\" data-layout=\"{FULL}\">{stage}{aside}</section>")
}

fn stage_html(class: &str, visual: &str) -> String {
    if visual.is_empty() {
        return String::new();
    }
    format!("<div class=\"{class}\" data-backdrop=\"{BACKDROP_STILL}\">{visual}</div>")
}

#[derive(Default)]
struct Group {
    title: String,
    step: u32,
    items: Vec<Stepped>,
    paras: Vec<Stepped>,
}

fn group_of(groups: &mut Vec<Group>) -> &mut Group {
    if groups.is_empty() {
        groups.push(Group::default());
    }
    groups.last_mut().expect("a group was just added")
}

fn group_html(g: &Group) -> String {
    let mut s = String::new();
    if !g.title.is_empty() {
        s += &format!("<h2>{}</h2>", inline(&g.title))
    }
    for p in &g.paras {
        s += &stepped_html("p", p)
    }
    if !g.items.is_empty() {
        s += "<ul>";
        for it in &g.items {
            s += &stepped_html("li", it);
        }
        s += "</ul>";
    }
    s
}

/// Speaker notes: the `Note:` paragraph and everything after it
fn split_notes(slide: &Slide) -> (Vec<&Block>, Vec<String>) {
    let (mut blocks, mut notes) = (Vec::new(), Vec::new());
    for b in &slide.blocks {
        match b {
            Block::Para(p) if !notes.is_empty() || note(&p.text).is_some() => {
                notes.push(note(&p.text).unwrap_or(&p.text).to_string())
            }
            b => blocks.push(b),
        }
    }
    (blocks, notes)
}

fn note(p: &str) -> Option<&str> {
    let rest = p.strip_prefix("Note")?.trim_start().strip_prefix(':')?;
    Some(rest.trim_start())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;

    struct NoAssets;

    impl Assets for NoAssets {
        fn skemo(&mut self, _: &str, _: Option<f64>) -> Result<String, String> {
            Ok("<svg data-step=\"2\"></svg>".into())
        }
        fn cast(&mut self, _: &str) -> Result<String, String> {
            unreachable!()
        }
    }

    fn html(md: &str) -> String {
        render(&parse(md), &mut NoAssets).unwrap().sections
    }

    #[test]
    fn two_d_navigation_stacks_each_chapter_under_its_divider() {
        let slides = [(false, "intro"), (true, "ch1"), (false, "a"), (false, "b"), (true, "ch2")]
            .map(|(c, s)| (c, s.to_string()))
            .to_vec();
        assert_eq!(arrange(Some("cover".to_string()), slides, true), [
            vec!["cover", "intro"],
            vec!["ch1", "a", "b"],
            vec!["ch2"]
        ]);
        let deck = html("---\ntitle: T\nnavigation: 2d\n---\n\n# C\n\n## S\n\n- a\n");
        assert_eq!(deck.matches("<section>\n").count(), 1, "{deck}");
    }

    #[test]
    fn chapters_are_numbered_and_listed_in_the_table_of_contents() {
        let deck = html("# One\n\nsub\n\n# Aside {-}\n\n# Two\n\n## Contents {.toc}\n");
        assert!(deck.contains("<span class=\"num\">01</span><h2>One</h2><p>sub</p>"));
        assert!(deck.contains("<span class=\"num\">02</span><h2>Two</h2>"));
        assert!(deck.contains("<a href=\"#/0\">01 One</a>") && deck.contains("<a href=\"#/1\">Aside</a>"), "{deck}");
        assert!(deck.contains("data-header=\"Two|Contents\""));
    }

    #[test]
    fn layouts_follow_the_class_of_the_heading() {
        let deck = html("## Q {.quote}\n\n> words\n\n## C {.cols}\n\ntop\n\n::left::\n\nleft\n\n::right::\n\nright\n");
        assert!(deck.contains("class=\"layout-quote\""));
        assert!(deck.contains("<div class=\"cols-header\"><p>top</p></div><div class=\"col\"><p>left</p></div><div class=\"col\"><p>right</p></div>"), "{deck}");
        let deck = html("## T {.cover}\n\nsub\n\nby\n\n## E {.end}\n\nbye\n");
        assert!(deck.contains("<h1>T</h1><p class=\"subtitle\">sub</p><p class=\"byline\">by</p>"));
        assert!(deck.contains("class=\"layout-end\"><div class=\"chapter\" data-backdrop=\"animated\"><h2>E</h2><p>bye</p>"));
    }

    #[test]
    fn media_beside_the_text() {
        let deck = html("## I {.image-right}\n\n- text\n\n![alt](a.png)\n");
        assert!(deck.contains("class=\"layout-beside media-right\""));
        assert!(deck.contains("<img class=\"layout-media\" src=\"a.png\" alt=\"alt\">"));
        assert_eq!(deck.matches("a.png").count(), 1);
        let deck = html("## W {.iframe url=\"https://example.net\"}\n");
        assert!(deck.contains("media-full") && deck.contains("data-src=\"https://example.net\""), "{deck}");
        let err = render(&parse("## W {.iframe-left}\n"), &mut NoAssets).unwrap_err();
        assert_eq!(err, "slide \"W\": {.iframe-left} needs url=\"…\"");
        let err = render(&parse("## W {.image-left}\n"), &mut NoAssets).unwrap_err();
        assert!(err.contains("needs an image"));
    }

    #[test]
    fn slide_settings_and_layers() {
        let deck = html(
            "---\nglobal-top: \"*top*\"\n---\n\n## S {transition=\"zoom\" background=\"bg.png\" zoom=\"0.8\" class=\"dark\" duration=\"95\" .animate}\n",
        );
        assert!(deck.contains("<section data-slide=\"0\" data-duration=\"95\" data-header=\"|S\" data-layout=\"full\" data-transition=\"zoom\" data-auto-animate data-zoom style=\"background:url('bg.png') center/cover no-repeat;--zoom:0.8\" class=\"dark\">"), "{deck}");
        assert!(deck.contains("<div class=\"global-layer global-top\"><em>top</em></div>"));
        assert!(html("## S {background=\"#123\"}\n").contains("style=\"background:#123\""));
    }

    #[test]
    fn text_panel_steps_and_captions() {
        let deck = html("## S\n\n### A\n\n- a\n\n### B @1\n\n- b\n\n```skemo\ncard#a A\n```\n\nNote: say [click] more\n");
        assert!(deck.contains("<div class=\"panel-steps\"><div class=\"fragment fade-out\" data-fragment-index=\"1\"><h2>A</h2>"));
        assert!(deck.contains("<span class=\"fragment sk-step\" data-fragment-index=\"2\"></span><aside class=\"notes\">"), "{deck}");
        let deck = html("## L {.large}\n\n- one\n- two @1\n\n![x](x.png)\n");
        assert!(deck.contains("<div class=\"caption\"><div class=\"caption-steps\"><p class=\"fragment fade-out\""));
        let deck = html("## F\n\n![x](x.png)\n");
        assert!(deck.contains("<div class=\"stage full\" data-backdrop=\"still\"><img"));
    }
}
