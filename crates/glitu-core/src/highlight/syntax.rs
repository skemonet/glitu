//! Highlighting by syntect: the Sublime Text grammars of bat (from two-face, about 200
//! languages), whose scopes (`keyword.control.rust`, `string.quoted.double.json`…) become the
//! highlight.js classes that themes style.

use std::sync::LazyLock;

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use super::span;
use crate::html::esc;

/// The grammars, read once (about a megabyte, embedded)
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

/// Names of the fences that the grammars do not know as such
const ALIASES: [(&str, &str); 9] = [
    ("shell", "bash"),
    ("console", "bash"),
    ("shellsession", "bash"),
    ("c++", "cpp"),
    ("csharp", "cs"),
    ("golang", "go"),
    ("jsonc", "json"),
    ("svg", "xml"),
    ("docker", "dockerfile"),
];

/// The grammar of a fence's language: by extension or by name, regardless of case
fn syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let lang = lang.to_lowercase();
    let lang = ALIASES.iter().find(|(a, _)| *a == lang).map_or(lang.as_str(), |(_, l)| l);
    if lang.is_empty() || ["text", "plaintext", "txt", "plain"].contains(&lang) {
        return None;
    }
    SYNTAXES.find_syntax_by_token(lang)
}

/// Line-by-line highlighting: each line is a self-contained HTML fragment. A language without a
/// grammar, or a grammar that fails, gives plain text.
pub fn lines(lang: &str, code: &str) -> Vec<String> {
    let plain = || code.lines().map(esc).collect();
    let Some(syntax) = syntax(lang) else { return plain() };
    if syntax.name == "Markdown"
        && let Some(out) = front_matter(code)
    {
        return out;
    }
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut out = Vec::new();
    for line in LinesWithEndings::from(code) {
        let Ok(ops) = state.parse_line(line, &SYNTAXES) else { return plain() };
        let line = line.trim_end_matches(['\n', '\r']);
        let mut html = Line::default();
        let mut at = 0;
        for (pos, op) in ops {
            let pos = pos.min(line.len());
            html.push(class(&stack), &line[at.min(pos)..pos]);
            at = at.max(pos);
            if stack.apply(&op).is_err() {
                return plain();
            }
        }
        html.push(class(&stack), &line[at.min(line.len())..]);
        out.push(html.finish());
    }
    out
}

/// A markdown document that opens on a front matter: the grammar does not know it (its closing
/// `---` would underline a heading), it is highlighted as YAML
fn front_matter(code: &str) -> Option<Vec<String>> {
    let body = code.strip_prefix("---\n")?;
    let end = body
        .find("\n---\n")
        .map(|e| (e, e + 5))
        .or_else(|| body.strip_suffix("\n---").map(|b| (b.len(), body.len())))?;
    let mut out = vec![span("meta", "---")];
    out.extend(lines("yaml", &body[..end.0]));
    out.push(span("meta", "---"));
    if end.1 < body.len() {
        out.extend(lines("markdown", &body[end.1..]));
    }
    Some(out)
}

/// A highlighted line being written: the text of the same class is gathered in one span;
/// whitespace joins it only between two texts of that class
#[derive(Default)]
struct Line {
    html: String,
    class: Option<&'static str>,
    text: String,
    /// Whitespace waiting for the next text
    gap: String,
}

impl Line {
    fn push(&mut self, class: Option<&'static str>, text: &str) {
        let lead = text.len() - text.trim_start().len();
        self.gap += &text[..lead];
        let text = &text[lead..];
        if text.is_empty() {
            return;
        }
        if class != self.class {
            self.flush();
            self.class = class;
        }
        self.text += &std::mem::take(&mut self.gap);
        self.text += text.trim_end();
        self.gap += &text[text.trim_end().len()..];
    }

    fn flush(&mut self) {
        let text = esc(&std::mem::take(&mut self.text));
        self.html += &match self.class {
            Some(c) if !text.is_empty() => span(c, &text),
            _ => text,
        };
        self.html += &esc(&std::mem::take(&mut self.gap));
    }

    fn finish(mut self) -> String {
        self.flush();
        self.html
    }
}

