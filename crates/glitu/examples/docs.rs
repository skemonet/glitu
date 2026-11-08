//! The documentation of the website: `site/docs/*.md` → `<site>/docs/<page>/index.html`, with a
//! shared navigation, the table of contents of each page, the tags of search engines and link
//! previews; then `sitemap.xml` and `robots.txt` at the root of the website (see `tools/site.sh`).
//!
//! `cargo run -p glitu --example docs -- <docs folder> <site folder>`
//!
//! A page starts with a front matter: `title`, `description` (the summary shown by search engines)
//! and `order` (its place in the navigation); `index.md` is the home page of the documentation.

use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::highlight;
use glitu_core::html::esc;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const SITE: &str = "https://glitu.net";

struct Doc {
    /// `""` for the home page of the documentation, otherwise the name of its file
    slug: String,
    title: String,
    description: String,
    order: i64,
    body: String,
}

impl Doc {
    fn path(&self) -> String {
        if self.slug.is_empty() { "/docs/".into() } else { format!("/docs/{}/", self.slug) }
    }
}

fn main() {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let [src, out] = args.as_slice() else {
        eprintln!("usage: docs <docs folder> <site folder>");
        std::process::exit(2);
    };
    if let Err(e) = run(src, out) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(src: &Path, out: &Path) -> Result<(), String> {
    let mut docs = Vec::new();
    for entry in fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "md") {
            docs.push(read(&path)?);
        }
    }
    docs.sort_by_key(|d| d.order);
    for (i, doc) in docs.iter().enumerate() {
        let (prev, next) = (i.checked_sub(1).map(|p| &docs[p]), docs.get(i + 1));
        let html = page(doc, &docs, prev, next);
        let dir = out.join(doc.path().trim_start_matches('/'));
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        fs::write(dir.join("index.html"), html).map_err(|e| e.to_string())?;
    }
    let css = src.join("docs.css");
    fs::copy(&css, out.join("docs/docs.css")).map_err(|e| format!("{}: {e}", css.display()))?;
    let urls: Vec<String> =
        ["/", "/demo.html"].into_iter().map(String::from).chain(docs.iter().map(Doc::path)).collect();
    let sitemap: String = urls.iter().map(|u| format!("  <url><loc>{SITE}{u}</loc></url>\n")).collect();
    fs::write(
        out.join("sitemap.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{sitemap}</urlset>\n"
        ),
    )
    .map_err(|e| e.to_string())?;
    fs::write(out.join("robots.txt"), format!("User-agent: *\nAllow: /\n\nSitemap: {SITE}/sitemap.xml\n"))
        .map_err(|e| e.to_string())?;
    println!("{}: {} documentation pages", out.display(), docs.len());
    Ok(())
}

fn read(path: &Path) -> Result<Doc, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rest = text.strip_prefix("---\n").ok_or_else(|| format!("{}: no front matter", path.display()))?;
    let (front, body) =
        rest.split_once("\n---\n").ok_or_else(|| format!("{}: unclosed front matter", path.display()))?;
    let field = |k: &str| {
        front
            .lines()
            .find_map(|l| l.strip_prefix(k)?.strip_prefix(':'))
            .map(|v| v.trim().to_string())
            .unwrap_or_default()
    };
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    Ok(Doc {
        slug: if stem == "index" { String::new() } else { stem.into_owned() },
        title: field("title"),
        description: field("description"),
        order: field("order").parse().unwrap_or(99),
        body: body.to_string(),
    })
}

/// An anchor for a heading: lower case, words joined by `-`
fn slug(text: &str) -> String {
    let mut s = String::new();
    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    s.trim_end_matches('-').to_string()
}

/// The markdown → HTML, the headings with an anchor, the code highlighted; and the `##` headings,
/// for the table of contents
fn render(md: &str) -> (String, Vec<(String, String)>) {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut events = Vec::new();
    let mut toc = Vec::new();
    let mut heading: Option<(HeadingLevel, Vec<Event>)> = None;
    let mut code: Option<(String, String)> = None;
    for ev in Parser::new_ext(md, opts) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) if level != HeadingLevel::H1 => {
                heading = Some((level, Vec::new()))
            }
            Event::End(TagEnd::Heading(_)) if heading.is_some() => {
                let (level, inner) = heading.take().unwrap_or((HeadingLevel::H2, Vec::new()));
                let text: String = inner
                    .iter()
                    .filter_map(|e| if let Event::Text(t) | Event::Code(t) = e { Some(&**t) } else { None })
                    .collect();
                let id = slug(&text);
                let mut html = String::new();
                pulldown_cmark::html::push_html(&mut html, inner.into_iter());
                if level == HeadingLevel::H2 {
                    toc.push((id.clone(), esc(&text)));
                }
                events.push(Event::Html(
                    format!("<{level} id=\"{id}\"><a class=\"anchor\" href=\"#{id}\">{html}</a></{level}>\n").into(),
                ));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or("").to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::Text(t) if code.is_some() => {
                if let Some((_, c)) = code.as_mut() {
                    c.push_str(&t);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let (lang, text) = code.take().unwrap_or_default();
                let lines = highlight::lines(&lang, text.trim_end_matches('\n')).join("\n");
                let class = if lang.is_empty() { String::new() } else { format!(" class=\"language-{lang}\"") };
                events.push(Event::Html(format!("<pre><code{class}>{lines}</code></pre>\n").into()));
            }
            e => match heading.as_mut() {
                Some((_, inner)) => inner.push(e),
                None => events.push(e),
            },
        }
    }
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    (html, toc)
}

