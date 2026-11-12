//! The highlighting of highlight.js, reproduced for the presentations written with reveal-md's
//! rules: the compatibility oracle compares them with reveal-md's own rendering. YAML, HTML/XML
//! and the brace languages (C, PHP, JavaScript, shell, Python…); other languages stay plain.

use super::{is_ident, span};
use crate::html::esc;

/// Line-by-line highlighting: each line is a self-contained HTML fragment
pub fn lines(lang: &str, code: &str) -> Vec<String> {
    match lang {
        "yaml" | "yml" => code.lines().map(yaml_line).collect(),
        "html" | "xml" | "svg" => xml(code),
        l => match keywords(l) {
            Some(kw) => clike(l, kw, code),
            None => code.lines().map(esc).collect(),
        },
    }
}

// ---------------------------------------------------------------- YAML

fn yaml_line(line: &str) -> String {
    let indent = line.len() - line.trim_start().len();
    let mut out = esc(&line[..indent]);
    let mut rest = &line[indent..];
    if rest.starts_with('#') {
        return out + &span("comment", &esc(rest));
    }
    if rest == "---" || rest == "..." {
        return out + &span("meta", rest);
    }
    while let Some(r) = rest.strip_prefix("- ").or_else(|| (rest == "-").then_some("")) {
        out += &span("bullet", "-");
        if rest.len() > 1 {
            out.push(' ')
        }
        rest = r;
    }
    if let Some((key, value)) = yaml_key(rest) {
        out += &span("attr", &esc(key));
        rest = value;
        let spaces = rest.len() - rest.trim_start().len();
        out += &rest[..spaces];
        rest = &rest[spaces..];
    }
    out + &yaml_value(rest)
}

/// `key:` followed by a space or the end of the line; returns `key:` and the rest
fn yaml_key(s: &str) -> Option<(&str, &str)> {
    let end = s.find(':')?;
    let key = &s[..end];
    let ok = !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || "_-./".contains(c));
    let after = &s[end + 1..];
    (ok && (after.is_empty() || after.starts_with(' '))).then(|| (&s[..=end], after))
}

fn yaml_value(v: &str) -> String {
    if v.is_empty() {
        return String::new();
    }
    if let Some(inner) = v.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        let items: Vec<String> = inner.split(", ").map(yaml_scalar).collect();
        return format!("[{}]", items.join(", "));
    }
    yaml_scalar(v)
}

fn yaml_scalar(v: &str) -> String {
    let t = v.trim();
    if t.is_empty() {
        return esc(v);
    }
    let class = if t.parse::<f64>().is_ok() && !t.starts_with('.') {
        "number"
    } else if ["true", "false", "null", "yes", "no", "on", "off", "~"].contains(&t.to_lowercase().as_str()) {
        "literal"
    } else {
        "string"
    };
    span(class, &esc(v))
}

// ---------------------------------------------------------------- XML / HTML

/// Comments may span several lines: the whole text is highlighted, then the spans are closed
/// and reopened at each line break.
fn xml(code: &str) -> Vec<String> {
    let mut out = String::new();
    let mut rest = code;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("<!--") {
            let end = r.find("-->").map_or(r.len(), |e| e + 3);
            out += &span("comment", &esc(&format!("<!--{}", &r[..end])));
            rest = &r[end..];
        } else if rest.starts_with('<') && rest[1..].starts_with(|c: char| c.is_alphabetic() || c == '/') {
            let end = rest.find('>').map_or(rest.len(), |e| e + 1);
            out += &xml_tag(&rest[..end]);
            rest = &rest[end..];
        } else {
            // from the second character on (the first one may take several bytes)
            let first = rest.chars().next().map_or(0, char::len_utf8);
            let end = rest[first..].find('<').map_or(rest.len(), |e| e + first);
            out += &esc(&rest[..end]);
            rest = &rest[end..];
        }
    }
    split_spans(&out)
}

