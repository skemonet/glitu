//! The SVG of a tool, sanitized before it enters the page (SEC-11): the page is published and
//! opened by others, and a tool fed with a hostile source can produce a hostile SVG.
//!
//! Kept: an allowlist of SVG elements and their attributes; `href` only to `#id` in the same SVG
//! (or a raster `data:` image for `<image>`); `url()` only to `#id`, a raster image or a font in a
//! `data:` address; skemo's `skemo-source` inside `metadata`. Removed: `script`, `foreignObject`,
//! animations, every `on*` attribute, `javascript:` addresses, external references, comments,
//! processing instructions and document types. A `<a>` is replaced by its content.
//!
//! The identifiers are made unique in the page (`prefix` + id), with every reference to them:
//! two Mermaid diagrams both define `#arrowhead`.

use std::collections::BTreeSet;

/// The elements kept
const ELEMENTS: [&str; 33] = [
    "svg",
    "g",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "text",
    "tspan",
    "textPath",
    "defs",
    "marker",
    "linearGradient",
    "radialGradient",
    "stop",
    "clipPath",
    "mask",
    "pattern",
    "symbol",
    "use",
    "image",
    "filter",
    "title",
    "desc",
    "metadata",
    "style",
    "skemo-source",
    "view",
    "font-face",
    "hkern",
    "vkern",
];

/// The elements replaced by their content
const UNWRAPPED: [&str; 2] = ["a", "switch"];

fn allowed(name: &str) -> bool {
    ELEMENTS.contains(&name) || name.starts_with("fe") && name.len() > 2 && name[2..].starts_with(char::is_uppercase)
}

/// The sanitized SVG, its identifiers prefixed; an error if there is no `<svg>` element
pub fn sanitize(svg: &str, prefix: &str) -> Result<String, String> {
    let tokens = tokenize(svg)?;
    let ids: BTreeSet<String> = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Open { attrs, .. } => attrs.iter().find(|(k, _)| k == "id").map(|(_, v)| v.clone()),
            _ => None,
        })
        .collect();
    let mut out = String::with_capacity(svg.len());
    // the open elements: (name, kept); inside a dropped one, everything is dropped
    let mut stack: Vec<(String, bool)> = Vec::new();
    let mut started = false;
    for t in tokens {
        let dropping = stack.iter().any(|(_, kept)| !kept);
        match t {
            Token::Open { name, attrs, closed } => {
                if !started && name != "svg" {
                    continue;
                }
                if started && stack.is_empty() {
                    break;
                }
                started = true;
                let parent = stack.last().map(|(n, _)| n.as_str());
                let unwrapped = UNWRAPPED.contains(&name.as_str());
                let mut keep = !dropping
                    && (allowed(&name) || unwrapped)
                    && (name != "skemo-source" || parent == Some("metadata"));
                let mut kept_attrs = Vec::new();
                if keep && !unwrapped {
                    for (k, v) in &attrs {
                        if let Some(v) = attribute(&name, k, v, &ids, prefix) {
                            kept_attrs.push((k.clone(), v));
                        } else if (k == "href" || k == "xlink:href") && matches!(name.as_str(), "use" | "image") {
                            // a `<use>` or an `<image>` elsewhere: nothing to show
                            keep = false;
                        }
                    }
                }
                if keep && !unwrapped {
                    out.push('<');
                    out.push_str(&name);
                    for (k, v) in &kept_attrs {
                        out.push_str(&format!(" {k}=\"{}\"", escape(v, true)));
                    }
                    out.push_str(if closed { "/>" } else { ">" });
                }
                if !closed {
                    // an unwrapped element stays "kept" for its children
                    stack.push((name, keep));
                }
            }
            Token::Close(name) => {
                let Some(at) = stack.iter().rposition(|(n, _)| *n == name) else { continue };
                // the elements left open inside it close with it
                while stack.len() > at {
                    let (n, kept) = stack.pop().unwrap_or_default();
                    let dropped_above = stack.iter().any(|(_, k)| !k);
                    if kept && !dropped_above && !UNWRAPPED.contains(&n.as_str()) {
                        out.push_str(&format!("</{n}>"));
                    }
                }
                if stack.is_empty() && started {
                    break;
                }
            }
            Token::Text(text) => {
                if dropping || stack.is_empty() {
                    continue;
                }
                if stack.last().is_some_and(|(n, _)| n == "style") {
                    match css(&text, &ids, prefix) {
                        Some(c) => out.push_str(&escape(&c, false)),
                        None => continue,
                    }
                } else {
                    out.push_str(&escape(&text, false));
                }
            }
        }
    }
    if !started {
        return Err("the tool's output holds no <svg> element".into());
    }
    while let Some((n, kept)) = stack.pop() {
        if kept && !stack.iter().any(|(_, k)| !k) && !UNWRAPPED.contains(&n.as_str()) {
            out.push_str(&format!("</{n}>"));
        }
    }
    Ok(out)
}