/// Scopes → highlight.js classes, the first prefix that matches (a prefix stops at a dot); None:
/// the text stays plain, whatever the outer scopes say
const CLASSES: &[(&str, Option<&str>)] = &[
    ("comment", Some("comment")),
    ("punctuation.definition.comment", Some("comment")),
    ("punctuation.definition.string", Some("string")),
    ("string.regexp", Some("regexp")),
    ("string", Some("string")),
    ("storage.type.string", Some("string")),
    ("constant.numeric", Some("number")),
    ("constant.language", Some("literal")),
    ("constant.character.escape", Some("subst")),
    ("constant.other.symbol", Some("symbol")),
    ("constant.other.placeholder", Some("subst")),
    ("constant", Some("literal")),
    ("variable.parameter", Some("params")),
    ("variable.language", Some("variable language_")),
    ("variable.function", Some("title function_")),
    ("variable.other.member", Some("property")),
    ("variable.other.property", Some("property")),
    ("variable.other.object.property", Some("property")),
    ("variable.annotation", Some("meta")),
    ("variable", Some("variable")),
    ("entity.name.function", Some("title function_")),
    ("entity.name.tag.yaml", Some("attr")),
    ("entity.name.tag.toml", Some("attr")),
    ("entity.name.tag.css", Some("selector-tag")),
    ("entity.name.tag", Some("name")),
    ("entity.other.attribute-name.class.css", Some("selector-class")),
    ("entity.other.attribute-name.id.css", Some("selector-id")),
    ("entity.other.attribute-name.pseudo-class.css", Some("selector-pseudo")),
    ("entity.other.pseudo-class.css", Some("selector-pseudo")),
    ("entity.other.attribute-name.class.scss", Some("selector-class")),
    ("entity.other.attribute-name.id.scss", Some("selector-id")),
    ("entity.other.attribute-name.pseudo-class.scss", Some("selector-pseudo")),
    ("entity.other.pseudo-class.scss", Some("selector-pseudo")),
    ("entity.other.attribute-name.class.less", Some("selector-class")),
    ("entity.other.attribute-name.id.less", Some("selector-id")),
    ("entity.other.attribute-name.pseudo-class.less", Some("selector-pseudo")),
    ("entity.other.pseudo-class.less", Some("selector-pseudo")),
    ("entity.other.attribute-name.class.sass", Some("selector-class")),
    ("entity.other.attribute-name.id.sass", Some("selector-id")),
    ("entity.other.attribute-name.pseudo-class.sass", Some("selector-pseudo")),
    ("entity.other.pseudo-class.sass", Some("selector-pseudo")),
    ("entity.other.attribute-name", Some("attr")),
    ("entity.other.inherited-class", Some("title class_")),
    ("entity.name.section", Some("section")),
    ("entity.name.class", Some("title class_")),
    ("entity.name.struct", Some("title class_")),
    ("entity.name.enum", Some("title class_")),
    ("entity.name.type", Some("title class_")),
    ("entity.name.trait", Some("title class_")),
    ("entity.name.interface", Some("title class_")),
    ("entity.name.union", Some("title class_")),
    ("entity.name", Some("title")),
    ("support.type.property-name", Some("attr")),
    ("support.function", Some("built_in")),
    ("support.macro", Some("built_in")),
    ("support.type", Some("type")),
    ("support.class", Some("type")),
    ("support.constant", Some("literal")),
    ("support", Some("built_in")),
    ("storage.type.annotation", Some("meta")),
    ("storage", Some("keyword")),
    ("keyword.operator.word", Some("keyword")),
    ("keyword.operator", Some("operator")),
    ("keyword", Some("keyword")),
    ("markup.heading", Some("section")),
    ("markup.bold", Some("strong")),
    ("markup.italic", Some("emphasis")),
    ("markup.inserted", Some("addition")),
    ("markup.deleted", Some("deletion")),
    ("markup.underline.link", Some("link")),
    ("markup.quote", Some("quote")),
    ("markup.raw", Some("code")),
    ("punctuation.definition.list", Some("bullet")),
    ("punctuation.definition.heading", Some("section")),
    ("punctuation.definition.bold", Some("strong")),
    ("punctuation.definition.italic", Some("emphasis")),
    ("punctuation.definition.raw", Some("code")),
    ("punctuation.definition.parameter", Some("params")),
    ("punctuation.section.embedded", Some("meta")),
    ("punctuation.definition.tag", Some("tag")),
    ("punctuation.definition.variable", Some("variable")),
    ("punctuation.definition.annotation", Some("meta")),
    ("meta.annotation", Some("meta")),
    ("meta.decorator", Some("meta")),
    ("meta.preprocessor", Some("meta")),
    ("meta.attribute.rust", Some("meta")),
    ("punctuation", None),
];

