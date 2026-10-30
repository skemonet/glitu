//! The command line (clap): arguments → [`Command`], kept apart from their execution so that it
//! can be tested.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::export::{self, Format};
use crate::serve::Range;

#[derive(Parser)]
#[command(
    name = "glitu",
    version,
    about = "Presentations written in markdown, rendered by a Rust engine: build, preview, export",
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
    /// Create a sample presentation: `slides.md` and its `skemo.css` (overwrites nothing)
    Init {
        /// Destination folder (created if needed)
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Theme of the presentation (see `glitu themes`)
        #[arg(long, default_value = "crystal")]
        theme: String,
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
        /// Timed rehearsal: the speaker view times each slide and writes the durations into the
        /// markdown
        #[arg(long)]
        rehearse: bool,
    },
    /// Build the presentation: a static site, or one page (--standalone, --web)
    Build(BuildArgs),
    /// Export to an editable PowerPoint, a PDF or PNG images (needs Chromium: `CHROMIUM`)
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
    /// The engine and the themes, one folder per version, to host for `build --web` pages
    Assets {
        /// Destination folder (the versions already there stay)
        #[arg(short, long, default_value = "static")]
        output: PathBuf,
        /// Folder of the themes (default: themes/ here or above, otherwise the embedded themes)
        #[arg(long)]
        themes: Option<PathBuf>,
    },
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
    /// Output: a folder for the site (default dist), a file for one page (default
    /// presentation.html)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// One page with everything inside (engine, theme, fonts, images): it opens without a network
    #[arg(long, conflicts_with = "web")]
    standalone: bool,
    /// One light page: the engine and the theme are loaded from this address
    /// (https://static.glitu.net, or where `glitu assets` was published)
    #[arg(long, value_name = "URL")]
    web: Option<String>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// The presentation
    #[arg(default_value = "slides.md")]
    file: PathBuf,
    /// Output: .pptx, .pdf, or a folder for PNG images (default presentation.pptx)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Format, when the output does not say it
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
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
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

/// How a presentation is built
#[derive(Debug, PartialEq)]
pub enum Build {
    /// A static site: index.html, the engine, the theme, the resources
    Site,
    /// A single page with everything inside: it opens without a network
    Standalone,
    /// A single light page: the engine and the theme are loaded from this address
    Web(String),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Init { dir: PathBuf, theme: String },
    Build { slides: PathBuf, mode: Build, out: Option<PathBuf> },
    Serve { slides: PathBuf, port: u16, allow: Vec<Range>, rehearse: bool },
    Export { slides: PathBuf, out: PathBuf, options: export::Options },
    Unpack { page: String, out: PathBuf, force: bool },
    Themes,
    /// The engine and the themes, one folder per version, for `build --web` pages
    Assets { out: PathBuf, themes: Option<PathBuf> },
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
        Cmd::Init { dir, theme } => Command::Init { dir, theme },
        Cmd::Serve { file, port, allow, rehearse } => Command::Serve { slides: file, port, allow, rehearse },
        Cmd::Build(b) => {
            let mode = match b.web {
                Some(base) => Build::Web(base),
                None if b.standalone => Build::Standalone,
                None => Build::Site,
            };
            Command::Build { slides: b.file, mode, out: b.output }
        }
        Cmd::Export(e) => export_command(e)?,
        Cmd::Unpack { page, output, force } => Command::Unpack { page, out: output, force },
        Cmd::Themes => Command::Themes,
        Cmd::Assets { output, themes } => Command::Assets { out: output, themes },
        #[cfg(feature = "oracle")]
        Cmd::Oracle(OracleCmd::Capture { url, out }) => Command::OracleCapture { url, out },
        #[cfg(feature = "oracle")]
        Cmd::Oracle(OracleCmd::Check { url, reference, threshold }) => Command::OracleCheck { url, reference, threshold },
    })
}

/// The format follows `--format`, otherwise the output (extension, or a folder for PNG)
fn export_command(e: ExportArgs) -> Result<Command, clap::Error> {
    let format = match (e.format, &e.output) {
        (Some(FormatArg::Pptx), _) => Format::Pptx,
        (Some(FormatArg::Pdf), _) => Format::Pdf,
        (Some(FormatArg::Png), _) => Format::Png,
        (None, Some(o)) => Format::of(o)
            .ok_or_else(|| invalid(format!("{}: unknown extension (.pptx, .pdf, or a folder for PNG images)", o.display())))?,
        (None, None) => Format::Pptx,
    };
    let range = e.range.as_deref().map(export::parse_range).transpose().map_err(invalid)?.unwrap_or_default();
    let out = e.output.unwrap_or_else(|| format.default_output());
    let options = export::Options { format, clicks: e.clicks, images: e.images, range };
    Ok(Command::Export { slides: e.file, out, options })
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
        assert_eq!(cmd("init").unwrap(), Command::Init { dir: ".".into(), theme: "crystal".into() });
        assert_eq!(cmd("init talk --theme paper").unwrap(), Command::Init { dir: "talk".into(), theme: "paper".into() });
    }

    #[test]
    fn build_modes() {
        assert_eq!(cmd("build").unwrap(), Command::Build { slides: "slides.md".into(), mode: Build::Site, out: None });
        assert_eq!(cmd("build talk.md --standalone -o t.html").unwrap(), Command::Build {
            slides: "talk.md".into(),
            mode: Build::Standalone,
            out: Some("t.html".into())
        });
        assert_eq!(cmd("build --web https://x.net/ talk.md").unwrap(), Command::Build {
            slides: "talk.md".into(),
            mode: Build::Web("https://x.net/".into()),
            out: None
        });
        assert!(cmd("build --standalone --web https://x.net/").is_err());
    }

    #[test]
    fn serve_options() {
        let Command::Serve { port, allow, rehearse, slides } =
            cmd("serve -p 9000 --allow 10.0.0.0/8 --allow 192.168.0.0/16 --rehearse d.md").unwrap()
        else {
            panic!()
        };
        assert_eq!((port, allow.len(), rehearse, slides), (9000, 2, true, PathBuf::from("d.md")));
        assert!(cmd("serve -p x").is_err());
        assert!(cmd("serve --allow nope").is_err());
        assert!(matches!(cmd("serve").unwrap(), Command::Serve { port: 8000, rehearse: false, .. }));
    }

    #[test]
    fn export_formats_follow_the_output() {
        let format = |line: &str| match cmd(line).unwrap() {
            Command::Export { options, out, .. } => (options.format, out),
            _ => panic!(),
        };
        assert_eq!(format("export"), (Format::Pptx, "presentation.pptx".into()));
        assert_eq!(format("export -o a.pdf"), (Format::Pdf, "a.pdf".into()));
        assert_eq!(format("export --output shots"), (Format::Png, "shots".into()));
        assert_eq!(format("export --format png"), (Format::Png, "presentation".into()));
        assert!(cmd("export -o a.odp").unwrap_err().to_string().contains("unknown extension"));
        let Command::Export { options, .. } = cmd("export --clicks --images --range 2-3").unwrap() else { panic!() };
        assert!(options.clicks && options.images && options.range == [(2, 3)]);
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
        assert_eq!(cmd("assets").unwrap(), Command::Assets { out: "static".into(), themes: None });
        assert_eq!(cmd("assets -o pub --themes t").unwrap(), Command::Assets { out: "pub".into(), themes: Some("t".into()) });
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