/// An attribute's value, if kept (references to identifiers prefixed)
fn attribute(element: &str, name: &str, value: &str, ids: &BTreeSet<String>, prefix: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("on") || name.contains(['<', '>', '"', '\'', '/', '=']) || name.is_empty() {
        return None;
    }
    let compact: String = value.chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect();
    let compact = compact.to_ascii_lowercase();
    if compact.contains("javascript:") || compact.contains("vbscript:") {
        return None;
    }
    match lower.as_str() {
        "href" | "xlink:href" => {
            if let Some(id) = value.strip_prefix('#') {
                return Some(format!("#{}", rename(id, ids, prefix)));
            }
            (element == "image" && raster(&compact)).then(|| value.to_string())
        }
        "id" => Some(rename(value, ids, prefix)),
        "style" => css(value, ids, prefix),
        "aria-labelledby" | "aria-describedby" => {
            Some(value.split_whitespace().map(|i| rename(i, ids, prefix)).collect::<Vec<_>>().join(" "))
        }
        // `xml:base` would change what the relative references mean
        "xml:base" | "xlink:base" => None,
        _ => {
            if compact.contains("url(") {
                urls(value, ids, prefix)
            } else {
                Some(value.to_string())
            }
        }
    }
}

/// `data:image/png|jpeg|gif|webp`
fn raster(compact: &str) -> bool {
    ["data:image/png", "data:image/jpeg", "data:image/jpg", "data:image/gif", "data:image/webp"]
        .iter()
        .any(|p| compact.starts_with(p))
}

/// A `data:` font
fn font(compact: &str) -> bool {
    ["data:font/", "data:application/font-woff", "data:application/x-font-woff", "data:application/font-sfnt"]
        .iter()
        .any(|p| compact.starts_with(p))
}

