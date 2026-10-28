//! Markdown with reveal-md's writing conventions (and those of reveal.js's markdown plugin):
//!
//! ```text
//!   ---                          separates two slides
//!   ----                         separates two vertical slides
//!   Note: …                      speaker notes (also `notes:`, case-insensitive)
//!   <!-- .slide: data-background="#000" -->      attributes of the slide
//!   - bullet <!-- .element: class="fragment" -->  attributes of the preceding element
//!   ```php [1|3-5]               lines highlighted step by step; [287: 2|4,6]: numbering from 287
//! ```
//!
//! The `.slide:` and `.element:` comments stay in the page: the engine applies them to the DOM,
//! with reveal.js's rule (the preceding sibling element, otherwise the parent).

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, html::push_html};

use crate::highlight;
use crate::html::esc;

/// A slide separator outside the front matter and the code blocks (the `---` of a YAML document
/// shown on screen is not one): the markdown follows reveal-md's conventions
pub fn detect(body: &str) -> bool {
    let mut fence: Option<&str> = None;
    body.lines().any(|l| {
        let l = l.trim_end_matches('\r');
        let mark = ["```", "~~~"].into_iter().find(|m| l.trim_start().starts_with(m));
        match (fence, mark) {
            (None, Some(m)) => fence = Some(m),
            (Some(f), Some(m)) if f == m && l.trim() == m => fence = None,
            _ => {}
        }
        fence.is_none() && matches!(l, "---" | "----")
    })
}

/// Splits on the lines exactly equal to `sep` (like the regex `\r?\n---\r?\n`)
fn split<'a>(text: &'a str, sep: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches('\n').trim_end_matches('\r');
        if bare == sep && pos > 0 {
            parts.push(&text[start..pos]);
            start = pos + line.len();
        }
        pos += line.len();
    }
    parts.push(&text[start..]);
    parts
}

/// Content and notes: from the first line that starts with `note:` or `notes:`
fn notes(md: &str) -> (&str, Option<&str>) {
    let mut pos = 0;
    for line in md.split_inclusive('\n') {
        let t = line.trim_start();
        let lower = t.to_lowercase();
        for key in ["notes:", "note:"] {
            if lower.starts_with(key) {
                let at = pos + (line.len() - t.len()) + key.len();
                return (&md[..pos], Some(&md[at..]));
            }
        }
        pos += line.len();
    }
    (md, None)
}

/// Heading id, like marked
fn slug(text: &str) -> String {
    text.to_lowercase()
        .trim()
        .chars()
        .filter(|c| {
            !matches!(c, '\u{2000}'..='\u{206F}' | '\u{2E00}'..='\u{2E7F}' | '\\' | '\'' | '!' | '"' | '#' | '$' | '%'
                | '&' | '(' | ')' | '*' | '+' | ',' | '.' | '/' | ':' | ';' | '<' | '=' | '>' | '?' | '@' | '['
                | ']' | '^' | '`' | '{' | '|' | '}' | '~')
        })
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect()
}

/// `` ```php [287: 2|4,6] `` → language, steps, first line
fn code_info(info: &str) -> (String, Option<String>, usize) {
    let lang = info.split_whitespace().next().unwrap_or("").to_string();
    let lang = lang.split('[').next().unwrap_or("").to_string();
    let Some(open) = info.find('[') else { return (lang, None, 1) };
    let Some(close) = info[open..].find(']') else { return (lang, None, 1) };
    let inner = &info[open + 1..open + close];
    let (start, steps) = match inner.split_once(':') {
        Some((s, rest)) => (s.trim().parse().unwrap_or(1), rest.trim()),
        None => (1, inner.trim()),
    };
    (lang, Some(steps.replace(' ', "")), start)
}

/// The heading that starts at `events[0]`, with an id made from its text
fn heading_with_id<'a>(events: &[Event<'a>]) -> Event<'a> {
    let Event::Start(Tag::Heading { level, classes, attrs, .. }) = &events[0] else { return events[0].clone() };
    let mut text = String::new();
    for e in &events[1..] {
        match e {
            Event::End(TagEnd::Heading(_)) => break,
            Event::Text(t) | Event::Code(t) => text.push_str(t),
            _ => {}
        }
    }
    Event::Start(Tag::Heading {
        level: *level,
        id: Some(CowStr::from(slug(&text))),
        classes: classes.clone(),
        attrs: attrs.clone(),
    })
}

