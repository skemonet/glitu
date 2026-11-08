//! Site assembly: index.html, engine (wasm), stylesheets, theme, resources.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::{Assets, Header, Theme};

use crate::confine::Reads;
use crate::engines::{Block, Engines};
use crate::settings::Settings;
use crate::source::{self, Source};

/// Version of the embedded engine: a presentation that asks for another one (`glitu:`) is refused.
/// The released binaries get it from the release tag (`GLITU_BUILD_VERSION`, set by the CI, see
/// tools/next-version.sh); other builds say the workspace's.
pub const VERSION: &str = match option_env!("GLITU_BUILD_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// Web engine compiled by build.rs (glitu-web → wasm → wasm-bindgen)
pub const ENGINE_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/pkg/glitu_web.js"));
pub const ENGINE_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pkg/glitu_web_bg.wasm"));
/// Engine styles: slide layout, steps, code, terminal
/// (the minified reveal.css is stored apart, so that searches in the code skip it)
pub const ENGINE_CSS: &str =
    concat!(include_str!("../assets/vendor/reveal.min.css"), include_str!("../assets/glitu.css"));
/// The template shared by the themes that ask for it (`"template": true` in glitu.yaml): header,
/// panel, stage, cover, chapters, progress; the theme only sets its variables
const TEMPLATE_CSS: &str = include_str!("../assets/template.css");
/// Formulas: their stylesheet and their font, added to the pages that have some
const MATH_CSS: &str = include_str!("../assets/math/math.css");
const MATH_FONT: &[u8] = include_bytes!("../assets/math/latinmodern-math.woff2");

/// Where the engine sits in the site, and how the page loads it
pub const ENGINE_JS_PATH: &str = "glitu/glitu_web.js";
pub const ENGINE_WASM_PATH: &str = "glitu/glitu_web_bg.wasm";
pub const ENGINE_BOOT: &str = "  import init, { start } from './glitu/glitu_web.js';\n  await init({ module_or_path: './glitu/glitu_web_bg.wasm' });\n";

/// The engine files, paths relative to `glitu/`; `math`: with those of the formulas
fn engine_files(math: bool) -> Vec<Output> {
    let mut files = vec![
        Output::new("glitu.css", ENGINE_CSS),
        Output::new("template.css", TEMPLATE_CSS),
        Output::new(ENGINE_JS_PATH.trim_start_matches("glitu/"), ENGINE_JS),
        Output::new(ENGINE_WASM_PATH.trim_start_matches("glitu/"), ENGINE_WASM),
    ];
    if math {
        files.push(Output::new("math.css", MATH_CSS));
        files.push(Output::new("latinmodern-math.woff2", MATH_FONT));
    }
    files
}

#[derive(Clone)]
pub struct Project {
    /// The first markdown file (slides.md), the one whose front matter counts
    pub slides: PathBuf,
    /// The files merged after it, if any
    pub more: Vec<PathBuf>,
    /// The folder the files were found in (`source::markdown_in`): looked at again on each read,
    /// so that `glitu server` takes a file added or removed
    pub dir: Option<PathBuf>,
    /// Root of the project (the markdown's folder): themes/, casts/, images
    pub root: PathBuf,
    /// What the user's machine decides: folders allowed, diagram tools (see `settings`)
    pub settings: Settings,
}

impl Project {
    pub fn new(slides: &Path) -> Self {
        let slides = slides.to_path_buf();
        let root = slides.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
        Project { slides, more: Vec::new(), dir: None, root, settings: Settings::default() }
    }

    /// The presentation given on the command line (see `source::inputs`)
    pub fn open(args: &[PathBuf], settings: Settings) -> Result<Self, String> {
        let files = source::inputs(args)?;
        let mut project = Project::new(&files[0]);
        project.settings = settings;
        project.more = files[1..].to_vec();
        project.dir = match args {
            [] => Some(PathBuf::from(".")),
            [one] if one.is_dir() => Some(one.clone()),
            _ => None,
        };
        Ok(project)
    }

    /// The markdown files, in order (those of the folder now, if it is one)
    pub fn files(&self) -> Vec<PathBuf> {
        match self.dir.as_deref().map(source::markdown_in) {
            Some(Ok(files)) => files,
            _ => std::iter::once(self.slides.clone()).chain(self.more.iter().cloned()).collect(),
        }
    }

    /// The presentation's files, read and merged, inclusions expanded (see `source`)
    pub fn read(&self) -> Result<Source, String> {
        source::read_all(&self.files(), &self.reads())
    }

    /// Where the presentation may read (SEC-16): its folder, the folders of the files named on the
    /// command line, those the user allowed
    pub fn reads(&self) -> Reads {
        let mut allowed = self.settings.allow_read.clone();
        let parent =
            |f: &PathBuf| f.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
        allowed.extend(self.more.iter().map(parent));
        Reads::new(&self.root, &allowed)
    }

    /// The presentation's markdown
    pub fn source(&self) -> Result<String, String> {
        Ok(self.read()?.text)
    }

    /// What is worth saying before building: the files merged, a front matter that does not count
    pub fn notes(&self) -> Vec<String> {
        let files = self.files();
        let mut notes = Vec::new();
        if files.len() > 1 {
            let names: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
            notes.push(format!("Merged {}", names.join(", ")));
        }
        for f in &files[1.min(files.len())..] {
            let text = std::fs::read_to_string(f).unwrap_or_default();
            if !glitu_core::markdown::front_matter(&text).0.is_empty() {
                notes.push(format!(
                    "warning: {}: front matter ignored; only the first file's counts ({})",
                    f.display(),
                    files[0].display()
                ));
            }
        }
        notes
    }

    /// The presentation, for messages: its file, or its folder and its number of files
    pub fn name(&self) -> String {
        match (self.files().len(), &self.dir) {
            (1, _) | (_, None) => self.slides.display().to_string(),
            (n, Some(dir)) if dir == Path::new(".") => format!("the current folder ({n} files)"),
            (n, Some(dir)) => format!("{} ({n} files)", dir.display()),
        }
    }

    /// `themes/<name>` next to the markdown, otherwise in the first parent folder that has one,
    /// otherwise the theme embedded in the binary (see `themes`)
    pub fn theme_dir(&self, name: &str) -> Result<PathBuf, String> {
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(format!("theme `{name}`: a theme's name, or a path starting with ./"));
        }
        let start = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        if let Some(dir) = start.ancestors().map(|d| d.join("themes").join(name)).find(|p| p.is_dir()) {
            return Ok(dir);
        }
        Ok(crate::themes::dir(name)?.unwrap_or_else(|| self.root.join("themes").join(name)))
    }

    /// The theme the presentation asks for, found on disk at the wanted version
    pub fn theme(&self, theme: &Theme) -> Result<LocalTheme, String> {
        let dir = match theme {
            Theme::Official { name, .. } => self.theme_dir(name)?,
            Theme::At(at) if at.contains("://") => {
                return Err(format!("theme {at}: only local themes are supported; copy it into themes/"));
            }
            Theme::At(at) => self.reads().open(&self.root, at).map_err(|e| format!("theme: {e}"))?,
        };
        LocalTheme::open(dir, theme)
    }

    /// A file of the presentation next to its markdown (its skemo.css, its style.css), if it has
    /// one; it is then copied into the page with the markdown
    fn deck_file(&self, name: &str, source: &mut Source) -> Result<Option<String>, String> {
        let path = self.root.join(name);
        self.reads().check(&path, name)?;
        let Some(text) = read_if_exists(&path)? else { return Ok(None) };
        source.files.push(source::File { path, text: text.clone() });
        Ok(Some(text))
    }
}

