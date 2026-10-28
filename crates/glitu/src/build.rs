//! Site assembly: index.html, engine (wasm), stylesheets, theme, resources.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::skemo::Skemo;
use glitu_core::{Assets, Header, Theme};

use crate::source::{self, Source};

/// Version of the embedded engine: a presentation that asks for another one (`glitu:`) is refused
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Web engine compiled by build.rs (glitu-web → wasm → wasm-bindgen)
pub const ENGINE_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/pkg/glitu_web.js"));
pub const ENGINE_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pkg/glitu_web_bg.wasm"));
/// Engine styles: slide layout, steps, code, terminal
/// (the minified reveal.css is stored apart, so that searches in the code skip it)
pub const ENGINE_CSS: &str =
    concat!(include_str!("../assets/vendor/reveal.min.css"), include_str!("../assets/glitu.css"));
/// The template shared by the themes that ask for it (`"template": true` in glitu.json): header,
/// panel, stage, cover, chapters, progress; the theme only sets its variables
const TEMPLATE_CSS: &str = include_str!("../assets/template.css");
/// Formulas: their stylesheet and their font, added to the pages that have some
const MATH_CSS: &str = include_str!("../assets/math/math.css");
const MATH_FONT: &[u8] = include_bytes!("../assets/math/latinmodern-math.woff2");

/// Where the engine sits in the site, and how the page loads it
pub const ENGINE_JS_PATH: &str = "glitu/glitu_web.js";
pub const ENGINE_WASM_PATH: &str = "glitu/glitu_web_bg.wasm";
pub const ENGINE_BOOT: &str =
    "  import init, { start } from './glitu/glitu_web.js';\n  await init({ module_or_path: './glitu/glitu_web_bg.wasm' });\n";

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

pub struct Project {
    /// slides.md
    pub slides: PathBuf,
    /// Root of the project (the markdown's folder): themes/, casts/, images
    pub root: PathBuf,
}

impl Project {
    pub fn new(slides: &Path) -> Self {
        let slides = slides.to_path_buf();
        let root = slides.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
        Project { slides, root }
    }

    /// The presentation's markdown, inclusions expanded (see `source`)
    pub fn source(&self) -> Result<String, String> {
        Ok(source::read(&self.slides)?.text)
    }

    /// `themes/<name>` next to the markdown, otherwise in the first parent folder that has one
    pub fn theme_dir(&self, name: &str) -> PathBuf {
        let start = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        start
            .ancestors()
            .map(|d| d.join("themes").join(name))
            .find(|p| p.is_dir())
            .unwrap_or_else(|| self.root.join("themes").join(name))
    }

    /// The theme the presentation asks for, found on disk at the wanted version
    pub fn theme(&self, theme: &Theme) -> Result<LocalTheme, String> {
        let dir = match theme {
            Theme::Official { name, .. } => self.theme_dir(name),
            Theme::At(at) if at.contains("://") => {
                return Err(format!("theme {at}: only themes on disk are supported, copy it into themes/"));
            }
            Theme::At(at) => self.root.join(at),
        };
        LocalTheme::open(dir, theme)
    }

    /// A file of the presentation next to its markdown (its skemo.css, its style.css), if it has
    /// one; it is then copied into the page with the markdown
    fn deck_file(&self, name: &str, source: &mut Source) -> Result<Option<String>, String> {
        let path = self.root.join(name);
        let Some(text) = read_if_exists(&path)? else { return Ok(None) };
        source.files.push(source::File { path, text: text.clone() });
        Ok(Some(text))
    }
}

/// A theme on disk: the one the page will load
pub struct LocalTheme {
    /// The name the engine knows
    pub name: String,
    /// Version of its `glitu.json`
    pub version: Option<String>,
    /// An official theme, published under `themes/<name>/<version>/`
    pub official: bool,
    pub dir: PathBuf,
    /// Its `glitu.json`
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
        Some(v) if v != VERSION => Err(format!("this presentation asks for glitu {v}; this one is glitu {VERSION}")),
        _ => Ok(()),
    }
}

/// The files the compiler asks for: diagrams rendered by skemo, terminal recordings
struct Files<'a> {
    project: &'a Project,
    skemo: Skemo,
}

