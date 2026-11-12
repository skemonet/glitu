//! Reading slides.md: front matter, chapters (`#`), slides (`##`) and their blocks.
//!
//! The markdown is read by pulldown-cmark; for each block the source text is kept (headings,
//! bullets, paragraphs) so that the `@n` markers and the `{…}` attributes can be read before the
//! HTML rendering.

use std::collections::BTreeMap;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Default)]
pub struct Deck {
    pub meta: BTreeMap<String, String>,
    pub slides: Vec<Slide>,
}

#[derive(Debug)]
pub struct Slide {
    /// 1: chapter, 2: slide
    pub level: u8,
    pub head: Head,
    pub blocks: Vec<Block>,
    /// Start of its heading line in the source (bytes, front matter included): the rehearsals of
    /// `glitu server` write the measured duration there
    pub at: usize,
}

#[derive(Debug, Default)]
pub struct Head {
    pub text: String,
    pub classes: Vec<String>,
    pub attrs: BTreeMap<String, String>,
    pub unnumbered: bool,
}

#[derive(Debug)]
pub enum Block {
    Title(Stepped),
    List(Vec<Stepped>),
    Fence(Fence),
    Image(Media),
    Para(Stepped),
}

/// An image, a video or a terminal recording: `![alt](src "caption"){attributes}`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Media {
    pub src: String,
    pub alt: String,
    /// Line shown under it (its source, for example): the title of the image, or `caption="…"`
    pub caption: Option<String>,
    /// `{…}` after it: `key="value"`, or a lone word (`muted`), without a value
    pub attrs: BTreeMap<String, String>,
    /// Appearance click (`@n` in the attributes)
    pub appear: Stepped,
}

/// Extensions of the files shown as a video (`![](img/demo.mp4)`)
pub const VIDEOS: [&str; 3] = ["mp4", "webm", "ogv"];

impl Media {
    pub fn at(src: impl Into<String>, alt: impl Into<String>) -> Media {
        Media { src: src.into(), alt: alt.into(), ..Media::default() }
    }

    pub fn is_video(&self) -> bool {
        let path = self.src.split(['?', '#']).next().unwrap_or("");
        path.rsplit_once('.').is_some_and(|(_, ext)| VIDEOS.contains(&ext.to_ascii_lowercase().as_str()))
    }

    /// The `pauses="0:12 1:05"` of a video: one click each, to resume after the pause
    pub fn pauses(&self) -> usize {
        self.attrs.get("pauses").map_or(0, |p| p.split([' ', ',']).filter(|t| !t.is_empty()).count())
    }
}

/// A text (inline markdown) and the click where it appears (0: from the start), written at the
/// end of the line: `@2`, relative to the previous click of the slide `@+1`, until a click `@2-4`
/// (visible from click 2 to click 4), with an effect `@2:up` (see `render::effect_class`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stepped {
    pub text: String,
    pub step: u32,
    /// Last click where it is visible
    pub until: Option<u32>,
    pub effect: Option<String>,
    /// Highlights (`@3-4:halo`, see `HIGHLIGHTS`), on top of the appearance
    pub marks: Vec<Stepped>,
    /// `@+n`: n clicks after the previous one, resolved by `Clicks::resolve`
    relative: Option<u32>,
}

/// Effects that highlight an element for the duration of their clicks, without driving its
/// appearance: `magnify` (the view zooms in around it), `halo` (a pulsing glow), `chase` (a light
/// running around it); they come on top of the appearance marker: `@1:up @3-4:halo`
pub const HIGHLIGHTS: [&str; 3] = ["magnify", "halo", "chase"];

impl Stepped {
    pub fn at(text: impl Into<String>, step: u32) -> Stepped {
        Stepped { text: text.into(), step, ..Stepped::default() }
    }

    /// A highlight marker (`@3:halo`) rather than an appearance
    pub fn highlights(&self) -> bool {
        self.effect.as_deref().is_some_and(|e| HIGHLIGHTS.contains(&e))
    }

