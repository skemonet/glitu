//! glitu: presentations written in markdown. `glitu --help` lists the commands, `glitu <command>
//! --help` their options (see `cli`).
//!
//! Exit codes: 0 = success, 1 = the command failed (or the oracle found differences), 2 = invalid
//! command line (clap). Errors go to the standard error as `error: …`, warnings as `warning: …`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use glitu::build::{self, Project};
use glitu::cli::{self, Assets, Command, ConfigAction};
use glitu::settings::{Given, Settings};
use glitu::{serve, standalone};

/// A line on the standard output; a closed output (`glitu init --list-themes | head -1`) ends the program
/// quietly, as other command-line tools do
fn out(text: impl std::fmt::Display) {
    use std::io::Write;
    if writeln!(std::io::stdout(), "{text}").is_err() {
        std::process::exit(0);
    }
}

/// What a command prints, unless `--quiet`
struct Say {
    quiet: bool,
}

impl Say {
    fn line(&self, text: impl std::fmt::Display) {
        if !self.quiet {
            out(text);
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    // a diagram tool to confine, then become (see engines::sandbox)
    if args.get(1).map(String::as_str) == Some(glitu::engines::sandbox::COMMAND) {
        eprintln!("error: {}", glitu::engines::sandbox::exec(&args[2..]));
        return ExitCode::FAILURE;
    }
    glitu::engines::sandbox::enable();
    let (command, quiet) = cli::parse(std::env::args()).unwrap_or_else(|e| e.exit());
    match run(command, &Say { quiet }) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {}", plain(&e));
            ExitCode::FAILURE
        }
    }
}

/// An error message without the system's error number: "x.md: No such file or directory (os
/// error 2)" → "x.md: no such file or directory"
fn plain(e: &str) -> String {
    let mut e = e.to_string();
    while let Some(at) = e.find(" (os error ") {
        let end = e[at..].find(')').map_or(e.len(), |i| at + i + 1);
        e.replace_range(at..end, "");
    }
    // the system's messages start with a capital letter, in the middle of ours
    e.replace(": No such file", ": no such file").replace(": Permission denied", ": permission denied")
}

/// A size for people: 850 KB, 1.2 MB
fn size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

/// The presentation of the command line, saying which files are merged and what is ignored
fn open(inputs: &[PathBuf], given: &Given, say: &Say) -> Result<Project, String> {
    let project = Project::open(inputs, Settings::load(given)?)?;
    for note in project.notes() {
        match note.strip_prefix("warning: ") {
            Some(w) => eprintln!("warning: {w}"),
            None => say.line(format_args!("{note}")),
        }
    }
    Ok(project)
}

/// `false`: the command ran but failed its check (the oracle)
fn run(command: Command, say: &Say) -> Result<bool, String> {
    match command {
        Command::Init { dir, theme, css } => init(&dir, &theme, css, say)?,
        Command::Config { file, action } => config(file.as_deref(), action, say)?,
        Command::Add { file, request, dry_run } => add(file.as_deref(), &request, dry_run, say)?,
        Command::Build { inputs, given, assets, out } => build(&open(&inputs, &given, say)?, assets, &out, say)?,
        Command::Serve { inputs, given, port, allow } => {
            serve::serve(Project::open(&inputs, Settings::load(&given)?)?, port, allow, say.quiet)?
        }
        Command::Export { inputs, given, out, mut options, download_browser, yes } => {
            use std::io::IsTerminal;
            let project = open(&inputs, &given, say)?;
            options.progress = !say.quiet && std::io::stderr().is_terminal();
            let browser = glitu::chromium::browser(download_browser, yes)?;
            let n = glitu::export::export(&project, &out, &options, &browser)?;
            let unit = options.format.unit();
            let unit = if n == 1 { unit.trim_end_matches('s') } else { unit };
            say.line(format_args!("Built {} ({n} {unit})", out.display()));
        }
        Command::Site { inputs, given, out } => {
            let site = build::site(&open(&inputs, &given, say)?, false)?;
            build::write(&site, &out)?;
            say.line(format_args!("Built {} (static website, {} files)", out.display(), site.len()));
        }
        Command::Unpack { page, out, force } => unpack(&page, &out, force, say)?,
        Command::Themes => themes()?,
        Command::Engines { id } => engines(id.as_deref())?,
        Command::Version => out(format_args!("glitu {}", build::VERSION)),
        #[cfg(feature = "oracle")]
        Command::OracleCapture { url, out } => {
            glitu::oracle::capture(&url, &out)?;
        }
        #[cfg(feature = "oracle")]
        Command::OracleCheck { url, reference, threshold } => return glitu::oracle::check(&url, &reference, threshold),
    }
    Ok(true)
}

