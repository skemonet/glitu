//! A presentation's markdown, split into several files if needed: a line
//! `<!-- include: chapters/deployment.md -->` inserts another file (path relative to the file that
//! includes it), which may include others in turn. The front matter of an included file is
//! ignored: only the presentation's counts. The paths of images and terminals stay relative to the
//! main file.
//!
//! A line `<<< src/main.rs#region rust title="main.rs" @2` inserts code taken from a file, as in
//! Slidev: the whole file, a region (`#name`: the lines between `#region name` and `#endregion`,
//! in a comment) or lines (`#L3-L8`); the language comes from the extension when it is not given;
//! the rest of the line goes to the block's info string.
//!
//! The built page keeps a copy of it (`embed`), file by file, which `glitu unpack` takes back out
//! (`extract`): a published presentation can be reopened to be edited.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::json;

/// The full markdown and the files read, as written (to be watched by `glitu serve`, to be copied
/// into the page)
pub struct Source {
    pub text: String,
    pub files: Vec<File>,
    /// Where each line of `text` comes from: (rank in `files`, line of that file); none for
    /// imported code (`<<<`)
    pub origins: Vec<Option<(usize, usize)>>,
}

pub struct File {
    pub path: PathBuf,
    pub text: String,
}

pub fn read(main: &Path) -> Result<Source, String> {
    let mut reader = Reader { stack: Vec::new(), files: Vec::new(), origins: Vec::new() };
    let text = reader.expand(main)?;
    Ok(Source { text, files: reader.files, origins: reader.origins })
}

impl Source {
    /// The file and the line (from 0) where byte `at` of `text` comes from
    pub fn origin(&self, at: usize) -> Option<(&Path, usize)> {
        let line = self.text.get(..at)?.matches('\n').count();
        let (file, n) = (*self.origins.get(line)?)?;
        Some((self.files[file].path.as_path(), n))
    }
}

/// `<!-- include: x.md -->` → x.md
fn include_target(line: &str) -> Option<&str> {
    let inner = line.trim().strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    Some(inner.strip_prefix("include:")?.trim()).filter(|p| !p.is_empty())
}

/// Where a file stands in its code blocks, line after line: inclusions are not expanded inside a
/// block (the syntax can be shown there)
#[derive(Default)]
struct Fences {
    /// The open block: its sign and its length (a four-backtick fence holds others, of three)
    open: Option<(char, usize)>,
    /// The open block is an animated code (` ````md magic-move `)
    magic: bool,
    /// Inside an animated code: the length of the open version's fence, if one is open; between
    /// two versions, a `<<<` line imports one
    version: Option<usize>,
}

impl Fences {
    fn update(&mut self, trimmed: &str) {
        for mark in ['`', '~'] {
            let run = trimmed.len() - trimmed.trim_start_matches(mark).len();
            if run < 3 {
                continue;
            }
            let alone = trimmed.trim_end().len() == run;
            match self.open {
                None => {
                    self.open = Some((mark, run));
                    self.magic = run > 3 && trimmed.contains("magic-move");
                }
                Some((m, n)) if self.magic && m == mark && run < n => {
                    self.version = match self.version {
                        Some(v) if run >= v && alone => None,
                        None => Some(run),
                        v => v,
                    };
                }
                Some((m, n)) if m == mark && run >= n && alone && self.version.is_none() => self.open = None,
                _ => {}
            }
        }
    }

    /// Outside any block
    fn outside(&self) -> bool {
        self.open.is_none()
    }

    /// Where a `<<<` line imports code: outside any block, or between two versions of an animated code
    fn imports(&self) -> bool {
        self.open.is_none() || (self.magic && self.version.is_none())
    }
}

/// The files read so far, and where each line of the result comes from
struct Reader {
    /// The files being included, to refuse a loop
    stack: Vec<PathBuf>,
    files: Vec<File>,
    origins: Vec<Option<(usize, usize)>>,
}