fn page(doc: &Doc, docs: &[Doc], prev: Option<&Doc>, next: Option<&Doc>) -> String {
    let (body, toc) = render(&doc.body);
    let url = format!("{SITE}{}", doc.path());
    let (title, description) = (esc(&doc.title), esc(&doc.description));
    let head_title =
        if doc.slug.is_empty() { "glitu documentation".to_string() } else { format!("{title} · glitu documentation") };
    let nav: String = docs
        .iter()
        .map(|d| {
            let current = if d.slug == doc.slug { " aria-current=\"page\"" } else { "" };
            format!("<li><a href=\"{}\"{current}>{}</a></li>", d.path(), esc(&d.title))
        })
        .collect();
    let on_page: String = toc.iter().map(|(id, text)| format!("<li><a href=\"#{id}\">{text}</a></li>")).collect();
    let on_page = if toc.len() > 1 {
        format!("<nav class=\"toc\" aria-label=\"On this page\"><p>On this page</p><ul>{on_page}</ul></nav>")
    } else {
        String::new()
    };
    let link = |d: Option<&Doc>, rel: &str, label: &str| {
        d.map(|d| {
            format!("<a class=\"{rel}\" rel=\"{rel}\" href=\"{}\"><span>{label}</span>{}</a>", d.path(), esc(&d.title))
        })
        .unwrap_or_else(|| "<span></span>".into())
    };
    let pager = format!("{}{}", link(prev, "prev", "Previous"), link(next, "next", "Next"));
    let mut crumbs =
        vec![(format!("{SITE}/"), "glitu".to_string()), (format!("{SITE}/docs/"), "Documentation".to_string())];
    if !doc.slug.is_empty() {
        crumbs.push((url.clone(), doc.title.clone()));
    }
    let crumbs: Vec<String> = crumbs
        .iter()
        .enumerate()
        .map(|(i, (u, n))| {
            format!("{{\"@type\":\"ListItem\",\"position\":{},\"name\":{},\"item\":\"{u}\"}}", i + 1, json(n))
        })
        .collect();
    let ld = format!(
        "{{\"@context\":\"https://schema.org\",\"@graph\":[{{\"@type\":\"TechArticle\",\"headline\":{},\"description\":{},\"url\":\"{url}\",\"inLanguage\":\"en\",\"isPartOf\":{{\"@type\":\"WebSite\",\"name\":\"glitu\",\"url\":\"{SITE}/\"}}}},{{\"@type\":\"BreadcrumbList\",\"itemListElement\":[{}]}}]}}",
        json(&doc.title),
        json(&doc.description),
        crumbs.join(",")
    );
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{head_title}</title>
<meta name="description" content="{description}">
<link rel="canonical" href="{url}">
<meta name="theme-color" content="#070b16">
<meta property="og:type" content="article">
<meta property="og:site_name" content="glitu">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{description}">
<meta property="og:url" content="{url}">
<meta property="og:image" content="{SITE}/img/cover.webp">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="{ICON}">
<link rel="stylesheet" href="/docs/docs.css">
<script type="application/ld+json">{ld}</script>
</head>
<body>
<a class="skip" href="#content">Skip to content</a>
<header class="top">
  <a class="brand" href="/">gl<span>i</span>tu</a>
  <nav aria-label="Site">
    <a href="/docs/" aria-current="true">Docs</a>
    <a href="/demo.html">Demo</a>
    <a href="https://github.com/skemonet/glitu">GitHub</a>
  </nav>
</header>
<div class="layout">
  <nav class="side" aria-label="Documentation"><ul>{nav}</ul></nav>
  <main id="content">
    <article>
      <h1>{title}</h1>
      <p class="lead">{description}</p>
{body}
    </article>
    <nav class="pager" aria-label="Pages">{pager}</nav>
  </main>
  {on_page}
</div>
<footer>glitu is dual-licensed under MIT or Apache-2.0. <a href="https://github.com/skemonet/glitu/edit/main/site/docs/{file}.md">Edit this page on GitHub</a></footer>
</body>
</html>
"##,
        file = if doc.slug.is_empty() { "index" } else { &doc.slug },
    )
}

/// A JSON string
fn json(s: &str) -> String {
    serde_json::Value::String(s.into()).to_string()
}

const ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' rx='7' fill='%230b1020'/%3E%3Cpath d='M9 22 15 10h4l-6 12zM15 22l6-12h3l-6 12z' fill='%2322d3ee'/%3E%3C/svg%3E";