/// A theme on disk: the one the page will load
pub struct LocalTheme {
    /// The name the engine knows
    pub name: String,
    /// Version of its `glitu.yaml`
    pub version: Option<String>,
    /// An official theme, published under `themes/<name>/<version>/`
    pub official: bool,
    pub dir: PathBuf,
    /// Its `glitu.yaml`
    pub config: serde_json::Value,
}

impl LocalTheme {
    /// The theme `theme` in this folder, at the wanted version
    pub fn open(dir: PathBuf, theme: &Theme) -> Result<LocalTheme, String> {
        if !dir.is_dir() {
            return Err(format!("theme not found: {}", dir.display()));
        }
        let config = theme_config(&dir)?;
        let version = config["version"].as_str().map(String::from);
        let name = config["name"].as_str().unwrap_or(theme.name()).to_string();
        let official = match theme {
            Theme::Official { version: Some(wanted), .. } if version.as_ref() != Some(wanted) => {
                return Err(format!(
                    "theme {name}:{wanted} requested, {} available ({})",
                    version.as_deref().unwrap_or("no version"),
                    dir.display()
                ));
            }
            Theme::Official { .. } => true,
            Theme::At(_) => false,
        };
        Ok(LocalTheme { name, version, official, dir, config })
    }

    /// Where it is published (`themes/<name>/<version>`); nowhere for a third-party theme or one
    /// without a version
    pub fn published(&self) -> Option<String> {
        Some(format!("themes/{}/{}", self.name, self.version.as_ref().filter(|_| self.official)?))
    }
}