impl Assets for Files<'_> {
    fn skemo(&mut self, source: &str, font_scale: Option<f64>) -> Result<String, String> {
        let root = self.project.root.clone();
        let mut load = |name: &str| {
            let p = root.join(name);
            fs::read(&p).map_err(|e| format!("skemo: image « {name} » ({}): {e}", p.display()))
        };
        self.skemo.render(source, font_scale, &mut load)
    }

    fn cast(&mut self, src: &str) -> Result<String, String> {
        let p = self.project.root.join(src);
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
/// `@n`) and reveal-md's (slides separated by `---`); default theme « crystal » and « black »
/// respectively.
pub fn site(project: &Project, dev: bool) -> Result<Vec<Output>, String> {
    let mut source = source::read(&project.slides)?;
    let header = glitu_core::header(&source.text);
    check_engine(header.glitu.as_deref())?;
    let LocalTheme { name: theme, dir: theme_dir, config, .. } = project.theme(&header.theme)?;
    // Diagram types: the theme's, then the presentation's own skemo.css
    let deck_css = project.deck_file("skemo.css", &mut source)?;
    let css = [read_if_exists(&theme_dir.join("skemo.css"))?, deck_css].into_iter().flatten().reduce(|a, b| format!("{a}\n{b}"));
    let skemo = Skemo { css, font_scale: config["skemo"]["fontScale"].as_f64() };
    let glitu_core::Compiled { title, sections } =
        glitu_core::compile(&source.text, &header, &mut Files { project, skemo })?;

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
    copy_dir(&theme_dir, "theme", &BUILD_ONLY, &mut out)?;
    for (dir, prefix) in [("img", "img"), ("public", "")] {
        let d = project.root.join(dir);
        if d.is_dir() {
            copy_dir(&d, prefix, &BUILD_ONLY, &mut out)?;
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

/// The theme's settings (`glitu.json`); no file: the engine's default settings
fn theme_config(theme_dir: &Path) -> Result<serde_json::Value, String> {
    match fs::read_to_string(theme_dir.join("glitu.json")) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}/glitu.json: {e}", theme_dir.display())),
        Err(_) => Ok(serde_json::json!({})),
    }
}

/// The engine options: the theme's settings, its name and its logo; `logo`: the presentation's,
/// already as a site path (see `deck_logo`), which replaces the theme's
fn engine_options(config: &serde_json::Value, theme: &str, logo: Option<&str>) -> serde_json::Value {
    let mut options = config.clone();
    if let Some(o) = options.as_object_mut() {
        for k in ["styles", "version", "name", "logo"] {
            o.remove(k);
        }
        o.insert("theme".into(), theme.into());
        let logo = logo.map(str::to_string).or_else(|| config["logo"].as_str().map(|l| format!("theme/{l}")));
        if let Some(logo) = logo.filter(|l| !l.is_empty()) {
            o.insert("logo".into(), logo.into());
        }
    }
    options
}

/// `logo:` in the front matter: an image of the presentation, in img/ or public/ (the folders
/// copied into the site), at its path in the site; `logo: none`: no logo (empty string)
fn deck_logo(project: &Project, meta: &BTreeMap<String, String>) -> Result<Option<String>, String> {
    let Some(logo) = meta.get("logo").map(|l| l.trim().trim_matches(['"', '\''])) else { return Ok(None) };
    if logo == "none" {
        return Ok(Some(String::new()));
    }
    if !project.root.join(logo).is_file() {
        return Err(format!("logo: {} not found", project.root.join(logo).display()));
    }
    match logo.strip_prefix("public/") {
        Some(rest) => Ok(Some(rest.to_string())),
        None if logo.starts_with("img/") => Ok(Some(logo.to_string())),
        None => Err(format!("logo: « {logo} » must be in img/ or public/, which are copied into the site")),
    }
}

/// `glitu serve`: reloads the page when the presentation changes, and keeps the open windows
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
  // Rehearsal: the durations timed by the speaker view, to be written in the markdown
  addEventListener('glitu:chrono', () => fetch('/__glitu/chrono', { method: 'POST', body: window.glituChrono }));
})();
</script>";