impl Reader {
    fn expand(&mut self, path: &Path) -> Result<String, String> {
        let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.stack.contains(&key) {
            let chain: Vec<String> = self.stack.iter().chain([&key]).map(|p| p.display().to_string()).collect();
            return Err(format!("inclusion loop: {}", chain.join(" → ")));
        }
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        // An included file: without its front matter
        let body =
            if self.stack.is_empty() { text.as_str() } else { glitu_core::markdown::front_matter(&text).1 }.to_string();
        // the lines of the body are counted after those of the removed front matter
        let (me, skipped) = (self.files.len(), text[..text.len() - body.len()].matches('\n').count());
        self.files.push(File { path: path.to_path_buf(), text });
        self.stack.push(key);
        let dir = path.parent().unwrap_or(Path::new(""));
        let mut out = String::with_capacity(body.len());
        let mut fences = Fences::default();
        for (n, line) in body.split_inclusive('\n').enumerate() {
            let trimmed = line.trim_start();
            fences.update(trimmed);
            if let Some(target) = include_target(line).filter(|_| fences.outside()) {
                out += &self.include(&dir.join(target))?;
            } else if let Some(spec) = trimmed.strip_prefix("<<< ").filter(|_| fences.imports()) {
                out += &self.import(path, dir, spec.trim(), &fences)?;
            } else {
                self.origins.push(Some((me, skipped + n)));
                out += line;
            }
        }
        self.stack.pop();
        Ok(out)
    }

    /// An included file, expanded, as whole lines
    fn include(&mut self, path: &Path) -> Result<String, String> {
        let before = self.origins.len();
        let included = self.expand(path)?;
        let kept = included.trim_end_matches('\n');
        self.origins.resize(before + kept.matches('\n').count() + 1, None);
        Ok(format!("{kept}\n"))
    }

    /// Code imported by a `<<<` line of `path`
    fn import(&mut self, path: &Path, dir: &Path, spec: &str, fences: &Fences) -> Result<String, String> {
        let code = snippet(dir, spec, &mut self.files)?;
        // the version must fit in the animated block: a fence shorter than its own
        if let Some((_, n)) = fences.open
            && code.len() - code.trim_start_matches('`').len() >= n
        {
            return Err(format!(
                "{}: « <<< {spec} » contains backticks, make the fence of the magic-move block longer",
                path.display()
            ));
        }
        self.origins.extend(std::iter::repeat_n(None, code.matches('\n').count()));
        Ok(code)
    }
}

/// `<<< path#part language attributes` → the code block
fn snippet(dir: &Path, spec: &str, files: &mut Vec<File>) -> Result<String, String> {
    let (target, rest) = spec.split_once(char::is_whitespace).unwrap_or((spec, ""));
    let (path, part) = target.split_once('#').unwrap_or((target, ""));
    let file = dir.join(path);
    let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let code = excerpt(&text, part).ok_or_else(|| format!("{}: part « {part} » not found", file.display()))?;
    files.push(File { path: file.clone(), text });
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let rest = rest.trim();
    // a language is given if it comes first and is neither an attribute nor a click
    let given = rest.split_whitespace().next().filter(|w| !w.contains('=') && !w.starts_with('@'));
    let info = match given {
        Some(_) => rest.to_string(),
        None => format!("{} {rest}", language(ext)).trim_end().to_string(),
    };
    // a fence longer than any run of backticks in the code
    let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    Ok(format!("{fence}{info}\n{}\n{fence}\n", code.trim_end_matches('\n')))
}

/// Part of a file: all of it (`""`), lines (`L3-L8`, `L5`), or a region (`name`)
fn excerpt(text: &str, part: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if part.is_empty() {
        return Some(text.to_string());
    }
    let number = |s: &str| s.trim_start_matches('L').parse::<usize>().ok().filter(|&n| n >= 1);
    if let Some(range) = part.strip_prefix('L') {
        let (a, b) = range.split_once('-').unwrap_or((range, range));
        let (a, b) = (number(a)?, number(b)?.min(lines.len()));
        return (a <= b).then(|| lines[a - 1..b].join("\n"));
    }
    fn marker<'a>(line: &'a str, word: &str) -> Option<&'a str> {
        line.trim_start().trim_start_matches(|c: char| !c.is_alphanumeric() && c != '#').strip_prefix(word).map(str::trim)
    }
    let start = lines
        .iter()
        .position(|l| marker(l, "#region").is_some_and(|n| n.trim_end_matches(['-', '>', '*', '/']).trim() == part))?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| marker(l, "#endregion").is_some())
        .map_or(lines.len(), |e| start + 1 + e);
    Some(dedent(&lines[start + 1..end]))
}

