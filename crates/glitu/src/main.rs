//! glitu: presentations written in markdown. `glitu --help` lists the commands, `glitu <command>
//! --help` their options (see `cli`).
//!
//! Exit codes: 0 = OK, 1 = the command failed (or the oracle found differences), 2 = misuse.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use glitu::build::{self, Project};
use glitu::cli::{self, Assets, Command};
use glitu::{serve, standalone};

/// A line on the standard output; a closed output (`glitu themes | head -1`) ends the program
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
        Command::Init { dir, theme, css } => init(&dir, &theme, css, say)?,
        Command::Build { slides, assets, out } => build(&Project::new(&slides), assets, &out, say)?,
        Command::Serve { slides, port, allow } => serve::serve(Project::new(&slides), port, allow)?,
        Command::Export { slides, out, options, download_browser, yes } => {
            let browser = glitu::chromium::browser(download_browser, yes)?;
            let n = glitu::export::export(&Project::new(&slides), &out, &options, &browser)?;
            say.line(format_args!("{}: {n} {}", out.display(), options.format.unit()));
        }
        Command::Site { slides, out } => {
            let site = build::site(&Project::new(&slides), false)?;
            build::write(&site, &out)?;
            say.line(format_args!("{}: website, {} files", out.display(), site.len()));
        }
        Command::Unpack { page, out, force } => unpack(&page, &out, force, say)?,
        Command::Themes => themes()?,
        Command::Version => out(format_args!("glitu {}", env!("CARGO_PKG_VERSION"))),
        #[cfg(feature = "oracle")]
        Command::OracleCapture { url, out } => {
            glitu::oracle::capture(&url, &out)?;
        }
        #[cfg(feature = "oracle")]
        Command::OracleCheck { url, reference, threshold } => return glitu::oracle::check(&url, &reference, threshold),
    }
    Ok(true)
}

/// `glitu init`: what it wrote, then how to see it, and how to adjust the theme
fn init(dir: &Path, theme: &str, css: bool, say: &Say) -> Result<(), String> {
    let written = glitu::init::run(dir, theme, css)?;
    let own = dir.join("themes").join(theme);
    if written.iter().any(|p| p.starts_with(&own)) {
        say.line(format_args!("created: {}/ (your theme: theme.css, glitu.json, skemo.css, fonts)", own.display()));
        if !written.iter().any(|p| p.ends_with("slides.md")) {
            say.line(format_args!("to use it, in the front matter of the presentation: theme: ./themes/{theme}"));
        }
    }
    for path in written.iter().filter(|p| !p.starts_with(&own)) {
        say.line(format_args!("created: {}", path.display()));
    }
    say.line(format_args!("try: glitu serve {}", dir.join("slides.md").display()));
    if !written.iter().any(|p| p.ends_with("style.css")) && !dir.join("style.css").exists() {
        say.line(format_args!(
            "tip: glitu init {} --css writes style.css, every variable of the theme (colours, fonts, sizes) to customise it",
            dir.display()
        ));
    }
    Ok(())
}

/// The embedded themes, one per line: name, version, kind, animated backdrop
fn themes() -> Result<(), String> {
    let root = glitu::themes::root()?;
    out(format_args!("{:<14} {:<8} {:<10} BACKDROP", "THEME", "VERSION", "KIND"));
    for name in glitu::themes::names() {
        let theme = build::LocalTheme::open(root.join(name), &glitu_core::Theme::Official { name: name.into(), version: None })?;
        let kind = if theme.config["template"].as_bool() == Some(true) { "template" } else { "reveal.js" };
        let backdrop = theme.config["backdrop"].as_str().unwrap_or("-");
        let default = match name {
            "crystal" => "  (default)",
            "black" => "  (default for reveal-md markdown)",
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
    say.line(format_args!("{}: {} KB", dest.display(), page.len() / 1024));
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
