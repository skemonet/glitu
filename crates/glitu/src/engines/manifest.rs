//! The description of an engine (`docs/design/diagram-engines.md`, §4): the tool it runs, its
//! command line, the options a presentation may give it and their types (SEC-5). The manifests
//! glitu supports are in the binary (`crates/glitu/engines/`); the user adds or overrides some in
//! `engines/<id>.yaml` of glitu's configuration folder, never a presentation (SEC-2).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// The built-in manifests
pub const BUILT_IN: [(&str, &str); 5] = [
    ("skemo.skemo", include_str!("../../engines/skemo.skemo.yaml")),
    ("graphviz.dot", include_str!("../../engines/graphviz.dot.yaml")),
    ("d2.d2", include_str!("../../engines/d2.d2.yaml")),
    ("mermaid.mmdc", include_str!("../../engines/mermaid.mmdc.yaml")),
    ("mermaid.chromium", include_str!("../../engines/mermaid.chromium.yaml")),
];

#[derive(Clone, Debug, PartialEq)]
pub enum Origin {
    BuiltIn,
    /// A file of the user's configuration
    User(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Runner {
    /// A program, started with this vector of arguments (placeholders: `{tool}`, `{input}`,
    /// `{output}`, `{file:NAME}`, `{stylesheets}`, `{options}`, `{args}`)
    Command(Vec<String>),
    /// mermaid.js in the browser glitu drives
    Chromium,
}

/// Where the source goes, and where the SVG comes from
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    Std,
    /// A file of that name in the private temporary folder
    File(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Choice(Vec<String>),
    Number {
        min: f64,
        max: f64,
        integer: bool,
    },
    Bool,
    /// Letters, digits and `_ . , : # % + -` (not first), at most 200
    Text,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionSpec {
    pub kind: Kind,
    /// The flag before the value; none: the option is not passed on the command line (the runner
    /// reads it)
    pub flag: Option<String>,
    /// `-Kneato` rather than `-K neato`
    pub attached: bool,
}

#[derive(Clone, Debug)]
pub struct Manifest {
    /// `<language>.<implementation>`
    pub id: String,
    pub fences: Vec<String>,
    /// The tool's name, looked up as in §3
    pub tool: String,
    /// Where to get it
    pub install: Option<String>,
    pub runner: Runner,
    pub input: Channel,
    pub output: Channel,
    pub version: Vec<String>,
    /// The command that tells the tool's features (`{"features": […]}` in JSON, SK-3); a tool
    /// that fails it has none
    pub probe: Vec<String>,
    /// Arguments added when the tool has a feature (`{features}` in the command): `root`,
    /// `diagnostics-json` (its messages read as JSON lines)…
    pub when: BTreeMap<String, Vec<String>>,
    /// Files glitu writes into the temporary folder for the tool (`{file:NAME}`): its safe mode
    pub files: BTreeMap<String, String>,
    /// The theme's and the presentation's stylesheet of that name, each passed after the flag
    pub stylesheet: Option<(String, String)>,
    /// Variables of the user's environment the tool gets (SEC-10)
    pub env: Vec<String>,
    /// Variables set for the tool
    pub set_env: BTreeMap<String, String>,
    pub options: BTreeMap<String, OptionSpec>,
    /// What glitu can do with its SVG: `steps`, `ids`, `frieze`, `highlights` (§6)
    pub capabilities: Vec<String>,
    /// The tool runs a browser: its sandbox lets it start one
    pub browser: bool,
    /// What the tool can read or reach, for `glitu config --engines`
    pub safe_mode: String,
    pub origin: Origin,
    /// SHA-256 of the manifest's text: part of the cache key (SEC-14)
    pub digest: String,
}

impl Manifest {
    pub fn language(&self) -> &str {
        self.id.split_once('.').map_or(&self.id, |(l, _)| l)
    }

    pub fn implementation(&self) -> &str {
        self.id.split_once('.').map_or(&self.id, |(_, i)| i)
    }

    pub fn can(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// A manifest's text: checked, every key known
    pub fn parse(text: &str, origin: Origin) -> Result<Manifest, String> {
        let v = crate::settings::parse(text)?;
        let o = v.as_object().ok_or("keys expected")?;
        let text_of = |k: &str| o.get(k).and_then(Value::as_str).map(String::from);
        let list = |k: &str| -> Result<Vec<String>, String> {
            match o.get(k) {
                None => Ok(Vec::new()),
                Some(Value::Array(a)) => {
                    a.iter().map(|x| scalar(x).ok_or(format!("{k}: texts expected"))).collect::<Result<_, _>>()
                }
                Some(_) => Err(format!("{k}: a list expected")),
            }
        };
        let map = |k: &str| -> Result<BTreeMap<String, String>, String> {
            match o.get(k) {
                None => Ok(BTreeMap::new()),
                Some(Value::Object(m)) => m
                    .iter()
                    .map(|(n, x)| Ok((n.clone(), scalar(x).ok_or(format!("{k}.{n}: a text expected"))?)))
                    .collect(),
                Some(_) => Err(format!("{k}: keys expected")),
            }
        };
        const KNOWN: [&str; 19] = [
            "probe",
            "when",
            "id",
            "fences",
            "tool",
            "install",
            "command",
            "runner",
            "input",
            "output",
            "version",
            "files",
            "stylesheet",
            "env",
            "set-env",
            "options",
            "capabilities",
            "sandbox",
            "safe-mode",
        ];
        if let Some(k) = o.keys().find(|k| !KNOWN.contains(&k.as_str())) {
            return Err(format!("unknown key `{k}`; known: {}", KNOWN.join(", ")));
        }
        let id = text_of("id").ok_or("id: missing (`<language>.<implementation>`)")?;
        check_id(&id)?;
        let runner = match (text_of("runner").as_deref(), list("command")?) {
            (Some("chromium"), _) => Runner::Chromium,
            (Some(r), _) => return Err(format!("runner: `{r}` unknown (chromium)")),
            (None, c) if c.is_empty() => return Err("command: missing".into()),
            (None, c) if c[0] != "{tool}" => return Err("command: must start with {tool}".into()),
            (None, c) => Runner::Command(c),
        };
        let channel = |k: &str| match o.get(k) {
            None => Ok(Channel::Std),
            Some(Value::Object(m)) => match m.get("file").and_then(Value::as_str) {
                Some(f) if plain_name(f) => Ok(Channel::File(f.to_string())),
                _ => Err(format!("{k}: `file: NAME` expected, a plain file name")),
            },
            Some(_) => Err(format!("{k}: `file: NAME` expected")),
        };
        let files = map("files")?;
        if let Some(f) = files.keys().find(|f| !plain_name(f)) {
            return Err(format!("files: `{f}` is not a plain file name"));
        }
        let stylesheet = match o.get("stylesheet") {
            None => None,
            Some(s) => {
                let (file, flag) = (s["file"].as_str(), s["flag"].as_str());
                match (file, flag) {
                    (Some(f), Some(g)) if plain_name(f) => Some((f.to_string(), g.to_string())),
                    _ => return Err("stylesheet: `{ file: NAME, flag: --FLAG }` expected".into()),
                }
            }
        };
        let mut options = BTreeMap::new();
        for (name, spec) in o.get("options").and_then(Value::as_object).into_iter().flatten() {
            options.insert(name.clone(), option_spec(spec).map_err(|e| format!("options.{name}: {e}"))?);
        }
        let fences = list("fences")?;
        if fences.is_empty() {
            return Err("fences: the fence names it renders, at least one".into());
        }
        Ok(Manifest {
            fences,
            tool: text_of("tool").filter(|t| plain_name(t)).ok_or("tool: a program's name expected (no path)")?,
            install: text_of("install"),
            runner,
            input: channel("input")?,
            output: channel("output")?,
            version: list("version")?,
            probe: list("probe")?,
            when: {
                let mut when = BTreeMap::new();
                for (feature, args) in o.get("when").and_then(Value::as_object).into_iter().flatten() {
                    let args = args.as_array().ok_or(format!("when.{feature}: a list expected"))?;
                    when.insert(feature.clone(), args.iter().filter_map(scalar).collect());
                }
                when
            },
            files,
            stylesheet,
            env: list("env")?,
            set_env: map("set-env")?,
            options,
            capabilities: list("capabilities")?,
            browser: text_of("sandbox").as_deref() == Some("browser"),
            safe_mode: text_of("safe-mode").unwrap_or_default(),
            origin,
            digest: Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect(),
            id,
        })
    }
}

/// `language.implementation`: letters, digits, `-`
pub fn check_id(id: &str) -> Result<(), String> {
    let word = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    match id.split_once('.') {
        Some((l, i)) if word(l) && word(i) => Ok(()),
        _ => Err(format!("`{id}`: an engine is named `<language>.<implementation>` (letters, digits, -)")),
    }
}

/// A file name without a folder
fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', ':', '\0'])
}

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn option_spec(v: &Value) -> Result<OptionSpec, String> {
    let kind = match v["type"].as_str() {
        Some("choice") => {
            let values: Vec<String> = v["values"].as_array().into_iter().flatten().filter_map(scalar).collect();
            if values.is_empty() {
                return Err("values: the words it accepts".into());
            }
            Kind::Choice(values)
        }
        Some("number") => Kind::Number {
            min: v["min"].as_f64().ok_or("min: a number expected")?,
            max: v["max"].as_f64().ok_or("max: a number expected")?,
            integer: v["integer"].as_bool().unwrap_or(false),
        },
        Some("bool") => Kind::Bool,
        Some("text") => Kind::Text,
        _ => return Err("type: choice, number, bool or text".into()),
    };
    let flag = v["flag"].as_str().map(String::from);
    if let Some(f) = &flag
        && !f.starts_with('-')
    {
        return Err(format!("flag: `{f}` must start with -"));
    }
    Ok(OptionSpec { kind, flag, attached: v["attached"].as_bool().unwrap_or(false) })
}

impl OptionSpec {
    /// A value given by a presentation, checked (SEC-5): the arguments it becomes
    pub fn arguments(&self, name: &str, value: &str) -> Result<Vec<String>, String> {
        let value = value.trim();
        if value.chars().any(char::is_control) {
            return Err(format!("{name}: a control character in the value"));
        }
        let shown = |what: &str| format!("{name}: `{value}`; {what} expected");
        let checked = match &self.kind {
            Kind::Choice(words) => {
                if !words.iter().any(|w| w == value) {
                    return Err(shown(&words.join(", ")));
                }
                value.to_string()
            }
            Kind::Number { min, max, integer } => {
                let n: f64 = value.parse().map_err(|_| shown("a number"))?;
                if !(n.is_finite() && n >= *min && n <= *max) || *integer && n.fract() != 0.0 {
                    let kind = if *integer { "an integer" } else { "a number" };
                    return Err(shown(&format!("{kind} between {min} and {max}")));
                }
                value.to_string()
            }
            Kind::Bool => match value {
                "" | "true" | "yes" | "on" => "true".into(),
                "false" | "no" | "off" => "false".into(),
                _ => return Err(shown("true or false")),
            },
            Kind::Text => {
                let ok = |c: char| c.is_alphanumeric() || " _.,:#%+-".contains(c);
                if value.starts_with('-') || value.len() > 200 || !value.chars().all(ok) {
                    return Err(shown("a plain text (letters, digits, _ . , : # % + -, not first)"));
                }
                value.to_string()
            }
        };
        if checked.starts_with('-') && !matches!(self.kind, Kind::Number { .. }) {
            return Err(shown("a value not starting with -"));
        }
        let Some(flag) = &self.flag else { return Ok(Vec::new()) };
        Ok(match (&self.kind, checked.as_str()) {
            (Kind::Bool, "true") => vec![flag.clone()],
            (Kind::Bool, _) => Vec::new(),
            _ if self.attached => vec![format!("{flag}{checked}")],
            _ => vec![flag.clone(), checked],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_manifests_are_valid() {
        for (id, text) in BUILT_IN {
            let m = Manifest::parse(text, Origin::BuiltIn).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(m.id, id);
        }
    }

    #[test]
    fn manifests_are_checked() {
        let base = "id: x.y\nfences: [x]\ntool: prog\ncommand: [\"{tool}\"]\n";
        assert!(Manifest::parse(base, Origin::BuiltIn).is_ok());
        let bad = |extra: &str| Manifest::parse(&format!("{base}{extra}"), Origin::BuiltIn).unwrap_err();
        assert!(bad("shell: true\n").starts_with("unknown key `shell`"));
        assert!(bad("files:\n  ../x: y\n").contains("plain file name"));
        assert!(bad("options:\n  a: { type: text, flag: x }\n").contains("must start with -"));
        let e = Manifest::parse("id: x.y\nfences: [x]\ntool: /bin/sh\ncommand: [\"{tool}\"]\n", Origin::BuiltIn);
        assert!(e.unwrap_err().contains("no path"));
        assert!(Manifest::parse("id: x.y\nfences: [x]\ntool: prog\ncommand: [sh]\n", Origin::BuiltIn).is_err());
        assert!(check_id("mermaid.chromium").is_ok() && check_id("/bin/sh").is_err() && check_id("../x").is_err());
    }

    #[test]
    fn values_from_a_presentation_are_typed() {
        let d2 = Manifest::parse(BUILT_IN[2].1, Origin::BuiltIn).unwrap();
        let o = |k: &str, v: &str| d2.options[k].arguments(k, v);
        assert_eq!(o("layout", "elk").unwrap(), ["--layout", "elk"]);
        assert!(o("layout", "../../bin/sh").is_err() && o("layout", "tala").is_err());
        assert_eq!(o("theme", "200").unwrap(), ["--theme", "200"]);
        assert!(o("theme", "1.5").is_err() && o("theme", "-1").is_err() && o("theme", "1e9").is_err());
        assert_eq!(o("sketch", "").unwrap(), ["--sketch"]);
        assert!(o("sketch", "false").unwrap().is_empty() && o("sketch", "maybe").is_err());
        let dot = Manifest::parse(BUILT_IN[1].1, Origin::BuiltIn).unwrap();
        assert_eq!(dot.options["layout"].arguments("layout", "neato").unwrap(), ["-Kneato"]);
        let text = OptionSpec { kind: Kind::Text, flag: Some("--x".into()), attached: false };
        assert!(text.arguments("x", "--output=/etc/x").is_err() && text.arguments("x", "a\u{1b}b").is_err());
        assert_eq!(text.arguments("x", "Hello, world").unwrap(), ["--x", "Hello, world"]);
    }
}