/// Lines without their common indentation
fn dedent(lines: &[&str]) -> String {
    let indent = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    lines.iter().map(|l| l.get(indent..).unwrap_or("")).collect::<Vec<_>>().join("\n")
}

/// The language of a file according to its extension, for highlighting
fn language(ext: &str) -> &str {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "sh" => "bash",
        "yml" => "yaml",
        "md" => "markdown",
        "h" => "c",
        "mjs" | "cjs" => "js",
        e => e,
    }
}

/// Id of the copy in the page
const EMBED_ID: &str = "glitu-source";

/// `a/./b/../c.md` → `a/c.md`; `None`: the path leaves the folder (`../x.md`) or is absolute
fn normalized(path: &Path) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for c in path.components() {
        match c {
            Component::Normal(p) => parts.push(p.to_str()?),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Copy of the markdown files, to be written in the page: JSON in a `<script>` that the browser
/// does not run. Paths relative to the main file, the first one being that file; a file included
/// from outside its folder keeps its path as is (`unpack` will report it).
pub fn embed(src: &Source) -> String {
    let root = src.files.first().and_then(|f| f.path.parent()).unwrap_or(Path::new(""));
    let files: Vec<_> = src
        .files
        .iter()
        .map(|f| {
            let rel = f.path.strip_prefix(root).unwrap_or(&f.path);
            let path = normalized(rel).unwrap_or_else(|| rel.display().to_string());
            json!({ "path": path, "text": f.text })
        })
        .collect();
    // `<` escaped: the text cannot close the script (`</script>`)
    let data = json!({ "glitu": 1, "files": files }).to_string().replace('<', "\\u003c");
    format!("<script type=\"application/json\" id=\"{EMBED_ID}\">{data}</script>")
}

/// The files taken back out of a page: (relative path, text), the main one first, and those that
/// cannot be (path outside the presentation's folder)
pub struct Extracted {
    pub files: Vec<(String, String)>,
    pub refused: Vec<String>,
}

/// The files copied into a page by `embed`
pub fn extract(html: &str) -> Result<Extracted, String> {
    let open = format!("id=\"{EMBED_ID}\">");
    let start = html.find(&open).ok_or("this page does not contain the markdown of its presentation")? + open.len();
    let len = html[start..].find("</script>").ok_or("truncated copy of the markdown")?;
    let data: serde_json::Value =
        serde_json::from_str(&html[start..start + len]).map_err(|e| format!("unreadable copy of the markdown: {e}"))?;
    let mut out = Extracted { files: Vec::new(), refused: Vec::new() };
    for f in data["files"].as_array().ok_or("copy of the markdown without files")? {
        let path = f["path"].as_str().unwrap_or_default();
        match normalized(Path::new(path)).filter(|p| p == path) {
            Some(path) => out.files.push((path, f["text"].as_str().unwrap_or_default().to_string())),
            None => out.refused.push(path.to_string()),
        }
    }
    if out.files.is_empty() {
        return Err("no file to take out of this page".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("glitu-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn included_files_are_inserted_in_place() {
        let dir = temp("include");
        fs::create_dir_all(dir.join("ch")).unwrap();
        fs::write(
            dir.join("slides.md"),
            "---\ntitle: T\n---\n\n<!-- include: ch/a.md -->\n\n```md\n<!-- include: x.md -->\n```\n",
        )
        .unwrap();
        fs::write(dir.join("ch/a.md"), "---\ntitle: ignored\n---\n# A\n<!-- include: b.md -->\n").unwrap();
        fs::write(dir.join("ch/b.md"), "## B\n").unwrap();
        let src = read(&dir.join("slides.md")).unwrap();
        assert_eq!(src.text, "---\ntitle: T\n---\n\n# A\n## B\n\n```md\n<!-- include: x.md -->\n```\n");
        assert_eq!(src.files.len(), 3);
        // the line of each heading points at its own file
        let at = |s: &str| src.text.find(s).unwrap();
        assert_eq!(src.origin(at("# A")), Some((dir.join("ch/a.md").as_path(), 3)));
        assert_eq!(src.origin(at("## B")), Some((dir.join("ch/b.md").as_path(), 0)));
        let page = format!("<body>{}</body>", embed(&src));
        assert!(!page.contains("<!-- include"), "the markdown must not be able to write a tag");
        let back = extract(&page).unwrap().files;
        let paths: Vec<&str> = back.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(paths, ["slides.md", "ch/a.md", "ch/b.md"]);
        assert_eq!(back[1].1, fs::read_to_string(dir.join("ch/a.md")).unwrap());
        fs::write(dir.join("ch/b.md"), "<!-- include: a.md -->\n").unwrap();
        assert!(read(&dir.join("slides.md")).err().unwrap().contains("inclusion loop"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn code_comes_from_files() {
        let dir = temp("snippet");
        fs::write(
            dir.join("a.rs"),
            "fn main() {\n    // #region loop\n    for i in 0..3 {\n        dbg!(i);\n    }\n    // #endregion\n}\n",
        )
        .unwrap();
        fs::write(dir.join("slides.md"), "<<< a.rs#loop title=\"x\" @2\n\n<<< a.rs#L1-L1 c\n\n<<< a.rs#L7\n").unwrap();
        let src = read(&dir.join("slides.md")).unwrap();
        assert_eq!(
            src.text,
            "```rust title=\"x\" @2\nfor i in 0..3 {\n    dbg!(i);\n}\n```\n\n```c\nfn main() {\n```\n\n```rust\n}\n```\n"
        );
        assert_eq!(src.files.len(), 4);
        fs::write(dir.join("slides.md"), "<<< a.rs#missing\n").unwrap();
        assert!(read(&dir.join("slides.md")).err().unwrap().contains("not found"));
        fs::write(dir.join("slides.md"), "<<< a.rs#L5-L2\n").unwrap();
        assert!(read(&dir.join("slides.md")).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_magic_move_version_can_come_from_a_file() {
        let dir = temp("magic");
        fs::write(dir.join("b.yml"), "b: 2\n").unwrap();
        // between two versions, the line is imported; inside a version, it stays code
        fs::write(dir.join("slides.md"), "````md magic-move\n```yaml\n<<< b.yml\n```\n<<< b.yml\n````\n").unwrap();
        let src = read(&dir.join("slides.md")).unwrap();
        assert_eq!(src.text, "````md magic-move\n```yaml\n<<< b.yml\n```\n```yaml\nb: 2\n```\n````\n");
        fs::write(dir.join("c.md"), "```sh\nls\n```\n").unwrap();
        fs::write(dir.join("slides.md"), "````md magic-move\n<<< c.md\n````\n").unwrap();
        assert!(read(&dir.join("slides.md")).err().unwrap().contains("make the fence"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unpacking_never_writes_outside_the_folder() {
        for bad in ["../x.md", "/etc/x.md", "a/../../x.md", "a/./b.md"] {
            let page = format!("<script id=\"{EMBED_ID}\">{}</script>", json!({ "files": [{ "path": bad, "text": "" }] }));
            let page = page.replace("\"files\":[", "\"files\":[{\"path\":\"ok.md\",\"text\":\"\"},");
            assert_eq!(extract(&page).unwrap().refused, [bad], "{bad}");
        }
        assert!(extract("<p>no copy</p>").is_err());
        assert!(extract(&format!("<script id=\"{EMBED_ID}\">{{\"files\":[]}}</script>")).is_err());
        assert_eq!(language("py"), "python");
        assert_eq!(language("toml"), "toml");
    }
}