/// Languages whose variables carry a sigil (`$x`): their variables are coloured, those of the
/// others are plain identifiers
const SIGILS: [&str; 6] = ["php", "shell", "perl", "powershell", "makefile", "scss"];

/// The class of a scope, if the table names it
fn of_scope(scope: Scope) -> Option<Option<&'static str>> {
    let name = scope.build_string();
    if name.starts_with("variable.other.") && !SIGILS.iter().any(|l| name.ends_with(&format!(".{l}"))) {
        return Some(None);
    }
    CLASSES
        .iter()
        .find(|(prefix, _)| name.strip_prefix(prefix).is_some_and(|r| r.is_empty() || r.starts_with('.')))
        .map(|(_, c)| *c)
}

/// The class of text under these scopes: the innermost scope the table names. The keys of a JSON
/// or TOML mapping are attributes, not strings.
fn class(stack: &ScopeStack) -> Option<&'static str> {
    let scopes = stack.as_slice();
    let class = scopes.iter().rev().find_map(|&s| of_scope(s)).flatten();
    let key = scopes.iter().any(|s| {
        let s = s.build_string();
        s.starts_with("meta.mapping.key") || s.starts_with("support.type.property-name")
    });
    if key && class == Some("string") { Some("attr") } else { class }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_by_name_extension_or_alias() {
        for lang in ["rust", "rs", "Python", "ts", "tsx", "go", "toml", "dockerfile", "hcl", "shell", "c++", "svg"] {
            assert!(syntax(lang).is_some(), "{lang}");
        }
        assert!(["plaintext", "", "no-such-language"].iter().all(|l| syntax(l).is_none()));
        assert_eq!(lines("no-such-language", "<a>\nb"), ["&lt;a&gt;", "b"]);
    }

    #[test]
    fn scopes_become_highlight_js_classes() {
        let rs = lines("rust", "fn add(a: u32) -> u32 { a + 1 } // sum\nlet s = \"x\\n\";");
        assert!(rs[0].starts_with("<span class=\"hljs-keyword\">fn</span>"), "{}", rs[0]);
        assert!(rs[0].contains("<span class=\"hljs-title function_\">add</span>"), "{}", rs[0]);
        assert!(rs[0].contains("<span class=\"hljs-number\">1</span>"), "{}", rs[0]);
        assert!(rs[0].ends_with("<span class=\"hljs-comment\">// sum</span>"), "{}", rs[0]);
        assert!(rs[1].contains("<span class=\"hljs-subst\">\\n</span>"), "{}", rs[1]);
        let json = lines("json", "{\"a\": true}");
        assert!(json[0].contains("<span class=\"hljs-attr\">"), "{}", json[0]);
        assert!(json[0].contains("<span class=\"hljs-literal\">true</span>"), "{}", json[0]);
        let yaml = lines("yaml", "name: web # c");
        assert!(yaml[0].starts_with("<span class=\"hljs-attr\">name</span>"), "{}", yaml[0]);
        let html = lines("html", "<p class=\"x\">a</p>");
        assert!(html[0].contains("<span class=\"hljs-name\">p</span>"), "{}", html[0]);
        assert!(
            html[0].contains("<span class=\"hljs-attr\">class</span>=<span class=\"hljs-string\">&quot;x&quot;</span>"),
            "{}",
            html[0]
        );
        let css = lines("css", ".a:hover { color: red }");
        assert!(css[0].contains("<span class=\"hljs-selector-class\">"), "{}", css[0]);
    }

    #[test]
    fn every_line_closes_its_spans() {
        let md = lines("md", "---\ntitle: x\n---\n\n# A");
        assert_eq!(md[0], "<span class=\"hljs-meta\">---</span>");
        assert!(md[1].starts_with("<span class=\"hljs-attr\">title</span>"), "{}", md[1]);
        assert_eq!((md[2].as_str(), md[3].as_str()), (md[0].as_str(), ""));
        assert!(md[4].contains("hljs-section"), "{}", md[4]);
        let c = lines("c", "/* a\nb */ int x;");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0], "<span class=\"hljs-comment\">/* a</span>");
        assert!(c[1].starts_with("<span class=\"hljs-comment\">b */</span>"), "{}", c[1]);
        for l in lines("python", "def f():\n    '''doc\n    more'''\n    return None") {
            assert_eq!(l.matches("<span").count(), l.matches("</span>").count(), "{l}");
        }
    }
}