    /// Adds a marker that was read: a highlight, or the appearance
    pub fn add(&mut self, m: Stepped) {
        if m.highlights() {
            self.marks.push(m);
        } else {
            let marks = std::mem::take(&mut self.marks);
            *self = Stepped { text: std::mem::take(&mut self.text), marks, ..m };
        }
    }
}

/// The last click written in the slide, for relative markers (`@+1`)
#[derive(Default)]
struct Clicks(u32);

impl Clicks {
    fn resolve(&mut self, mut s: Stepped) -> Stepped {
        if let Some(n) = s.relative.take() {
            s.step = self.0.saturating_add(n);
            if let Some(u) = s.until.as_mut() {
                *u = u.saturating_add(self.0);
            }
        }
        self.0 = self.0.max(s.step);
        let marks = std::mem::take(&mut s.marks);
        s.marks = marks.into_iter().map(|m| self.resolve(m)).collect();
        s
    }
}

/// `@2`, `@+1`, `@2-4`, `@2:up` → the marker (without the text)
pub fn marker(tok: &str) -> Option<Stepped> {
    let spec = tok.strip_prefix('@')?;
    let (spec, effect) = match spec.split_once(':') {
        Some((s, e)) if !e.is_empty() && e.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') => {
            (s, Some(e.to_string()))
        }
        Some(_) => return None,
        None => (spec, None),
    };
    let (relative, spec) = match spec.strip_prefix('+') {
        Some(rest) => (true, rest),
        None => (false, spec),
    };
    let (from, until) = match spec.split_once('-') {
        Some((a, b)) => (a, Some(b)),
        None => (spec, None),
    };
    let digits = |v: &str| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit());
    if !(digits(from) || relative && from.is_empty()) || until.is_some_and(|u| !digits(u)) {
        return None;
    }
    let from: u32 = if from.is_empty() { 1 } else { from.parse().ok()? };
    let until: Option<u32> = until.map(str::parse).transpose().ok()?;
    Some(Stepped {
        step: if relative { 0 } else { from },
        until,
        effect,
        relative: relative.then_some(from),
        ..Stepped::default()
    })
}

#[derive(Debug, Clone, Default)]
pub struct Fence {
    pub lang: String,
    /// `key="value"` pairs of the block's info string; a lone word (`magic-move`) is there, without a value
    pub attrs: BTreeMap<String, String>,
    /// Appearance click (`@n` in the info string; only `step` matters for code)
    pub appear: Stepped,
    pub content: String,
    /// Animated code: the click of its second version, the following ones coming one per click
    pub next_click: u32,
}

impl Fence {
    pub fn step(&self) -> u32 {
        self.appear.step
    }

    /// Animated code (` ````md magic-move `, four backticks): its versions, each one a code block
    /// inside
    pub fn versions(&self) -> Option<Vec<Fence>> {
        if self.lang != "magic-move" && !self.attrs.contains_key("magic-move") {
            return None;
        }
        let (mut versions, mut open): (Vec<Fence>, Option<(&str, Vec<&str>)>) = (Vec::new(), None);
        for line in self.content.lines() {
            let t = line.trim();
            match open.as_mut() {
                None => open = t.strip_prefix("```").map(|info| (info, Vec::new())),
                Some(_) if t.len() >= 3 && t.bytes().all(|b| b == b'`') => {
                    let (info, body) = open.take().unwrap_or_default();
                    versions.push(fence_of(info, body.join("\n")));
                }
                Some((_, body)) => body.push(line),
            }
        }
        Some(versions)
    }
}

/// `text @3` → text, 3 (see `marker`)
pub fn step_of(s: &str) -> Stepped {
    // markers are read from the end: one appearance, and highlights
    let mut t = s.trim_end();
    let mut found: Vec<Stepped> = Vec::new();
    while let Some(at) = t.rfind('@') {
        let head = &t[..at];
        let Some(m) = marker(&t[at..]).filter(|_| head.ends_with(char::is_whitespace)) else { break };
        if !m.highlights() && found.iter().any(|f| !f.highlights()) {
            break;
        }
        found.push(m);
        t = head.trim_end();
    }
    let mut out = Stepped::at(t, 0);
    for m in found.into_iter().rev() {
        out.add(m);
    }
    out
}

