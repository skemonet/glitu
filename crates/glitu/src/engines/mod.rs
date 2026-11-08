//! Diagram engines (`docs/design/diagram-engines.md`): a block ` ```d2 ` is rendered by a tool of
//! the machine, chosen by glitu from the manifests it knows, never by the presentation (§9).
//!
//! - `manifest`: what an engine is (built in, or the user's `engines/<id>.yaml`);
//! - `tool`: where its program is;
//! - `run` and `sandbox`: how it runs, confined;
//! - `svg`: its output, sanitized;
//! - `mermaid`: the engine that runs mermaid.js in the browser.
//!
//! The engine of a language: `--engine` > the block's `engine=` > the front matter's
//! `engine.<language>` > `engines:` of the user's configuration > glitu's default. The options of a
//! diagram, each level over the previous one: the theme's, the user's configuration, the front
//! matter (`<language>.<option>`), the block (`font-scale=…`, `<language>.<option>=…`); each typed by
//! the manifest. Raw arguments (`args`) come from the user; from the presentation only if allowed.

pub mod manifest;
pub mod mermaid;
pub mod run;
pub mod sandbox;
pub mod svg;
pub mod tool;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::settings::Settings;
use manifest::{Channel, Manifest, Origin, Runner};

/// The engine of a language when nobody chose one
const DEFAULTS: [(&str, &str); 1] = [("mermaid", "mermaid.chromium")];

/// The manifests: the built-in ones, then the user's (`engines/*.yaml` of the configuration
/// folder), which add engines or replace the built-in ones of the same id. Problems found in the
/// user's files are returned apart, to be said.
pub fn manifests() -> (Vec<Manifest>, Vec<String>) {
    let mut all: Vec<Manifest> = manifest::BUILT_IN
        .iter()
        .map(|(_, text)| Manifest::parse(text, Origin::BuiltIn).expect("built-in manifests are tested"))
        .collect();
    let mut problems = Vec::new();
    let dir = crate::settings::config_dir().map(|d| d.join("engines"));
    let mut files: Vec<PathBuf> = dir
        .and_then(|d| std::fs::read_dir(d).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .collect();
    files.sort();
    for f in files {
        let read = std::fs::read_to_string(&f).map_err(|e| e.to_string());
        match read.and_then(|t| Manifest::parse(&t, Origin::User(f.clone()))) {
            Ok(m) => {
                all.retain(|o| o.id != m.id);
                all.push(m);
            }
            Err(e) => problems.push(format!("{}: {e}; ignored", f.display())),
        }
    }
    (all, problems)
}

/// A front matter key of the engines: `engine.<language>`, `<language>.<option>`, `<language>.args`
pub fn front_matter_key(key: &str) -> bool {
    let Some((head, _)) = key.split_once('.') else { return false };
    head == "engine" || manifests().0.iter().any(|m| m.language() == head || m.fences.iter().any(|f| f == head))
}

/// What a build knows to render its diagrams
pub struct Engines {
    manifests: Vec<Manifest>,
    settings: Settings,
    /// The presentation's root: the tools run there
    root: PathBuf,
    /// The front matter
    meta: BTreeMap<String, String>,
    /// The theme's folder (its stylesheets for the engines) and its settings
    theme_dir: PathBuf,
    theme: Value,
    /// Diagrams rendered so far: the prefix of the next one's identifiers
    count: usize,
    /// The tools already announced (`-v`)
    announced: Vec<PathBuf>,
    /// The browser of `mermaid.chromium`, once started
    browser: Option<mermaid::Session>,
}

/// SVGs kept between two builds of `glitu server` (`keep`), by the key of what made them
static CACHE: Mutex<Option<HashMap<String, Vec<u8>>>> = Mutex::new(None);
/// Each tool's version, by its path
static VERSIONS: Mutex<Option<HashMap<PathBuf, String>>> = Mutex::new(None);
/// Each tool's features (its manifest's `probe`), by its path
static FEATURES: Mutex<Option<HashMap<PathBuf, Vec<String>>>> = Mutex::new(None);

/// `glitu server`: the diagrams that did not change are not rendered again
pub fn keep() {
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    c.get_or_insert_with(HashMap::new);
}

/// A diagram block
pub struct Block<'a> {
    pub lang: &'a str,
    pub source: &'a str,
    pub attrs: &'a BTreeMap<String, String>,
}

