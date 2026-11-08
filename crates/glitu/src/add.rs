//! `glitu add <kind>`: a slide filled with an example to adapt, added to the presentation (before
//! its closing `{.end}` slide, if it has one). Some kinds also write the files their example
//! needs: a terminal recording (`casts/`), an image or a video (`img/`). An existing file is never
//! overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use clap::ValueEnum;

/// Languages of the code examples
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Lang {
    Rust,
    Python,
    Javascript,
    Yaml,
    Bash,
}

/// How a step comes in (`@2:up`)
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Effect {
    Up,
    Down,
    Left,
    Right,
    Grow,
    Shrink,
    Zoom,
    Red,
    Green,
    Blue,
    Dim,
}

/// Transition into the slide
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Transition {
    None,
    Fade,
    Slide,
    Convex,
    Concave,
    Zoom,
}

/// Layout of a slide with a visual (code, diagram, terminal…)
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum Layout {
    /// The text on the left, the visual on the right
    #[default]
    Panel,
    /// The visual across the width, the text as a caption above it
    Large,
    /// The visual alone, without margins
    Full,
}

/// Where the image of an image slide goes
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum ImageLayout {
    /// Covers the right half
    #[default]
    Right,
    /// Covers the left half
    Left,
    /// In the visual area, next to the text
    Panel,
    /// The whole slide
    Full,
}

/// Where the web page of an iframe slide goes
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum PageLayout {
    #[default]
    Full,
    Left,
    Right,
}

/// Direction of a diagram
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum Dir {
    #[default]
    Right,
    Down,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Bullets,
    Code {
        lang: Lang,
        annotated: bool,
    },
    MagicMove {
        lang: Lang,
    },
    Diagram {
        dir: Dir,
    },
    Terminal {
        commands: Vec<String>,
        speed: f64,
    },
    Formula,
    Image {
        src: Option<String>,
        layout: ImageLayout,
    },
    /// A video; `pauses`: where it stops, each click resuming it
    Video {
        src: Option<String>,
        pauses: Vec<String>,
    },
    Emoji,
    Cols,
    Quote {
        author: String,
    },
    Fact {
        figure: String,
    },
    Statement,
    Toc,
    Title,
    Interlude,
    End,
    Iframe {
        url: String,
        layout: PageLayout,
    },
    Animate,
    Chapter,
}

/// What `glitu add` was asked for
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub kind: Kind,
    pub title: Option<String>,
    /// A chapter (`# …`) opened before the slide
    pub chapter: Option<String>,
    pub transition: Option<Transition>,
    pub background: Option<String>,
    pub notes: bool,
    /// Number of clicks; 0: everything at once
    pub steps: usize,
    pub effect: Option<Effect>,
    pub layout: Layout,
}

impl Request {
    pub fn new(kind: Kind) -> Request {
        Request {
            kind,
            title: None,
            chapter: None,
            transition: None,
            background: None,
            notes: true,
            steps: 3,
            effect: None,
            layout: Layout::Panel,
        }
    }
}

/// A slide to add: its markdown, and the files its example needs (relative to the folder of the
/// presentation's main file)
#[derive(Debug)]
pub struct Slide {
    pub markdown: String,
    pub files: Vec<(PathBuf, Vec<u8>)>,
}

fn value_name<T: ValueEnum>(v: T) -> String {
    v.to_possible_value().map(|p| p.get_name().to_string()).unwrap_or_default()
}

/// ` @n`, or ` @n:effect`; nothing without steps
fn at(n: usize, r: &Request) -> String {
    match (r.steps, r.effect) {
        (0, _) => String::new(),
        (_, Some(e)) => format!(" @{n}:{}", value_name(e)),
        (_, None) => format!(" @{n}"),
    }
}

/// `## Title {.class key="value"}`
fn heading(title: &str, classes: &[&str], r: &Request) -> String {
    let mut attrs: Vec<String> = classes.iter().map(|c| format!(".{c}")).collect();
    if let Some(t) = r.transition {
        attrs.push(format!("transition=\"{}\"", value_name(t)));
    }
    if let Some(b) = &r.background {
        attrs.push(format!("background=\"{b}\""));
    }
    if attrs.is_empty() { format!("## {title}") } else { format!("## {title} {{{}}}", attrs.join(" ")) }
}

/// The layout class of a slide with a visual
fn layout_class(r: &Request) -> Option<&'static str> {
    match r.layout {
        Layout::Panel => None,
        Layout::Large => Some("large"),
        Layout::Full => Some("full"),
    }
}

/// A slide with a visual: the text beside it (above it with `large`); with `full`, the visual alone
fn with_visual(r: &Request, heading: &str, text: &str, visual: &str) -> String {
    match r.layout {
        Layout::Full => format!("{heading}\n\n{visual}"),
        _ => format!("{heading}\n\n{text}\n{visual}"),
    }
}