/// `glitu config --engines`: each engine, where its manifest comes from, its tool and what confines
/// it (SEC-15); with an id, its manifest
fn engines(id: Option<&str>) -> Result<(), String> {
    use glitu::engines::{self, manifest::Origin, tool};
    let (all, problems) = engines::manifests();
    for p in problems {
        eprintln!("warning: {p}");
    }
    let dir = glitu::settings::config_dir().map(|d| d.join("engines"));
    let dir = dir.map_or("the configuration folder".into(), |d| d.display().to_string());
    if let Some(id) = id {
        let m = all.iter().find(|m| m.id == id).ok_or_else(|| {
            let ids: Vec<&str> = all.iter().map(|m| m.id.as_str()).collect();
            format!("{id}: no such engine; the engines: {}", ids.join(", "))
        })?;
        let text = match &m.origin {
            Origin::BuiltIn => engines::manifest::BUILT_IN.iter().find(|(i, _)| *i == id).map(|(_, t)| t.to_string()),
            Origin::User(p) => std::fs::read_to_string(p).ok(),
        };
        out(format_args!(
            "# {id}: to change it, write this into {dir}/{id}.yaml\n{}",
            text.unwrap_or_default().trim_end()
        ));
        return Ok(());
    }
    let settings = Settings::load(&Given::default())?;
    let here = std::env::current_dir().unwrap_or_default();
    out(format_args!("{:<17} {:<14} {:<9} {:<40} {}", "ENGINE", "FENCES", "FROM", "TOOL", "SAFE MODE"));
    for m in &all {
        let found = match m.runner {
            engines::manifest::Runner::Chromium if !settings.tools.contains_key(&m.tool) => {
                glitu::chromium::find().ok().flatten().map(|p| p.display().to_string())
            }
            _ => tool::find(&m.tool, &settings, &here).ok().flatten().map(|f| f.path().display().to_string()),
        };
        let from = match &m.origin {
            Origin::BuiltIn => "built in".to_string(),
            Origin::User(_) => "user".to_string(),
        };
        let tool = found.unwrap_or_else(|| format!("{}: not found", m.tool));
        out(format_args!("{:<17} {:<14} {from:<9} {tool:<40} {}", m.id, m.fences.join(","), m.safe_mode));
    }
    let sandbox = if cfg!(target_os = "linux") {
        "Landlock (Linux 5.13+)"
    } else if cfg!(target_os = "macos") {
        "Seatbelt (sandbox-exec)"
    } else {
        "none (killed with their children at the time limit)"
    };
    out(format_args!(
        "\nTools confined by: {sandbox}; time limit: {} s per diagram.\nConfiguration: {}\n\
         An engine's manifest: glitu config --engines <ENGINE>; yours in {dir}/<ENGINE>.yaml.",
        settings.timeout.as_secs_f64(),
        settings.file.as_ref().map_or_else(
            || glitu::settings::config_file().map_or("none".into(), |f| format!("{} (absent)", f.display())),
            |f| f.display().to_string()
        )
    ));
    Ok(())
}

/// `glitu init`: what it wrote, then the next steps
fn init(dir: &Path, theme: &str, css: bool, say: &Say) -> Result<(), String> {
    let written = glitu::init::run(dir, theme, css)?;
    let own = dir.join("themes").join(theme);
    let slides = dir.join("slides.md");
    for path in written.iter().filter(|p| !p.starts_with(&own)) {
        say.line(format_args!("Created {}", path.display()));
    }
    if written.iter().any(|p| p.starts_with(&own)) {
        say.line(format_args!("Created {}/ (custom theme, copied from crystal)", own.display()));
        if !written.contains(&slides) {
            say.line(format_args!(
                "\nTo use it, set `theme: ./themes/{theme}` in the front matter of {}.",
                slides.display()
            ));
        }
    }
    say.line("\nNext steps:");
    let preview = format!("glitu server {}", slides.display());
    let customize = format!("glitu init {} --css", dir.display());
    let width = preview.len().max(customize.len()) + 4;
    say.line(format_args!("  {preview:<width$}Preview the presentation"));
    if !written.iter().any(|p| p.ends_with("style.css")) && !dir.join("style.css").exists() {
        say.line(format_args!("  {customize:<width$}Create style.css to customize the theme"));
    }
    Ok(())
}