impl Engines {
    pub fn new(
        settings: &Settings,
        root: &Path,
        meta: &BTreeMap<String, String>,
        theme_dir: &Path,
        theme: &Value,
    ) -> Engines {
        let (manifests, problems) = manifests();
        for p in problems {
            eprintln!("warning: {p}");
        }
        Engines {
            manifests,
            settings: settings.clone(),
            root: root.to_path_buf(),
            meta: meta.clone(),
            theme_dir: theme_dir.to_path_buf(),
            theme: theme.clone(),
            count: 0,
            announced: Vec::new(),
            browser: None,
        }
    }

    /// Does an engine render this fence?
    pub fn renders(&self, lang: &str) -> bool {
        self.manifests.iter().any(|m| m.fences.iter().any(|f| f == lang))
    }

    /// The engine of a block (SEC-1: only a known identifier)
    pub fn engine(&self, block: &Block) -> Result<&Manifest, String> {
        let candidates: Vec<&Manifest> =
            self.manifests.iter().filter(|m| m.fences.iter().any(|f| f == block.lang)).collect();
        let language =
            candidates.first().map(|m| m.language()).ok_or(format!("no engine renders ```{}", block.lang))?;
        let keys = [language, block.lang];
        let chosen = keys
            .iter()
            .find_map(|k| self.settings.forced_engines.get(*k))
            .map(|e| (e.clone(), "--engine"))
            .or_else(|| block.attrs.get("engine").map(|e| (e.clone(), "the block's engine=")))
            .or_else(|| {
                keys.iter().find_map(|k| self.meta.get(&format!("engine.{k}"))).map(|e| (e.clone(), "the front matter"))
            })
            .or_else(|| {
                keys.iter()
                    .find_map(|k| self.settings.engines.get(*k))
                    .map(|e| (e.clone(), "engines: of the user's configuration"))
            });
        let Some((name, from)) = chosen else {
            let default = DEFAULTS.iter().find(|(l, _)| *l == language).map(|(_, id)| *id);
            return Ok(default.and_then(|d| candidates.iter().find(|m| m.id == d)).copied().unwrap_or(candidates[0]));
        };
        let name = name.trim().trim_matches(['"', '\'']);
        let id = if name.contains('.') { name.to_string() } else { format!("{language}.{name}") };
        candidates.iter().find(|m| m.id == id).copied().ok_or_else(|| {
            let known: Vec<&str> = candidates.iter().map(|m| m.implementation()).collect();
            format!("```{}: engine `{name}` ({from}) unknown; known: {}", block.lang, known.join(", "))
        })
    }

