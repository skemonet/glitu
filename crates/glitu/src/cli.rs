//! The command line (clap): arguments → [`Command`], kept apart from their execution so that it
//! can be tested.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::export::{self, Format};
use crate::serve::Range;

#[derive(Parser)]
#[command(
    name = "glitu",
    version = crate::build::VERSION,
    about = "Presentations written in markdown, rendered by a Rust engine: preview, build, export",
    after_help = "Start a presentation: glitu init talk && glitu serve talk/slides.md",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Only show errors
    #[arg(long, short, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Create a sample presentation, `slides.md`; with --css, its `style.css` too (overwrites
    /// nothing)
    Init {
        /// Destination folder (created if needed)
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Theme of a new presentation (see `glitu themes`); another name creates your own theme,
        /// `themes/<name>/`, a copy of crystal to change
        #[arg(long, default_value = "crystal")]
        theme: String,
        /// Also write `style.css`: every variable of the presentation's theme, commented out, to
        /// adjust it (only this file if `slides.md` exists)
        #[arg(long)]
        css: bool,
    },
    /// Preview: serves the presentation, reloads it at every save, keeps the open windows together
    Serve {
        /// The presentation
        #[arg(default_value = "slides.md")]
        file: PathBuf,
        /// Port of the server
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        /// Also serve these addresses besides this machine: phone, tablet, remote (repeatable)
        #[arg(long, value_name = "CIDR")]
        allow: Vec<Range>,
    },
    /// Build the presentation: one page, everything inside (engine, theme, fonts, images)
    Build(BuildArgs),
    /// Export to a static website, an editable PowerPoint, a PDF or PNG images (all but the
    /// website need a Chromium-based browser: found on the machine, CHROMIUM, or --download-browser)
    Export(ExportArgs),
    /// Get the markdown back out of a built page, to edit the presentation
    Unpack {
        /// A page built by glitu, a file or an https:// address
        page: String,
        /// Destination folder
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Replace existing files
        #[arg(long)]
        force: bool,
    },
    /// List the themes embedded in the binary
    Themes,
    /// Print the version (same as --version)
    #[command(hide = true)]
    Version,
    /// Rendering oracle of the engine's tests: a screenshot per state (slide × click)
    #[cfg(feature = "oracle")]
    #[command(subcommand)]
    Oracle(OracleCmd),
}

#[derive(Args)]
pub struct BuildArgs {
    /// The presentation
    #[arg(default_value = "slides.md")]
    file: PathBuf,
    /// The page
    #[arg(short, long, default_value = "presentation.html")]
    output: PathBuf,
    /// Where the page takes the engine and the theme from: `local`, written inside (about 2 MB, it
    /// opens without a network), or an address where they are published, for a light page
    /// (https://glitu.net/static)
    #[arg(long, value_name = "local|URL", default_value = "local")]
    assets: String,
}

#[derive(Args)]
pub struct ExportArgs {
    /// The presentation
    #[arg(default_value = "slides.md")]
    file: PathBuf,
    /// Output: .pptx, .pdf, or a folder with --format site or png (default presentation.pptx)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Format, when the output's extension does not say it
    #[arg(long, value_enum)]
    format: Option<FormatArg>,
    /// One slide per click rather than per slide
    #[arg(long)]
    clicks: bool,
    /// Only these slides, in reading order: 1-3,5
    #[arg(long, value_name = "SLIDES")]
    range: Option<String>,
    /// PowerPoint made of screenshots (faithful, not editable)
    #[arg(long)]
    images: bool,
    /// Without a Chromium-based browser on this machine: download Chrome's headless shell (once,
    /// about 100 MB, into GLITU_BROWSER_DIR or ~/.cache/glitu/browser), after a confirmation
    #[arg(long)]
    download_browser: bool,
    /// Accept the download without a question (scripts, CI)
    #[arg(short, long, requires = "download_browser")]
    yes: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    /// A static website (index.html, the engine, the theme, the resources), to host anywhere
    Site,
    Pptx,
    Pdf,
    Png,
}

#[cfg(feature = "oracle")]
#[derive(Subcommand)]
pub enum OracleCmd {
    /// Capture every state of the presentation served at this address
    Capture {
        url: String,
        /// Folder of the screenshots
        out: PathBuf,
    },
    /// Compare the presentation served at this address with the reference screenshots
    Check {
        url: String,
        /// Folder of the reference screenshots
        #[arg(default_value = "tests/oracle")]
        reference: PathBuf,
        /// Percentage of differing pixels above which a state fails
        #[arg(default_value_t = 0.5)]
        threshold: f64,
    },
}