/// `Note: …`, then `[click] …` for each step
fn notes(r: &Request, intro: &str, clicks: &[String]) -> String {
    if !r.notes {
        return String::new();
    }
    let mut s = format!("\nNote: {intro}");
    if r.steps > 0 {
        for c in clicks {
            s += &format!(" [click] {c}");
        }
    }
    s + "\n"
}

/// The lines `from-to` shown at each of `n` clicks, over `lines` lines
fn line_groups(lines: usize, n: usize) -> Vec<String> {
    let n = n.min(lines).max(1);
    (0..n)
        .map(|i| {
            let (a, b) = (i * lines / n + 1, (i + 1) * lines / n);
            if a == b { a.to_string() } else { format!("{a}-{b}") }
        })
        .collect()
}

/// A code example: (file name, code, annotated code, three versions for magic-move)
struct Sample {
    file: &'static str,
    code: &'static str,
    annotated: &'static str,
    versions: [&'static str; 3],
}

fn sample(lang: Lang) -> Sample {
    match lang {
        Lang::Rust => Sample {
            file: "main.rs",
            code: "fn main() {\n    let words = [\"markdown\", \"slides\", \"glitu\"];\n    let mut total = 0;\n    for w in words {\n        total += w.len();\n    }\n    println!(\"{total} letters\");\n}",
            annotated: "fn main() {\n    let words = [\"markdown\", \"slides\", \"glitu\"];\n    let mut total = 0; // [!code --]\n    for w in words { total += w.len(); } // [!code --]\n    let total: usize = words.iter().map(|w| w.len()).sum(); // [!code ++]\n    println!(\"{total} letters\"); // [!code highlight]\n}",
            versions: [
                "fn total(v: &Vec<i32>) -> i32 {\n    let mut s = 0;\n    for x in v {\n        s += x;\n    }\n    s\n}",
                "fn total(v: &[i32]) -> i32 {\n    let mut s = 0;\n    for x in v {\n        s += x;\n    }\n    s\n}",
                "fn total(v: &[i32]) -> i32 {\n    v.iter().sum()\n}",
            ],
        },
        Lang::Python => Sample {
            file: "app.py",
            code: "def greet(names):\n    for name in names:\n        message = \"Hello \" + name\n        print(message)\n\n\ngreet([\"Ada\", \"Alan\"])",
            annotated: "def greet(names):\n    for name in names:\n        message = \"Hello \" + name  # [!code --]\n        message = f\"Hello {name}\"  # [!code ++]\n        print(message)  # [!code highlight]\n\n\ngreet([\"Ada\", \"Alan\"])",
            versions: [
                "def greet(name):\n    print(\"Hello \" + name)",
                "def greet(name):\n    print(f\"Hello {name}\")",
                "def greet(name: str) -> None:\n    \"\"\"Say hello.\"\"\"\n    print(f\"Hello {name}\")",
            ],
        },
        Lang::Javascript => Sample {
            file: "total.js",
            code: "function total(prices) {\n  let sum = 0;\n  for (const p of prices) {\n    sum += p;\n  }\n  return sum;\n}",
            annotated: "function total(prices) {\n  let sum = 0; // [!code --]\n  for (const p of prices) sum += p; // [!code --]\n  return sum; // [!code --]\n  return prices.reduce((sum, p) => sum + p, 0); // [!code ++]\n}",
            versions: [
                "function total(prices) {\n  let sum = 0;\n  for (const p of prices) {\n    sum += p;\n  }\n  return sum;\n}",
                "const total = (prices) => {\n  let sum = 0;\n  for (const p of prices) {\n    sum += p;\n  }\n  return sum;\n};",
                "const total = (prices) =>\n  prices.reduce((sum, p) => sum + p, 0);",
            ],
        },
        Lang::Yaml => Sample {
            file: "deployment.yml",
            code: "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: web\nspec:\n  replicas: 3\n  template:\n    spec:\n      containers:\n        - image: nginx:1.27",
            annotated: "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: web  # [!code highlight]\nspec:\n  replicas: 1  # [!code --]\n  replicas: 3  # [!code ++]",
            versions: [
                "kind: Deployment\nspec:\n  replicas: 1",
                "kind: Deployment\nspec:\n  replicas: 3",
                "kind: Deployment\nspec:\n  replicas: 3\n  template:\n    spec:\n      containers:\n        - image: nginx:1.27",
            ],
        },
        Lang::Bash => Sample {
            file: "publish.sh",
            code: "#!/usr/bin/env bash\nset -euo pipefail\n\nglitu build -o talk.html\nscp talk.html server:/var/www/\necho \"Published\"",
            annotated: "#!/usr/bin/env bash\nset -euo pipefail  # [!code highlight]\n\nglitu build -o talk.html\nscp talk.html server:/var/www/  # [!code --]\nrsync -a talk.html server:/var/www/  # [!code ++]",
            versions: [
                "glitu build",
                "glitu build -o talk.html",
                "glitu build -o talk.html\nscp talk.html server:/var/www/",
            ],
        },
    }
}

/// A name for a file: the title in lowercase, words joined by `-`
pub fn slug(title: &str) -> String {
    let mut s = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "slide".into() } else { s }
}

