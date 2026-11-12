//! A presentation's markdown, split into several files if needed. Given a folder (the current one
//! by default), glitu reads its `slides.md`, or without one all its `.md` files, merged in natural
//! order (`2-a.md` before `10-b.md`), leaving out README, CHANGELOG, LICENSE and the names
//! starting with `_` or `.` (named on the command line, any file is taken). Only the first file's
//! front matter counts. And a line
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

use crate::confine::Reads;

/// The full markdown and the files read, as written (to be watched by `glitu server`, to be copied
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

/// A presentation alone, confined to its folder
pub fn read(main: &Path) -> Result<Source, String> {
    let root = main.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    read_all(&[main.to_path_buf()], &Reads::new(root, &[]))
}

/// Several files one after the other, each starting on a line of its own after a blank one; the
/// front matter of the first one alone is kept. The files they include or import are read only
/// inside `reads` (the files named here are the user's choice)
pub fn read_all(paths: &[PathBuf], reads: &Reads) -> Result<Source, String> {
    let mut reader = Reader { stack: Vec::new(), files: Vec::new(), origins: Vec::new(), reads };
    let mut text = String::new();
    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
            reader.origins.push(None);
        }
        text += &reader.expand(path, i > 0)?;
    }
    Ok(Source { text, files: reader.files, origins: reader.origins })
}

/// The markdown files of a presentation, from the command line: none (the current folder), one
/// folder, or files (taken as they are, in this order)
pub fn inputs(args: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    match args {
        [] => markdown_in(Path::new(".")),
        [one] if one.is_dir() => markdown_in(one),
        files => match files.iter().find(|f| f.is_dir()) {
            Some(dir) => Err(format!("{}: a folder among files; give one folder, or files", dir.display())),
            None => Ok(files.to_vec()),
        },
    }
}

/// The presentation of a folder: its `slides.md`, otherwise its `.md` files in natural order, but
/// README, CHANGELOG, LICENSE and the names starting with `_` or `.`
pub fn markdown_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let at = |name: &std::ffi::OsStr| if dir == Path::new(".") { PathBuf::from(name) } else { dir.join(name) };
    // these files are found, not named by the user: a link leading out of the folder is refused
    let inside = Reads::only(dir);
    let found = |name: &str| -> Result<PathBuf, String> {
        if inside.contains(&dir.join(name)) {
            return Ok(at(name.as_ref()));
        }
        Err(format!(
            "{name}: a symbolic link out of the presentation's folder; to use it anyway, name it on the command line"
        ))
    };
    if dir.join("slides.md").is_file() {
        return Ok(vec![found("slides.md")?]);
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".md") && !left_out(n))
        .collect();
    names.sort_by(|a, b| natural(a, b));
    if names.is_empty() {
        return Err(format!(
            "no markdown file in {}: create a presentation with `glitu init`, or give the files to use",
            if dir == Path::new(".") { "this folder".to_string() } else { dir.display().to_string() }
        ));
    }
    names.iter().map(|n| found(n)).collect()
}

/// A file of the folder that is not part of the presentation, unless named on the command line
fn left_out(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    name.starts_with(['_', '.']) || ["readme", "changelog", "license"].iter().any(|p| lower.starts_with(p))
}

/// Natural order: the numbers by their value (`2` before `10`), the rest without case
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    let chunks = |s: &str| -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let digit = c.is_ascii_digit();
            match out.last_mut() {
                Some((d, run)) if *d == digit => run.push(c),
                _ => out.push((digit, c.to_string())),
            }
        }
        out
    };
    let (x, y) = (chunks(a), chunks(b));
    for (p, q) in x.iter().zip(&y) {
        let order = match (p, q) {
            ((true, m), (true, n)) => {
                let (m, n) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                m.len().cmp(&n.len()).then_with(|| m.cmp(n))
            }
            ((_, m), (_, n)) => m.to_lowercase().cmp(&n.to_lowercase()),
        };
        if order.is_ne() {
            return order;
        }
    }
    x.len().cmp(&y.len()).then_with(|| a.cmp(b))
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
struct Reader<'a> {
    /// The files being included, to refuse a loop
    stack: Vec<PathBuf>,
    files: Vec<File>,
    origins: Vec<Option<(usize, usize)>>,
    /// Where the presentation may read
    reads: &'a Reads,
}