/// Where a built page takes the engine and the theme from
#[derive(Debug, PartialEq)]
pub enum Assets {
    /// Everything in the page: it opens without a network
    Local,
    /// A light page: the engine and the theme are loaded from this address
    At(String),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Init { dir: PathBuf, theme: String, css: bool },
    Build { slides: PathBuf, assets: Assets, out: PathBuf },
    Serve { slides: PathBuf, port: u16, allow: Vec<Range> },
    /// `download_browser`, `yes`: see `chromium::browser`
    Export { slides: PathBuf, out: PathBuf, options: export::Options, download_browser: bool, yes: bool },
    /// `export --format site`: the static website, without a browser
    Site { slides: PathBuf, out: PathBuf },
    Unpack { page: String, out: PathBuf, force: bool },
    Themes,
    Version,
    #[cfg(feature = "oracle")]
    OracleCapture { url: String, out: PathBuf },
    #[cfg(feature = "oracle")]
    OracleCheck { url: String, reference: PathBuf, threshold: f64 },
}

/// The command written on the command line, program name included; a misuse, `--help` or
/// `--version` is a clap error, which prints itself (`Error::exit`)
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<(Command, bool), clap::Error> {
    let cli = Cli::try_parse_from(args)?;
    Ok((command(cli.cmd)?, cli.quiet))
}

fn command(cmd: Cmd) -> Result<Command, clap::Error> {
    Ok(match cmd {
        Cmd::Init { dir, theme, css } => Command::Init { dir, theme, css },
        Cmd::Serve { file, port, allow } => Command::Serve { slides: file, port, allow },
        Cmd::Build(b) => {
            let assets = match b.assets.as_str() {
                "local" => Assets::Local,
                url if url.starts_with("https://") || url.starts_with("http://") => Assets::At(url.into()),
                other => return Err(invalid(format!("--assets {other}: `local`, or an http(s):// address"))),
            };
            Command::Build { slides: b.file, assets, out: b.output }
        }
        Cmd::Export(e) => export_command(e)?,
        Cmd::Unpack { page, output, force } => Command::Unpack { page, out: output, force },
        Cmd::Themes => Command::Themes,
        Cmd::Version => Command::Version,
        #[cfg(feature = "oracle")]
        Cmd::Oracle(OracleCmd::Capture { url, out }) => Command::OracleCapture { url, out },
        #[cfg(feature = "oracle")]
        Cmd::Oracle(OracleCmd::Check { url, reference, threshold }) => Command::OracleCheck { url, reference, threshold },
    })
}

/// The format follows `--format`, otherwise the output's extension
fn export_command(e: ExportArgs) -> Result<Command, clap::Error> {
    let format = match (e.format, &e.output) {
        (Some(FormatArg::Site), out) => {
            return Ok(Command::Site { slides: e.file, out: out.clone().unwrap_or_else(|| "dist".into()) });
        }
        (Some(FormatArg::Pptx), _) => Format::Pptx,
        (Some(FormatArg::Pdf), _) => Format::Pdf,
        (Some(FormatArg::Png), _) => Format::Png,
        (None, Some(o)) => Format::of(o).ok_or_else(|| {
            invalid(match o.extension() {
                Some(_) => format!("{}: unknown extension (.pptx, .pdf, or a folder with --format site or png)", o.display()),
                None => format!("{}: a folder, for a website (--format site) or PNG images (--format png)?", o.display()),
            })
        })?,
        (None, None) => Format::Pptx,
    };
    let range = e.range.as_deref().map(export::parse_range).transpose().map_err(invalid)?.unwrap_or_default();
    let out = e.output.unwrap_or_else(|| format.default_output());
    let options = export::Options { format, clicks: e.clicks, images: e.images, range };
    Ok(Command::Export { slides: e.file, out, options, download_browser: e.download_browser, yes: e.yes })
}