/// The rehearsal timing of a slide, `⏳95🏃3`: its planned duration in seconds and the number of
/// rehearsals it averages (🏃 may be missing)
pub fn timing(tok: &str) -> Option<(u32, Option<u32>)> {
    let rest = tok.strip_prefix('⏳')?;
    let (secs, runs) = rest.split_once('🏃').map_or((rest, None), |(d, r)| (d, Some(r)));
    let runs = match runs {
        Some(r) => Some(r.parse().ok()?),
        None => None,
    };
    Some((secs.parse().ok()?, runs))
}

/// `Title ⏳95🏃3 {.large - kicker="X"}`: the timing may also sit in the braces, and is read from
/// the former `duration="95" runs="3"` too
fn head_of(s: &str) -> Head {
    let s = s.trim();
    let mut head = Head { text: s.to_string(), ..Head::default() };
    let take_timing = |head: &mut Head, tok: &str| match timing(tok) {
        Some((secs, runs)) => {
            head.attrs.insert("duration".into(), secs.to_string());
            if let Some(r) = runs {
                head.attrs.insert("runs".into(), r.to_string());
            }
            true
        }
        None => false,
    };
    let (text, braces) = match (s.rfind('{'), s.ends_with('}')) {
        (Some(open), true) => (s[..open].trim(), Some(&s[open + 1..s.len() - 1])),
        _ => (s, None),
    };
    head.text = match text.rsplit_once(char::is_whitespace) {
        Some((before, last)) if take_timing(&mut head, last) => before.trim_end().to_string(),
        _ => text.to_string(),
    };
    let Some(braces) = braces else { return head };
    for tok in tokens(braces) {
        if take_timing(&mut head, &tok) {
            continue;
        }
        if tok == "-" {
            head.unnumbered = true;
        } else if let Some(c) = tok.strip_prefix('.') {
            head.classes.push(c.to_string());
        } else if let Some((k, v)) = tok.split_once('=') {
            head.attrs.insert(k.to_string(), v.trim_matches('"').to_string());
        }
    }
    head
}

/// Splits on whitespace, except between double quotes (the attributes of a heading or a block)
pub fn tokens(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c)
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur))
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur)
    }
    out
}

/// `` ```yaml title="x" lines="1|2" @1 ``
fn fence_of(info: &str, content: String) -> Fence {
    let toks = tokens(info);
    let mut fence = Fence { lang: toks.first().cloned().unwrap_or_default(), content, ..Fence::default() };
    for tok in toks.iter().skip(1) {
        if let Some(m) = marker(tok) {
            fence.appear.add(m);
        } else {
            let (k, v) = tok.split_once('=').unwrap_or((tok, ""));
            fence.attrs.insert(k.to_string(), v.trim_matches('"').to_string());
        }
    }
    fence
}

pub fn front_matter(src: &str) -> (BTreeMap<String, String>, &str) {
    let mut meta = BTreeMap::new();
    // lines ending the Windows way (`\r\n`) too
    let Some(rest) = src.strip_prefix("---\n").or_else(|| src.strip_prefix("---\r\n")) else { return (meta, src) };
    let (end, len) = match (rest.find("\n---\n"), rest.find("\r\n---\r\n")) {
        (Some(unix), Some(windows)) if windows < unix => (windows, 7),
        (Some(unix), _) => (unix, 5),
        (None, Some(windows)) => (windows, 7),
        (None, None) => return (meta, src),
    };
    for line in rest[..end].lines() {
        if let Some((k, v)) = line.split_once(':') {
            meta.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    (meta, &rest[end + len..])
}

/// Text of a heading or a bullet, without its block marker or its line breaks
fn source_text(raw: &str) -> String {
    raw.lines().map(|l| l.trim().trim_start_matches('#').trim_start()).collect::<Vec<_>>().join(" ").trim().to_string()
}

pub fn parse(src: &str) -> Deck {
    let (meta, body) = front_matter(src);
    let start = src.len() - body.len();
    let mut r = Reader { body, start, deck: Deck { meta, slides: Vec::new() }, clicks: Clicks::default() };
    let mut events = Parser::new_ext(body, Options::empty()).into_offset_iter();
    while let Some((ev, range)) = events.next() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => r.heading(level, range, &mut events),
            Event::Start(Tag::List(_)) => r.list(&mut events),
            Event::Start(Tag::CodeBlock(kind)) => r.code(&kind, &mut events),
            Event::Start(Tag::Paragraph) => r.paragraph(range, &mut events),
            _ => {}
        }
    }
    r.deck
}