/// The presentation's page
struct Page<'a> {
    lang: &'a str,
    title: &'a str,
    /// Formulas: their stylesheet
    math: bool,
    /// The theme relies on the shared template: its stylesheet, before the theme's
    template: bool,
    /// The presentation has its own style.css: loaded last
    deck_style: bool,
    /// Fonts chosen in the front matter: their stylesheet, after the theme's
    fonts: bool,
    /// The theme's stylesheets, in the order of `glitu.json`
    styles: Vec<&'a str>,
    sections: &'a str,
    options: serde_json::Value,
    /// Copy of the markdown (see `source::embed`), or nothing
    source: String,
    /// Automatic reload (`glitu serve`)
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
        let Page { lang, title, sections, options, source, .. } = self;
        let styles = self.stylesheets();
        let reload = if self.dev { DEV_SCRIPT } else { "" };
        format!(
            r#"<!doctype html>
<html lang="{lang}" class="reveal-full-page">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
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
/// (the engine) and `themes/<name>/<version>/` (the version given by its `glitu.json`)
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
        let dir = at
            .published()
            .ok_or_else(|| format!("theme {name}: no \"version\" in {}/glitu.json", at.dir.display()))?;
        let mut files = Vec::new();
        copy_dir(&theme.path(), "", &BUILD_ONLY, &mut files)?;
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
const BUILD_ONLY: [&str; 2] = ["skemo.css", "glitu.json"];

fn copy_dir(dir: &Path, prefix: &str, skip: &[&str], out: &mut Vec<Output>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &rel, skip, out)?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_presentation_logo_replaces_the_theme_one() {
        let config = serde_json::json!({ "logo": "logo.svg", "version": "1.0.0" });
        assert_eq!(engine_options(&config, "t", None)["logo"], "theme/logo.svg");
        assert_eq!(engine_options(&config, "t", Some("img/max.png"))["logo"], "img/max.png");
        assert!(engine_options(&config, "t", Some("")).get("logo").is_none());

        let dir = TempDir::new("logo");
        dir.write("img/max.png", "x").write("max.png", "x").write("public/p.png", "x");
        let project = Project { slides: dir.0.join("slides.md"), root: dir.0.clone() };
        let meta = |v: &str| [("logo".to_string(), v.to_string())].into_iter().collect();
        assert_eq!(deck_logo(&project, &meta("img/max.png")).unwrap().as_deref(), Some("img/max.png"));
        assert_eq!(deck_logo(&project, &meta("public/p.png")).unwrap().as_deref(), Some("p.png"));
        assert_eq!(deck_logo(&project, &meta("none")).unwrap().as_deref(), Some(""));
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
        dir.write("demo/glitu.json", r#"{ "version": "1.2.0" }"#);
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
        for p in ["themes/demo/1.2.0/glitu.json", "themes/demo/1.2.0/skemo.css"] {
            assert!(!paths.iter().any(|x| x == p), "{p} is only used by the build");
        }
        dir.write("bare/theme.css", "x");
        assert!(runtime(&dir.0).err().unwrap().contains("no \"version\""));
    }

    #[test]
    fn a_site_holds_the_page_the_engine_the_theme_and_the_resources() {
        let dir = TempDir::new("site");
        dir.write("slides.md", "---\ntitle: Demo\nlang: fr\n---\n\n## One\n\n- a @1\n\n$x$\n")
            .write("themes/crystal/glitu.json", r#"{ "version": "1.0.0", "styles": ["theme.css"], "template": true, "logo": "logo.svg" }"#)
            .write("themes/crystal/theme.css", "body {}")
            .write("themes/crystal/logo.svg", "<svg/>")
            .write("style.css", ".x {}")
            .write("img/a.png", "P")
            .write("public/robots.txt", "R");
        let site = site(&Project::new(&dir.0.join("slides.md")), true).unwrap();
        let paths: Vec<&str> = site.iter().map(|f| f.path.as_str()).collect();
        for p in ["index.html", "glitu/glitu.css", "glitu/math.css", "theme/theme.css", "img/a.png", "robots.txt", "style.css"] {
            assert!(paths.contains(&p), "{p} missing: {paths:?}");
        }
        assert!(!paths.contains(&"theme/glitu.json"));
        let index = String::from_utf8_lossy(&site[0].bytes);
        assert!(index.contains("<html lang=\"fr\"") && index.contains("<title>Demo</title>"));
        assert!(index.contains("href=\"glitu/template.css\">\n  <link rel=\"stylesheet\" href=\"theme/theme.css\">"));
        assert!(index.contains("<link rel=\"stylesheet\" href=\"style.css\">"));
        assert!(index.contains("\"logo\":\"theme/logo.svg\"") && index.contains("\"theme\":\"crystal\""));
        assert!(index.contains("id=\"glitu-source\"") && index.contains("EventSource"));
    }

    #[test]
    fn versions_must_match() {
        assert!(check_engine(None).is_ok() && check_engine(Some(VERSION)).is_ok());
        assert!(check_engine(Some("0.0.0-old")).unwrap_err().contains("asks for glitu 0.0.0-old"));
        let dir = TempDir::new("versions");
        dir.write("themes/t/glitu.json", r#"{ "version": "2.0.0" }"#);
        let project = Project { slides: dir.0.join("slides.md"), root: dir.0.clone() };
        assert!(project.theme(&Theme::parse("t:1.0.0")).err().unwrap().contains("t:1.0.0 requested, 2.0.0 available"));
        assert_eq!(project.theme(&Theme::parse("t")).unwrap().published().as_deref(), Some("themes/t/2.0.0"));
        assert_eq!(project.theme(&Theme::parse("./themes/t")).unwrap().published(), None);
        assert!(project.theme(&Theme::parse("https://x/t/")).err().unwrap().contains("only themes on disk"));
        assert!(project.theme(&Theme::parse("absent")).err().unwrap().contains("theme not found"));
    }
}