/// A value clap accepted but glitu cannot use
fn invalid(msg: String) -> clap::Error {
    clap::Error::raw(clap::error::ErrorKind::InvalidValue, format!("{msg}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(line: &str) -> Result<Command, clap::Error> {
        parse(std::iter::once("glitu").chain(line.split_whitespace()).map(String::from)).map(|(c, _)| c)
    }

    #[test]
    fn init_defaults() {
        assert_eq!(cmd("init").unwrap(), Command::Init { dir: ".".into(), theme: "crystal".into(), css: false });
        assert_eq!(cmd("init talk --theme paper --css").unwrap(), Command::Init {
            dir: "talk".into(),
            theme: "paper".into(),
            css: true
        });
    }

    #[test]
    fn build_assets() {
        assert_eq!(cmd("build").unwrap(), Command::Build {
            slides: "slides.md".into(),
            assets: Assets::Local,
            out: "presentation.html".into()
        });
        assert_eq!(cmd("build talk.md --assets local -o t.html").unwrap(), Command::Build {
            slides: "talk.md".into(),
            assets: Assets::Local,
            out: "t.html".into()
        });
        assert_eq!(cmd("build --assets https://x.net/ talk.md").unwrap(), Command::Build {
            slides: "talk.md".into(),
            assets: Assets::At("https://x.net/".into()),
            out: "presentation.html".into()
        });
        assert!(cmd("build --assets x.net").unwrap_err().to_string().contains("`local`, or an http(s)://"));
        assert!(cmd("build --standalone").is_err() && cmd("build --web https://x.net/").is_err());
    }

    #[test]
    fn serve_options() {
        let Command::Serve { port, allow, slides } =
            cmd("serve -p 9000 --allow 10.0.0.0/8 --allow 192.168.0.0/16 d.md").unwrap()
        else {
            panic!()
        };
        assert_eq!((port, allow.len(), slides), (9000, 2, PathBuf::from("d.md")));
        assert!(cmd("serve -p x").is_err());
        assert!(cmd("serve --allow nope").is_err());
        assert!(matches!(cmd("serve").unwrap(), Command::Serve { port: 8000, .. }));
    }

    #[test]
    fn export_formats_follow_the_output() {
        let format = |line: &str| match cmd(line).unwrap() {
            Command::Export { options, out, .. } => (options.format, out),
            _ => panic!(),
        };
        assert_eq!(format("export"), (Format::Pptx, "presentation.pptx".into()));
        assert_eq!(format("export -o a.pdf"), (Format::Pdf, "a.pdf".into()));
        assert_eq!(format("export --output shots --format png"), (Format::Png, "shots".into()));
        assert_eq!(format("export --format png"), (Format::Png, "presentation".into()));
        assert!(cmd("export -o a.odp").unwrap_err().to_string().contains("unknown extension"));
        assert!(cmd("export -o shots").unwrap_err().to_string().contains("--format site"));
        assert_eq!(cmd("export t.md --format site").unwrap(), Command::Site { slides: "t.md".into(), out: "dist".into() });
        assert_eq!(cmd("export --format site -o www").unwrap(), Command::Site { slides: "slides.md".into(), out: "www".into() });
        let Command::Export { options, .. } = cmd("export --clicks --images --range 2-3").unwrap() else { panic!() };
        assert!(options.clicks && options.images && options.range == [(2, 3)]);
        assert!(matches!(cmd("export --download-browser -y").unwrap(), Command::Export { download_browser: true, yes: true, .. }));
        assert!(cmd("export -y").is_err());
    }

    #[test]
    fn other_commands() {
        assert_eq!(cmd("unpack p.html -o out --force").unwrap(), Command::Unpack {
            page: "p.html".into(),
            out: "out".into(),
            force: true
        });
        assert!(cmd("unpack").is_err());
        assert_eq!(cmd("themes").unwrap(), Command::Themes);
        assert!(cmd("assets").is_err());
        assert_eq!(cmd("version").unwrap(), Command::Version);
        assert!(cmd("nope").is_err() && cmd("").is_err());
        assert_eq!(cmd("--help").unwrap_err().kind(), clap::error::ErrorKind::DisplayHelp);
        assert_eq!(cmd("-V").unwrap_err().kind(), clap::error::ErrorKind::DisplayVersion);
        assert!(parse(["glitu", "-q", "themes"].map(String::from)).unwrap().1);
    }

    #[cfg(feature = "oracle")]
    #[test]
    fn oracle_commands() {
        assert_eq!(cmd("oracle capture http://x/ out").unwrap(), Command::OracleCapture {
            url: "http://x/".into(),
            out: "out".into()
        });
        assert_eq!(cmd("oracle check http://x/").unwrap(), Command::OracleCheck {
            url: "http://x/".into(),
            reference: "tests/oracle".into(),
            threshold: 0.5
        });
        assert!(matches!(cmd("oracle check http://x/ ref 2").unwrap(), Command::OracleCheck { threshold: 2.0, .. }));
    }

    #[cfg(not(feature = "oracle"))]
    #[test]
    fn no_oracle_in_the_released_binary() {
        assert!(cmd("oracle check http://x/").is_err());
    }

    #[test]
    fn the_command_line_is_well_formed() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