impl Reader<'_> {
    /// `included`: the front matter is left out
    fn expand(&mut self, path: &Path, included: bool) -> Result<String, String> {
        let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.stack.contains(&key) {
            let chain: Vec<String> = self.stack.iter().chain([&key]).map(|p| p.display().to_string()).collect();
            return Err(format!("inclusion loop: {}", chain.join(" → ")));
        }
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        // An included file: without its front matter
        let body = if included { glitu_core::markdown::front_matter(&text).1 } else { text.as_str() }.to_string();
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
                let at = self.reads.open(dir, target).map_err(|e| format!("{}: include: {e}", path.display()))?;
                out += &self.include(&at)?;
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
        let included = self.expand(path, true)?;
        let kept = included.trim_end_matches('\n');
        self.origins.resize(before + kept.matches('\n').count() + 1, None);
        Ok(format!("{kept}\n"))
    }

    /// Code imported by a `<<<` line of `path`
    fn import(&mut self, path: &Path, dir: &Path, spec: &str, fences: &Fences) -> Result<String, String> {
        let code = snippet(dir, spec, &mut self.files, self.reads).map_err(|e| match e.strip_prefix('!') {
            Some(refused) => format!("{}: <<< {refused}", path.display()),
            None => e,
        })?;
        // the version must fit in the animated block: a fence shorter than its own
        if let Some((_, n)) = fences.open
            && code.len() - code.trim_start_matches('`').len() >= n
        {
            return Err(format!(
                "{}: `<<< {spec}` contains backticks, make the fence of the magic-move block longer",
                path.display()
            ));
        }
        self.origins.extend(std::iter::repeat_n(None, code.matches('\n').count()));
        Ok(code)
    }
}

/// `<<< path#part language attributes` → the code block
/// (a path refused starts with `!`)
fn snippet(dir: &Path, spec: &str, files: &mut Vec<File>, reads: &Reads) -> Result<String, String> {
    let (target, rest) = spec.split_once(char::is_whitespace).unwrap_or((spec, ""));
    let (path, part) = target.split_once('#').unwrap_or((target, ""));
    let file = reads.open(dir, path).map_err(|e| format!("!{e}"))?;
    let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let code = excerpt(&text, part).ok_or_else(|| format!("{}: part `{part}` not found", file.display()))?;
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
        line.trim_start()
            .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '#')
            .strip_prefix(word)
            .map(str::trim)
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
    let indent =
        lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
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
    // `<` escaped: the text cannot close the script (`</script>`)
    let data = copy(src).replace('<', "\\u003c");
    format!("<script type=\"application/json\" id=\"{EMBED_ID}\">{data}</script>")
}

/// The copy itself, as JSON: `{ "glitu": 1, "files": [ { "path", "text" } ] }` (also the
/// `source.json` of a `.glituplay` file)
pub fn copy(src: &Source) -> String {
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
    json!({ "glitu": 1, "files": files }).to_string()
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
    files_of(&html[start..start + len])
}