    /// The options of a block for its engine, layered and typed: the arguments they become, and
    /// the values (for a runner that reads them)
    fn options(&self, m: &Manifest, block: &Block) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
        let language = m.language();
        let mut values: BTreeMap<String, (String, String)> = BTreeMap::new();
        let mut set = |name: &str, value: String, from: &str, strict: bool| -> Result<(), String> {
            if m.options.contains_key(name) {
                values.insert(name.to_string(), (value, from.to_string()));
                Ok(())
            } else if strict {
                let known: Vec<&str> = m.options.keys().map(String::as_str).collect();
                Err(format!(
                    "{from}: {language}.{name}: not an option of {} (its options: {})",
                    m.id,
                    if known.is_empty() { "none".to_string() } else { known.join(", ") }
                ))
            } else {
                Ok(())
            }
        };
        // the theme's (`skemo: { fontScale: 1.35 }`), names in camelCase
        for (k, v) in self.theme[language].as_object().into_iter().flatten() {
            set(&kebab(k), plain(v), "the theme", false)?;
        }
        for (k, v) in self.settings.options.get(language).into_iter().flatten() {
            set(k, plain(v), "the user's configuration", true)?;
        }
        let prefix = format!("{language}.");
        for (k, v) in &self.meta {
            if let Some(name) = k.strip_prefix(&prefix).filter(|n| *n != "args") {
                set(name, v.trim().trim_matches(['"', '\'']).to_string(), "the front matter", true)?;
            }
        }
        for (k, v) in block.attrs {
            match k.strip_prefix(&prefix) {
                Some(name) => set(name, v.clone(), "the block", true)?,
                // the common options, written without a prefix
                None if k != "engine" && k != "args" => set(k, v.clone(), "the block", false)?,
                None => {}
            }
        }
        let mut args = Vec::new();
        let mut plain_values = BTreeMap::new();
        for (name, (value, from)) in values {
            let spec = &m.options[&name];
            args.extend(spec.arguments(&name, &value).map_err(|e| format!("{from}: {e}"))?);
            plain_values.insert(name, value);
        }
        Ok((args, plain_values))
    }

    /// The raw arguments (SEC-6): the user's as they are; the presentation's only if allowed
    fn raw_args(&self, m: &Manifest, block: &Block) -> Result<Vec<String>, String> {
        let language = m.language();
        let mut args = self.settings.args.get(language).cloned().unwrap_or_default();
        let given = [
            self.meta
                .get(&format!("{language}.args"))
                .map(|a| (a.trim().trim_matches(['"', '\'']), "the front matter")),
            block.attrs.get("args").map(|a| (a.as_str(), "the block")),
        ];
        for (text, from) in given.into_iter().flatten() {
            let words: Vec<String> = text.split_whitespace().map(String::from).collect();
            if !self.settings.allow_document_args {
                let allowed = self.settings.document_args.get(language);
                let mut expect_value = false;
                for w in &words {
                    let flag = w.split_once('=').map_or(w.as_str(), |(f, _)| f);
                    let ok_flag = w.starts_with('-') && allowed.is_some_and(|a| a.iter().any(|x| x == flag));
                    let ok_value = expect_value && !w.starts_with('-') && !w.chars().any(char::is_control);
                    if !ok_flag && !ok_value {
                        let at = crate::settings::config_file()
                            .map_or("the user's configuration".into(), |p| p.display().to_string());
                        return Err(format!(
                            "{from}: args=\"{text}\": raw arguments from a presentation are refused (`{w}`); \
                             to accept them, build with --allow-document-args, or allow the flag in {at}: \
                             `options: {{ {language}: {{ allow-document-args: [{flag}] }} }}`"
                        ));
                    }
                    if w.contains('=') || ok_value {
                        let value = w.split_once('=').map_or("", |(_, v)| v);
                        if value.starts_with('-') || value.chars().any(char::is_control) {
                            return Err(format!("{from}: args: `{w}`: a value not starting with -"));
                        }
                    }
                    expect_value = ok_flag && !w.contains('=');
                }
            }
            args.extend(words);
        }
        Ok(args)
    }

    /// The SVG of a block, sanitized; `None` if no engine renders its fence
    pub fn render(&mut self, block: &Block) -> Option<Result<String, String>> {
        if !self.renders(block.lang) {
            return None;
        }
        self.count += 1;
        let prefix = format!("d{}-", self.count);
        Some(self.render_with(block).and_then(|raw| {
            let text =
                String::from_utf8(raw).map_err(|_| format!("```{}: the tool's output is not UTF-8", block.lang))?;
            svg::sanitize(&text, &prefix).map_err(|e| format!("```{}: {e}", block.lang))
        }))
    }

    fn render_with(&mut self, block: &Block) -> Result<Vec<u8>, String> {
        let m = self.engine(block)?.clone();
        let (option_args, values) = self.options(&m, block)?;
        let raw = self.raw_args(&m, block)?;
        let found = match m.runner {
            // the browser of the exports, found as they find it, unless the user named one
            Runner::Chromium if !self.settings.tools.contains_key(&m.tool) => {
                crate::chromium::find()?.map(tool::Found::Path)
            }
            _ => tool::find(&m.tool, &self.settings, &self.root)?,
        };
        let found = found.ok_or_else(|| missing(&m, block))?;
        let tool = found.path().to_path_buf();
        let version = self.version(&m, &tool)?;
        let features = self.features(&m, &tool)?;
        let json = features.iter().any(|f| f == "diagnostics-json") && m.when.contains_key("diagnostics-json");
        let stylesheet = m.stylesheet.as_ref().map(|(file, _)| self.stylesheet(file)).transpose()?.flatten();
        // SEC-14: the key holds what ran
        let key = {
            let mut h = Sha256::new();
            for part in [
                m.digest.as_str(),
                &tool.display().to_string(),
                &version,
                &features.join(" "),
                block.source,
                stylesheet.as_deref().unwrap_or(""),
            ] {
                h.update(part.as_bytes());
                h.update([0]);
            }
            for a in option_args.iter().chain(&raw).chain(values.iter().flat_map(|(k, v)| [k, v])) {
                h.update(a.as_bytes());
                h.update([0]);
            }
            h.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()
        };
        if let Some(svg) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|c| c.get(&key)) {
            return Ok(svg.clone());
        }
        let out = match &m.runner {
            Runner::Chromium => {
                if self.browser.is_none() {
                    self.browser = Some(mermaid::Session::start(&tool)?);
                }
                self.announce(&m, &tool, &version, "the browser's (CSP, no network)");
                self.browser.as_mut().map_or(Err("no browser".to_string()), |b| b.render(block.source, &values))?
            }
            Runner::Command(command) => {
                let scratch = run::Scratch::new()?;
                let mut args = Vec::new();
                for part in &command[1..] {
                    match part.as_str() {
                        "{options}" => args.extend(option_args.iter().cloned()),
                        "{features}" => {
                            for (feature, extra) in &m.when {
                                if features.contains(feature) {
                                    let root = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
                                    args.extend(extra.iter().map(|a| a.replace("{root}", &root.display().to_string())));
                                }
                            }
                        }
                        "{args}" => args.extend(raw.iter().cloned()),
                        "{stylesheets}" => {
                            if let (Some((file, flag)), Some(text)) = (&m.stylesheet, &stylesheet) {
                                let p = scratch.put(file, text.as_bytes())?;
                                args.extend([flag.clone(), p.display().to_string()]);
                            }
                        }
                        "{input}" => match &m.input {
                            Channel::File(name) => {
                                args.push(scratch.put(name, block.source.as_bytes())?.display().to_string())
                            }
                            Channel::Std => return Err(format!("{}: {{input}} without `input: {{ file: … }}`", m.id)),
                        },
                        "{output}" => match &m.output {
                            Channel::File(name) => args.push(scratch.0.join(name).display().to_string()),
                            Channel::Std => {
                                return Err(format!("{}: {{output}} without `output: {{ file: … }}`", m.id));
                            }
                        },
                        p if p.starts_with("{file:") => {
                            let name = &p[6..p.len() - 1];
                            let text = m.files.get(name).ok_or(format!("{}: {p}: no such file in `files`", m.id))?;
                            args.push(scratch.put(name, text.as_bytes())?.display().to_string());
                        }
                        p => args.push(p.to_string()),
                    }
                }
                let stdin = matches!(m.input, Channel::Std).then_some(block.source.as_bytes());
                let job = run::Job {
                    tool: &tool,
                    args,
                    cwd: &self.root,
                    stdin,
                    scratch: &scratch,
                    env: &m.env,
                    set_env: &m.set_env,
                    timeout: self.settings.timeout,
                    browser: m.browser,
                };
                let ran = run::run(&job)?;
                self.announce(&m, &tool, &version, ran.sandbox);
                if !ran.success {
                    return Err(if json {
                        json_failure(&m, block, &ran.stderr)
                    } else {
                        failure(&m, block, &ran.stderr)
                    });
                }
                match &m.output {
                    Channel::Std => ran.stdout,
                    Channel::File(name) => {
                        let p = scratch.0.join(name);
                        let len = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                        if len as usize > run::MAX_OUTPUT {
                            return Err(format!("{}: more than {} MB of output, refused", m.id, run::MAX_OUTPUT >> 20));
                        }
                        std::fs::read(&p).map_err(|e| format!("{}: no output ({e})\n{}", m.id, said(&ran.stderr)))?
                    }
                }
            }
        };
        if let Some(c) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            c.insert(key, out.clone());
        }
        Ok(out)
    }

    /// The theme's stylesheet `file` for an engine, then the presentation's, if any
    fn stylesheet(&self, file: &str) -> Result<Option<String>, String> {
        let deck = self.root.join(file);
        crate::confine::Reads::new(&self.root, &self.settings.allow_read).check(&deck, file)?;
        let read = |p: &Path| {
            p.is_file().then(|| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))).transpose()
        };
        let parts: Vec<String> = [read(&self.theme_dir.join(file))?, read(&deck)?].into_iter().flatten().collect();
        Ok((!parts.is_empty()).then(|| parts.join("\n")))
    }

    /// Can the engine of a language do this (`steps`, `highlights`…)? Its manifest's capabilities,
    /// and the features its tool tells (a tool that is missing can do nothing more)
    pub fn can(&mut self, lang: &str, capability: &str) -> bool {
        let none = BTreeMap::new();
        let Ok(m) = self.engine(&Block { lang, source: "", attrs: &none }).cloned() else { return false };
        if m.can(capability) {
            return true;
        }
        let Ok(Some(found)) = tool::find(&m.tool, &self.settings, &self.root) else { return false };
        self.features(&m, found.path()).is_ok_and(|f| f.iter().any(|x| x == capability))
    }

    /// The tool's features (once per path), from its manifest's `probe`
    fn features(&self, m: &Manifest, tool: &Path) -> Result<Vec<String>, String> {
        if m.probe.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(f) = FEATURES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|f| f.get(tool)) {
            return Ok(f.clone());
        }
        let scratch = run::Scratch::new()?;
        let job = run::Job {
            tool,
            args: m.probe[1..].to_vec(),
            cwd: &scratch.0,
            stdin: None,
            scratch: &scratch,
            env: &m.env,
            set_env: &m.set_env,
            timeout: self.settings.timeout,
            browser: m.browser,
        };
        let ran = run::run(&job)?;
        let said: Value =
            if ran.success { serde_json::from_slice(&ran.stdout).unwrap_or(Value::Null) } else { Value::Null };
        let features: Vec<String> =
            said["features"].as_array().into_iter().flatten().filter_map(|f| f.as_str().map(String::from)).collect();
        FEATURES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_or_insert_with(HashMap::new)
            .insert(tool.to_path_buf(), features.clone());
        Ok(features)
    }

    /// The tool's version (once per path), part of the cache key
    fn version(&self, m: &Manifest, tool: &Path) -> Result<String, String> {
        if let Some(v) = VERSIONS.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|v| v.get(tool)) {
            return Ok(v.clone());
        }
        let version = if m.version.is_empty() {
            if matches!(m.runner, Runner::Chromium) {
                format!("mermaid.js {}", mermaid::VERSION)
            } else {
                String::new()
            }
        } else {
            let scratch = run::Scratch::new()?;
            let args = m.version[1..].to_vec();
            let empty = BTreeMap::new();
            let job = run::Job {
                tool,
                args,
                cwd: &scratch.0,
                stdin: None,
                scratch: &scratch,
                env: &m.env,
                set_env: &empty,
                timeout: self.settings.timeout,
                browser: m.browser,
            };
            let ran = run::run(&job)?;
            let text = if ran.stdout.is_empty() {
                ran.stderr.clone()
            } else {
                String::from_utf8_lossy(&ran.stdout).into_owned()
            };
            clean(text.lines().next().unwrap_or("").trim(), 200)
        };
        VERSIONS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_or_insert_with(HashMap::new)
            .insert(tool.to_path_buf(), version.clone());
        Ok(version)
    }

    /// `-v`: the tools started (SEC-15)
    fn announce(&mut self, m: &Manifest, tool: &Path, version: &str, sandbox: &str) {
        if self.settings.verbose && !self.announced.iter().any(|t| t == tool) {
            self.announced.push(tool.to_path_buf());
            eprintln!("Ran {} for {}: {} ({version}), confined by: {sandbox}", m.tool, m.id, tool.display());
        }
    }
}