type Events<'a> = pulldown_cmark::OffsetIter<'a>;

/// The reading in progress: the source (for the `@n` markers and `{…}`), the presentation being
/// built and the last click of the current slide
struct Reader<'a> {
    body: &'a str,
    /// Position of the body in the source (after the front matter)
    start: usize,
    deck: Deck,
    clicks: Clicks,
}

impl Reader<'_> {
    fn push(&mut self, block: Block) {
        if let Some(slide) = self.deck.slides.last_mut() {
            slide.blocks.push(block);
        }
    }

    /// `#` chapter, `##` slide, `###` heading inside the slide
    fn heading(&mut self, level: HeadingLevel, range: Range<usize>, events: &mut Events) {
        let at = self.start + range.start;
        let text = source_text(&self.body[range]);
        for (e, _) in events.by_ref() {
            if matches!(e, Event::End(TagEnd::Heading(_))) {
                break;
            }
        }
        if matches!(level, HeadingLevel::H1 | HeadingLevel::H2) {
            self.clicks = Clicks::default();
            let level = if level == HeadingLevel::H1 { 1 } else { 2 };
            self.deck.slides.push(Slide { level, head: head_of(&text), blocks: Vec::new(), at });
        } else {
            let title = self.clicks.resolve(step_of(&text));
            self.push(Block::Title(title));
        }
    }

    /// A list: its top-level bullets, each with its click
    fn list(&mut self, events: &mut Events) {
        let mut items = Vec::new();
        let mut depth = 1;
        for (e, range) in events.by_ref() {
            match e {
                Event::Start(Tag::List(_)) => depth += 1,
                Event::End(TagEnd::List(_)) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Event::Start(Tag::Item) if depth == 1 => {
                    let raw = self.body[range].trim_start().trim_start_matches(['-', '*', '+']).trim_start();
                    items.push(self.clicks.resolve(step_of(&source_text(raw))));
                }
                _ => {}
            }
        }
        self.push(Block::List(items));
    }

    fn code(&mut self, kind: &CodeBlockKind, events: &mut Events) {
        let info = match kind {
            CodeBlockKind::Fenced(i) => i.to_string(),
            CodeBlockKind::Indented => String::new(),
        };
        let mut content = String::new();
        for (e, _) in events.by_ref() {
            match e {
                Event::Text(t) => content.push_str(&t),
                Event::End(TagEnd::CodeBlock) => break,
                _ => {}
            }
        }
        let mut fence = fence_of(&info, content.trim_end_matches('\n').to_string());
        fence.appear = self.clicks.resolve(std::mem::take(&mut fence.appear));
        // Clicks taken by the block after its appearance: versions of an animated code,
        // highlighting steps (`lines="1|2-3"`)
        let versions = fence.versions().map(|v| v.len());
        let more = match versions {
            Some(n) => n.saturating_sub(1) as u32,
            None => fence.attrs.get("lines").map_or(0, |l| l.split('|').count() as u32 - 1),
        };
        if more > 0 && (fence.step() > 0 || versions.is_some()) {
            fence.next_click = if fence.step() > 0 { fence.step() } else { self.clicks.0 }.saturating_add(1);
            self.clicks.0 = self.clicks.0.max(fence.next_click.saturating_add(more - 1));
        }
        self.push(Block::Fence(fence));
    }

    /// A paragraph made of a single image is an image (`![alt](src "caption")`), possibly followed
    /// by its attributes (`{@2 muted pauses="0:12"}`); otherwise text
    fn paragraph(&mut self, range: Range<usize>, events: &mut Events) {
        let raw = self.body[range].trim().to_string();
        let (mut media, mut in_image, mut others) = (None::<Media>, false, 0);
        for (e, _) in events.by_ref() {
            match e {
                Event::End(TagEnd::Paragraph) => break,
                Event::Start(Tag::Image { dest_url, title, .. }) if media.is_none() => {
                    let caption = Some(title.to_string()).filter(|t| !t.trim().is_empty());
                    media = Some(Media { caption, ..Media::at(dest_url.to_string(), "") });
                    in_image = true;
                }
                Event::End(TagEnd::Image) => in_image = false,
                Event::Text(t) if in_image => media.as_mut().expect("inside an image").alt.push_str(&t),
                _ => others += 1,
            }
        }
        let media = match media {
            Some(m) if others == 0 => Some(m),
            Some(m) => with_attrs(m, &raw),
            None => None,
        };
        match media {
            Some(m) => {
                let m = self.media(m);
                self.push(Block::Image(m));
            }
            None => {
                let para = self.clicks.resolve(step_of(&raw));
                self.push(Block::Para(para));
            }
        }
    }

    /// The clicks of a medium: its appearance, the pauses of a video (one click each, after its
    /// appearance), then the clicks of its focus zones, made absolute
    fn media(&mut self, mut m: Media) -> Media {
        m.appear = self.clicks.resolve(std::mem::take(&mut m.appear));
        let pauses = m.pauses() as u32;
        if pauses > 0 {
            self.clicks.0 = self.clicks.0.max(m.appear.step.saturating_add(pauses));
        }
        if let Some(focus) = m.attrs.get("focus") {
            let zones: Vec<String> = focus
                .split(';')
                .map(|zone| {
                    let toks: Vec<String> = zone
                        .split_whitespace()
                        .map(|tok| match marker(tok) {
                            Some(mk) => absolute(&self.clicks.resolve(mk)),
                            None => tok.to_string(),
                        })
                        .collect();
                    toks.join(" ")
                })
                .collect();
            m.attrs.insert("focus".into(), zones.join("; "));
        }
        m
    }
}