/// `glitu config`: the front matter of the presentation, listed, read or changed
fn config(file: Option<&Path>, action: ConfigAction, say: &Say) -> Result<(), String> {
    use glitu::config;
    let md = config::file(file)?;
    let src = std::fs::read_to_string(&md).map_err(|e| format!("{}: {e}", md.display()))?;
    let unknown = |key: &str| {
        let names: Vec<&str> = config::KEYS.iter().map(|k| k.name).collect();
        format!("{key}: not a setting of glitu; the settings: {}", names.join(", "))
    };
    match action {
        ConfigAction::List => {
            let short = |v: &str| {
                if v.chars().count() > 26 { v.chars().take(25).collect::<String>() + "…" } else { v.to_string() }
            };
            out(format_args!("{:<14} {:<26}  ABOUT", "KEY", "VALUE"));
            for k in &config::KEYS {
                let value = config::get(&src, k.name).map_or_else(|| "-".to_string(), |v| short(&v));
                out(format_args!("{:<14} {value:<26}  {}", k.name, k.about));
            }
            type Pairs = Vec<(String, String)>;
            let (diagrams, others): (Pairs, Pairs) = config::written(&src)
                .into_iter()
                .filter(|(k, _)| config::find(k).is_none())
                .partition(|(k, _)| glitu::engines::front_matter_key(k));
            if !diagrams.is_empty() {
                let pairs: Vec<String> = diagrams.iter().map(|(k, v)| format!("{k}: {v}")).collect();
                out(format_args!("\nDiagram engines (glitu config --engines): {}", pairs.join(", ")));
            }
            let others: Vec<String> = others.into_iter().map(|(k, _)| k).collect();
            if !others.is_empty() {
                out(format_args!("\nNot read by glitu: {}", others.join(", ")));
            }
            say.line(format_args!("\nIn {}. To change a setting: glitu config <KEY> <VALUE>", md.display()));
        }
        ConfigAction::Get(key) => match config::get(&src, &key) {
            Some(v) => out(v),
            None if config::find(&key).is_some() => return Err(format!("{key} is not set in {}", md.display())),
            None => return Err(unknown(&key)),
        },
        ConfigAction::Set(key, value) => {
            let k = config::find(&key).ok_or_else(|| unknown(&key))?;
            let value = value.trim();
            if let Some(w) = config::check(k, value, &md)? {
                eprintln!("warning: {w}");
            }
            config::edit(&md, |src| Ok(config::set(src, &key, value)))?;
            say.line(format_args!("Set {key}: {value} in {}", md.display()));
        }
        ConfigAction::Unset(key) => {
            config::edit(&md, |src| {
                config::unset(src, &key).ok_or_else(|| format!("{key} is not set in {}", md.display()))
            })?;
            say.line(format_args!("Removed {key} from {}", md.display()));
        }
    }
    Ok(())
}