/// Every `url(…)` of a value to `#id` (renamed), a raster image or a font; otherwise refused
fn urls(value: &str, ids: &BTreeSet<String>, prefix: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.to_ascii_lowercase().find("url(") {
        out.push_str(&rest[..at + 4]);
        rest = &rest[at + 4..];
        let end = rest.find(')')?;
        let inner = rest[..end].trim();
        let quote = inner.chars().next().filter(|c| *c == '"' || *c == '\'');
        let bare = quote.map_or(inner, |q| inner.trim_matches(q));
        let compact = bare.to_ascii_lowercase();
        if let Some(id) = bare.strip_prefix('#') {
            let q = quote.map(String::from).unwrap_or_default();
            out.push_str(&format!("{q}#{}{q}", rename(id, ids, prefix)));
        } else if raster(&compact) || font(&compact) {
            out.push_str(&rest[..end]);
        } else {
            return None;
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    Some(out)
}

/// A stylesheet (element or attribute), if harmless: no `@import`, no escapes, no `expression`,
/// `url()` as above; its `#id` selectors renamed
fn css(text: &str, ids: &BTreeSet<String>, prefix: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let bad = ["@import", "expression", "javascript:", "behavior", "-moz-binding", "\\", "</"];
    if bad.iter().any(|b| lower.contains(b)) {
        return None;
    }
    let text = urls(text, ids, prefix)?;
    // `#id` as a selector
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        out.push(c);
        if c != '#' {
            continue;
        }
        let tail = &text[i + 1..];
        let len = tail.find(|ch: char| !(ch.is_alphanumeric() || ch == '-' || ch == '_')).unwrap_or(tail.len());
        let word = &tail[..len];
        if ids.contains(word) {
            out.push_str(&rename(word, ids, prefix));
            for _ in 0..word.chars().count() {
                chars.next();
            }
        }
    }
    Some(out)
}

fn rename(id: &str, ids: &BTreeSet<String>, prefix: &str) -> String {
    if ids.contains(id) { format!("{prefix}{id}") } else { id.to_string() }
}

/// Text for the page: `&`, `<`, `>` (and `"` in an attribute) escaped, after the entities are read
fn escape(raw: &str, attr: bool) -> String {
    let text = unescape(raw);
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            c if c.is_control() && !matches!(c, '\n' | '\t' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// The XML entities: the five named ones and the numeric ones; any other stays as written
pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest[..rest.len().min(12)].find(';');
        let decoded = end.and_then(|e| {
            let name = &rest[1..e];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                n if n.starts_with("#x") || n.starts_with("#X") => {
                    u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32)
                }
                n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
                _ => None,
            };
            c.map(|c| (c, e + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[derive(Debug, PartialEq)]
enum Token {
    Open {
        name: String,
        attrs: Vec<(String, String)>,
        closed: bool,
    },
    Close(String),
    /// Raw text (entities not read), a CDATA section's content escaped
    Text(String),
}

/// The tags and texts of an XML document; comments, processing instructions and document types
/// dropped
fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            out.push(Token::Text(rest.to_string()));
            break;
        };
        if at > 0 {
            out.push(Token::Text(rest[..at].to_string()));
        }
        rest = &rest[at..];
        if let Some(r) = rest.strip_prefix("<!--") {
            rest = r.find("-->").map_or("", |e| &r[e + 3..]);
        } else if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let end = r.find("]]>").unwrap_or(r.len());
            out.push(Token::Text(r[..end].replace('&', "&amp;").replace('<', "&lt;")));
            rest = r.get(end + 3..).unwrap_or("");
        } else if rest.starts_with("<?") {
            rest = rest.find("?>").map_or("", |e| &rest[e + 2..]);
        } else if rest.starts_with("<!") {
            // a document type, with its internal subset (`[…]`)
            let end = match (rest.find('['), rest.find('>')) {
                (Some(b), Some(g)) if b < g => rest.find("]>").map_or(rest.len(), |e| e + 2),
                (_, Some(g)) => g + 1,
                _ => rest.len(),
            };
            rest = &rest[end..];
        } else if let Some(r) = rest.strip_prefix("</") {
            let end = r.find('>').ok_or("a closing tag without its >")?;
            out.push(Token::Close(r[..end].trim().to_string()));
            rest = &r[end + 1..];
        } else {
            let (token, len) = open_tag(rest)?;
            out.push(token);
            rest = &rest[len..];
        }
    }
    Ok(out)
}

/// `<name a="1" b='2'>` → the token and its length
fn open_tag(s: &str) -> Result<(Token, usize), String> {
    let b = s.as_bytes();
    let mut i = 1;
    let name_end = s[i..].find(|c: char| c.is_whitespace() || c == '>' || c == '/').map_or(s.len(), |e| i + e);
    let name = s[i..name_end].to_string();
    if name.is_empty() {
        return Err("a tag without a name".into());
    }
    i = name_end;
    let mut attrs = Vec::new();
    loop {
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return Err(format!("<{name}: unfinished tag"));
        }
        if b[i] == b'>' {
            return Ok((Token::Open { name, attrs, closed: false }, i + 1));
        }
        if s[i..].starts_with("/>") {
            return Ok((Token::Open { name, attrs, closed: true }, i + 2));
        }
        if b[i] == b'/' {
            i += 1;
            continue;
        }
        let key_end =
            s[i..].find(|c: char| c.is_whitespace() || c == '=' || c == '>' || c == '/').map_or(s.len(), |e| i + e);
        let key = s[i..key_end].to_string();
        i = key_end;
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && (b[i] as char).is_whitespace() {
                i += 1;
            }
            let value = match b.get(i) {
                Some(&q @ (b'"' | b'\'')) => {
                    let end = s[i + 1..].find(q as char).ok_or(format!("<{name} {key}=: unfinished value"))?;
                    let v = &s[i + 1..i + 1 + end];
                    i += end + 2;
                    v
                }
                _ => {
                    let end = s[i..].find(|c: char| c.is_whitespace() || c == '>').map_or(s.len(), |e| i + e);
                    let v = &s[i..end];
                    i = end;
                    v
                }
            };
            attrs.push((key, unescape(value)));
        } else if !key.is_empty() {
            attrs.push((key, String::new()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_svg_passes_with_its_identifiers_prefixed() {
        let svg = "<?xml version=\"1.0\"?><!-- made by x --><svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 9 9\">\
                   <defs><filter id=\"sh\"><feGaussianBlur stdDeviation=\"2\"/></filter><marker id=\"arrowhead\"><path d=\"M0 0\"/></marker></defs>\
                   <g class=\"card\" data-id=\"a\" filter=\"url(#sh)\"><path marker-end=\"url(#arrowhead)\" d=\"M1 1\"/><text>A &amp; B</text></g>\
                   <use href=\"#arrowhead\"/><metadata><skemo-source xmlns=\"https://skemo.net/source\">{&quot;s&quot;:1}</skemo-source></metadata></svg>";
        let out = sanitize(svg, "d1-").unwrap();
        assert_eq!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 9 9\"><defs><filter id=\"d1-sh\"><feGaussianBlur stdDeviation=\"2\"/></filter>\
             <marker id=\"d1-arrowhead\"><path d=\"M0 0\"/></marker></defs><g class=\"card\" data-id=\"a\" filter=\"url(#d1-sh)\">\
             <path marker-end=\"url(#d1-arrowhead)\" d=\"M1 1\"/><text>A &amp; B</text></g><use href=\"#d1-arrowhead\"/>\
             <metadata><skemo-source xmlns=\"https://skemo.net/source\">{\"s\":1}</skemo-source></metadata></svg>"
        );
    }

    #[test]
    fn hostile_parts_are_removed() {
        let svg = "<svg onload=\"alert(1)\"><script>alert(2)</script><g onclick='x()'><a href=\"javascript:alert(3)\"><text>t</text></a></g>\
                   <foreignObject><div>html</div></foreignObject><image href=\"file:///etc/passwd\"/><image href=\"data:image/png;base64,AA\"/>\
                   <use href=\"https://evil.net/x.svg#a\"/><rect style=\"fill:url(https://evil.net/t)\" fill=\"url(http://x/y)\"/>\
                   <style>@import url(https://evil.net/x.css); .a{}</style><style>.b{fill:red}</style>\
                   <animate attributeName=\"href\" to=\"javascript:alert(4)\"/><text>&lt;script&gt;</text>\
                   <path d=\"M0\" fill=\"&#106;avascript:x\"/><skemo-source>loose</skemo-source></svg><script>after</script>";
        let out = sanitize(svg, "p-").unwrap();
        for bad in [
            "onload",
            "alert",
            "onclick",
            "script>",
            "foreignObject",
            "html",
            "passwd",
            "evil",
            "animate",
            "javascript",
            "loose",
            "after",
        ] {
            assert!(!out.contains(bad), "{bad} in {out}");
        }
        assert!(out.contains("<text>t</text>") && out.contains("data:image/png;base64,AA"));
        assert!(out.contains("<style>.b{fill:red}</style>") && out.contains("<text>&lt;script&gt;</text>"));
        assert!(out.contains("<rect/>"), "{out}");
        assert!(sanitize("<html>no svg</html>", "p-").is_err());
        // a document type and its entities
        let out =
            sanitize("<!DOCTYPE svg [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><svg><text>&x;</text></svg>", "p-")
                .unwrap();
        assert_eq!(out, "<svg><text>&amp;x;</text></svg>");
    }

    #[test]
    fn styles_follow_the_renamed_identifiers() {
        let svg = "<svg id=\"mermaid-1\"><style>#mermaid-1 .node rect{fill:#fff}@font-face{src:url(\"data:font/woff2;base64,AA\")}</style></svg>";
        let out = sanitize(svg, "g2-").unwrap();
        assert!(out.contains("id=\"g2-mermaid-1\"") && out.contains("#g2-mermaid-1 .node rect{fill:#fff}"), "{out}");
        assert!(out.contains("data:font/woff2"));
    }
}