fn xml_tag(tag: &str) -> String {
    let closing = tag.starts_with("</");
    let body = tag.trim_start_matches("</").trim_start_matches('<').trim_end_matches('>');
    let self_closing = body.ends_with('/');
    let body = body.trim_end_matches('/');
    let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
    let mut inner = format!("&lt;{}{}", if closing { "/" } else { "" }, span("name", &esc(&body[..name_end])));
    inner += &xml_attributes(&body[name_end..]);
    if self_closing {
        inner += "/"
    }
    span("tag", &(inner + "&gt;"))
}

/// The attributes of a tag, `name="value"`, with the whitespace between them
fn xml_attributes(mut attrs: &str) -> String {
    let mut out = String::new();
    while !attrs.is_empty() {
        let ws = attrs.len() - attrs.trim_start().len();
        out += &attrs[..ws];
        attrs = &attrs[ws..];
        if attrs.is_empty() {
            break;
        }
        let name_len = attrs.find(|c: char| c == '=' || c.is_whitespace()).unwrap_or(attrs.len());
        out += &span("attr", &esc(&attrs[..name_len]));
        attrs = &attrs[name_len..];
        if let Some(r) = attrs.strip_prefix('=') {
            let q = r.chars().next().unwrap_or('"');
            let len = if q == '"' || q == '\'' {
                // `r` is empty when nothing follows the `=`
                r.get(1..).and_then(|v| v.find(q)).map_or(r.len(), |e| e + 2)
            } else {
                r.find(char::is_whitespace).unwrap_or(r.len())
            };
            out += &format!("={}", span("string", &esc(&r[..len])));
            attrs = &r[len..];
        }
    }
    out
}

/// Splits highlighted HTML into lines, closing then reopening the open spans
fn split_spans(html: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let (mut cur, mut open): (String, Vec<String>) = (String::new(), Vec::new());
    let mut rest = html;
    while !rest.is_empty() {
        if rest.starts_with("<span") {
            let end = rest.find('>').map_or(rest.len(), |e| e + 1);
            open.push(rest[..end].to_string());
            cur += &rest[..end];
            rest = &rest[end..];
        } else if let Some(r) = rest.strip_prefix("</span>") {
            open.pop();
            cur += "</span>";
            rest = r;
        } else if let Some(r) = rest.strip_prefix('\n') {
            cur += &"</span>".repeat(open.len());
            lines.push(std::mem::take(&mut cur));
            cur = open.concat();
            rest = r;
        } else {
            let len = rest.chars().next().map_or(1, char::len_utf8);
            cur += &rest[..len];
            rest = &rest[len..];
        }
    }
    lines.push(cur);
    lines
}

// ---------------------------------------------------------------- brace languages

