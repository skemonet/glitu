//! glitu: presentations written in markdown.
//!
//!   glitu build [slides.md] [-o dist]     builds the static site
//!   glitu build [slides.md] --standalone [-o page.html]
//!                                        a single page with everything inside: opens offline
//!   glitu build [slides.md] --web <url> [-o page.html]
//!                                        a single light page: engine and theme loaded from <url>
//!   glitu serve [slides.md] [-p 8000] [--allow 192.168.0.0/16]… [--rehearse]
//!                                        previews, reloads on every change, keeps the windows
//!                                        together; --allow: address ranges allowed besides the
//!                                        machine; --rehearse: timed rehearsal
//!   glitu export [slides.md] [-o presentation.pptx|.pdf|folder] [--format pptx|pdf|png]
//!                [--clicks] [--range 1-3,5] [--images]
//!                                        editable PowerPoint, PDF or PNG images; --clicks: one
//!                                        slide per click; --images: PowerPoint made of screenshots
//!   glitu unpack <page.html|url> [-o folder] [--force]
//!                                        takes the markdown copied in a built page back out, to
//!                                        edit it; --force: replaces existing files
//!   glitu assets [-o static] [--themes themes]
//!                                        the engine and the themes to host for --web pages
//!   glitu oracle capture <url> <folder>   captures every state (slide × click) of a presentation
//!   glitu oracle check <url> [oracle]     compares the rendering with the oracle (tests/oracle)

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use glitu::build::{self, Project, VERSION};
use glitu::cli::{self, Build, Command, USAGE};
use glitu::{oracle, serve, standalone};

fn main() -> ExitCode {
    glitu_core::skemo::speak_english();
    match cli::parse(std::env::args().skip(1).collect()).and_then(run) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("glitu: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `false`: the command ran but failed its check (the oracle)
fn run(command: Command) -> Result<bool, String> {
    match command {
        Command::Build { slides, mode, out } => build(&Project::new(&slides), mode, out)?,
        Command::Serve { slides, port, allow, rehearse } => serve::serve(Project::new(&slides), port, allow, rehearse)?,
        Command::Export { slides, out, options } => {
            let n = glitu::export::export(&Project::new(&slides), &out, &options)?;
            println!("{}: {n} {}", out.display(), options.format.unit());
        }
        Command::Unpack { page, out, force } => unpack(&page, &out, force)?,
        Command::Assets { out, themes } => assets(&out, themes)?,
        Command::OracleCapture { url, out } => {
            oracle::capture(&url, &out)?;
        }
        Command::OracleCheck { url, reference, threshold } => return oracle::check(&url, &reference, threshold),
        Command::Help => println!("{USAGE}"),
        Command::Version => println!("glitu {VERSION}"),
    }
    Ok(true)
}

fn build(project: &Project, mode: Build, out: Option<PathBuf>) -> Result<(), String> {
    let site = build::site(project, false)?;
    let single = match mode {
        Build::Site => None,
        Build::Standalone => Some(standalone::Mode::Offline),
        Build::Web(base) => Some(standalone::Mode::Online { base, theme: published_theme(project)? }),
    };
    let Some(mode) = single else {
        let dest = out.unwrap_or_else(|| "dist".into());
        build::write(&site, &dest)?;
        println!("{}: {} files", dest.display(), site.len());
        return Ok(());
    };
    let dest = out.unwrap_or_else(|| "presentation.html".into());
    let page = standalone::bundle(&site, &mode)?;
    build::write_file(&dest, page.as_bytes())?;
    println!("{}: standalone page, {} KB", dest.display(), page.len() / 1024);
    Ok(())
}

/// Where the presentation's theme is published, if it is (official theme)
fn published_theme(project: &Project) -> Result<Option<String>, String> {
    Ok(project.theme(&glitu_core::header(&project.source()?).theme)?.published())
}

fn unpack(page: &str, dest: &Path, force: bool) -> Result<(), String> {
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
        println!("{}", target.display());
    }
    Ok(())
}

/// The engine and the themes, each one at its version: `glitu/<version>/` and
/// `themes/<name>/<version>/`. The versions already in the output stay: publishing adds to them.
fn assets(out: &Path, themes: Option<PathBuf>) -> Result<(), String> {
    let themes = themes
        .or_else(|| build::themes_root(Path::new(".")))
        .ok_or("no themes/ folder here or above (--themes)")?;
    let mut n = 0;
    for published in build::runtime(&themes)? {
        let dest = out.join(&published.dir);
        build::write(&published.files, &dest)?;
        println!("{}: {} files", dest.display(), published.files.len());
        n += published.files.len();
    }
    println!("{}: {n} files", out.display());
    Ok(())
}