/// `dir/stem.ext`, or `dir/stem-2.ext`… if it exists (on disk or among `taken`)
fn free(base: &Path, dir: &str, stem: &str, ext: &str, taken: &[PathBuf]) -> PathBuf {
    (1..)
        .map(|i| {
            let name = if i == 1 { format!("{stem}.{ext}") } else { format!("{stem}-{i}.{ext}") };
            Path::new(dir).join(name)
        })
        .find(|p| !base.join(p).exists() && !taken.contains(p))
        .unwrap()
}

/// The line `glitu add diagram` writes above the diagram when the skemo tool is not found
pub const NEEDS_SKEMO: &str =
    "<!-- This diagram needs skemo (https://skemo.net/install): not found on this machine -->";

/// The slide asked for; `base` is the folder of the presentation's main file (for the files to
/// write, and the listing of `ls` in a terminal recording)
pub fn slide(r: &Request, base: &Path) -> Slide {
    let mut files = Vec::new();
    let title = |default: &str| r.title.clone().unwrap_or_else(|| default.to_string());
    let n = r.steps;
    let md = match &r.kind {
        Kind::Chapter => {
            return Slide { markdown: format!("# {}\n\nThe subtitle of the chapter\n", title("Chapter")), files };
        }
        Kind::Bullets => {
            let mut s = format!(
                "{}\n\n### Each click shows the next point\n\n- Visible from the start\n",
                heading(&title("Steps"), &[], r)
            );
            if n == 0 {
                s += "- A second point\n- A [key word]{.circle} circled\n";
            }
            for i in 1..=n {
                let text = match i {
                    _ if i == n && n > 1 => format!("A [key word]{{.circle @{i}}} circled at click {i}"),
                    1 => "Appears at the first click".into(),
                    _ => format!("Appears at click {i}"),
                };
                s += &format!("- {text}{}\n", at(i, r));
            }
            let clicks: Vec<String> = (1..=n).map(|i| format!("Point {i}.")).collect();
            s + &notes(r, "What to say on this slide, shown by the S key.", &clicks)
        }
        Kind::Code { lang, annotated } => {
            let sm = sample(*lang);
            let lang_name = value_name(*lang);
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title("Code"), &classes, r);
            if *annotated {
                let text = "### Annotated code\n\n- `[!code --]` removed, `[!code ++]` added\n- `[!code highlight]` emphasised\n";
                let visual = format!("```{lang_name} title=\"{}\"\n{}\n```\n", sm.file, sm.annotated);
                let s = with_visual(r, &h, text, &visual);
                return Slide { markdown: s + &notes(r, "The annotations disappear from the code shown.", &[]), files };
            }
            let lines = sm.code.lines().count();
            let groups = line_groups(lines, n);
            let mut text = String::from("### Lines highlighted click by click\n\n");
            for (i, g) in groups.iter().enumerate() {
                text += &format!("- Lines {g}{}\n", at(i + 1, r));
            }
            let attrs = if n == 0 { String::new() } else { format!(" lines=\"{}\" @1", groups.join("|")) };
            let visual = format!("```{lang_name} title=\"{}\"{attrs}\n{}\n```\n", sm.file, sm.code);
            let clicks: Vec<String> = groups.iter().map(|g| format!("Lines {g}.")).collect();
            with_visual(r, &h, &text, &visual) + &notes(r, "Walk through the code.", &clicks)
        }
        Kind::MagicMove { lang } => {
            let sm = sample(*lang);
            let lang_name = value_name(*lang);
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title("Animated code"), &classes, r);
            let text =
                "### One version per click\n\n- The first version\n- Then the second @1\n- And the last one @2\n";
            let mut visual = format!("````md magic-move title=\"{}\"\n", sm.file);
            for v in sm.versions {
                visual += &format!("```{lang_name}\n{v}\n```\n");
            }
            visual += "````\n";
            with_visual(r, &h, text, &visual)
                + &notes(r, "One version per click: the words kept slide to their new place.", &[])
        }
        Kind::Diagram { dir } => {
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title("Diagram"), &classes, r);
            let mut text = String::from("### A skemo diagram\n\n");
            let cards = [
                ("source", "file", "file-text", "Source", "what comes in"),
                ("tool", "tool", "terminal", "Tool", "what transforms it"),
                ("result", "output", "presentation", "Result", "what you get"),
            ];
            // a card per click; three, visible at once, without clicks
            let count = if n == 0 { 3 } else { n };
            let card = |i: usize| {
                let (id, kind, icon, label, about) = cards[(i - 1).min(2)];
                if i > 3 {
                    (format!("step{i}"), kind, icon, format!("Step {i}"), about)
                } else {
                    (id.into(), kind, icon, label.into(), about)
                }
            };
            for i in 1..=count {
                text += &format!("- {}{}\n", card(i).3, at(i, r));
            }
            let mut s = format!("```skemo\ndiagram(dir=\"{}\")\n", value_name(*dir));
            for i in 1..=count {
                let (id, kind, icon, label, about) = card(i);
                s += &format!("  card#{id}.{kind}(icon=\"{icon}\") {label}{}\n    | {about}\n", at(i, r));
                if i < count {
                    s += &format!("    to({})\n", card(i + 1).0);
                }
            }
            s += "```\n";
            let clicks: Vec<String> = (1..=n).map(|i| format!("Card {i}.")).collect();
            with_visual(r, &h, &text, &s) + &notes(r, "The cards appear with the clicks of the slide.", &clicks)
        }
        Kind::Terminal { commands, speed } => {
            let title = title("Terminal");
            let path = free(base, "casts", &slug(&title), "cast", &[]);
            let commands: Vec<String> =
                if commands.is_empty() { vec!["echo \"Hello, glitu\"".into(), "ls".into()] } else { commands.clone() };
            files.push((path.clone(), cast(&title, &commands, *speed, base).into_bytes()));
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title, &classes, r);
            let mut text = String::from("### One command per click\n\n");
            let r1 = Request { steps: commands.len(), ..r.clone() };
            for (i, c) in commands.iter().enumerate() {
                text += &format!("- `{}`{}\n", c.replace('`', "'"), at(i + 1, &r1));
            }
            let visual = format!("![~/{}]({})\n", dir_name(base), path.display().to_string().replace('\\', "/"));
            let clicks: Vec<String> = commands.iter().map(|c| format!("`{c}`.")).collect();
            with_visual(r, &h, &text, &visual)
                + &notes(
                    &r1,
                    &format!(
                        "A sample recording, written by glitu add: record your own with `asciinema rec {}` (add markers to stop between commands).",
                        path.display()
                    ),
                    &clicks,
                )
        }
        Kind::Formula => {
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title("Formulas"), &classes, r);
            let mut text = String::from("### Formulas in LaTeX\n\n");
            let inline = [
                "The equivalence $E = mc^2$",
                "A sum $\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$",
                "Pythagoras $a^2 + b^2 = c^2$",
                "Euler $e^{i\\pi} + 1 = 0$",
            ];
            let count = n.clamp(1, 4);
            for (i, f) in inline.iter().take(count).enumerate() {
                text += &format!("- {f}{}\n", at(i + 1, r));
            }
            let block_at =
                if n == 0 || r.layout == Layout::Full { String::new() } else { format!(" @{}", count.min(n)) };
            let visual =
                format!("```math{block_at}\n\\int_{{-\\infty}}^{{+\\infty}} e^{{-x^2}}\\,dx = \\sqrt{{\\pi}}\n```\n");
            with_visual(r, &h, &text, &visual)
                + &notes(r, "`$…$` in the text, `$$…$$` as a block, a math block as the visual.", &[])
        }
        Kind::Image { src, layout } => {
            let src = match src {
                Some(s) => s.clone(),
                // one placeholder for all the image slides
                None => {
                    if !base.join(PLACEHOLDER_PATH).exists() {
                        files.push((PLACEHOLDER_PATH.into(), PLACEHOLDER.as_bytes().to_vec()));
                    }
                    PLACEHOLDER_PATH.to_string()
                }
            };
            let t = title("Image");
            let text = "### The text\n\n- The image is in `img/`\n- Replace it with your own\n";
            let s = match layout {
                ImageLayout::Right | ImageLayout::Left => {
                    let class = if *layout == ImageLayout::Right { ".image-right" } else { ".image-left" };
                    let h = heading(&t, &[], r);
                    let h = match h.strip_suffix('}') {
                        Some(open) => open.replacen('{', &format!("{{{class} image=\"{src}\" "), 1) + "}",
                        None => format!("{h} {{{class} image=\"{src}\"}}"),
                    };
                    format!("{h}\n\n{text}")
                }
                ImageLayout::Panel => format!("{}\n\n{text}\n![]({src})\n", heading(&t, &[], r)),
                ImageLayout::Full => format!("{}\n\n![]({src})\n", heading(&t, &["full"], r)),
            };
            s + &notes(r, "An image beside the text.", &[])
        }
        Kind::Video { src, pauses } => {
            let placeholder = src.is_none();
            let src = match src {
                Some(s) => s.clone(),
                None => {
                    if !base.join(VIDEO_PLACEHOLDER_PATH).exists() {
                        files.push((VIDEO_PLACEHOLDER_PATH.into(), VIDEO_PLACEHOLDER.to_vec()));
                    }
                    VIDEO_PLACEHOLDER_PATH.to_string()
                }
            };
            // the placeholder pauses at 2 and 4 s; another video, where it is asked to
            let pauses: Vec<String> =
                if placeholder && pauses.is_empty() { vec!["0:02".into(), "0:04".into()] } else { pauses.clone() };
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title("Video"), &classes, r);
            let mut text;
            let mut attrs = String::from("muted");
            if pauses.is_empty() {
                text = String::from("### A video\n\n- It starts when the slide is shown\n- `muted`: without sound\n");
                attrs += " autoplay";
            } else {
                text = format!("### Pauses on the way\n\n- It plays, then stops at {}\n", pauses[0]);
                for (i, p) in pauses.iter().enumerate().skip(1) {
                    text += &format!("- A click: on to {p} @{i}\n");
                }
                text += &format!("- A click: to the end @{}\n", pauses.len());
                attrs += &format!(" pauses=\"{}\"", pauses.join(" "));
            }
            // on the placeholder, a circle around its clock during the first pause
            if placeholder && pauses.first().is_some_and(|p| p == "0:02") {
                attrs += " focus=\"33 50 34 16 0:02-0:03\"";
            }
            let visual = format!("![]({src} \"Source: where the video comes from\"){{{attrs}}}\n");
            let clicks: Vec<String> = (1..=pauses.len()).map(|i| format!("The video resumes ({i}).")).collect();
            let intro = if pauses.is_empty() {
                "The video starts on its own; remove `muted` for the sound (the browser may then wait for a first key press)."
            } else {
                "The video stops at each pause, the time to comment."
            };
            let r0 = Request { steps: pauses.len(), ..r.clone() };
            with_visual(r, &h, &text, &visual) + &notes(&r0, intro, &clicks)
        }
        Kind::Emoji => {
            let classes: Vec<&str> = layout_class(r).into_iter().collect();
            let h = heading(&title("Reactions"), &classes, r);
            let mut text = String::from("### Big emojis\n\n");
            let set = [("😱", "The problem"), ("🤔", "The idea"), ("🛠️", "The work"), ("🎉", "The result")];
            // the first one, then one per click
            let count = (n + 1).min(set.len());
            for (i, (_, label)) in set.iter().take(count).enumerate() {
                text += &format!("- {label}{}\n", if i == 0 { String::new() } else { at(i, r) });
            }
            let mut s = String::from("```emoji\n");
            for (i, (e, _)) in set.iter().take(count).enumerate() {
                s += &format!("{e}{}\n", if i == 0 { String::new() } else { at(i, r) });
            }
            s += "```\n";
            with_visual(r, &h, &text, &s) + &notes(r, "Each emoji replaces the previous one at a click.", &[])
        }
        Kind::Cols => {
            let s = format!(
                "{}\n\nWhat comes before `::left::` spans both columns\n\n::left::\n\n### Before\n\n- A first point\n- A second one{}\n\n::right::\n\n### After\n\nA paragraph that appears{}\n",
                heading(&title("Two columns"), &["cols"], r),
                at(1, r),
                at(2, r)
            );
            s + &notes(r, "Left, then right.", &[])
        }
        Kind::Quote { author } => format!(
            "{}\n\nSimplicity is the ultimate sophistication.\n\n— {author}\n",
            heading(&title("Quote"), &["quote"], r)
        ),
        Kind::Fact { figure } => format!(
            "{}\n\n### {figure}\n\nof incidents avoided by automation\n",
            heading(&title("Key figure"), &["fact"], r)
        ),
        Kind::Statement => format!(
            "{}\n\nCode is the [documentation]{{.underline .cyan{}}} that [never]{{.circle{}}} [lies]{{.highlight{}}}.\n",
            heading(&title("Statement"), &["statement"], r),
            at(1, r),
            at(2, r),
            at(3, r)
        ),
        Kind::Toc => format!("{}\n", heading(&title("Contents"), &["toc"], r)),
        Kind::Title => {
            format!("{}\n\nThe subtitle, in large\n\nYour name · 2026\n", heading(&title("Title"), &["intro"], r))
        }
        Kind::Interlude => format!(
            "{}\n\nA divider, without a number nor a change of chapter\n",
            heading(&title("Interlude"), &["section"], r)
        ),
        Kind::End => format!("{}\n\nQuestions?\n", heading(&title("Thank you"), &["end"], r)),
        Kind::Iframe { url, layout } => {
            let class = match layout {
                PageLayout::Full => "iframe",
                PageLayout::Left => "iframe-left",
                PageLayout::Right => "iframe-right",
            };
            let h = heading(&title("Web page"), &[class], r);
            let h = h.trim_end_matches('}').to_string() + &format!(" url=\"{url}\"}}");
            match layout {
                PageLayout::Full => format!("{h}\n"),
                _ => format!("{h}\n\n### Live\n\n- The page beside the text\n"),
            }
        }
        Kind::Animate => {
            let t = title("Animated slides");
            let h = heading(&t, &["animate", "center"], r);
            format!(
                "{h}\n\n[Shared]{{#label .highlight}}\n\n- A first point\n\n{h}\n\n- A first point\n- A second one, fading in\n\n[Shared]{{#label .highlight}}\n{}",
                notes(r, "The elements shared by the two slides slide; the new ones fade in.", &[])
            )
        }
    };
    let md = match &r.chapter {
        Some(c) => format!("# {c}\n\nThe subtitle of the chapter\n\n{md}"),
        None => md,
    };
    Slide { markdown: md, files }
}