/// Does the presentation ask for another engine than this one?
fn check_engine(wanted: Option<&str>) -> Result<(), String> {
    match wanted {
        Some(v) if v != VERSION => Err(format!("this presentation requires glitu {v} (this is glitu {VERSION})")),
        _ => Ok(()),
    }
}

/// The files the compiler asks for: diagrams rendered by their engines, terminal recordings
struct Files<'a> {
    project: &'a Project,
    engines: Engines,
    /// `glitu server`: a diagram that fails shows its error in its place, and the terminal says it
    dev: bool,
}

impl Assets for Files<'_> {
    fn diagram_can(&mut self, lang: &str, capability: &str) -> bool {
        self.engines.can(lang, capability)
    }

    fn diagram(
        &mut self,
        lang: &str,
        source: &str,
        attrs: &BTreeMap<String, String>,
    ) -> Option<Result<String, String>> {
        let rendered = self.engines.render(&Block { lang, source, attrs })?;
        Some(match rendered {
            Err(e) if self.dev => {
                eprintln!("error: {}: {e}", self.project.slides.display());
                Ok(format!("<pre class=\"diagram-error\">{}</pre>", glitu_core::html::esc(&e)))
            }
            r => r,
        })
    }

    fn cast(&mut self, src: &str) -> Result<String, String> {
        let p = self.project.reads().open(&self.project.root, src)?;
        fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
    }
}

/// A file of the site: relative path and content
pub struct Output {
    pub path: String,
    pub bytes: Vec<u8>,
}

impl Output {
    fn new(path: &str, bytes: impl AsRef<[u8]>) -> Output {
        Output { path: path.into(), bytes: bytes.as_ref().to_vec() }
    }
}

/// Builds the site in memory. `dev`: adds the automatic reload.
///
/// Two writing conventions are recognised automatically: glitu's own (`#` chapters, `##` slides,
/// `@n`) and reveal-md's (slides separated by `---`); default theme `crystal` and `black`
/// respectively.
pub fn site(project: &Project, dev: bool) -> Result<Vec<Output>, String> {
    let mut source = project.read()?;
    let header = glitu_core::header(&source.text);
    check_engine(header.glitu.as_deref())?;
    let LocalTheme { name: theme, dir: theme_dir, config, .. } = project.theme(&header.theme)?;
    // the presentation's skemo.css, copied into the page with the markdown (the engine reads it
    // with the theme's)
    project.deck_file("skemo.css", &mut source)?;
    let engines = Engines::new(&project.settings, &project.root, &header.meta, &theme_dir, &config);
    let glitu_core::Compiled { title, sections } =
        glitu_core::compile(&source.text, &header, &mut Files { project, engines, dev })?;

    let sections = crate::online::icons(&sections)?;
    let fonts = crate::online::fonts(&header.meta)?;
    let math = glitu_core::math::used(&sections);
    // The presentation's own adjustments: its style.css, after the theme's stylesheets
    let deck_style = project.deck_file("style.css", &mut source)?;
    let page = Page {
        deck_style: deck_style.is_some(),
        lang: &header.lang,
        math,
        template: config["template"].as_bool().unwrap_or(false),
        fonts: !fonts.is_empty(),
        meta: head_meta(&header.meta, &title),
        title: &title,
        styles: config["styles"].as_array().map(|a| a.iter().filter_map(|s| s.as_str()).collect()).unwrap_or_default(),
        sections: &sections,
        options: engine_options(&config, &theme, deck_logo(project, &header.meta)?.as_deref()),
        source: embedded_source(&header, &source),
        dev,
    };
    let mut out = vec![Output::new("index.html", page.html())];
    out.extend(engine_files(math).into_iter().map(|f| Output { path: format!("glitu/{}", f.path), ..f }));
    out.extend(fonts);
    if let Some(text) = deck_style {
        out.push(Output::new("style.css", text));
    }
    copy_dir(&theme_dir, "theme", &BUILD_ONLY, &Reads::only(&theme_dir), &mut out)?;
    for (dir, prefix) in [("img", "img"), ("public", "")] {
        let d = project.root.join(dir);
        if d.is_dir() {
            copy_dir(&d, prefix, &BUILD_ONLY, &project.reads(), &mut out)?;
        }
    }
    Ok(out)
}

