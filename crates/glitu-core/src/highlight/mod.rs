//! Build-time code highlighting, with highlight.js classes (`hljs-attr`, `hljs-string`…) that
//! themes style: by syntect's grammars (`syntax`), or, for the presentations written with
//! reveal-md's rules, like highlight.js itself (`hljs`). Then the markup of line numbers and
//! highlighting steps
//! (`data-line-numbers="8|9-11"`): one copy of the block per step, each one a fragment.
//!
//! On top of it come Shiki's end-of-line annotations (`// [!code ++]`, `--`, `focus`,
//! `highlight`) and animated code (`magic_move`): from one version of the block to the next, the
//! words that are kept slide to their new place.

use crate::html::esc;

mod hljs;
mod syntax;

pub use syntax::lines;

/// An identifier's character
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn span(class: &str, inner: &str) -> String {
    format!("<span class=\"hljs-{class}\">{inner}</span>")
}

// ---------------------------------------------------------------- blocks

/// `9-11,14` → line numbers
fn ranges(spec: &str) -> Vec<(usize, usize)> {
    spec.split(',')
        .filter_map(|r| {
            let (a, b) = r.split_once('-').unwrap_or((r, r));
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .collect()
}

fn table(lines: &[String], highlight: Option<&str>, first: usize) -> String {
    let hl = highlight.map(ranges).unwrap_or_default();
    let mut out = String::from("<table class=\"hljs-ln\"><tbody>");
    for (i, line) in lines.iter().enumerate() {
        let on = hl.iter().any(|&(a, b)| a <= i + 1 && i < b);
        let n = first + i;
        let code = if line.is_empty() { " " } else { line.as_str() };
        out += &format!(
            "<tr{}><td class=\"hljs-ln-line hljs-ln-numbers\" data-line-number=\"{n}\"><div class=\"hljs-ln-n\" data-line-number=\"{n}\"></div></td>\
             <td class=\"hljs-ln-line hljs-ln-code\" data-line-number=\"{n}\">{code}</td></tr>",
            if on { " class=\"highlight-line\"" } else { "" }
        );
    }
    out + "</tbody></table>"
}

// ---------------------------------------------------------------- annotations

/// Recognised annotations, `[!code …]` → class of the line
const ANNOTATIONS: [(&str, &str); 7] = [
    ("++", "add"),
    ("--", "remove"),
    ("focus", "focused"),
    ("highlight", "highlighted"),
    ("hl", "highlighted"),
    ("error", "error"),
    ("warning", "warning"),
];

/// The code without its annotations, and the class of each line (empty: none)
fn annotations(code: &str) -> (String, Vec<&'static str>) {
    let (lines, classes): (Vec<String>, Vec<&str>) = code.lines().map(annotation).unzip();
    (lines.join("\n"), classes)
}

/// One line without its annotation, and the annotation's class (empty: none)
fn annotation(line: &str) -> (String, &'static str) {
    let found = line.find("[!code ").and_then(|at| {
        let end = at + line[at..].find(']')?;
        let class = ANNOTATIONS.iter().find(|(k, _)| *k == line[at + 7..end].trim())?.1;
        Some((at, end, class))
    });
    let Some((at, end, class)) = found else { return (line.to_string(), "") };
    // the comment that carried the annotation goes with it when it is empty
    let mut rest = format!("{}{}", &line[..at], &line[end + 1..]);
    for close in ["-->", "*/"] {
        rest = rest.trim_end().strip_suffix(close).unwrap_or(&rest).to_string();
    }
    let rest = rest.trim_end();
    let rest = ["//", "#", "<!--", "/*", "--", ";"].iter().find_map(|open| rest.strip_suffix(open)).unwrap_or(rest);
    (rest.trim_end().to_string(), class)
}

/// Highlighted lines, each one in its annotation class
fn annotated(lines: Vec<String>, classes: &[&str]) -> Vec<String> {
    if classes.iter().all(|c| c.is_empty()) {
        return lines;
    }
    lines
        .into_iter()
        .zip(classes.iter().chain(std::iter::repeat(&"")))
        .map(|(l, c)| format!("<span class=\"line{}{c}\">{l}</span>", if c.is_empty() { "" } else { " " }))
        .collect()
}

/// Classes of the block according to its annotations: `has-focus`, `has-diff`, `has-highlighted`
fn block_classes(classes: &[&str]) -> String {
    let mut out = String::new();
    for (c, block) in
        [("focused", " has-focus"), ("add", " has-diff"), ("remove", " has-diff"), ("highlighted", " has-highlighted")]
    {
        if classes.contains(&c) && !out.contains(block) {
            out += block;
        }
    }
    out
}

// ---------------------------------------------------------------- animated code

/// A word of the code: an identifier or a number, or a single sign; an HTML entity counts as a
/// sign
fn word_end(text: &str) -> usize {
    let mut chars = text.char_indices();
    let Some((_, first)) = chars.next() else { return 0 };
    if first == '&' {
        return text.find(';').map_or(1, |i| i + 1);
    }
    if !is_ident(first) {
        return first.len_utf8();
    }
    chars.find(|(_, c)| !is_ident(*c)).map_or(text.len(), |(i, _)| i)
}

/// Each word of a highlighted line passed to `wrap` (its text → its HTML); tags and whitespace
/// copied as is
fn each_word(line: &str, mut wrap: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(line.len() * 2);
    let mut rest = line;
    while !rest.is_empty() {
        let end = if rest.starts_with('<') {
            rest.find('>').map_or(rest.len(), |i| i + 1)
        } else if rest.starts_with(char::is_whitespace) {
            rest.find(|c: char| !c.is_whitespace()).unwrap_or(rest.len())
        } else {
            let end = word_end(rest).min(rest.find('<').unwrap_or(rest.len())).max(1);
            out += &wrap(&rest[..end]);
            rest = &rest[end..];
            continue;
        };
        out += &rest[..end];
        rest = &rest[end..];
    }
    out
}

/// Pairs (i, j) of the words common to `a` and `b`, in order (longest common subsequence)
fn common(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    let at = |i: usize, j: usize| i * (m + 1) + j;
    let mut len = vec![0u32; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            len[at(i, j)] =
                if a[i] == b[j] { len[at(i + 1, j + 1)] + 1 } else { len[at(i + 1, j)].max(len[at(i, j + 1)]) };
        }
    }
    let (mut i, mut j, mut pairs) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            pairs.push((i, j));
            (i, j) = (i + 1, j + 1);
        } else if len[at(i + 1, j)] >= len[at(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// The words of highlighted lines, in order
fn words_of(lines: &[String]) -> Vec<String> {
    let mut words = Vec::new();
    for l in lines {
        each_word(l, |t| {
            words.push(t.to_string());
            String::new()
        });
    }
    words
}

/// Animated code: the successive versions of a block (language, code), stacked in the same
/// `<pre>`, the first one visible from the start, each following one being a fragment (at click
/// `first_step`, then one per click). Each word carries an id (`data-mm`), shared by two
/// consecutive versions when the word is kept: the engine slides it.
pub fn magic_move(versions: &[(&str, &str)], first_step: u32) -> String {
    let colored: Vec<(Vec<String>, Vec<&str>)> = versions
        .iter()
        .map(|(lang, code)| {
            let (code, classes) = annotations(code);
            (lines(lang, &code), classes)
        })
        .collect();
    let words: Vec<Vec<String>> = colored.iter().map(|(lines, _)| words_of(lines)).collect();
    let mut next_id = 0;
    let mut previous: Vec<usize> = Vec::new();
    let mut out = String::from("<pre class=\"code-wrapper magic-move\">");
    for (k, ((lang, _), (lines, classes))) in versions.iter().zip(&colored).enumerate() {
        let mut ids: Vec<Option<usize>> = vec![None; words[k].len()];
        if k > 0 {
            for (i, j) in common(&words[k - 1], &words[k]) {
                ids[j] = Some(previous[i]);
            }
        }
        let ids: Vec<usize> = ids
            .into_iter()
            .map(|id| {
                id.unwrap_or_else(|| {
                    next_id += 1;
                    next_id
                })
            })
            .collect();
        let mut n = 0;
        let html: Vec<String> = lines
            .iter()
            .map(|l| {
                each_word(l, |t| {
                    n += 1;
                    format!("<span class=\"mm\" data-mm=\"{}\">{t}</span>", ids[n - 1])
                })
            })
            .collect();
        let lang = if lang.is_empty() { "plaintext".into() } else { esc(lang) };
        let step = if k == 0 {
            String::new()
        } else {
            format!(" fragment\" data-fragment-index=\"{}", first_step + k as u32 - 1)
        };
        out += &format!(
            "<code class=\"language-{lang} hljs{}{step}\">{}</code>",
            block_classes(classes),
            annotated(html, classes).join("\n")
        );
        previous = ids;
    }
    out + "</pre>"
}

/// A code block to display
pub struct Code<'a> {
    pub lang: &'a str,
    pub code: &'a str,
    /// Highlighting steps, `8|9-11|17-21` (numbers relative to the block)
    pub lines: Option<&'a str>,
    /// Click of the second highlighting (None: document order)
    pub first_step: Option<u32>,
    /// Displayed number of the first line
    pub start_from: usize,
    /// reveal-md's classes (`php hljs language-php`) rather than `language-php hljs`
    pub reveal_md: bool,
}

pub fn block(c: &Code) -> String {
    // Shiki's annotations do not exist in reveal-md
    let (code, classes) = if c.reveal_md { (c.code.to_string(), Vec::new()) } else { annotations(c.code) };
    let colored = if c.reveal_md { hljs::lines(c.lang, &code) } else { lines(c.lang, &code) };
    let lines = annotated(colored, &classes);
    // escaped: the language is written into an attribute
    let lang = if c.lang.is_empty() { "plaintext".into() } else { esc(c.lang) };
    let class = if c.reveal_md {
        format!("{lang} hljs language-{lang}")
    } else if lang == "html" {
        "language-html hljs language-xml".into()
    } else {
        format!("language-{lang} hljs")
    } + &block_classes(&classes);
    let start = if c.start_from != 1 { format!(" data-ln-start-from=\"{}\"", c.start_from) } else { String::new() };
    let Some(spec) = c.lines else {
        return format!("<pre class=\"code-wrapper\"><code class=\"{class}\">{}</code></pre>", lines.join("\n"));
    };
    let mut out = String::from("<pre class=\"code-wrapper\">");
    for (k, part) in spec.split('|').enumerate() {
        let frag = if k == 0 { String::new() } else { " fragment".into() };
        let index = match (k, c.first_step) {
            (0, _) | (_, None) => String::new(),
            (k, Some(s)) => format!(" data-fragment-index=\"{}\"", s + k as u32 - 1),
        };
        let part = part.trim();
        let numbered = if part.is_empty() { None } else { Some(part) };
        out += &format!(
            "<code class=\"{class}{frag} has-highlights\" data-line-numbers=\"{}\"{start}{index}>{}</code>",
            esc(part),
            table(&lines, numbered, c.start_from)
        );
    }
    out + "</pre>"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotations_become_line_classes() {
        let (code, classes) =
            annotations("i-- // [!code --]\nb = 2 # [!code ++]\n<p/> <!-- [!code focus] -->\nc // keep [!code hl]");
        assert_eq!(code, "i--\nb = 2\n<p/>\nc // keep");
        assert_eq!(classes, ["remove", "add", "focused", "highlighted"]);
        assert_eq!(block_classes(&classes), " has-focus has-diff has-highlighted");
        let (code, classes) = annotations("x [!code unknown]");
        assert_eq!((code.as_str(), classes[0]), ("x [!code unknown]", ""));
    }

    #[test]
    fn kept_words_share_their_id() {
        let html = magic_move(&[("rust", "let x = 1;"), ("rust", "let mut x = 2;")], 3);
        let ids = |code: &str| -> Vec<(String, String)> {
            code.split("data-mm=\"")
                .skip(1)
                .map(|p| {
                    let (id, rest) = p.split_once('"').unwrap();
                    (id.to_string(), rest[1..rest.find("</span>").unwrap()].to_string())
                })
                .collect()
        };
        let versions: Vec<&str> = html.split("<code").skip(1).collect();
        let (a, b) = (ids(versions[0]), ids(versions[1]));
        let id_of = |v: &[(String, String)], w: &str| v.iter().find(|(_, t)| t.contains(w)).unwrap().0.clone();
        assert_eq!(id_of(&a, "let"), id_of(&b, "let"));
        assert_eq!(id_of(&a, "x"), id_of(&b, "x"));
        assert_ne!(id_of(&a, "1"), id_of(&b, "2"));
        assert!(versions[1].contains("fragment\" data-fragment-index=\"3\""));
    }

    #[test]
    fn steps_number_and_highlight_lines() {
        let html = block(&Code {
            lang: "sh",
            code: "a\nb\nc",
            lines: Some("1|2-3"),
            first_step: Some(4),
            start_from: 7,
            reveal_md: false,
        });
        assert_eq!(html.matches("<code").count(), 2);
        assert!(html.contains("data-line-numbers=\"2-3\" data-ln-start-from=\"7\" data-fragment-index=\"4\""));
        assert!(html.contains("data-line-number=\"9\""));
        assert_eq!(html.matches("highlight-line").count(), 3);
        let html = block(&Code { lang: "", code: "x", lines: None, first_step: None, start_from: 1, reveal_md: true });
        assert_eq!(
            html,
            "<pre class=\"code-wrapper\"><code class=\"plaintext hljs language-plaintext\">x</code></pre>"
        );
    }
}