/// The message of a missing tool
fn missing(m: &Manifest, block: &Block) -> String {
    let install = m.install.as_deref().map(|u| format!("install it ({u}) or ")).unwrap_or_default();
    let others: Vec<&str> = manifest::BUILT_IN
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| id.starts_with(&format!("{}.", m.language())) && *id != m.id)
        .collect();
    let other = match others.as_slice() {
        [] => String::new(),
        o => format!(
            "; or choose another engine: --engine {}={}",
            m.language(),
            o[0].split_once('.').map_or("", |(_, i)| i)
        ),
    };
    format!(
        "```{}: the {} tool is needed for this diagram ({}); {install}give its path: --tool {}=PATH{other}",
        block.lang, m.tool, m.id, m.tool
    )
}

/// A tool's failure, its standard error as text (SEC-12)
fn failure(m: &Manifest, block: &Block, stderr: &str) -> String {
    format!("```{} ({}):\n{}", block.lang, m.id, said(stderr))
}

/// A tool's failure told in JSON lines (SK-2): `line N of the diagram: message (help) [code]`, its
/// other lines as text
fn json_failure(m: &Manifest, block: &Block, stderr: &str) -> String {
    let lines: Vec<String> = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| match serde_json::from_str::<Value>(l) {
            Ok(d) if d["message"].is_string() => {
                let at = match (d["file"].as_str(), d["line"].as_u64()) {
                    (Some("-") | None, Some(n)) => format!("line {n} of the diagram: "),
                    (Some(f), Some(n)) => format!("{f}:{n}: "),
                    (Some("-") | None, None) => String::new(),
                    (Some(f), None) => format!("{f}: "),
                };
                let help = d["help"].as_str().map(|h| format!(" ({h})")).unwrap_or_default();
                let code = d["code"].as_str().map(|c| format!(" [{c}]")).unwrap_or_default();
                let severity = if d["severity"] == "warning" { "warning: " } else { "" };
                format!("{severity}{at}{}{help}{code}", d["message"].as_str().unwrap_or(""))
            }
            _ => l.to_string(),
        })
        .collect();
    failure(m, block, &lines.join("\n"))
}

