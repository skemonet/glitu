//! `glitu config`: the settings of a presentation, that is its front matter. Lists the keys glitu
//! knows with their values, reads one, sets one (checking its value) or removes one; the rest of
//! the file is left as written, line for line.

use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::Theme;

/// What a key accepts
#[derive(Clone, Copy)]
enum Accepts {
    /// Any text on one line
    Text,
    /// One of these words
    Choice(&'static [&'static str]),
    /// A theme: official (`paper`, `paper:0.1.0`), or the folder of a theme
    Theme,
    /// A language tag: `en`, `fr`, `pt-BR`
    Lang,
    /// A version (`0.1.0`) or `latest`
    Version,
    /// A file of the presentation, or an https:// address
    File,
}

/// A key of the front matter
pub struct Key {
    pub name: &'static str,
    pub about: &'static str,
    accepts: Accepts,
}

const fn key(name: &'static str, accepts: Accepts, about: &'static str) -> Key {
    Key { name, about, accepts }
}

/// The keys glitu reads, in the order of `glitu config`
pub const KEYS: [Key; 17] = [
    key("title", Accepts::Text, "Title of the cover and of the page"),
    key("subtitle", Accepts::Text, "Subtitle of the cover"),
    key("author", Accepts::Text, "Author, on the cover and in the page's metadata"),
    key("date", Accepts::Text, "Date, on the cover"),
    key("eyebrow", Accepts::Text, "Small line above the cover's title"),
    key("description", Accepts::Text, "Description for search engines and link previews [default: the subtitle]"),
    key("theme", Accepts::Theme, "Theme: crystal, paper… (glitu init --list-themes), name:version, or a folder"),
    key("navigation", Accepts::Choice(&["2d"]), "2d: one chapter per column (↓ ↑ within a chapter) [default: one row]"),
    key("lang", Accepts::Lang, "Language of the page: en, fr… [default: en]"),
    key("logo", Accepts::File, "Logo in the header: an image of img/ or public/, or an https:// address"),
    key("global-top", Accepts::Text, "Markdown on every slide, in front of the content (bottom right)"),
    key("global-bottom", Accepts::Text, "Markdown on every slide, behind the content (bottom left)"),
    key("font", Accepts::Text, "Font of the text, from Google Fonts (Inter) [default: the theme's]"),
    key("font-title", Accepts::Text, "Font of the titles, from Google Fonts (Poppins)"),
    key("font-mono", Accepts::Text, "Font of the code, from Google Fonts (Fira Code)"),
    key(
        "source",
        Accepts::Choice(&["true", "false"]),
        "false: the built page does not embed its markdown [default: true]",
    ),
    key("glitu", Accepts::Version, "Expected engine version (0.1.0) [default: the latest]"),
];

pub fn find(name: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.name == name)
}

/// The markdown file holding the front matter: `given` (a file, or a folder: its presentation),
/// otherwise the presentation of the current folder; for several files, the first one
pub fn file(given: Option<&Path>) -> Result<PathBuf, String> {
    let inputs = crate::source::inputs(&given.map(Path::to_path_buf).into_iter().collect::<Vec<_>>())?;
    let first = inputs.into_iter().next().ok_or("no markdown file")?;
    if !first.is_file() {
        return Err(format!("{}: no such file; create a presentation with `glitu init`", first.display()));
    }
    Ok(first)
}

/// The block of the front matter: (start of its first line, end of its last line), as
/// `glitu_core::markdown::front_matter` reads it
fn block(src: &str) -> Option<(usize, usize)> {
    let rest = src.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    Some((4, 4 + end + 1))
}

/// The key of a line of the front matter
fn key_of(line: &str) -> Option<&str> {
    line.split_once(':').map(|(k, _)| k.trim())
}

/// The value of `key`, as glitu reads it
pub fn get(src: &str, key: &str) -> Option<String> {
    glitu_core::markdown::front_matter(src).0.get(key).cloned()
}

/// The keys written in the front matter, in their order, with their values
pub fn written(src: &str) -> Vec<(String, String)> {
    let Some((start, end)) = block(src) else { return Vec::new() };
    src[start..end]
        .lines()
        .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
        .collect()
}

/// `src` with `key: value`: the line of the key replaced (the last one: it is the one glitu reads),
/// otherwise a line added at the end of the front matter, created if there is none
pub fn set(src: &str, key: &str, value: &str) -> String {
    let line = format!("{key}: {value}");
    let Some((start, end)) = block(src) else {
        return format!("---\n{line}\n---\n\n{src}");
    };
    let lines: Vec<&str> = src[start..end].lines().collect();
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    match lines.iter().rposition(|l| key_of(l) == Some(key)) {
        Some(i) => out[i] = line,
        None => out.push(line),
    }
    format!("{}{}\n{}", &src[..start], out.join("\n"), &src[end..])
}

/// `src` without `key` (`None` if it was not set); a front matter left empty goes away: glitu would
/// read its two `---` lines as slide separators
pub fn unset(src: &str, key: &str) -> Option<String> {
    let (start, end) = block(src)?;
    let lines: Vec<&str> = src[start..end].lines().collect();
    let kept: Vec<&str> = lines.iter().copied().filter(|l| key_of(l) != Some(key)).collect();
    if kept.len() == lines.len() {
        return None;
    }
    if kept.iter().all(|l| l.trim().is_empty()) {
        // the block and its closing `---`, then the blank lines that followed it
        let rest = &src[end + 4..];
        return Some(rest.trim_start_matches('\n').to_string());
    }
    Some(format!("{}{}\n{}", &src[..start], kept.join("\n"), &src[end..]))
}