fn keywords(lang: &str) -> Option<&'static [&'static str]> {
    Some(match lang {
        "c" | "h" | "cpp" | "c++" | "cc" => &[
            "auto",
            "break",
            "case",
            "char",
            "const",
            "continue",
            "default",
            "do",
            "double",
            "else",
            "enum",
            "extern",
            "float",
            "for",
            "goto",
            "if",
            "inline",
            "int",
            "long",
            "register",
            "return",
            "short",
            "signed",
            "sizeof",
            "static",
            "struct",
            "switch",
            "typedef",
            "union",
            "unsigned",
            "void",
            "volatile",
            "while",
            "class",
            "namespace",
            "new",
            "delete",
            "public",
            "private",
            "protected",
            "template",
            "this",
            "virtual",
            "bool",
        ],
        "php" => &[
            "abstract",
            "and",
            "array",
            "as",
            "break",
            "case",
            "catch",
            "class",
            "clone",
            "const",
            "continue",
            "declare",
            "default",
            "do",
            "echo",
            "else",
            "elseif",
            "empty",
            "extends",
            "final",
            "finally",
            "fn",
            "for",
            "foreach",
            "function",
            "global",
            "if",
            "implements",
            "include",
            "instanceof",
            "interface",
            "isset",
            "list",
            "match",
            "namespace",
            "new",
            "or",
            "print",
            "private",
            "protected",
            "public",
            "readonly",
            "require",
            "return",
            "static",
            "switch",
            "throw",
            "trait",
            "try",
            "unset",
            "use",
            "var",
            "while",
            "yield",
        ],
        "js" | "javascript" | "ts" | "typescript" | "mjs" => &[
            "as",
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "export",
            "extends",
            "finally",
            "for",
            "from",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "let",
            "new",
            "of",
            "return",
            "static",
            "super",
            "switch",
            "this",
            "throw",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "yield",
            "interface",
            "type",
            "enum",
            "implements",
        ],
        "java" | "kotlin" | "cs" | "csharp" | "go" | "rust" | "rs" | "swift" => &[
            "abstract",
            "as",
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "default",
            "defer",
            "do",
            "else",
            "enum",
            "extends",
            "final",
            "fn",
            "for",
            "func",
            "go",
            "if",
            "impl",
            "implements",
            "import",
            "in",
            "interface",
            "let",
            "loop",
            "match",
            "mod",
            "mut",
            "new",
            "package",
            "private",
            "protected",
            "pub",
            "public",
            "return",
            "select",
            "static",
            "struct",
            "super",
            "switch",
            "this",
            "throw",
            "trait",
            "try",
            "type",
            "use",
            "var",
            "void",
            "where",
            "while",
        ],
        "sh" | "bash" | "shell" | "zsh" => &[
            "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac", "in",
            "function", "return", "local", "export", "readonly",
        ],
        "python" | "py" => &[
            "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else",
            "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal", "not",
            "or", "pass", "raise", "return", "try", "while", "with", "yield",
        ],
        _ => return None,
    })
}

const LITERALS: &[&str] = &["true", "false", "null", "nil", "None", "True", "False", "undefined", "NULL"];

/// A common lexicon: comments, strings, numbers, keywords, `$x` variables, and
/// `function name(params)` like highlight.js for PHP and JavaScript
fn clike(lang: &str, kw: &[&str], code: &str) -> Vec<String> {
    let lx = Lexicon::of(lang, kw);
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some((html, end)) = lx.function(&chars, i) {
            out += &html;
            i = end;
            continue;
        }
        let (class, end) = lx.token(&chars, i);
        let text: String = chars[i..end].iter().collect();
        out += &match class {
            Some(class) => span(class, &esc(&text)),
            None => esc(&text),
        };
        i = end;
    }
    split_spans(&out)
}

/// What the common lexicon recognises in a language
struct Lexicon<'a> {
    keywords: &'a [&'a str],
    /// `# …` (shell, Python, PHP)
    hash_comments: bool,
    /// `// …` and `/* … */`
    slash_comments: bool,
    /// `$x` (shell, PHP)
    dollar_vars: bool,
    /// Strings between backticks (JavaScript)
    backticks: bool,
    /// `function name(params)` (PHP, JavaScript)
    functions: bool,
}