/// What a tool said, as text: escapes and control characters removed, cut, indented
fn said(stderr: &str) -> String {
    let text = clean(stderr.trim(), 4096);
    if text.is_empty() {
        "  (it said nothing)".into()
    } else {
        text.lines().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n")
    }
}

/// Terminal escapes and control characters out (newlines and tabs kept), at most `max` bytes
pub fn clean(text: &str, max: usize) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                // CSI `ESC [ … letter`, OSC `ESC ] … BEL|ESC \`, or a single character
                match chars.next() {
                    Some('[') => while chars.next().is_some_and(|x| !x.is_ascii_alphabetic() && x != '~') {},
                    Some(']') => {
                        while let Some(x) = chars.next() {
                            if x == '\u{7}' || x == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() || ('\u{80}'..='\u{9f}').contains(&c) => {}
            c => out.push(c),
        }
        if out.len() >= max {
            let mut at = max;
            while !out.is_char_boundary(at) {
                at -= 1;
            }
            out.truncate(at);
            out.push_str(" […]");
            break;
        }
    }
    out
}

/// `fontScale` → `font-scale`
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_uppercase() {
            out.push('-');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engines(meta: &[(&str, &str)], settings: Settings) -> Engines {
        let meta = meta.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let theme = serde_json::json!({ "skemo": { "fontScale": 1.35 } });
        Engines::new(&settings, Path::new("."), &meta, Path::new("."), &theme)
    }

    fn attrs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn the_engine_is_chosen_by_the_user_first() {
        let none = attrs(&[]);
        let block = |a| Block { lang: "mermaid", source: "graph TD", attrs: a };
        let e = engines(&[], Settings::default());
        assert_eq!(e.engine(&block(&none)).unwrap().id, "mermaid.chromium");
        let mmdc = attrs(&[("engine", "mmdc")]);
        assert_eq!(e.engine(&block(&mmdc)).unwrap().id, "mermaid.mmdc");
        let e = engines(&[("engine.mermaid", "mmdc")], Settings::default());
        assert_eq!(e.engine(&block(&none)).unwrap().id, "mermaid.mmdc");
        let mut s = Settings::default();
        s.forced_engines.insert("mermaid".into(), "chromium".into());
        let e = engines(&[("engine.mermaid", "mmdc")], s);
        assert_eq!(e.engine(&block(&mmdc)).unwrap().id, "mermaid.chromium");
        // SEC-1: a path or a command is not an engine
        let e = engines(&[("engine.mermaid", "/usr/bin/python")], Settings::default());
        assert!(e.engine(&block(&none)).unwrap_err().contains("unknown; known: mmdc, chromium"));
        let fresh = engines(&[], Settings::default());
        for bad in ["/bin/sh", "../x", "skemo.skemo"] {
            let a = attrs(&[("engine", bad)]);
            assert!(fresh.engine(&Block { lang: "mermaid", source: "", attrs: &a }).is_err(), "{bad}");
        }
        assert!(!e.renders("rust") && e.renders("dot") && e.renders("graphviz"));
    }

    #[test]
    fn options_are_layered_and_typed() {
        let e = engines(&[("d2.layout", "elk"), ("d2.theme", "3")], Settings::default());
        let d2 = e.manifests.iter().find(|m| m.id == "d2.d2").unwrap().clone();
        let a = attrs(&[("d2.theme", "4"), ("title", "x")]);
        let (args, _) = e.options(&d2, &Block { lang: "d2", source: "", attrs: &a }).unwrap();
        assert_eq!(args, ["--layout", "elk", "--theme", "4"]);
        let a = attrs(&[("d2.layout", "../../tmp/x")]);
        let err = e.options(&d2, &Block { lang: "d2", source: "", attrs: &a }).unwrap_err();
        assert!(err.starts_with("the block: layout: `../../tmp/x`; dagre, elk expected"), "{err}");
        let a = attrs(&[("d2.output", "/tmp/x")]);
        assert!(
            e.options(&d2, &Block { lang: "d2", source: "", attrs: &a })
                .unwrap_err()
                .contains("not an option of d2.d2")
        );
        // the theme's text size, under the block's
        let skemo = e.manifests.iter().find(|m| m.id == "skemo.skemo").unwrap().clone();
        let none = attrs(&[]);
        assert_eq!(
            e.options(&skemo, &Block { lang: "skemo", source: "", attrs: &none }).unwrap().0,
            ["--font-scale", "1.35"]
        );
        let a = attrs(&[("font-scale", "2")]);
        assert_eq!(
            e.options(&skemo, &Block { lang: "skemo", source: "", attrs: &a }).unwrap().0,
            ["--font-scale", "2"]
        );
    }

    #[test]
    fn raw_arguments_come_from_the_user() {
        let mut s = Settings::default();
        s.args.insert("d2".into(), vec!["--pad".into(), "0".into()]);
        let e = engines(&[("d2.args", "--sketch")], s.clone());
        let d2 = e.manifests.iter().find(|m| m.id == "d2.d2").unwrap().clone();
        let none = attrs(&[]);
        let err = e.raw_args(&d2, &Block { lang: "d2", source: "", attrs: &none }).unwrap_err();
        assert!(err.contains("--allow-document-args") && err.contains("allow-document-args: [--sketch]"), "{err}");
        s.document_args.insert("d2".into(), vec!["--sketch".into(), "--pad".into()]);
        let e = engines(&[("d2.args", "--sketch")], s.clone());
        let a = attrs(&[("args", "--pad 5")]);
        assert_eq!(
            e.raw_args(&d2, &Block { lang: "d2", source: "", attrs: &a }).unwrap(),
            ["--pad", "0", "--sketch", "--pad", "5"]
        );
        let a = attrs(&[("args", "--sketch --output=/etc/x")]);
        assert!(e.raw_args(&d2, &Block { lang: "d2", source: "", attrs: &a }).is_err());
        let a = attrs(&[("args", "--pad -5")]);
        assert!(e.raw_args(&d2, &Block { lang: "d2", source: "", attrs: &a }).is_err());
        s.allow_document_args = true;
        let e = engines(&[], s);
        let a = attrs(&[("args", "--anything")]);
        assert!(e.raw_args(&d2, &Block { lang: "d2", source: "", attrs: &a }).is_ok());
    }

    #[test]
    fn what_a_tool_says_is_text() {
        assert_eq!(clean("\u{1b}[31mred\u{1b}[0m \u{1b}]0;title\u{7}ok\u{8}\nnext", 100), "red ok\nnext");
        assert!(clean(&"x".repeat(5000), 4096).ends_with(" […]"));
        assert_eq!(kebab("fontScale"), "font-scale");
    }
}