/// The copy of the markdown the page keeps (see `source::embed`); `source: false` in the front
/// matter: none
fn embedded_source(header: &Header, source: &Source) -> String {
    if header.meta.get("source").is_some_and(|v| v == "false") { String::new() } else { source::embed(source) }
}

/// A text file, if it exists
fn read_if_exists(path: &Path) -> Result<Option<String>, String> {
    path.is_file().then(|| fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))).transpose()
}

/// The file of a theme's settings
pub const THEME_SETTINGS: &str = "glitu.yaml";

/// The theme's settings (`glitu.yaml`; a former theme's `glitu.yaml` is still read); no file: the
/// engine's default settings
fn theme_config(theme_dir: &Path) -> Result<serde_json::Value, String> {
    let at = |name: &str| theme_dir.join(name);
    let value = if let Ok(text) = fs::read_to_string(at(THEME_SETTINGS)) {
        crate::settings::parse(&text).map_err(|e| format!("{}: {e}", at(THEME_SETTINGS).display()))?
    } else if let Ok(text) = fs::read_to_string(at("glitu.json")) {
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", at("glitu.json").display()))?
    } else {
        serde_json::Value::Null
    };
    match value {
        serde_json::Value::Null => Ok(serde_json::json!({})),
        v @ serde_json::Value::Object(_) => Ok(v),
        _ => Err(format!("{}: keys expected", at(THEME_SETTINGS).display())),
    }
}

/// The engine options: the theme's settings, its name and the presentation's logo (see
/// `deck_logo`: a site path or an address); no `logo:` in the front matter, no logo
fn engine_options(config: &serde_json::Value, theme: &str, logo: Option<&str>) -> serde_json::Value {
    let mut options = config.clone();
    if let Some(o) = options.as_object_mut() {
        for k in ["styles", "version", "name", "logo"] {
            o.remove(k);
        }
        o.insert("theme".into(), theme.into());
        if let Some(logo) = logo.filter(|l| !l.is_empty()) {
            o.insert("logo".into(), logo.into());
        }
    }
    options
}

/// `logo:` in the front matter: an image of the presentation, in img/ or public/ (the folders
/// copied into the site), at its path in the site, or an http(s) address; `logo: none` or no
/// `logo:`, no logo
fn deck_logo(project: &Project, meta: &BTreeMap<String, String>) -> Result<Option<String>, String> {
    let Some(logo) = meta.get("logo").map(|l| l.trim().trim_matches(['"', '\''])) else { return Ok(None) };
    if logo == "none" || logo.is_empty() {
        return Ok(None);
    }
    if logo.starts_with("https://") || logo.starts_with("http://") {
        return Ok(Some(logo.to_string()));
    }
    project.reads().open(&project.root, logo).map_err(|e| format!("logo: {e}"))?;
    if !project.root.join(logo).is_file() {
        return Err(format!("logo: {} not found", project.root.join(logo).display()));
    }
    match logo.strip_prefix("public/") {
        Some(rest) => Ok(Some(rest.to_string())),
        None if logo.starts_with("img/") => Ok(Some(logo.to_string())),
        None => Err(format!("logo: `{logo}` must be in img/ or public/, which are copied into the site")),
    }
}