/// A token that was recognised: its class (none: plain text) and the index after it
type Token = (Option<&'static str>, usize);

impl<'a> Lexicon<'a> {
    fn of(lang: &str, keywords: &'a [&'a str]) -> Self {
        let shell = matches!(lang, "sh" | "bash" | "shell" | "zsh");
        let python = matches!(lang, "python" | "py");
        Lexicon {
            keywords,
            hash_comments: shell || python || lang == "php",
            slash_comments: !shell && !python,
            dollar_vars: shell || lang == "php",
            backticks: lang.starts_with('j'),
            functions: matches!(lang, "php" | "js" | "javascript" | "ts" | "typescript" | "mjs"),
        }
    }

    /// The token that starts at `i`
    fn token(&self, chars: &[char], i: usize) -> Token {
        self.comment(chars, i)
            .or_else(|| self.string(chars, i))
            .or_else(|| number(chars, i))
            .or_else(|| self.variable(chars, i))
            .or_else(|| self.word(chars, i))
            .unwrap_or((None, i + 1))
    }

    fn comment(&self, chars: &[char], i: usize) -> Option<Token> {
        let (c, next) = (chars[i], chars.get(i + 1).copied());
        if self.slash_comments && c == '/' && next == Some('*') {
            let close = (i + 2..chars.len().saturating_sub(1)).find(|&j| chars[j] == '*' && chars[j + 1] == '/');
            return Some((Some("comment"), close.map_or(chars.len(), |j| j + 2)));
        }
        let line = (self.slash_comments && c == '/' && next == Some('/')) || (self.hash_comments && c == '#');
        line.then(|| (Some("comment"), scan(chars, i, |ch| ch != '\n')))
    }

    fn string(&self, chars: &[char], i: usize) -> Option<Token> {
        let quote = chars[i];
        if !(quote == '"' || quote == '\'' || (quote == '`' && self.backticks)) {
            return None;
        }
        let mut j = i + 1;
        while j < chars.len() && chars[j] != quote && chars[j] != '\n' {
            if chars[j] == '\\' {
                j += 1
            }
            j += 1;
        }
        Some((Some("string"), (j + 1).min(chars.len())))
    }

    fn variable(&self, chars: &[char], i: usize) -> Option<Token> {
        let starts = chars.get(i + 1).is_some_and(|&n| n.is_alphabetic() || n == '_');
        (self.dollar_vars && chars[i] == '$' && starts).then(|| (Some("variable"), scan(chars, i + 1, is_ident)))
    }

    /// A keyword, a literal, or a plain identifier
    fn word(&self, chars: &[char], i: usize) -> Option<Token> {
        if !(chars[i].is_alphabetic() || chars[i] == '_') {
            return None;
        }
        let end = scan(chars, i, is_ident);
        let word: String = chars[i..end].iter().collect();
        let class = if self.keywords.contains(&word.as_str()) {
            Some("keyword")
        } else if LITERALS.contains(&word.as_str()) {
            Some("literal")
        } else {
            None
        };
        Some((class, end))
    }

    /// `function name(params)` at `i`, marked up like highlight.js, and its end
    fn function(&self, chars: &[char], i: usize) -> Option<(String, usize)> {
        const WORD: &str = "function";
        let end = i + WORD.len();
        let starts_word = !i.checked_sub(1).is_some_and(|p| is_ident(chars[p]));
        if !self.functions || !starts_word || chars.get(i..end)?.iter().collect::<String>() != WORD {
            return None;
        }
        let rest: String = chars[end..].iter().collect();
        if rest.starts_with(is_ident) {
            return None;
        }
        let ws = rest.len() - rest.trim_start().len();
        let name_len = rest[ws..].find(|ch: char| !is_ident(ch)).unwrap_or(0);
        let after = &rest[ws + name_len..];
        let close = after.strip_prefix('(').and(after.find(')'))?;
        let name = &rest[ws..ws + name_len];
        let html = format!(
            "<span class=\"hljs-function\">{}{}{}(<span class=\"hljs-params\">{}</span>)</span>",
            span("keyword", WORD),
            &rest[..ws],
            if name.is_empty() { String::new() } else { span("title", name) },
            esc(&after[1..close])
        );
        // `rest` is indexed in bytes: the end is counted in characters
        Some((html, end + rest[..ws + name_len + close + 1].chars().count()))
    }
}