fn dir_name(base: &Path) -> String {
    base.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "talk".into())
}

/// A JSON string
fn json(s: &str) -> String {
    serde_json::to_string(s).unwrap()
}

/// A sample asciicast (v2): each command typed at `speed` characters per second, its output, then
/// a marker where the player waits for the next click. `echo` prints its text, `ls` the folder
/// of the presentation, `pwd` its path; other commands print nothing (record a real session)
fn cast(title: &str, commands: &[String], speed: f64, base: &Path) -> String {
    let dir = dir_name(base);
    let prompt = format!("\u{1b}[32m\u{1b}[1mme@desk\u{1b}[0m:\u{1b}[34m\u{1b}[1m~/{dir}\u{1b}[0m$ ");
    let mut out = format!(
        "{{\"version\": 2, \"width\": 72, \"height\": 12, \"title\": {}, \"idle_time_limit\": 1, \"env\": {{\"TERM\": \"xterm-256color\", \"SHELL\": \"/bin/bash\"}}}}\n",
        json(title)
    );
    let mut t = 0.0f64;
    let mut ev = |t: f64, kind: &str, data: &str| out += &format!("[{t:.3}, \"{kind}\", {}]\n", json(data));
    let step = 1.0 / speed.max(0.1);
    ev(t, "o", &prompt);
    t += 0.5;
    for c in commands {
        // an irregular rhythm, as by hand, always the same
        for (i, ch) in c.chars().enumerate() {
            t += step * [1.0, 0.7, 1.3, 0.9, 1.1][i % 5];
            ev(t, "o", &ch.to_string());
        }
        t += 0.3;
        ev(t, "o", "\r\n");
        let output = output_of(c, base);
        if !output.is_empty() {
            t += 0.2;
            ev(t, "o", &(output.replace('\n', "\r\n") + "\r\n"));
        }
        t += 0.1;
        ev(t, "o", &prompt);
        ev(t, "m", c);
        t += 0.8;
    }
    out
}