/// The files of a copy written by `copy`
pub fn files_of(json: &str) -> Result<Extracted, String> {
    let data: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("unreadable copy of the markdown: {e}"))?;
    let mut out = Extracted { files: Vec::new(), refused: Vec::new() };
    for f in data["files"].as_array().ok_or("copy of the markdown without files")? {
        let path = f["path"].as_str().unwrap_or_default();
        match normalized(Path::new(path)).filter(|p| p == path) {
            Some(path) => out.files.push((path, f["text"].as_str().unwrap_or_default().to_string())),
            None => out.refused.push(path.to_string()),
        }
    }
    if out.files.is_empty() {
        return Err("this page contains no source file".into());
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
    fn a_folder_without_slides_md_merges_its_markdown() {
        let dir = temp("merge");
        for (name, text) in [
            ("10-end.md", "# End\n"),
            ("2-deploy.md", "---\ntitle: ignored\n---\n# Deploy"),
            ("01-intro.md", "---\ntitle: T\n---\n# Intro\n"),
            ("README.md", "# readme\n"),
            ("_draft.md", "# draft\n"),
            ("notes.txt", "x"),
        ] {
            fs::write(dir.join(name), text).unwrap();
        }
        let files = markdown_in(&dir).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["01-intro.md", "2-deploy.md", "10-end.md"]);
        let src = read_all(&files, &Reads::new(&dir, &[])).unwrap();
        // the first front matter alone, each file on lines of its own (the second ends without one)
        assert_eq!(src.text, "---\ntitle: T\n---\n# Intro\n\n# Deploy\n\n# End\n");
        assert_eq!(src.origin(src.text.find("# End").unwrap()), Some((files[2].as_path(), 0)));
        assert_eq!(src.origin(src.text.find("# Deploy").unwrap()), Some((files[1].as_path(), 3)));
        // named on the command line, a README counts
        let named = inputs(&[dir.join("README.md"), dir.join("01-intro.md")]).unwrap();
        assert_eq!(named.len(), 2);
        assert_eq!(inputs(std::slice::from_ref(&dir)).unwrap(), files);
        assert!(inputs(&[dir.clone(), dir.join("README.md")]).unwrap_err().contains("a folder among files"));
        // a slides.md is the presentation
        fs::write(dir.join("slides.md"), "# S\n").unwrap();
        assert_eq!(markdown_in(&dir).unwrap(), [dir.join("slides.md")]);
        let empty = temp("merge-empty");
        assert!(markdown_in(&empty).unwrap_err().contains("no markdown file"));
    }

    /// The files glitu finds by itself in a folder are not the user's choice: a link among them
    /// that leads out of the folder is refused (a cloned folder would publish `~/.ssh/…`)
    #[cfg(unix)]
    #[test]
    fn a_folder_does_not_read_markdown_linked_from_outside() {
        use std::os::unix::fs::symlink;
        let base = temp("linked");
        fs::write(base.join("secret.txt"), "SECRET\n").unwrap();
        for link in ["leak.md", "slides.md"] {
            let dir = base.join(format!("talk-{link}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("01-intro.md"), "# Intro\n").unwrap();
            symlink("../secret.txt", dir.join(link)).unwrap();
            let err = markdown_in(&dir).unwrap_err();
            assert!(err.contains(link) && err.contains("symbolic link"), "{err}");
            // named by the user, it is their choice
            assert_eq!(inputs(&[dir.join(link)]).unwrap(), [dir.join(link)]);
        }
        // a link that stays in the folder is a file like another
        let dir = base.join("talk-inside");
        fs::create_dir_all(dir.join("parts")).unwrap();
        fs::write(dir.join("parts/a.txt"), "# A\n").unwrap();
        symlink("parts/a.txt", dir.join("a.md")).unwrap();
        assert_eq!(markdown_in(&dir).unwrap(), [dir.join("a.md")]);
    }

    #[test]
    fn natural_order() {
        let mut v = ["b10.md", "B2.md", "a.md", "b02x.md", "b1.md"];
        v.sort_by(|a, b| natural(a, b));
        assert_eq!(v, ["a.md", "b1.md", "B2.md", "b02x.md", "b10.md"]);
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
            let page =
                format!("<script id=\"{EMBED_ID}\">{}</script>", json!({ "files": [{ "path": bad, "text": "" }] }));
            let page = page.replace("\"files\":[", "\"files\":[{\"path\":\"ok.md\",\"text\":\"\"},");
            assert_eq!(extract(&page).unwrap().refused, [bad], "{bad}");
        }
        assert!(extract("<p>no copy</p>").is_err());
        assert!(extract(&format!("<script id=\"{EMBED_ID}\">{{\"files\":[]}}</script>")).is_err());
        assert_eq!(language("py"), "python");
        assert_eq!(language("toml"), "toml");
    }
}
