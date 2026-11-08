//! What the user's machine decides, never the presentation (§9 of
//! `docs/design/diagram-engines.md`): the folders a presentation may read besides its own, where
//! the diagram tools are, the default engine of each language, the options and raw arguments of
//! each engine, the limits. From the command line, the environment and the user's configuration,
//! `config.yaml` in glitu's configuration folder (`$GLITU_CONFIG_DIR`, otherwise
//! `$XDG_CONFIG_HOME/glitu`, `~/.config/glitu`; `%APPDATA%\glitu` on Windows,
//! `~/Library/Application Support/glitu` on macOS):
//!
//! ```yaml
//! files:
//!   allow: [~/talks/shared]        # folders a presentation may read besides its own
//! tools:
//!   skemo: /opt/skemo/bin/skemo    # where a tool is (otherwise GLITU_TOOL_SKEMO, then the PATH)
//! engines:
//!   mermaid: chromium              # the default engine of a language
//! options:
//!   skemo:                         # options of every diagram of an engine
//!     font-scale: 1.4
//!     args: [--contrast, high]     # raw arguments, passed as they are
//!   d2:
//!     allow-document-args: [--sketch]   # raw arguments a presentation may give
//! limits:
//!   timeout: 30                    # seconds per diagram
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

/// The options of the command line that belong to the machine (`build`, `server`, the formats)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Given {
    /// `--allow-read DIR`
    pub allow_read: Vec<PathBuf>,
    /// `--tool NAME=PATH`
    pub tools: Vec<(String, PathBuf)>,
    /// `--engine LANGUAGE=IMPLEMENTATION`
    pub engines: Vec<(String, String)>,
    /// `--allow-document-args`
    pub allow_document_args: bool,
    /// `-v`: say which tools run
    pub verbose: bool,
}

/// The machine's settings, merged
#[derive(Clone, Debug)]
pub struct Settings {
    /// Folders a presentation may read besides its own
    pub allow_read: Vec<PathBuf>,
    /// Tools named by the user: the command line, then `GLITU_TOOL_*`, then the configuration
    pub tools: BTreeMap<String, PathBuf>,
    /// Engines chosen on the command line: they win over the presentation
    pub forced_engines: BTreeMap<String, String>,
    /// Engines chosen in the configuration: the presentation wins over them
    pub engines: BTreeMap<String, String>,
    /// Options of each engine (`options.<engine>`), raw arguments (`args`) apart
    pub options: BTreeMap<String, serde_json::Map<String, Value>>,
    /// Raw arguments of each engine, from the configuration
    pub args: BTreeMap<String, Vec<String>>,
    /// The raw arguments a presentation may give, per engine
    pub document_args: BTreeMap<String, Vec<String>>,
    /// Every raw argument of the presentation accepted, for this run
    pub allow_document_args: bool,
    pub timeout: Duration,
    pub verbose: bool,
    /// The configuration file read, if any
    pub file: Option<PathBuf>,
}

/// The default time a diagram may take
pub const TIMEOUT: Duration = Duration::from_secs(30);

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            allow_read: Vec::new(),
            tools: BTreeMap::new(),
            forced_engines: BTreeMap::new(),
            engines: BTreeMap::new(),
            options: BTreeMap::new(),
            args: BTreeMap::new(),
            document_args: BTreeMap::new(),
            allow_document_args: false,
            timeout: TIMEOUT,
            verbose: false,
            file: None,
        }
    }
}

/// glitu's configuration folder
pub fn config_dir() -> Option<PathBuf> {
    let var = |n: &str| std::env::var_os(n).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = var("GLITU_CONFIG_DIR") {
        return Some(d);
    }
    if cfg!(windows) {
        return var("APPDATA").map(|d| d.join("glitu"));
    }
    if cfg!(target_os = "macos") {
        return var("HOME").map(|h| h.join("Library/Application Support/glitu"));
    }
    var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config"))).map(|d| d.join("glitu"))
}

/// The user's configuration file (it may not exist)
pub fn config_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.yaml"))
}

impl Settings {
    /// The settings of a run: the configuration file, the environment, then the command line
    pub fn load(given: &Given) -> Result<Settings, String> {
        let file = config_file().filter(|f| f.is_file());
        let config = match &file {
            Some(f) => {
                let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
                parse(&text).map_err(|e| format!("{}: {e}", f.display()))?
            }
            None => Value::Null,
        };
        let mut s = Settings::from_config(&config)?;
        s.file = file;
        for (name, value) in std::env::vars_os() {
            let (Some(name), Some(value)) = (name.to_str(), value.to_str()) else { continue };
            if let Some(tool) = name.strip_prefix("GLITU_TOOL_").filter(|_| !value.is_empty()) {
                s.tools.insert(tool.to_ascii_lowercase().replace('_', "-"), PathBuf::from(value));
            }
        }
        s.allow_read.extend(given.allow_read.iter().cloned());
        s.tools.extend(given.tools.iter().cloned());
        s.forced_engines.extend(given.engines.iter().cloned());
        s.allow_document_args = given.allow_document_args;
        s.verbose = given.verbose;
        Ok(s)
    }