fn output_of(command: &str, base: &Path) -> String {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.first() {
        Some(&"echo") => command["echo".len()..].trim().trim_matches(['"', '\'']).to_string(),
        Some(&"pwd") => format!("/home/me/{}", dir_name(base)),
        Some(&"ls") if words.len() == 1 => {
            let mut names: Vec<String> = fs::read_dir(base)
                .map(|d| {
                    d.flatten()
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .filter(|n| !n.starts_with('.'))
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            names.join("  ")
        }
        _ => String::new(),
    }
}

/// The video written by `glitu add video` without `--src`, unless it exists: 6 s, its clock in the
/// middle (rendered by ffmpeg, `gradients` and `drawtext`)
const VIDEO_PLACEHOLDER_PATH: &str = "img/placeholder.webm";
const VIDEO_PLACEHOLDER: &[u8] = include_bytes!("../assets/placeholder.webm");

/// The image written by `glitu add image` without `--src`, unless it exists
const PLACEHOLDER_PATH: &str = "img/placeholder.svg";
const PLACEHOLDER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600" width="800" height="600">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#1e3a8a"/>
      <stop offset="1" stop-color="#0891b2"/>
    </linearGradient>
  </defs>
  <rect width="800" height="600" fill="url(#g)"/>
  <circle cx="560" cy="200" r="70" fill="#fbbf24" opacity=".9"/>
  <path d="M0 600 L260 300 L420 460 L560 330 L800 600 Z" fill="#0f172a" opacity=".55"/>
  <text x="400" y="560" fill="#fff" font-family="sans-serif" font-size="28" text-anchor="middle">Replace this image with your own</text>
</svg>
"##;

/// Lines of `src` outside fenced code blocks: (byte offset, line)
fn lines_outside_code(src: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut at = 0;
    for line in src.split_inclusive('\n') {
        let t = line.trim_start();
        let mark = t.chars().next().filter(|c| *c == '`' || *c == '~');
        let len = mark.map_or(0, |m| t.chars().take_while(|c| *c == m).count());
        match (fence, mark) {
            (None, Some(m)) if len >= 3 => fence = Some((m, len)),
            (Some((f, n)), Some(m)) if m == f && len >= n && t.trim_end().chars().all(|c| c == m) => fence = None,
            (None, _) => out.push((at, line.trim_end_matches(['\n', '\r']))),
            _ => {}
        }
        at += line.len();
    }
    out
}

/// Where the slide goes in `src`: before a closing `## … {.end}` slide, otherwise at the end
fn insertion_point(src: &str) -> usize {
    let headings: Vec<(usize, &str)> =
        lines_outside_code(src).into_iter().filter(|(_, l)| l.starts_with("# ") || l.starts_with("## ")).collect();
    match headings.last() {
        Some((at, l)) if l.starts_with("## ") && l.contains('{') && l.contains(".end") => *at,
        _ => src.len(),
    }
}

/// `src` with `markdown` inserted, a blank line on each side; and the line where it starts
pub fn insert(src: &str, markdown: &str) -> (String, usize) {
    let at = insertion_point(src);
    let (before, after) = src.split_at(at);
    let before = before.trim_end_matches('\n');
    let lead = if before.is_empty() { "" } else { "\n\n" };
    let line = before.lines().count() + if before.is_empty() { 1 } else { 2 };
    let tail = if after.is_empty() { String::new() } else { format!("\n{after}") };
    (format!("{before}{lead}{}\n{tail}", markdown.trim_end()), line)
}

/// The main file of the presentation (`given`, a file or a folder; otherwise the current folder),
/// whose folder holds the files of the examples, and its last file, which the slide goes into
pub fn target(given: Option<&Path>) -> Result<(PathBuf, PathBuf), String> {
    let inputs = crate::source::inputs(&given.map(Path::to_path_buf).into_iter().collect::<Vec<_>>())?;
    let main = inputs.first().ok_or("no markdown file")?.clone();
    if !main.is_file() {
        return Err(format!("{}: no such file; create a presentation with `glitu init`", main.display()));
    }
    Ok((main, inputs.last().unwrap().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(r: &Request) -> String {
        slide(r, Path::new("/nonexistent")).markdown
    }

    #[test]
    fn slides_follow_the_options() {
        let mut r = Request::new(Kind::Bullets);
        let s = md(&r);
        assert!(s.starts_with("## Steps\n") && s.contains("@1\n") && s.contains("{.circle @3} circled at click 3 @3"));
        assert!(s.contains("Note:") && s.matches("[click]").count() == 3);
        r.effect = Some(Effect::Up);
        r.transition = Some(Transition::Zoom);
        r.title = Some("Why".into());
        let s = md(&r);
        assert!(s.starts_with("## Why {transition=\"zoom\"}") && s.contains("@2:up"));
        r.steps = 0;
        r.notes = false;
        let s = md(&r);
        assert!(!s.contains('@') && !s.contains("Note:"));
    }

    #[test]
    fn code_highlights_lines_click_by_click() {
        assert_eq!(line_groups(8, 3), ["1-2", "3-5", "6-8"]);
        assert_eq!(line_groups(2, 5), ["1", "2"]);
        let mut r = Request::new(Kind::Code { lang: Lang::Python, annotated: false });
        r.layout = Layout::Large;
        let s = md(&r);
        assert!(s.starts_with("## Code {.large}") && s.contains("```python title=\"app.py\" lines=\"1-2|3-4|5-7\" @1"));
        // full: the visual alone
        let r = Request { layout: Layout::Full, ..Request::new(Kind::Code { lang: Lang::Yaml, annotated: false }) };
        let s = md(&r);
        assert!(s.starts_with("## Code {.full}\n\n```yaml") && !s.contains("###"));
        let r = Request::new(Kind::Code { lang: Lang::Bash, annotated: true });
        assert!(md(&r).contains("# [!code ++]"));
        let s = md(&Request::new(Kind::MagicMove { lang: Lang::Rust }));
        assert!(s.contains("````md magic-move title=\"main.rs\"") && s.matches("```rust").count() == 3);
    }

    #[test]
    fn a_diagram_has_a_card_per_step() {
        let mut r = Request::new(Kind::Diagram { dir: Dir::Down });
        r.steps = 4;
        let s = md(&r);
        assert!(
            s.contains("diagram(dir=\"down\")") && s.contains("card#step4.output(icon=\"presentation\") Step 4 @4")
        );
        assert!(s.contains("to(step4)") && s.contains("to(tool)"));
        r.steps = 0;
        assert!(!md(&r).contains('@') && md(&r).matches("card#").count() == 3);
    }

    #[test]
    fn a_terminal_comes_with_its_recording() {
        let r = Request {
            title: Some("Build it".into()),
            ..Request::new(Kind::Terminal { commands: vec!["echo hi".into(), "make".into()], speed: 10.0 })
        };
        let s = slide(&r, Path::new("/nonexistent"));
        let emoji = md(&Request::new(Kind::Emoji));
        assert!(
            emoji.contains("🎉 @3\n")
                && md(&Request { steps: 0, ..Request::new(Kind::Emoji) }).matches('\n').count() > 3
        );
        assert!(s.markdown.contains("](casts/build-it.cast)") && s.markdown.contains("- `make` @2"));
        let (path, cast) = &s.files[0];
        assert_eq!(path, Path::new("casts/build-it.cast"));
        let cast = String::from_utf8_lossy(cast);
        let header: serde_json::Value = serde_json::from_str(cast.lines().next().unwrap()).unwrap();
        assert_eq!(header["title"], "Build it");
        // echo prints its text; a marker after each command
        assert!(cast.contains("\"o\", \"hi\\r\\n\"]"));
        assert_eq!(cast.matches(", \"m\", ").count(), 2);
        // every event is valid JSON, in time order
        let times: Vec<f64> = cast
            .lines()
            .skip(1)
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()[0].as_f64().unwrap())
            .collect();
        assert!(times.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn an_image_without_source_gets_a_placeholder() {
        let s = slide(&Request::new(Kind::Image { src: None, layout: ImageLayout::Left }), Path::new("/nonexistent"));
        assert!(s.markdown.starts_with("## Image {.image-left image=\"img/placeholder.svg\"}"));
        assert_eq!(s.files[0].0, Path::new("img/placeholder.svg"));
        let r = Request {
            background: Some("#123".into()),
            ..Request::new(Kind::Image { src: Some("img/a.jpg".into()), layout: ImageLayout::Right })
        };
        let s = slide(&r, Path::new("/nonexistent"));
        assert!(
            s.markdown.starts_with("## Image {.image-right image=\"img/a.jpg\" background=\"#123\"}")
                && s.files.is_empty()
        );
        let r = Request::new(Kind::Iframe { url: "https://glitu.net".into(), layout: PageLayout::Right });
        assert!(md(&r).starts_with("## Web page {.iframe-right url=\"https://glitu.net\"}"));
    }

    #[test]
    fn a_video_pauses_one_click_each() {
        let s = slide(&Request::new(Kind::Video { src: None, pauses: Vec::new() }), Path::new("/nonexistent"));
        assert_eq!(s.files[0].0, Path::new("img/placeholder.webm"));
        assert!(s.files[0].1.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]), "a webm file");
        assert!(s.markdown.contains(
            "![](img/placeholder.webm \"Source: where the video comes from\"){muted pauses=\"0:02 0:04\" focus=\"33 50 34 16 0:02-0:03\"}"
        ));
        assert!(s.markdown.contains("- A click: on to 0:04 @1\n- A click: to the end @2\n"));
        assert_eq!(s.markdown.matches("[click]").count(), 2);
        let r = Request::new(Kind::Video { src: Some("img/demo.mp4".into()), pauses: Vec::new() });
        let s = slide(&r, Path::new("/nonexistent"));
        assert!(s.files.is_empty() && s.markdown.contains("{muted autoplay}") && !s.markdown.contains("[click]"));
        let r = Request::new(Kind::Video { src: Some("img/demo.mp4".into()), pauses: vec!["12".into()] });
        assert!(md(&r).contains("{muted pauses=\"12\"}") && !md(&r).contains("focus"));
    }

    #[test]
    fn inserted_before_the_end_slide() {
        let src = "# A\n\n## B\n\n```md\n## Fake {.end}\n```\n\n## Thank you {.end}\n\nQuestions?\n";
        let (out, line) = insert(src, "## New\n");
        assert!(out.contains("```\n\n## New\n\n## Thank you {.end}"));
        assert_eq!(out.lines().nth(line - 1), Some("## New"));
        let (out, _) = insert("# A\n\n## B\n", "## New\n");
        assert_eq!(out, "# A\n\n## B\n\n## New\n");
        // an end slide in a code block does not count
        let (out, _) = insert("# A\n\n```md\n## X {.end}\n```\n", "## New\n");
        assert!(out.ends_with("```\n\n## New\n"));
        let r = Request { chapter: Some("Part two".into()), ..Request::new(Kind::Toc) };
        assert!(md(&r).starts_with("# Part two\n\nThe subtitle of the chapter\n\n## Contents {.toc}"));
        assert_eq!(slug("Build it: now!"), "build-it-now");
    }
}