/// Checks `value` for `key`; `file` is the markdown, for the relative paths. Returns a warning
/// for a value that can be written but is not usable yet (a missing logo)
pub fn check(key: &Key, value: &str, file: &Path) -> Result<Option<String>, String> {
    if value.contains('\n') {
        return Err(format!("{}: a value is on one line", key.name));
    }
    if value.trim().is_empty() {
        return Err(format!("{}: an empty value; to remove the key: glitu config --unset {}", key.name, key.name));
    }
    let dir = file.parent().unwrap_or(Path::new("."));
    match key.accepts {
        Accepts::Text => {}
        Accepts::Choice(words) => {
            if !words.contains(&value) {
                return Err(format!("{}: `{value}`; expected {}", key.name, words.join(" or ")));
            }
        }
        Accepts::Lang => {
            let ok =
                value.split('-').all(|p| !p.is_empty() && p.len() <= 8 && p.chars().all(|c| c.is_ascii_alphanumeric()));
            if !ok {
                return Err(format!("{}: `{value}` is not a language tag (en, fr, pt-BR)", key.name));
            }
        }
        Accepts::Version => {
            let ok = value == "latest"
                || value.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
            if !ok {
                return Err(format!("{}: `{value}`; expected a version (0.1.0) or latest", key.name));
            }
        }
        Accepts::Theme => match Theme::parse(value) {
            Theme::Official { name, .. } => {
                let names = crate::themes::names();
                if !names.contains(&name.as_str()) {
                    return Err(format!(
                        "theme: `{name}` is not a built-in theme ({}); to create your own: glitu init --theme {name}",
                        names.join(", ")
                    ));
                }
            }
            Theme::At(at)
                if !at.contains("://")
                    && !dir.join(&at).join(crate::build::THEME_SETTINGS).is_file()
                    && !dir.join(&at).join("glitu.json").is_file() =>
            {
                return Ok(Some(format!("{}: no theme there (no glitu.yaml)", dir.join(&at).display())));
            }
            Theme::At(_) => {}
        },
        Accepts::File => {
            let path = value.trim_matches(['"', '\'']);
            if !path.contains("://") && !dir.join(path).is_file() && !dir.join("public").join(path).is_file() {
                return Ok(Some(format!("{}: no such file yet", dir.join(path).display())));
            }
        }
    }
    Ok(None)
}

/// Reads `file`, applies `change`, writes it back
pub fn edit(file: &Path, change: impl FnOnce(&str) -> Result<String, String>) -> Result<(), String> {
    let src = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let new = change(&src)?;
    fs::write(file, new).map_err(|e| format!("{}: {e}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "---\ntitle: Talk\ntheme: crystal\n---\n\n# A\n\n## B\n";

    #[test]
    fn reads_the_front_matter_as_glitu_does() {
        assert_eq!(get(SRC, "theme").as_deref(), Some("crystal"));
        assert_eq!(get(SRC, "logo"), None);
        assert_eq!(written(SRC), [("title".into(), "Talk".into()), ("theme".into(), "crystal".into())]);
        assert!(written("# A\n").is_empty());
    }

    #[test]
    fn sets_a_key_in_place_or_at_the_end() {
        let s = set(SRC, "theme", "paper");
        assert_eq!(s, "---\ntitle: Talk\ntheme: paper\n---\n\n# A\n\n## B\n");
        let s = set(&s, "logo", "img/logo.svg");
        assert_eq!(s, "---\ntitle: Talk\ntheme: paper\nlogo: img/logo.svg\n---\n\n# A\n\n## B\n");
        assert_eq!(get(&s, "logo").as_deref(), Some("img/logo.svg"));
        // no front matter: one is created, the body is kept
        assert_eq!(set("# A\n", "lang", "fr"), "---\nlang: fr\n---\n\n# A\n");
        // a value with a colon
        assert_eq!(get(&set(SRC, "title", "Rust: why"), "title").as_deref(), Some("Rust: why"));
    }

    #[test]
    fn unsets_a_key_and_an_empty_front_matter() {
        assert_eq!(unset(SRC, "title").unwrap(), "---\ntheme: crystal\n---\n\n# A\n\n## B\n");
        assert_eq!(unset(SRC, "logo"), None);
        let one = "---\nlang: fr\n---\n\n# A\n";
        assert_eq!(unset(one, "lang").unwrap(), "# A\n");
    }

    #[test]
    fn checks_the_values() {
        let md = Path::new("slides.md");
        let k = |n| find(n).unwrap();
        assert!(check(k("theme"), "paper", md).unwrap().is_none());
        assert!(check(k("theme"), "crystal:0.1.0", md).unwrap().is_none());
        assert!(check(k("theme"), "nope", md).unwrap_err().contains("glitu init --theme nope"));
        assert!(check(k("theme"), "./themes/none", md).unwrap().unwrap().contains("no theme there"));
        assert!(check(k("navigation"), "3d", md).unwrap_err().contains("expected 2d"));
        assert!(check(k("source"), "false", md).unwrap().is_none());
        assert!(check(k("lang"), "pt-BR", md).unwrap().is_none() && check(k("lang"), "fr fr", md).is_err());
        assert!(check(k("glitu"), "0.1.0", md).is_ok() && check(k("glitu"), "v 1", md).is_err());
        assert!(check(k("logo"), "img/none.png", md).unwrap().unwrap().contains("no such file"));
        assert!(check(k("title"), "", md).unwrap_err().contains("--unset"));
    }
}