/// A number, unless it ends an identifier (`x2`)
fn number(chars: &[char], i: usize) -> Option<Token> {
    let after_ident = i.checked_sub(1).is_some_and(|p| is_ident(chars[p]));
    (chars[i].is_ascii_digit() && !after_ident)
        .then(|| (Some("number"), scan(chars, i, |ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_')))
}

/// The first index from `from` whose character no longer satisfies `keep`
fn scan(chars: &[char], from: usize, keep: impl Fn(char) -> bool) -> usize {
    chars[from..].iter().position(|&c| !keep(c)).map_or(chars.len(), |p| from + p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The highlighted lines must reproduce highlight.js (the oracle's rendering)
    fn reference(file: &str) -> Vec<String> {
        let html = std::fs::read_to_string(format!("{}/tests/fixtures/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let first = html.split("</code>").next().unwrap();
        first
            .split("<td class=\"hljs-ln-line hljs-ln-code\" data-line-number=\"")
            .skip(1)
            .map(|td| {
                let start = td.find('>').unwrap() + 1;
                td[start..td.find("</td>").unwrap()].replace('"', "&quot;").replace("&quot;", "\"")
            })
            .collect()
    }

    fn ours(lang: &str, code: &str) -> Vec<String> {
        lines(lang, code)
            .into_iter()
            .map(|l| if l.is_empty() { " ".into() } else { l.replace("&quot;", "\"") })
            .collect()
    }

    #[test]
    fn brace_languages() {
        let js = lines("js", "let s = `a`; function add(a, b) { return x2 + 1.5; } // end");
        assert!(js[0].contains("<span class=\"hljs-string\">`a`</span>"));
        assert!(js[0].contains("<span class=\"hljs-title\">add</span>(<span class=\"hljs-params\">a, b</span>)"));
        assert!(js[0].contains("x2 + <span class=\"hljs-number\">1.5</span>"));
        assert!(js[0].ends_with("<span class=\"hljs-comment\">// end</span>"));
        let sh = lines("sh", "echo $HOME # home\nexport A=\"b\\\"c\"");
        assert!(sh[0].contains("<span class=\"hljs-variable\">$HOME</span>"));
        assert!(sh[1].contains("<span class=\"hljs-keyword\">export</span>"));
        let py = lines("py", "x = None\n'''a'''");
        assert!(py[0].contains("<span class=\"hljs-literal\">None</span>"));
        let rs = lines("rust", "/* a\nb */ fn");
        assert_eq!(rs[0], "<span class=\"hljs-comment\">/* a</span>");
        assert_eq!(rs[1], "<span class=\"hljs-comment\">b */</span> <span class=\"hljs-keyword\">fn</span>");
        assert_eq!(lines("unknown", "<a>"), ["&lt;a&gt;"]);
    }

    #[test]
    fn yaml_like_hljs() {
        let src = include_str!("../../tests/fixtures/deployment.yml");
        assert_eq!(ours("yaml", src.trim_end()), reference("hljs-yaml.html"));
        assert_eq!(
            lines("yaml", "- [1, two, true]\n# c\n---"),
            [
                "<span class=\"hljs-bullet\">-</span> [<span class=\"hljs-number\">1</span>, \
             <span class=\"hljs-string\">two</span>, <span class=\"hljs-literal\">true</span>]",
                "<span class=\"hljs-comment\"># c</span>",
                "<span class=\"hljs-meta\">---</span>",
            ]
        );
    }

    #[test]
    fn php_like_hljs() {
        assert_eq!(ours("php", include_str!("../../tests/fixtures/php.txt").trim_end()), reference("hljs-php.html"));
    }

    #[test]
    fn c_like_hljs() {
        assert_eq!(ours("c", include_str!("../../tests/fixtures/c.txt").trim_end()), reference("hljs-c.html"));
    }

    #[test]
    fn html_with_text_of_several_bytes_and_unfinished_attributes() {
        assert_eq!(ours("html", "<h1>日本語</h1>")[0].matches("日本語").count(), 1);
        assert!(ours("html", "été <b>été</b>")[0].starts_with("été "));
        // `href=` with nothing after it: at the end of the tag, and of the text
        assert!(ours("html", "<a href=>x</a>")[0].contains("href"));
        assert!(ours("xml", "<a href=")[0].contains("href"));
    }

    #[test]
    fn html_like_hljs() {
        let src = include_str!("../../tests/fixtures/split-slide.html");
        assert_eq!(ours("html", src.trim_end()), reference("hljs-html.html"));
    }
}