/// `glitu server`: reloads the page when the presentation changes, and keeps the open windows
/// together (see `serve`); the previews of the speaker view follow their own
const DEV_SCRIPT: &str = "
<script>
(() => {
  const follow = /[?&](remote|follow)/.test(location.search);
  const es = new EventSource('/__glitu/reload' + (follow ? '?follow' : ''));
  es.onmessage = () => location.reload();
  if (/[?&]receiver/.test(location.search)) return;
  // Not Math.random: the animated backdrop draws its sequence from it, which the oracle replays identically
  const id = crypto.getRandomValues(new Uint32Array(2)).join('-');
  let last = location.hash;
  es.addEventListener('position', (e) => {
    const p = JSON.parse(e.data);
    if (p.id !== id && p.hash !== location.hash) { last = p.hash; location.hash = p.hash; }
  });
  addEventListener('glitu:position', () => {
    if (location.hash === last) return;
    last = location.hash;
    fetch('/__glitu/position', { method: 'POST', body: JSON.stringify({ id, hash: last }) });
  });
  // Rehearsal: the durations kept by the speaker view, written in the markdown; the new timings
  // come back and change in place (no reload: it would reopen the speaker view)
  addEventListener('glitu:chrono', async () => {
    const r = await fetch('/__glitu/chrono', { method: 'POST', body: window.glituChrono });
    const timings = await r.json().catch(() => ({}));
    if (timings.error) { alert('Rehearsal not saved: ' + timings.error); return; }
    for (const [rank, [secs, runs]] of Object.entries(timings)) {
      const s = document.querySelector('section[data-slide=\"' + Number(rank) + '\"]');
      if (s) { s.dataset.duration = secs; s.dataset.runs = runs; }
    }
    dispatchEvent(new Event('glitu:timings'));
  });
})();
</script>";

/// The presentation's page
/// The description (`description:`, otherwise `subtitle:`) and the author of the front matter, as
/// `<meta>` tags, and the Open Graph tags of a link preview; `title` is already escaped
fn head_meta(meta: &std::collections::BTreeMap<String, String>, title: &str) -> String {
    let esc = glitu_core::html::esc;
    let description = meta.get("description").or_else(|| meta.get("subtitle")).filter(|d| !d.is_empty());
    let mut tags = format!(
        "\n  <meta property=\"og:type\" content=\"website\">\n  <meta property=\"og:title\" content=\"{title}\">"
    );
    if let Some(d) = description {
        let d = esc(d);
        tags = format!(
            "\n  <meta name=\"description\" content=\"{d}\">{tags}\n  <meta property=\"og:description\" content=\"{d}\">"
        );
    }
    if let Some(a) = meta.get("author").filter(|a| !a.is_empty()) {
        tags += &format!("\n  <meta name=\"author\" content=\"{}\">", esc(a));
    }
    tags
}

struct Page<'a> {
    lang: &'a str,
    title: &'a str,
    /// `<meta>` tags for search engines and link previews (see `head_meta`)
    meta: String,
    /// Formulas: their stylesheet
    math: bool,
    /// The theme relies on the shared template: its stylesheet, before the theme's
    template: bool,
    /// The presentation has its own style.css: loaded last
    deck_style: bool,
    /// Fonts chosen in the front matter: their stylesheet, after the theme's
    fonts: bool,
    /// The theme's stylesheets, in the order of `glitu.yaml`
    styles: Vec<&'a str>,
    sections: &'a str,
    options: serde_json::Value,
    /// Copy of the markdown (see `source::embed`), or nothing
    source: String,
    /// Automatic reload (`glitu server`)
    dev: bool,
}

impl Page<'_> {
    fn stylesheets(&self) -> String {
        let link = |href: &str| format!("\n  <link rel=\"stylesheet\" href=\"{href}\">");
        let mut styles = String::new();
        if self.math {
            styles += &link("glitu/math.css");
        }
        if self.template {
            styles += &link("glitu/template.css");
        }
        for s in &self.styles {
            styles += &link(&format!("theme/{s}"));
        }
        if self.fonts {
            styles += &link(crate::online::FONTS_CSS);
        }
        if self.deck_style {
            styles += &link("style.css");
        }
        styles
    }

    fn html(&self) -> String {
        let Page { lang, title, meta, sections, options, source, .. } = self;
        let styles = self.stylesheets();
        let reload = if self.dev { DEV_SCRIPT } else { "" };
        format!(
            r#"<!doctype html>
<html lang="{lang}" class="reveal-full-page">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>{meta}
  <link rel="stylesheet" href="glitu/glitu.css">{styles}
</head>
<body class="reveal-viewport">
<div class="reveal">
<div class="slides">
{sections}
</div>
</div>{source}
<script type="module">
{ENGINE_BOOT}  start({options});
</script>{reload}
</body>
</html>
"#
        )
    }
}