/// The code block that starts at `events[0]`, highlighted, and the number of events it spans
fn code_block(kind: &CodeBlockKind, events: &[Event]) -> (Event<'static>, usize) {
    let info = match kind {
        CodeBlockKind::Fenced(i) => i.to_string(),
        CodeBlockKind::Indented => String::new(),
    };
    let mut code = String::new();
    let mut n = 1;
    while n < events.len() && !matches!(events[n], Event::End(TagEnd::CodeBlock)) {
        if let Event::Text(t) = &events[n] {
            code.push_str(t)
        }
        n += 1;
    }
    let (lang, steps, start_from) = code_info(&info);
    let block = highlight::block(&highlight::Code {
        lang: &lang,
        code: code.trim_end_matches('\n'),
        lines: steps.as_deref(),
        first_step: None,
        start_from,
        reveal_md: true,
    });
    (Event::Html(CowStr::from(block + "\n")), n + 1)
}

fn markdown(md: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_MATH;
    let events: Vec<Event> = Parser::new_ext(md, opts).collect();
    let mut out: Vec<Event> = Vec::with_capacity(events.len());
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { .. }) => out.push(heading_with_id(&events[i..])),
            Event::Start(Tag::CodeBlock(kind)) => {
                let (block, len) = code_block(kind, &events[i..]);
                out.push(block);
                i += len;
                continue;
            }
            e => out.push(e.clone()),
        }
        i += 1;
    }
    let mut html = String::new();
    push_html(&mut html, crate::html::with_math(out.into_iter()));
    html
}

fn slide(md: &str) -> String {
    let (content, notes) = notes(md);
    let mut s = format!("<section data-markdown=\"\">{}", markdown(content));
    if let Some(n) = notes {
        s += &format!("<aside class=\"notes\">{}</aside>", markdown(n));
    }
    s + "</section>"
}

pub struct Rendered {
    pub sections: String,
}

pub fn render(body: &str) -> Rendered {
    let mut out = Vec::new();
    for h in split(body, "---") {
        let v = split(h, "----");
        if v.len() > 1 {
            out.push(format!("<section>{}</section>", v.iter().map(|s| slide(s)).collect::<String>()));
        } else {
            out.push(slide(h));
        }
    }
    Rendered { sections: out.join("\n") }
}

/// Title: the front matter's, otherwise the first heading of the document
pub fn title(body: &str) -> String {
    body.lines()
        .find_map(|l| l.trim_start().strip_prefix('#').map(|t| t.trim_start_matches('#').trim().to_string()))
        .map(|t| esc(&t))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        assert!(detect("# A\n\n---\n\nB\n"));
        assert!(!detect("# A\n\n## B\n"));
        assert!(!detect("## A\n\n```yaml\n---\nwhat: world\n```\n"));
        assert!(detect("```yaml\nx: 1\n```\n\n---\n\nB\n"));
        assert_eq!(split("a\n---\nb\n---\nc", "---"), vec!["a\n", "b\n", "c"]);
        assert_eq!(slug("External 3.3 (Image)"), "external-33-image");
        assert_eq!(code_info("c [287: 2|4,6]"), ("c".into(), Some("2|4,6".into()), 287));
        assert_eq!(code_info("php [1|3-5]"), ("php".into(), Some("1|3-5".into()), 1));
        assert_eq!(code_info("php [1"), ("php".into(), None, 1));
        let (c, n) = notes("## T\n\nx\n\nNote: says this\n");
        assert_eq!((c, n), ("## T\n\nx\n\n", Some(" says this\n")));
        let html = render("## T\n- a <!-- .element: class=\"fragment\" -->\n---\nB\n----\nC\n").sections;
        assert!(html.contains("<h2 id=\"t\">T</h2>"));
        assert!(html.contains("<!-- .element: class=\"fragment\" -->"));
        assert!(html.contains("<section><section data-markdown=\"\"><p>B</p>"));
    }

    #[test]
    fn code_blocks_and_notes_are_rendered() {
        let html = render("## `code` title\n\n```php [1|2]\n$a;\n$b;\n```\n\nNotes: *n*\n").sections;
        assert!(html.contains("<h2 id=\"code-title\"><code>code</code> title</h2>"), "{html}");
        assert!(html.contains("<code class=\"php hljs language-php has-highlights\" data-line-numbers=\"1\">"));
        assert!(html.contains("<aside class=\"notes\"><p><em>n</em></p>\n</aside>"), "{html}");
        assert_eq!(title("text\n## A <b>\n"), "A &lt;b&gt;");
        assert_eq!(title("no heading"), "");
    }
}