    /// The settings a configuration holds
    fn from_config(config: &Value) -> Result<Settings, String> {
        let mut s = Settings::default();
        let Some(top) = object(config, "the configuration")? else { return Ok(s) };
        for (key, value) in top {
            match key.as_str() {
                "files" => {
                    for (k, v) in object(value, "files")?.into_iter().flatten() {
                        match k.as_str() {
                            "allow" => s.allow_read = strings(v, "files.allow")?.iter().map(|p| home(p)).collect(),
                            _ => return Err(unknown(&format!("files.{k}"), &["files.allow"])),
                        }
                    }
                }
                "tools" => {
                    for (k, v) in object(value, "tools")?.into_iter().flatten() {
                        let path = v.as_str().ok_or(format!("tools.{k}: a path expected"))?;
                        s.tools.insert(k.clone(), home(path));
                    }
                }
                "engines" => {
                    for (k, v) in object(value, "engines")?.into_iter().flatten() {
                        let e = v.as_str().ok_or(format!("engines.{k}: an engine name expected"))?;
                        s.engines.insert(k.clone(), e.to_string());
                    }
                }
                "options" => {
                    for (engine, v) in object(value, "options")?.into_iter().flatten() {
                        let mut opts = object(v, &format!("options.{engine}"))?.cloned().unwrap_or_default();
                        if let Some(a) = opts.remove("args") {
                            s.args.insert(engine.clone(), strings(&a, &format!("options.{engine}.args"))?);
                        }
                        if let Some(a) = opts.remove("allow-document-args") {
                            let flags = strings(&a, &format!("options.{engine}.allow-document-args"))?;
                            s.document_args.insert(engine.clone(), flags);
                        }
                        s.options.insert(engine.clone(), opts);
                    }
                }
                "limits" => {
                    for (k, v) in object(value, "limits")?.into_iter().flatten() {
                        match k.as_str() {
                            "timeout" => {
                                let t = v.as_f64().filter(|t| *t > 0.0 && *t <= 3600.0);
                                let t = t.ok_or("limits.timeout: a number of seconds between 0 and 3600 expected")?;
                                s.timeout = Duration::from_secs_f64(t);
                            }
                            _ => return Err(unknown(&format!("limits.{k}"), &["limits.timeout"])),
                        }
                    }
                }
                _ => return Err(unknown(key, &["files", "tools", "engines", "options", "limits"])),
            }
        }
        Ok(s)
    }
}

/// A YAML document as a tree (an empty document: nothing)
pub fn parse(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_saphyr::from_str::<Value>(text).map_err(|e| e.to_string())
}

fn object<'a>(v: &'a Value, at: &str) -> Result<Option<&'a serde_json::Map<String, Value>>, String> {
    match v {
        Value::Null => Ok(None),
        Value::Object(o) => Ok(Some(o)),
        _ => Err(format!("{at}: keys expected")),
    }
}

/// A list of texts (one text alone counts as a list)
fn strings(v: &Value, at: &str) -> Result<Vec<String>, String> {
    let one = |x: &Value| match x {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(format!("{at}: texts expected")),
    };
    match v {
        Value::Array(a) => a.iter().map(one).collect(),
        Value::Null => Ok(Vec::new()),
        x => Ok(vec![one(x)?]),
    }
}

fn unknown(key: &str, known: &[&str]) -> String {
    format!("unknown key `{key}`; known: {}", known.join(", "))
}

/// `~/x` → the home folder's x
fn home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))) {
        (Some(rest), Some(h)) => Path::new(&h).join(rest),
        _ => PathBuf::from(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuration_is_read_and_checked() {
        let text = "files:\n  allow: [/srv/shared]\ntools:\n  skemo: /opt/skemo\nengines:\n  mermaid: chromium\n\
                    options:\n  d2:\n    layout: elk\n    args: [--pad, 10]\n    allow-document-args: --sketch\n\
                    limits:\n  timeout: 5\n";
        let s = Settings::from_config(&parse(text).unwrap()).unwrap();
        assert_eq!(s.allow_read, [PathBuf::from("/srv/shared")]);
        assert_eq!(s.tools["skemo"], PathBuf::from("/opt/skemo"));
        assert_eq!(s.engines["mermaid"], "chromium");
        assert_eq!(s.options["d2"]["layout"], "elk");
        assert_eq!(s.args["d2"], ["--pad", "10"]);
        assert_eq!(s.document_args["d2"], ["--sketch"]);
        assert_eq!(s.timeout, Duration::from_secs(5));
        let e = Settings::from_config(&parse("file:\n  allow: x\n").unwrap()).unwrap_err();
        assert!(e.starts_with("unknown key `file`"), "{e}");
        assert!(Settings::from_config(&parse("limits:\n  timeout: -1\n").unwrap()).is_err());
        assert_eq!(Settings::from_config(&parse("").unwrap()).unwrap().timeout, TIMEOUT);
    }
}