/// Where the engine of this version is published
pub fn engine_published() -> String {
    format!("glitu/{VERSION}")
}

/// A folder published as a whole: one version of the engine or of a theme
pub struct Published {
    /// `glitu/<version>` or `themes/<name>/<version>`
    pub dir: String,
    /// Paths relative to this folder
    pub files: Vec<Output>,
}

/// What is published for the light pages (`--web`), each one at its version: `glitu/<version>/`
/// (the engine) and `themes/<name>/<version>/` (the version given by its `glitu.yaml`)
pub fn runtime(themes: &Path) -> Result<Vec<Published>, String> {
    let mut out = vec![Published { dir: engine_published(), files: engine_files(true) }];
    let mut names: Vec<_> = fs::read_dir(themes)
        .map_err(|e| format!("{}: {e}", themes.display()))?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    names.sort_by_key(fs::DirEntry::file_name);
    for theme in names {
        let name = theme.file_name().to_string_lossy().to_string();
        let at = LocalTheme::open(theme.path(), &Theme::Official { name: name.clone(), version: None })?;
        let dir =
            at.published().ok_or_else(|| format!("theme {name}: no `version` in {}/glitu.yaml", at.dir.display()))?;
        let mut files = Vec::new();
        copy_dir(&theme.path(), "", &BUILD_ONLY, &Reads::only(&theme.path()), &mut files)?;
        out.push(Published { dir, files });
    }
    Ok(out)
}

/// The `themes/` folder of the first folder, going up from `start`, that has one
pub fn themes_root(start: &Path) -> Option<PathBuf> {
    let start = start.canonicalize().ok()?;
    start.ancestors().map(|d| d.join("themes")).find(|p| p.is_dir())
}

/// Theme files only used by the build: pages do not need them
const BUILD_ONLY: [&str; 3] = ["skemo.css", THEME_SETTINGS, "glitu.json"];

/// The files of `dir`, under `prefix`, but `skip`; a symbolic link that leads outside `within` is
/// an error (SEC-16: a folder copied into the site cannot bring in a file of the machine)
fn copy_dir(dir: &Path, prefix: &str, skip: &[&str], within: &Reads, out: &mut Vec<Output>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let path = entry.path();
        within.check(&path, &path.display().to_string())?;
        if path.is_dir() {
            copy_dir(&path, &rel, skip, within, out)?;
        } else if !skip.contains(&name.as_str()) {
            out.push(Output { path: rel, bytes: fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))? });
        }
    }
    Ok(())
}

/// Writes a file, creating its folder if needed
pub fn write_file(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = dest.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))
}

/// Writes the site into `dest`, which is replaced
pub fn write(out: &[Output], dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    for f in out {
        write_file(&dest.join(&f.path), &f.bytes)?;
    }
    Ok(())
}