/// A resolved marker, written back: `@3-4:halo`
fn absolute(m: &Stepped) -> String {
    let until = m.until.map(|u| format!("-{u}")).unwrap_or_default();
    let effect = m.effect.as_deref().map(|e| format!(":{e}")).unwrap_or_default();
    format!("@{}{until}{effect}", m.step)
}

/// `![alt](src){…}`: the image of the paragraph, read again without its attributes, and these
/// (`.class`, `@n`, `key="value"`, `word`); `None` if the paragraph holds anything else
fn with_attrs(media: Media, raw: &str) -> Option<Media> {
    let open = raw.strip_suffix('}')?.find("){")? + 1;
    let alone = parse(&format!("## x\n\n{}\n", &raw[..open]));
    let Some(Block::Image(mut m)) = alone.slides.into_iter().next()?.blocks.pop() else { return None };
    if m.src != media.src {
        return None;
    }
    for tok in tokens(&raw[open + 1..raw.len() - 1]) {
        if let Some(mk) = marker(&tok) {
            m.appear.add(mk);
        } else if let Some(c) = tok.strip_prefix('.') {
            let classes = m.attrs.entry("class".into()).or_default();
            if !classes.is_empty() {
                classes.push(' ');
            }
            classes.push_str(c);
        } else {
            let (k, v) = tok.split_once('=').unwrap_or((&tok, ""));
            m.attrs.insert(k.to_string(), v.trim_matches('"').to_string());
        }
    }
    if let Some(c) = m.attrs.remove("caption") {
        m.caption = Some(c);
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timing_of_a_heading() {
        let h = head_of("Ingress ⏳95🏃3 {.large}");
        assert_eq!((h.text.as_str(), h.attrs["duration"].as_str(), h.attrs["runs"].as_str()), ("Ingress", "95", "3"));
        assert_eq!(h.classes, ["large"]);
        let h = head_of("Pods {.cols ⏳40}");
        assert_eq!((h.text.as_str(), h.attrs["duration"].as_str(), h.attrs.get("runs")), ("Pods", "40", None));
        // the former attributes
        let h = head_of("Old {duration=\"30\" runs=\"2\"}");
        assert_eq!((h.attrs["duration"].as_str(), h.attrs["runs"].as_str()), ("30", "2"));
        assert_eq!(head_of("Waiting ⏳ for it").text, "Waiting ⏳ for it");
        assert_eq!(timing("⏳12🏃x"), None);
    }

    #[test]
    fn steps_and_heads() {
        let s = step_of("List the pods @2");
        assert_eq!((s.text.as_str(), s.step), ("List the pods", 2));
        assert_eq!(step_of("admin@corp").step, 0);
        assert_eq!(
            step_of("bullet @2-4:up"),
            Stepped { text: "bullet".into(), step: 2, until: Some(4), effect: Some("up".into()), ..Stepped::default() }
        );
        assert_eq!(step_of("x @3:").step, 0);
        assert_eq!(step_of("x @3:a_b").step, 0);
        assert_eq!(step_of("x @2-b").step, 0);
        // one appearance and highlights, in any order
        let s = step_of("bullet @1 @2-3:halo @5:magnify");
        assert_eq!((s.text.as_str(), s.step), ("bullet", 1));
        let marks: Vec<(&str, u32, Option<u32>)> =
            s.marks.iter().map(|m| (m.effect.as_deref().unwrap(), m.step, m.until)).collect();
        assert_eq!(marks, [("halo", 2, Some(3)), ("magnify", 5, None)]);
        let s = step_of("bullet @4:chase @1:up");
        assert_eq!((s.step, s.effect.as_deref(), s.marks.len()), (1, Some("up"), 1));
        // two appearances: the second one stays text
        assert_eq!(step_of("x @1 @2").text, "x @1");
        let s = Clicks(3).resolve(step_of("y @+1 @+2:halo"));
        assert_eq!((s.step, s.marks[0].step), (4, 6));
        let s = Clicks(3).resolve(step_of("z @+1-2"));
        assert_eq!((s.step, s.until), (4, Some(5)));
        let mut clicks = Clicks::default();
        let steps: Vec<u32> =
            ["a @2", "b @+1", "c @+", "d @+0"].iter().map(|t| clicks.resolve(step_of(t)).step).collect();
        assert_eq!(steps, [2, 3, 4, 4]);
        let h = head_of("How to {kicker=\"Template\" .compact}");
        assert_eq!(h.text, "How to");
        assert_eq!(h.classes, vec!["compact"]);
        assert_eq!(h.attrs["kicker"], "Template");
        assert!(head_of("Questions? {-}").unnumbered);
        assert_eq!(head_of("No attributes").text, "No attributes");
    }

    #[test]
    fn blocks() {
        let d = parse(
            "---\ntitle: T\n---\n\n# C\n\nsubtitle\n\n## S\n\n### A @1\n\n- one @1\n- two\n\n\
             ```yaml title=\"f.yml\" @1\na: b\n```\n\n![t](x.cast)\n\nNote: n\n",
        );
        assert_eq!(d.meta["title"], "T");
        assert_eq!(d.slides.len(), 2);
        assert_eq!(d.slides[1].at, "---\ntitle: T\n---\n\n# C\n\nsubtitle\n\n".len());
        let b = &d.slides[1].blocks;
        assert!(matches!(&b[0], Block::Title(t) if t.text == "A" && t.step == 1));
        assert!(matches!(&b[1], Block::List(l) if l.len() == 2 && l[0].step == 1));
        assert!(
            matches!(&b[2], Block::Fence(f) if f.lang == "yaml" && f.step() == 1 && f.attrs["title"] == "f.yml" && f.content == "a: b")
        );
        assert!(matches!(&b[3], Block::Image(m) if m.src == "x.cast" && m.alt == "t" && m.caption.is_none()));
        assert!(matches!(&b[4], Block::Para(p) if p.text == "Note: n"));
    }

    #[test]
    fn media_with_a_caption_attributes_and_clicks() {
        let d = parse(
            "## S\n\n- a @1\n\n![Map](img/map.png \"Source: IGN\"){@2 .wide focus=\"10 20 30 40 @+1; 50 50 10 10 @+1:magnify\"}\n\n\
             ![](img/demo.mp4){muted pauses=\"0:12 1:05\" caption=\"Demo\"}\n\n- b @+1\n",
        );
        let b = &d.slides[0].blocks;
        let Block::Image(map) = &b[1] else { panic!("{:?}", b[1]) };
        assert_eq!(
            (map.src.as_str(), map.alt.as_str(), map.caption.as_deref()),
            ("img/map.png", "Map", Some("Source: IGN"))
        );
        assert_eq!(map.appear.step, 2);
        assert_eq!(map.attrs["class"], "wide");
        assert_eq!(map.attrs["focus"], "10 20 30 40 @3; 50 50 10 10 @4:magnify");
        let Block::Image(video) = &b[2] else { panic!("{:?}", b[2]) };
        assert!(video.is_video() && !map.is_video());
        assert_eq!((video.caption.as_deref(), video.pauses()), (Some("Demo"), 2));
        assert!(video.attrs.contains_key("muted"));
        // the two pauses take clicks 1 and 2 after its appearance (0): the next bullet comes after the zones
        assert!(matches!(&b[3], Block::List(l) if l[0].step == 5));
        // text after the image: a paragraph
        let d = parse("## S\n\n![](a.png) and text\n");
        assert!(matches!(&d.slides[0].blocks[0], Block::Para(_)));
    }

    #[test]
    fn nested_lists_and_indented_code() {
        let d = parse("## S\n\n- a\n  - inner\n- b @1\n\npara\n\n    indented\n");
        let b = &d.slides[0].blocks;
        assert!(matches!(&b[0], Block::List(l) if l.len() == 2 && l[1].step == 1));
        assert!(matches!(&b[2], Block::Fence(f) if f.lang.is_empty() && f.content == "indented"));
        assert!(front_matter("no front matter").0.is_empty());
        assert!(front_matter("---\nunclosed\n").0.is_empty());
        let (meta, body) = front_matter("---\r\ntitle: T\r\ntheme: paper\r\n---\r\n# One\r\n");
        assert_eq!((meta["title"].as_str(), meta["theme"].as_str(), body), ("T", "paper", "# One\r\n"));
    }

    #[test]
    fn highlighted_lines_take_one_click_each() {
        let d = parse("## S\n\n```yaml lines=\"1|2|3\" @2\na\nb\nc\n```\n\n- next @+1\n");
        let b = &d.slides[0].blocks;
        assert!(matches!(&b[0], Block::Fence(f) if f.next_click == 3));
        assert!(matches!(&b[1], Block::List(l) if l[0].step == 5));
    }

    #[test]
    fn animated_code_takes_one_click_per_version() {
        let d = parse(
            "## S\n\n- a @1\n\n````md magic-move title=\"x\"\n```rust\nlet a = 1;\n```\n```rust\nlet a = 2;\n```\n\
             ```rust\nlet a = 3;\n```\n````\n\n- b @+1\n",
        );
        let b = &d.slides[0].blocks;
        let Block::Fence(f) = &b[1] else { panic!() };
        assert_eq!(
            f.versions().unwrap().iter().map(|v| v.content.as_str()).collect::<Vec<_>>(),
            ["let a = 1;", "let a = 2;", "let a = 3;"]
        );
        assert_eq!((f.next_click, f.attrs["title"].as_str()), (2, "x"));
        assert!(matches!(&b[2], Block::List(l) if l[0].step == 4));
    }
}