/// `glitu add`: the slide written into the presentation, with its files
fn add(file: Option<&Path>, request: &glitu::add::Request, dry_run: bool, say: &Say) -> Result<(), String> {
    let (main, last) = glitu::add::target(file)?;
    let base = main.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    if glitu_core::header(&read(&main)?).reveal_md {
        return Err(format!(
            "{}: written with reveal-md's rules (slides separated by ---); glitu add writes glitu's",
            main.display()
        ));
    }
    let mut slide = glitu::add::slide(request, base);
    // a diagram needs the skemo tool: said in the slide when this machine does not have it
    if matches!(request.kind, glitu::add::Kind::Diagram { .. }) {
        let settings = Settings::load(&Given::default()).unwrap_or_default();
        if glitu::engines::tool::find("skemo", &settings, base).ok().flatten().is_none() {
            slide.markdown = slide.markdown.replacen("```skemo", &format!("{}\n```skemo", glitu::add::NEEDS_SKEMO), 1);
        }
    }
    // `casts/x.cast` rather than `./casts/x.cast`
    let at = |p: &Path| if base == Path::new(".") { p.to_path_buf() } else { base.join(p) };
    if dry_run {
        out(slide.markdown.trim_end());
        for (path, _) in &slide.files {
            say.line(format_args!("\n(would also create {})", at(path).display()));
        }
        return Ok(());
    }
    for (path, bytes) in &slide.files {
        build::write_file(&at(path), bytes)?;
        say.line(format_args!("Created {}", at(path).display()));
    }
    let (text, line) = glitu::add::insert(&read(&last)?, &slide.markdown);
    std::fs::write(&last, text).map_err(|e| format!("{}: {e}", last.display()))?;
    say.line(format_args!("Added the slide to {}, line {line}", last.display()));
    Ok(())
}

/// The embedded themes, one per line: name, version, kind, animated backdrop
fn themes() -> Result<(), String> {
    let root = glitu::themes::root()?;
    out(format_args!("{:<14} {:<8} {:<10} BACKDROP", "THEME", "VERSION", "KIND"));
    for name in glitu::themes::names() {
        let theme = build::LocalTheme::open(
            root.join(name),
            &glitu_core::Theme::Official { name: name.into(), version: None },
        )?;
        let kind = if theme.config["template"].as_bool() == Some(true) { "template" } else { "reveal.js" };
        let backdrop = theme.config["backdrop"].as_str().unwrap_or("-");
        let default = match name {
            "crystal" => "  (default)",
            "black" => "  (default for the reveal-md syntax)",
            _ => "",
        };
        out(format_args!("{name:<14} {:<8} {kind:<10} {backdrop}{default}", theme.version.as_deref().unwrap_or("-")));
    }
    Ok(())
}

fn build(project: &Project, assets: Assets, dest: &Path, say: &Say) -> Result<(), String> {
    let site = build::site(project, false)?;
    let mode = match assets {
        Assets::Local => standalone::Mode::Offline,
        Assets::At(base) => standalone::Mode::Online { base, theme: published_theme(project)? },
    };
    let page = standalone::bundle(&site, &mode)?;
    build::write_file(dest, page.as_bytes())?;
    say.line(format_args!("Built {} ({})", dest.display(), size(page.len())));
    Ok(())
}

/// Where the presentation's theme is published, if it is (official theme)
fn published_theme(project: &Project) -> Result<Option<String>, String> {
    Ok(project.theme(&glitu_core::header(&project.source()?).theme)?.published())
}

/// The markdown of a built page, or of a `.glituplay` file (a zip archive: its `source.json`)
fn unpack(page: &str, dest: &Path, force: bool, say: &Say) -> Result<(), String> {
    let data = if page.contains("://") {
        glitu::online::download(page)?
    } else {
        std::fs::read(page).map_err(|e| format!("{page}: {e}"))?
    };
    let extracted = if data.starts_with(b"PK\x03\x04") {
        let copy = glitu::zip::entry(&data, "source.json")
            .map_err(|e| format!("{page}: {e}"))?
            .ok_or_else(|| format!("{page}: exported without its markdown (`source: false`, or an older glitu)"))?;
        glitu::source::files_of(&String::from_utf8_lossy(&copy))
    } else {
        glitu::source::extract(&String::from_utf8_lossy(&data))
    };
    let glitu::source::Extracted { files, refused } = extracted.map_err(|e| format!("{page}: {e}"))?;
    for path in &refused {
        eprintln!("warning: {path} is outside the presentation directory; skipped");
    }
    let targets: Vec<PathBuf> = files.iter().map(|(p, _)| dest.join(p)).collect();
    if let Some(t) = targets.iter().find(|t| t.exists()).filter(|_| !force) {
        return Err(format!("{} already exists; use --force to overwrite it", t.display()));
    }
    for (target, (_, text)) in targets.iter().zip(&files) {
        build::write_file(target, text.as_bytes())?;
        say.line(format_args!("Recovered {}", target.display()));
    }
    Ok(())
}