/// The CSS variables of a theme, as its pages get them: the template's (a template theme), then
/// its own stylesheets', each name once with its last value, in order of first appearance
pub fn theme_variables(theme: &LocalTheme) -> Result<Vec<(String, String)>, String> {
    let mut css = String::new();
    if theme.config["template"].as_bool() == Some(true) {
        css += TEMPLATE_CSS;
    }
    for style in theme.config["styles"].as_array().into_iter().flatten().filter_map(|s| s.as_str()) {
        let path = Reads::only(&theme.dir).open(&theme.dir, style)?;
        css += &fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mut vars: Vec<(String, String)> = Vec::new();
    for (name, value) in root_variables(&css) {
        match vars.iter_mut().find(|(n, _)| *n == name) {
            Some(v) => v.1 = value,
            None => vars.push((name, value)),
        }
    }
    Ok(vars)
}

/// `--name: value` declarations of the `:root` blocks, comments removed, values on one line
fn root_variables(css: &str) -> Vec<(String, String)> {
    let mut plain = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        plain += &rest[..start];
        rest = rest[start..].find("*/").map_or("", |end| &rest[start + end + 2..]);
    }
    plain += rest;
    let mut out = Vec::new();
    let mut blocks = plain.as_str();
    while let Some(at) = blocks.find(":root") {
        let after = &blocks[at..];
        let (Some(open), Some(close)) = (after.find('{'), after.find('}')) else { break };
        for decl in after[open + 1..close].split(';') {
            if let Some((name, value)) = decl.trim().split_once(':').filter(|(n, _)| n.trim().starts_with("--")) {
                out.push((name.trim().to_string(), value.split_whitespace().collect::<Vec<_>>().join(" ")));
            }
        }
        blocks = &after[close..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_describes_itself() {
        let meta = [("subtitle", "A & B"), ("author", "Ada")].map(|(k, v)| (k.to_string(), v.to_string())).into();
        let tags = head_meta(&meta, "T");
        assert!(tags.contains("<meta name=\"description\" content=\"A &amp; B\">"), "{tags}");
        assert!(tags.contains("<meta property=\"og:description\" content=\"A &amp; B\">"));
        assert!(tags.contains("<meta name=\"author\" content=\"Ada\">"));
        let meta = [("subtitle", "S"), ("description", "D")].map(|(k, v)| (k.to_string(), v.to_string())).into();
        assert!(head_meta(&meta, "T").contains("content=\"D\""));
    }

    /// A throw-away folder, removed when dropped
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!("glitu-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn write(&self, path: &str, text: &str) -> &TempDir {
            write_file(&self.0.join(path), text.as_bytes()).unwrap();
            self
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_site_replaces_its_folder() {
        let dir = TempDir::new("write");
        dir.write("site/old.html", "x").write("themes/demo/theme.css", "x").write("talk/slides.md", "x");
        let out = [
            Output { path: "index.html".into(), bytes: b"<p>".to_vec() },
            Output { path: "a/b.css".into(), bytes: b"x".to_vec() },
        ];
        write(&out, &dir.0.join("site")).unwrap();
        assert!(!dir.0.join("site/old.html").exists());
        assert_eq!(fs::read(dir.0.join("site/a/b.css")).unwrap(), b"x");
        let themes = dir.0.canonicalize().unwrap().join("themes");
        assert_eq!(themes_root(&dir.0.join("talk")), Some(themes));
        assert_eq!(themes_root(&dir.0.join("absent")), None);
    }

    #[test]
    fn the_script_of_glitu_serve_has_no_broken_quotes() {
        // a quote doubled by an escape lost in the Rust string: '' or "" inside an expression
        for line in DEV_SCRIPT.lines() {
            assert!(!line.contains("=''") && !line.contains("'' +") && !line.contains("+ ''"), "{line}");
        }
    }

    #[test]
    fn variables_of_the_root_blocks() {
        let css = ":root {\n  --a: 1px; /* note */\n  --b:\n    linear-gradient(red,\n blue);\n}\n.x { --c: 2; }\n:root { --a: 3px; color: red }";
        let vars = root_variables(css);
        assert_eq!(
            vars,
            [("--a", "1px"), ("--b", "linear-gradient(red, blue)"), ("--a", "3px")]
                .map(|(n, v)| (n.to_string(), v.to_string()))
        );
    }

    #[test]
    fn the_logo_comes_from_the_presentation_only() {
        let config = serde_json::json!({ "logo": "logo.svg", "version": "1.0.0" });
        assert!(engine_options(&config, "t", None).get("logo").is_none());
        assert_eq!(engine_options(&config, "t", Some("img/max.png"))["logo"], "img/max.png");

        let dir = TempDir::new("logo");
        dir.write("img/max.png", "x").write("max.png", "x").write("public/p.png", "x");
        let project = Project::new(&dir.0.join("slides.md"));
        let meta = |v: &str| [("logo".to_string(), v.to_string())].into_iter().collect();
        assert_eq!(deck_logo(&project, &meta("img/max.png")).unwrap().as_deref(), Some("img/max.png"));
        assert_eq!(deck_logo(&project, &meta("public/p.png")).unwrap().as_deref(), Some("p.png"));
        assert_eq!(deck_logo(&project, &meta("none")).unwrap(), None);
        assert_eq!(deck_logo(&project, &meta("https://x.net/l.svg")).unwrap().as_deref(), Some("https://x.net/l.svg"));
        assert!(deck_logo(&project, &meta("max.png")).unwrap_err().contains("img/ or public/"));
        assert!(deck_logo(&project, &meta("img/absent.png")).unwrap_err().contains("not found"));
        assert_eq!(deck_logo(&project, &Default::default()).unwrap(), None);
    }

    #[test]
    fn the_runtime_keeps_only_what_pages_load() {
        let dir = TempDir::new("runtime");
        for f in ["demo/skemo.css", "demo/theme.css", "demo/fonts/a.woff2"] {
            dir.write(f, "x");
        }
        dir.write("demo/glitu.yaml", "version: \"1.2.0\"\n");
        let paths: Vec<String> = runtime(&dir.0)
            .unwrap()
            .into_iter()
            .flat_map(|p| p.files.into_iter().map(move |f| format!("{}/{}", p.dir, f.path)))
            .collect();
        let engine = engine_published();
        for p in [
            format!("{engine}/glitu_web_bg.wasm"),
            format!("{engine}/glitu.css"),
            "themes/demo/1.2.0/theme.css".into(),
            "themes/demo/1.2.0/fonts/a.woff2".into(),
        ] {
            assert!(paths.contains(&p), "{p} missing from the runtime");
        }
        for p in ["themes/demo/1.2.0/glitu.yaml", "themes/demo/1.2.0/skemo.css"] {
            assert!(!paths.iter().any(|x| x == p), "{p} is only used by the build");
        }
        dir.write("bare/theme.css", "x");
        assert!(runtime(&dir.0).err().unwrap().contains("no `version`"));
    }

    #[test]
    fn a_site_holds_the_page_the_engine_the_theme_and_the_resources() {
        let dir = TempDir::new("site");
        dir.write("slides.md", "---\ntitle: Demo\nlang: fr\nlogo: img/a.png\n---\n\n## One\n\n- a @1\n\n$x$\n")
            .write("themes/crystal/glitu.yaml", "version: \"1.0.0\"\nstyles: [theme.css]\ntemplate: true\n")
            .write("themes/crystal/theme.css", "body {}")
            .write("style.css", ".x {}")
            .write("img/a.png", "P")
            .write("public/robots.txt", "R");
        let site = site(&Project::new(&dir.0.join("slides.md")), true).unwrap();
        let paths: Vec<&str> = site.iter().map(|f| f.path.as_str()).collect();
        for p in [
            "index.html",
            "glitu/glitu.css",
            "glitu/math.css",
            "theme/theme.css",
            "img/a.png",
            "robots.txt",
            "style.css",
        ] {
            assert!(paths.contains(&p), "{p} missing: {paths:?}");
        }
        assert!(!paths.contains(&"theme/glitu.yaml"));
        let index = String::from_utf8_lossy(&site[0].bytes);
        assert!(index.contains("<html lang=\"fr\"") && index.contains("<title>Demo</title>"));
        assert!(
            index.contains("<meta property=\"og:title\" content=\"Demo\">") && !index.contains("name=\"description\"")
        );
        assert!(index.contains("href=\"glitu/template.css\">\n  <link rel=\"stylesheet\" href=\"theme/theme.css\">"));
        assert!(index.contains("<link rel=\"stylesheet\" href=\"style.css\">"));
        assert!(index.contains("\"logo\":\"img/a.png\"") && index.contains("\"theme\":\"crystal\""));
        assert!(index.contains("id=\"glitu-source\"") && index.contains("EventSource"));
    }

    #[test]
    fn versions_must_match() {
        assert!(check_engine(None).is_ok() && check_engine(Some(VERSION)).is_ok());
        assert!(check_engine(Some("0.0.0-old")).unwrap_err().contains("requires glitu 0.0.0-old"));
        let dir = TempDir::new("versions");
        dir.write("themes/t/glitu.json", r#"{ "version": "2.0.0" }"#); // a former theme: still read
        let project = Project::new(&dir.0.join("slides.md"));
        assert!(project.theme(&Theme::parse("t:1.0.0")).err().unwrap().contains("t:1.0.0 requested, 2.0.0 available"));
        assert_eq!(project.theme(&Theme::parse("t")).unwrap().published().as_deref(), Some("themes/t/2.0.0"));
        assert_eq!(project.theme(&Theme::parse("./themes/t")).unwrap().published(), None);
        assert!(project.theme(&Theme::parse("https://x/t/")).err().unwrap().contains("only local themes"));
        assert!(project.theme(&Theme::parse("absent")).err().unwrap().contains("theme not found"));
    }
}
