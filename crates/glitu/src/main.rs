//! glitu: presentations written in markdown. `glitu --help` lists the commands, `glitu <command>
//! --help` their options (see `cli`).
//!
//! Exit codes: 0 = OK, 1 = the command failed (or the oracle found differences), 2 = misuse.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use glitu::build::{self, Project};
use glitu::cli::{self, Build, Command};
use glitu::{serve, standalone};

/// What a command prints, unless `--quiet`
struct Say {
    quiet: bool,
}

impl Say {
    fn line(&self, text: impl std::fmt::Display) {
        if !self.quiet {
            println!("{text}");
        }
    }
}

fn main() -> ExitCode {
    glitu_core::skemo::speak_english();
    let (command, quiet) = cli::parse(std::env::args()).unwrap_or_else(|e| e.exit());
    match run(command, &Say { quiet }) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("glitu: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `false`: the command ran but failed its check (the oracle)
fn run(command: Command, say: &Say) -> Result<bool, String> {
    match command {
        Command::Init { dir, theme } => init(&dir, &theme, say)?,
        Command::Build { slides, mode, out } => build(&Project::new(&slides), mode, out, say)?,
        Command::Serve { slides, port, allow, rehearse } => serve::serve(Project::new(&slides), port, allow, rehearse)?,
        Command::Export { slides, out, options } => {
            let n = glitu::export::export(&Project::new(&slides), &out, &options)?;
            say.line(format_args!("{}: {n} {}", out.display(), options.format.unit()));
        }
        Command::Unpack { page, out, force } => unpack(&page, &out, force, say)?,
        Command::Themes => themes()?,
        Command::Assets { out, themes } => assets(&out, themes, say)?,
        #[cfg(feature = "oracle")]
        Command::OracleCapture { url, out } => {
            glitu::oracle::capture(&url, &out)?;
        }
        #[cfg(feature = "oracle")]
        Command::OracleCheck { url, reference, threshold } => return glitu::oracle::check(&url, &reference, threshold),
    }
    Ok(true)
}

/// The sample presentation of `glitu init`
const INIT: [(&str, &str); 2] = [("slides.md", include_str!("../init/slides.md")), ("skemo.css", include_str!("../init/skemo.css"))];

/// Writes the sample presentation into `dir`, with the theme `theme`; never overwrites a file
fn init(dir: &Path, theme: &str, say: &Say) -> Result<(), String> {
    let names = glitu::themes::names();
    if !names.contains(&theme) {
        return Err(format!("unknown theme: {theme} (embedded themes: {})", names.join(", ")));
    }
    if let Some((name, _)) = INIT.iter().find(|(n, _)| dir.join(n).exists()) {
        return Err(format!("{} already exists: nothing was written", dir.join(name).display()));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (name, content) in INIT {
        let path = dir.join(name);
        let content = content.replacen("theme: crystal\n", &format!("theme: {theme}\n"), 1);
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
        say.line(format_args!("created: {}", path.display()));
    }
    say.line(format_args!("try: glitu serve {}", dir.join("slides.md").display()));
    Ok(())
}

/// The embedded themes: name, version, animated backdrop
fn themes() -> Result<(), String> {
    let root = glitu::themes::root()?;
    for name in glitu::themes::names() {
        let theme = build::LocalTheme::open(root.join(name), &glitu_core::Theme::Official { name: name.into(), version: None })?;
        let backdrop = theme.config["backdrop"].as_str().map_or(String::new(), |b| format!("  backdrop {b}"));
        println!("{name:<14} {}{backdrop}", theme.version.as_deref().unwrap_or("-"));
    }
    Ok(())
}

fn build(project: &Project, mode: Build, out: Option<PathBuf>, say: &Say) -> Result<(), String> {
    let site = build::site(project, false)?;
    let single = match mode {
        Build::Site => None,
        Build::Standalone => Some(standalone::Mode::Offline),
        Build::Web(base) => Some(standalone::Mode::Online { base, theme: published_theme(project)? }),
    };
    let Some(mode) = single else {
        let dest = out.unwrap_or_else(|| "dist".into());
        build::write(&site, &dest)?;
        say.line(format_args!("{}: {} files", dest.display(), site.len()));
        return Ok(());
    };
    let dest = out.unwrap_or_else(|| "presentation.html".into());
    let page = standalone::bundle(&site, &mode)?;
    build::write_file(&dest, page.as_bytes())?;
    say.line(format_args!("{}: standalone page, {} KB", dest.display(), page.len() / 1024));
    Ok(())
}

/// Where the presentation's theme is published, if it is (official theme)
fn published_theme(project: &Project) -> Result<Option<String>, String> {
    Ok(project.theme(&glitu_core::header(&project.source()?).theme)?.published())
}

fn unpack(page: &str, dest: &Path, force: bool, say: &Say) -> Result<(), String> {
    let html = if page.contains("://") {
        String::from_utf8_lossy(&glitu::online::download(page)?).into_owned()
    } else {
        std::fs::read_to_string(page).map_err(|e| format!("{page}: {e}"))?
    };
    let glitu::source::Extracted { files, refused } = glitu::source::extract(&html)?;
    for path in &refused {
        eprintln!("glitu: {path}: outside the presentation folder, not unpacked");
    }
    let targets: Vec<PathBuf> = files.iter().map(|(p, _)| dest.join(p)).collect();
    if let Some(t) = targets.iter().find(|t| t.exists()).filter(|_| !force) {
        return Err(format!("{} already exists (--force to replace it)", t.display()));
    }
    for (target, (_, text)) in targets.iter().zip(&files) {
        build::write_file(target, text.as_bytes())?;
        say.line(target.display());
    }
    Ok(())
}

/// The engine and the themes, each one at its version: `glitu/<version>/` and
/// `themes/<name>/<version>/`. The versions already in the output stay: publishing adds to them.
fn assets(out: &Path, themes: Option<PathBuf>, say: &Say) -> Result<(), String> {
    let themes = match themes.or_else(|| build::themes_root(Path::new("."))) {
        Some(dir) => dir,
        None => glitu::themes::root()?,
    };
    let mut n = 0;
    for published in build::runtime(&themes)? {
        let dest = out.join(&published.dir);
        build::write(&published.files, &dest)?;
        say.line(format_args!("{}: {} files", dest.display(), published.files.len()));
        n += published.files.len();
    }
    say.line(format_args!("{}: {n} files", out.display()));
    Ok(())
}
