//! The command line (clap): arguments → [`Command`], kept apart from their execution so that it
//! can be tested.

// The doc comments are the help of the command line, printed as written: plain addresses and placeholders like <NAME>
#![allow(rustdoc::bare_urls, rustdoc::invalid_html_tags)]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::add::{self, Dir, Effect, ImageLayout, Kind, Lang, Layout, PageLayout, Transition};
use crate::export::{self, Format};
use crate::serve::Range;
use crate::settings::Given;

const EXAMPLES: &str = "\
Examples:
  glitu init talk                Create a presentation in talk/
  glitu config theme paper       Change a setting of the presentation
  glitu add diagram              Add a slide with an example diagram
  glitu server talk               Preview it at http://localhost:8000
  glitu build talk               Build presentation.html
  glitu build pptx talk          Build presentation.pptx (see glitu build --help)

Documentation: https://glitu.net/docs/";

#[derive(Parser)]
#[command(
    name = "glitu",
    version = crate::build::VERSION,
    about = "Build and present slide decks written in Markdown",
    long_about = "Build and present slide decks written in Markdown.\n\n\
        glitu turns Markdown files into a presentation, with diagrams, code, formulas and \
        terminal recordings. It previews the deck with live reload, and builds a self-contained \
        HTML page, a static website, PowerPoint, PDF, PNG images or a file for glitu-play.",
    after_help = EXAMPLES,
    disable_help_subcommand = true,
    help_expected = true
)]
pub struct Cli {
    /// Print errors only
    #[arg(long, short, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Create a new presentation
    ///
    /// Creates slides.md from a template. Existing files are never overwritten: in an existing
    /// presentation, only the files requested with --css or --theme are added.
    #[command(after_help = "\
Examples:
  glitu init talk                 Create talk/slides.md with the default theme
  glitu init talk --theme paper   Use another built-in theme
  glitu init talk --css           Also create talk/style.css to customize the theme
  glitu init talk --theme mine    Also create a custom theme in talk/themes/mine/
  glitu init --list-themes        List the built-in themes")]
    Init {
        /// Directory of the presentation, created if missing
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Theme of the presentation (see --list-themes). Any other name creates a custom theme in
        /// themes/<NAME>/, copied from crystal
        #[arg(long, default_value = "crystal")]
        theme: String,
        /// Also create style.css, which lists the theme's variables for customization
        #[arg(long)]
        css: bool,
        /// List the built-in themes and exit
        #[arg(long, conflicts_with_all = ["dir", "theme", "css"])]
        list_themes: bool,
    },
    /// View or change the settings of the presentation (its front matter)
    ///
    /// Without a key, lists every setting glitu knows, with its value in the presentation. With a
    /// key, prints its value; with a key and a value, sets it, after checking the value. The rest
    /// of the file is kept as written.
    #[command(after_help = "\
Examples:
  glitu config                         List the settings and their values
  glitu config theme                   Print the theme
  glitu config theme paper             Use the paper theme
  glitu config navigation 2d           One chapter per column
  glitu config --unset logo            Remove the logo
  glitu config -f talk/ author Ada     Set the author of talk/slides.md
  glitu config --engines               List the diagram engines and their tools
  glitu config --engines d2.d2         Print an engine's manifest, to copy and change")]
    Config {
        /// Setting to print or to change
        key: Option<String>,
        /// New value of the setting
        value: Option<String>,
        /// Remove the setting
        #[arg(long, requires = "key", conflicts_with = "value")]
        unset: bool,
        /// Presentation: its markdown file or its folder [default: the current folder]
        #[arg(short, long, value_name = "PATH")]
        file: Option<PathBuf>,
        /// List the diagram engines: their tool, its version, what confines it. With an engine,
        /// print its manifest
        #[arg(long, value_name = "ENGINE", num_args = 0..=1, default_missing_value = "",
              conflicts_with_all = ["key", "unset", "file"])]
        engines: Option<String>,
    },
    /// Add a slide, filled with an example to adapt
    ///
    /// Adds a slide of the chosen kind at the end of the presentation (before its closing
    /// `{.end}` slide, if it has one), with example content showing its syntax. The terminal and
    /// image kinds also write the sample files they need (casts/, img/); an existing file is
    /// never overwritten. Each kind has its own options (glitu add code --help).
    #[command(
        subcommand_value_name = "KIND",
        subcommand_help_heading = "Kinds",
        after_help = "\
Examples:
  glitu add bullets --steps 4 --effect up          Four points, coming up one per click
  glitu add code --lang python --layout large      Python code across the width
  glitu add diagram -t \"How it works\"              A skemo diagram, under this title
  glitu add terminal -c \"cargo build\" --speed 20   A terminal recording, typed fast
  glitu add formula --chapter Maths                A new chapter, then a slide of formulas
  glitu add quote --dry-run                        Print the slide instead of adding it"
    )]
    Add(AddArgs),
    /// Build the presentation: an HTML page, or another format
    ///
    /// Without a format, builds a single HTML file that contains the engine, the theme, the fonts
    /// and the images (about 2 MB): it works offline and embeds its Markdown source, which glitu
    /// unpack can recover. Give a format for the others; each has its own options
    /// (glitu build pptx --help). All formats but html and site render the slides in a
    /// Chromium-based browser, looked up in this order: the CHROMIUM environment variable, the
    /// headless shell downloaded by glitu, then an installed Chrome, Chromium, Edge, Brave or
    /// Vivaldi; if none is found, use --download-browser.
    #[command(
        args_conflicts_with_subcommands = true,
        override_usage = "glitu build [OPTIONS] [FILES]...\n       glitu build <FORMAT> [OPTIONS] [FILES]...",
        subcommand_value_name = "FORMAT",
        subcommand_help_heading = "Formats",
        after_help = "\
Inputs: without files, the current folder: its slides.md, otherwise all its .md files merged
in natural order (01-intro.md, 02-deploy.md…), leaving out README, CHANGELOG, LICENSE and the
names starting with _ or a dot. Only the first file's front matter counts.

Examples:
  glitu build                                     Build presentation.html
  glitu build talk/ -o talk.html                  Build the presentation of another folder
  glitu build intro.md deploy.md                  Build these files, merged in this order
  glitu build --assets https://glitu.net/static   Build a lightweight page
  glitu build pptx -o talk.pptx                   Build an editable PowerPoint
  glitu build pdf --clicks                        Build a PDF, one page per click"
    )]
    Build(BuildArgs),
    /// Preview a presentation with live reload
    ///
    /// Serves the presentation locally and reloads it on every change. All open windows
    /// (projector, laptop, speaker view) stay on the same slide. Use --allow to accept devices
    /// on your network, such as a phone used as a remote control (append ?remote to the URL).
    #[command(after_help = "\
Examples:
  glitu server                           Serve the presentation of this folder at http://localhost:8000
  glitu server talk/slides.md -p 9000    Serve another file on another port
  glitu server --allow 192.168.0.0/16    Also accept devices on the local network")]
    Server {
        #[command(flatten)]
        inputs: Inputs,
        /// Port to listen on
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        /// Also accept connections from this network, in CIDR notation. Can be repeated
        #[arg(long, value_name = "CIDR")]
        allow: Vec<Range>,
    },
    /// Recover the Markdown source of a built page or of a .glituplay file
    ///
    /// Every page built by glitu, and every .glituplay file, embeds its Markdown source,
    /// including included files and imported code. Images and terminal recordings are not
    /// embedded and must be restored separately.
    #[command(after_help = "\
Examples:
  glitu unpack talk.html -o talk/                    Recover the source of a local page
  glitu unpack https://glitu.net/demo.html -o demo/  Recover the source of a published page
  glitu unpack talk.glituplay -o talk/               Recover the source of a player's file")]
    Unpack {
        /// Built page or .glituplay file: a file path or an https:// URL
        page: String,
        /// Destination directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Print the version (same as --version)
    #[command(hide = true)]
    Version,
    /// Rendering oracle of the engine's tests: a screenshot per state (slide × click)
    #[cfg(feature = "oracle")]
    #[command(subcommand)]
    Oracle(OracleCmd),
}

#[derive(Args)]
pub struct AddArgs {
    #[command(subcommand)]
    kind: AddCmd,
    #[command(flatten)]
    slide: SlideArgs,
}

/// What every kind of slide accepts
#[derive(Args)]
struct SlideArgs {
    /// Presentation: its markdown file or its folder [default: the current folder]. The slide goes
    /// into its last file
    #[arg(short, long, value_name = "PATH", global = true)]
    file: Option<PathBuf>,
    /// Title of the slide [default: the kind's]
    #[arg(short, long, global = true)]
    title: Option<String>,
    /// Open a new chapter before the slide: `# NAME`
    #[arg(long, value_name = "NAME", global = true)]
    chapter: Option<String>,
    /// Transition into the slide
    #[arg(long, value_enum, global = true)]
    transition: Option<Transition>,
    /// Background of the slide: a colour (#1e3a8a) or an image (img/x.jpg)
    #[arg(long, value_name = "COLOR|IMAGE", global = true)]
    background: Option<String>,
    /// Without the example of speaker notes
    #[arg(long, global = true)]
    no_notes: bool,
    /// Print the slide instead of adding it
    #[arg(long, global = true)]
    dry_run: bool,
}

/// The clicks of a slide
#[derive(Args)]
struct StepArgs {
    /// Number of clicks; 0: everything at once
    #[arg(short = 'n', long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(0..=9))]
    steps: u8,
    /// How each step comes in
    #[arg(long, value_enum)]
    effect: Option<Effect>,
}

/// The layout of a slide with a visual
#[derive(Args)]
struct LayoutArg {
    /// Layout of the slide
    #[arg(long, value_enum, default_value_t)]
    layout: Layout,
}

#[derive(Subcommand)]
enum AddCmd {
    /// Points that appear click by click
    #[command(visible_alias = "steps")]
    Bullets {
        #[command(flatten)]
        steps: StepArgs,
    },
    /// Code, its lines highlighted click by click (or annotated)
    Code {
        /// Language of the example
        #[arg(long, value_enum, default_value_t = Lang::Rust)]
        lang: Lang,
        /// Show the annotations instead: [!code ++] added, [!code --] removed…
        #[arg(long)]
        annotated: bool,
        #[command(flatten)]
        steps: StepArgs,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// Animated code: one version per click, the words kept slide to their new place
    #[command(visible_alias = "animated-code")]
    MagicMove {
        /// Language of the example
        #[arg(long, value_enum, default_value_t = Lang::Rust)]
        lang: Lang,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// A skemo diagram, its cards appearing click by click (needs the skemo tool:
    /// https://skemo.net/install)
    #[command(visible_alias = "skemo")]
    Diagram {
        /// Direction of the diagram
        #[arg(long, value_enum, default_value_t)]
        dir: Dir,
        #[command(flatten)]
        steps: StepArgs,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// A terminal recording (asciicast), one command per click; writes a sample recording into
    /// casts/
    #[command(visible_aliases = ["asciicast", "cast"])]
    Terminal {
        /// A command of the recording, typed then run; can be repeated [default: echo, ls]
        #[arg(short, long = "command", value_name = "COMMAND")]
        commands: Vec<String>,
        /// Typing speed of the recording, in characters per second
        #[arg(long, default_value_t = 12.0, value_name = "CHARS")]
        speed: f64,
        /// How each point of the text comes in
        #[arg(long, value_enum)]
        effect: Option<Effect>,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// Formulas in LaTeX, in the text and as the visual
    #[command(visible_alias = "math")]
    Formula {
        #[command(flatten)]
        steps: StepArgs,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// An image beside the text; without --src, writes a placeholder into img/
    Image {
        /// The image: a file of the presentation (img/photo.jpg) or an https:// address
        #[arg(long, value_name = "PATH|URL")]
        src: Option<String>,
        /// Where the image goes
        #[arg(long, value_enum, default_value_t)]
        layout: ImageLayout,
    },
    /// A video that stops at given times, each click resuming it; without --src, writes a
    /// placeholder into img/
    #[command(visible_alias = "movie")]
    Video {
        /// The video: a file of the presentation (img/demo.mp4) or an https:// address
        #[arg(long, value_name = "PATH|URL")]
        src: Option<String>,
        /// Times where the video stops, in increasing order: "0:12 1:05" [default with the
        /// placeholder: 0:02 0:04; otherwise none, the video starts on its own]
        #[arg(long, value_name = "TIMES")]
        pauses: Option<String>,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// Big emojis that replace each other, click by click
    Emoji {
        #[command(flatten)]
        steps: StepArgs,
        #[command(flatten)]
        layout: LayoutArg,
    },
    /// Two columns, separated by a ::right:: line
    Cols {
        #[command(flatten)]
        steps: StepArgs,
    },
    /// A quote in large, and its author
    Quote {
        /// Author of the quote
        #[arg(long, default_value = "Leonardo da Vinci")]
        author: String,
    },
    /// A key figure in large, with a caption
    Fact {
        /// The figure
        #[arg(long, default_value = "98 %")]
        figure: String,
    },
    /// A statement in large, its words marked click by click
    Statement {
        #[command(flatten)]
        steps: StepArgs,
    },
    /// A table of contents: the chapters, as links, generated
    Toc,
    /// A title slide in the middle of the presentation
    #[command(visible_alias = "intro")]
    Title,
    /// A divider, without a number nor a change of chapter
    #[command(visible_alias = "section")]
    Interlude,
    /// The closing slide
    End,
    /// A web page, on the whole slide or beside the text
    Iframe {
        /// Address of the page
        #[arg(long, default_value = "https://glitu.net")]
        url: String,
        /// Where the page goes
        #[arg(long, value_enum, default_value_t)]
        layout: PageLayout,
    },
    /// Two animated slides: the elements they share slide from one to the other
    Animate,
    /// A chapter: a numbered divider (`# Title`) that the next slides belong to
    Chapter,
}

/// The markdown of the presentation
#[derive(Args)]
pub struct Inputs {
    /// Markdown files of the presentation, merged in this order, or its folder [default: the
    /// current folder]
    #[arg(value_name = "FILES")]
    files: Vec<PathBuf>,
    /// Also let the presentation read the files of this folder (it reads only its own folder
    /// otherwise). Can be repeated
    #[arg(long, value_name = "DIR", help_heading = "Machine Options")]
    allow_read: Vec<PathBuf>,
    /// The program of a diagram tool, such as skemo=/opt/skemo/bin/skemo. Can be repeated
    #[arg(long, value_name = "NAME=PATH", value_parser = pair::<PathBuf>, help_heading = "Machine Options")]
    tool: Vec<(String, PathBuf)>,
    /// The engine of a diagram language, over the presentation's choice, such as mermaid=mmdc.
    /// Can be repeated
    #[arg(long, value_name = "LANGUAGE=ENGINE", value_parser = pair::<String>, help_heading = "Machine Options")]
    engine: Vec<(String, String)>,
    /// Pass the raw arguments the presentation gives to diagram tools (args=…), for this run
    #[arg(long, help_heading = "Machine Options")]
    allow_document_args: bool,
    /// Say which diagram tools run
    #[arg(short, long, help_heading = "Machine Options")]
    verbose: bool,
}

impl Inputs {
    fn given(&self) -> Given {
        Given {
            allow_read: self.allow_read.clone(),
            tools: self.tool.clone(),
            engines: self.engine.clone(),
            allow_document_args: self.allow_document_args,
            verbose: self.verbose,
        }
    }
}

/// `name=value`
fn pair<T: From<String>>(s: &str) -> Result<(String, T), String> {
    match s.split_once('=') {
        Some((n, v)) if !n.trim().is_empty() && !v.is_empty() => Ok((n.trim().to_string(), T::from(v.to_string()))),
        _ => Err(format!("`{s}`: NAME=VALUE expected")),
    }
}

#[derive(Args)]
pub struct BuildArgs {
    #[command(subcommand)]
    format: Option<FormatCmd>,
    #[command(flatten)]
    html: HtmlArgs,
}

#[derive(Subcommand)]
enum FormatCmd {
    /// A self-contained HTML page (the default)
    #[command(after_help = "\
Examples:
  glitu build html -o talk.html                        Build talk.html
  glitu build html --assets https://glitu.net/static   Build a lightweight page")]
    Html(HtmlArgs),
    /// A static website, to host on any web server
    #[command(after_help = "\
Examples:
  glitu build site               Build the website into dist/
  glitu build site talk/ -o www  Build the presentation of talk/ into www/")]
    Site(SiteArgs),
    /// An editable PowerPoint presentation, with speaker notes and animations
    #[command(after_help = "\
Examples:
  glitu build pptx -o talk.pptx          Build talk.pptx
  glitu build pptx --images              One picture per slide (exact, not editable)
  glitu build pptx --range 1-3,5         Only some slides")]
    Pptx(PptxArgs),
    /// A vector PDF with selectable text; with --clicks, one picture per click and a .pdfpc file
    /// (speaker notes for pdfpc)
    #[command(after_help = "\
Examples:
  glitu build pdf -o talk.pdf                     Build talk.pdf
  glitu build pdf --clicks --width 1024           One page per click, for pdfpc on a slow computer")]
    Pdf(PdfArgs),
    /// One PNG picture per slide, or per click, in a folder
    #[command(after_help = "\
Examples:
  glitu build png -o shots           One picture per slide in shots/
  glitu build png --clicks           One picture per click")]
    Png(PngArgs),
    /// A file for glitu-play, the native player for slow computers (.glituplay): one picture per
    /// click and its animations
    #[command(after_help = "\
Examples:
  glitu build play --width 1024 -o talk     Build talk.glituplay, then: glitu-play talk.glituplay")]
    Play(PlayArgs),
}

#[derive(Args)]
struct HtmlArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Output file [default: presentation.html]
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Where the page loads the engine and the theme from: `local` embeds them; a URL such as
    /// https://glitu.net/static produces a lightweight page that requires network access
    #[arg(long, value_name = "local|URL", default_value = "local")]
    assets: String,
}

#[derive(Args)]
struct SiteArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Output folder [default: dist]
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// What the formats rendered by the browser share
#[derive(Args)]
struct Shots {
    /// Only these slides, numbered in reading order
    #[arg(long, value_name = "1-3,5")]
    range: Option<String>,
    /// Width of the pictures, in pixels: the screen's width for a slow computer (1024)
    #[arg(long, value_name = "PIXELS", default_value_t = export::PIXELS,
          value_parser = clap::value_parser!(u32).range(64..))]
    width: u32,
    /// Download Chrome's headless shell if no browser is found (about 100 MB, once, into
    /// $GLITU_BROWSER_DIR or ~/.cache/glitu/browser). Asks for confirmation
    #[arg(long, help_heading = "Browser Options")]
    download_browser: bool,
    /// Skip the download confirmation
    #[arg(short, long, requires = "download_browser", help_heading = "Browser Options")]
    yes: bool,
}

#[derive(Args)]
struct PptxArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Output file [default: presentation.pptx]
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// One picture per slide: the exact rendering, not editable
    #[arg(long)]
    images: bool,
    #[command(flatten)]
    shots: Shots,
}

#[derive(Args)]
struct PdfArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Output file [default: presentation.pdf]
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// One page per click instead of one per slide (pictures), and the .pdfpc file
    #[arg(long)]
    clicks: bool,
    #[command(flatten)]
    shots: Shots,
}

#[derive(Args)]
struct PngArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Output folder [default: presentation]
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// One picture per click instead of one per slide
    #[arg(long)]
    clicks: bool,
    #[command(flatten)]
    shots: Shots,
}

#[derive(Args)]
struct PlayArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Output file [default: presentation.glituplay]
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[command(flatten)]
    shots: Shots,
}

#[cfg(feature = "oracle")]
#[derive(Subcommand)]
pub enum OracleCmd {
    /// Capture every state of the presentation served at this address
    Capture {
        /// URL of the served presentation
        url: String,
        /// Folder of the screenshots
        out: PathBuf,
    },
    /// Compare the presentation served at this address with the reference screenshots
    Check {
        /// URL of the served presentation
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
    Init {
        dir: PathBuf,
        theme: String,
        css: bool,
    },
    /// `glitu config`: `file`, the presentation given with `--file`
    Config {
        file: Option<PathBuf>,
        action: ConfigAction,
    },
    Add {
        file: Option<PathBuf>,
        request: add::Request,
        dry_run: bool,
    },
    /// `inputs`: the command line's files or folder (see `source::inputs`)
    Build {
        inputs: Vec<PathBuf>,
        given: Given,
        assets: Assets,
        out: PathBuf,
    },
    Serve {
        inputs: Vec<PathBuf>,
        given: Given,
        port: u16,
        allow: Vec<Range>,
    },
    /// A format rendered by the browser; `download_browser`, `yes`: see `chromium::browser`
    Export {
        inputs: Vec<PathBuf>,
        given: Given,
        out: PathBuf,
        options: export::Options,
        download_browser: bool,
        yes: bool,
    },
    /// `build site`: the static website, without a browser
    Site {
        inputs: Vec<PathBuf>,
        given: Given,
        out: PathBuf,
    },
    Unpack {
        page: String,
        out: PathBuf,
        force: bool,
    },
    /// `init --list-themes`
    Themes,
    /// `config --engines [ID]`
    Engines {
        id: Option<String>,
    },
    Version,
    #[cfg(feature = "oracle")]
    OracleCapture {
        url: String,
        out: PathBuf,
    },
    #[cfg(feature = "oracle")]
    OracleCheck {
        url: String,
        reference: PathBuf,
        threshold: f64,
    },
}

/// What `glitu config` does with the front matter
#[derive(Debug, PartialEq)]
pub enum ConfigAction {
    List,
    Get(String),
    Set(String, String),
    Unset(String),
}

/// The command written on the command line, program name included; a misuse, `--help` or
/// `--version` is a clap error, which prints itself (`Error::exit`)
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<(Command, bool), clap::Error> {
    let cli = Cli::try_parse_from(args)?;
    Ok((command(cli.cmd)?, cli.quiet))
}

fn command(cmd: Cmd) -> Result<Command, clap::Error> {
    Ok(match cmd {
        Cmd::Init { list_themes: true, .. } => Command::Themes,
        Cmd::Init { dir, theme, css, .. } => Command::Init { dir, theme, css },
        Cmd::Config { engines: Some(id), .. } => Command::Engines { id: Some(id).filter(|i| !i.is_empty()) },
        Cmd::Config { key, value, unset, file, .. } => {
            let action = match (key, value) {
                (None, _) => ConfigAction::List,
                (Some(k), _) if unset => ConfigAction::Unset(k),
                (Some(k), None) => ConfigAction::Get(k),
                (Some(k), Some(v)) => ConfigAction::Set(k, v),
            };
            Command::Config { file, action }
        }
        Cmd::Add(a) => add_command(a)?,
        Cmd::Server { inputs, port, allow } => {
            Command::Serve { given: inputs.given(), inputs: inputs.files, port, allow }
        }
        Cmd::Build(b) => build_command(b.format.unwrap_or(FormatCmd::Html(b.html)))?,
        Cmd::Unpack { page, output, force } => Command::Unpack { page, out: output, force },
        Cmd::Version => Command::Version,
        #[cfg(feature = "oracle")]
        Cmd::Oracle(OracleCmd::Capture { url, out }) => Command::OracleCapture { url, out },
        #[cfg(feature = "oracle")]
        Cmd::Oracle(OracleCmd::Check { url, reference, threshold }) => {
            Command::OracleCheck { url, reference, threshold }
        }
    })
}

/// The output file of a format: `-o`, otherwise `presentation`, given the format's extension if it
/// has none; the extension of another format is a mistake (`build -o talk.pptx`)
fn output(given: Option<PathBuf>, format: &str, ext: &str) -> Result<PathBuf, clap::Error> {
    let mut out = given.unwrap_or_else(|| PathBuf::from("presentation"));
    let other = [("html", "html"), ("htm", "html"), ("pptx", "pptx"), ("pdf", "pdf"), ("glituplay", "play")];
    match out.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase) {
        None => {
            out.set_extension(ext);
        }
        Some(e) => {
            if let Some((_, f)) = other.iter().find(|(x, f)| *x == e && *f != format) {
                let kind = match *f {
                    "html" => "an HTML page",
                    "pptx" => "a PowerPoint file",
                    "pdf" => "a PDF file",
                    _ => "a glitu-play file",
                };
                return Err(invalid(format!(
                    "{}: {kind}; to build it: glitu build {f} -o {}",
                    out.display(),
                    out.display()
                )));
            }
        }
    }
    Ok(out)
}

fn build_command(format: FormatCmd) -> Result<Command, clap::Error> {
    let export = |inputs: Inputs, out, format, clicks, images, shots: Shots| -> Result<Command, clap::Error> {
        let range = shots.range.as_deref().map(export::parse_range).transpose().map_err(invalid)?.unwrap_or_default();
        let options = export::Options { format, clicks, images, range, pixels: shots.width, progress: false };
        Ok(Command::Export {
            given: inputs.given(),
            inputs: inputs.files,
            out,
            options,
            download_browser: shots.download_browser,
            yes: shots.yes,
        })
    };
    match format {
        FormatCmd::Html(h) => {
            let assets = match h.assets.as_str() {
                "local" => Assets::Local,
                url if url.starts_with("https://") || url.starts_with("http://") => Assets::At(url.into()),
                other => return Err(invalid(format!("--assets {other}: `local`, or an http(s):// address"))),
            };
            Ok(Command::Build {
                given: h.inputs.given(),
                inputs: h.inputs.files,
                assets,
                out: output(h.output, "html", "html")?,
            })
        }
        FormatCmd::Site(a) => Ok(Command::Site {
            given: a.inputs.given(),
            inputs: a.inputs.files,
            out: a.output.unwrap_or_else(|| "dist".into()),
        }),
        FormatCmd::Pptx(a) => {
            let out = output(a.output, "pptx", "pptx")?;
            export(a.inputs, out, Format::Pptx, false, a.images, a.shots)
        }
        FormatCmd::Pdf(a) => {
            let out = output(a.output, "pdf", "pdf")?;
            export(a.inputs, out, Format::Pdf, a.clicks, false, a.shots)
        }
        FormatCmd::Png(a) => {
            let out = a.output.unwrap_or_else(|| "presentation".into());
            export(a.inputs, out, Format::Png, a.clicks, false, a.shots)
        }
        // the player goes click by click
        FormatCmd::Play(a) => {
            let out = output(a.output, "play", "glituplay")?;
            export(a.inputs, out, Format::Play, true, false, a.shots)
        }
    }
}

fn add_command(a: AddArgs) -> Result<Command, clap::Error> {
    let mut steps = 3;
    let mut effect = None;
    let mut layout = Layout::Panel;
    let mut clicks: Option<StepArgs> = None;
    let kind = match a.kind {
        AddCmd::Bullets { steps } => {
            clicks = Some(steps);
            Kind::Bullets
        }
        AddCmd::Code { lang, annotated, steps, layout: l } => {
            clicks = Some(steps);
            layout = l.layout;
            Kind::Code { lang, annotated }
        }
        AddCmd::MagicMove { lang, layout: l } => {
            layout = l.layout;
            Kind::MagicMove { lang }
        }
        AddCmd::Diagram { dir, steps, layout: l } => {
            clicks = Some(steps);
            layout = l.layout;
            Kind::Diagram { dir }
        }
        AddCmd::Terminal { commands, speed, effect: e, layout: l } => {
            if !(speed > 0.0 && speed.is_finite()) {
                return Err(invalid(format!("--speed {speed}: a number of characters per second, above 0")));
            }
            if let Some(c) = commands.iter().find(|c| c.trim().is_empty() || c.contains('\n')) {
                return Err(invalid(format!("--command {c:?}: a command on one line")));
            }
            effect = e;
            layout = l.layout;
            Kind::Terminal { commands, speed }
        }
        AddCmd::Formula { steps, layout: l } => {
            clicks = Some(steps);
            layout = l.layout;
            Kind::Formula
        }
        AddCmd::Image { src, layout } => Kind::Image { src, layout },
        AddCmd::Video { src, pauses, layout: l } => {
            layout = l.layout;
            let pauses: Vec<String> =
                pauses.unwrap_or_default().split([' ', ',']).filter(|t| !t.is_empty()).map(String::from).collect();
            let time = |t: &str| t.split(':').all(|p| !p.is_empty() && p.parse::<f64>().is_ok_and(|v| v >= 0.0));
            if let Some(t) = pauses.iter().find(|t| !time(t)) {
                return Err(invalid(format!("--pauses: `{t}` is not a time (12, 1:05, 1:02:03)")));
            }
            Kind::Video { src, pauses }
        }
        AddCmd::Emoji { steps, layout: l } => {
            clicks = Some(steps);
            layout = l.layout;
            Kind::Emoji
        }
        AddCmd::Cols { steps } => {
            clicks = Some(steps);
            Kind::Cols
        }
        AddCmd::Quote { author } => Kind::Quote { author },
        AddCmd::Fact { figure } => Kind::Fact { figure },
        AddCmd::Statement { steps } => {
            clicks = Some(steps);
            Kind::Statement
        }
        AddCmd::Toc => Kind::Toc,
        AddCmd::Title => Kind::Title,
        AddCmd::Interlude => Kind::Interlude,
        AddCmd::End => Kind::End,
        AddCmd::Iframe { url, layout } => Kind::Iframe { url, layout },
        AddCmd::Animate => Kind::Animate,
        AddCmd::Chapter => Kind::Chapter,
    };
    if let Some(c) = clicks {
        steps = c.steps as usize;
        effect = c.effect;
    }
    let s = a.slide;
    let request = add::Request {
        kind,
        title: s.title,
        chapter: s.chapter,
        transition: s.transition,
        background: s.background,
        notes: !s.no_notes,
        steps,
        effect,
        layout,
    };
    Ok(Command::Add { file: s.file, request, dry_run: s.dry_run })
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
        assert_eq!(
            cmd("init talk --theme paper --css").unwrap(),
            Command::Init { dir: "talk".into(), theme: "paper".into(), css: true }
        );
    }

    #[test]
    fn build_html_by_default() {
        let html = |inputs: &[&str], assets, out: &str| Command::Build {
            inputs: inputs.iter().map(PathBuf::from).collect(),
            given: Given::default(),
            assets,
            out: out.into(),
        };
        assert_eq!(cmd("build").unwrap(), html(&[], Assets::Local, "presentation.html"));
        assert_eq!(cmd("build talk.md --assets local -o t.html").unwrap(), html(&["talk.md"], Assets::Local, "t.html"));
        assert_eq!(
            cmd("build --assets https://x.net/ a.md b.md").unwrap(),
            html(&["a.md", "b.md"], Assets::At("https://x.net/".into()), "presentation.html")
        );
        assert_eq!(cmd("build html talk/ -o t").unwrap(), html(&["talk/"], Assets::Local, "t.html"));
        assert!(cmd("build --assets x.net").unwrap_err().to_string().contains("`local`, or an http(s)://"));
        assert!(cmd("build --standalone").is_err() && cmd("build --web https://x.net/").is_err());
        // the extension of another format: its own command
        assert!(cmd("build -o talk.pptx").unwrap_err().to_string().contains("glitu build pptx -o talk.pptx"));
        // the options of a format belong to it
        assert!(cmd("build --clicks").is_err() && cmd("build html --range 1").is_err());
    }

    #[test]
    fn build_formats() {
        let export = |line: &str| match cmd(line).unwrap() {
            Command::Export { options, out, inputs, .. } => (options.format, out, inputs),
            other => panic!("{other:?}"),
        };
        assert_eq!(export("build pptx"), (Format::Pptx, "presentation.pptx".into(), vec![]));
        assert_eq!(export("build pdf talk.md -o a"), (Format::Pdf, "a.pdf".into(), vec!["talk.md".into()]));
        assert_eq!(export("build png -o shots").1, PathBuf::from("shots"));
        assert_eq!(export("build png").1, PathBuf::from("presentation"));
        assert_eq!(export("build play -o talk").1, PathBuf::from("talk.glituplay"));
        assert!(cmd("build pdf -o a.pptx").unwrap_err().to_string().contains("glitu build pptx"));
        assert_eq!(
            cmd("build site t.md").unwrap(),
            Command::Site { inputs: vec!["t.md".into()], given: Given::default(), out: "dist".into() }
        );
        assert_eq!(
            cmd("build site -o www").unwrap(),
            Command::Site { inputs: vec![], given: Given::default(), out: "www".into() }
        );
        let Command::Export { options, .. } = cmd("build pptx --images --range 2-3").unwrap() else { panic!() };
        assert!(options.images && !options.clicks && options.range == [(2, 3)]);
        let Command::Export { options, .. } = cmd("build pdf --clicks --width 1024").unwrap() else { panic!() };
        assert!(options.clicks && options.pixels == 1024);
        let Command::Export { options, .. } = cmd("build play").unwrap() else { panic!() };
        assert!(options.clicks);
        assert!(matches!(
            cmd("build pdf --download-browser -y").unwrap(),
            Command::Export { download_browser: true, yes: true, .. }
        ));
        assert!(cmd("build pdf -y").is_err());
        // each format its options
        assert!(cmd("build pptx --clicks").is_err() && cmd("build pdf --images").is_err());
        assert!(cmd("build site --width 10").is_err() && cmd("build play --clicks").is_err());
        assert!(cmd("export -o a.pptx").is_err());
    }

    #[test]
    fn server_options() {
        let Command::Serve { port, allow, inputs, .. } =
            cmd("server -p 9000 --allow 10.0.0.0/8 --allow 192.168.0.0/16 d.md").unwrap()
        else {
            panic!()
        };
        assert_eq!((port, allow.len(), inputs), (9000, 2, vec![PathBuf::from("d.md")]));
        assert!(cmd("server -p x").is_err() && cmd("serve").is_err());
        assert!(cmd("server --allow nope").is_err());
        assert!(matches!(cmd("server").unwrap(), Command::Serve { port: 8000, .. }));
    }

    #[test]
    fn other_commands() {
        assert_eq!(
            cmd("unpack p.html -o out --force").unwrap(),
            Command::Unpack { page: "p.html".into(), out: "out".into(), force: true }
        );
        assert!(cmd("unpack").is_err());
        assert_eq!(cmd("init --list-themes").unwrap(), Command::Themes);
        assert!(cmd("themes").is_err() && cmd("init talk --list-themes").is_err());
        assert!(cmd("assets").is_err());
        assert_eq!(cmd("version").unwrap(), Command::Version);
        assert!(cmd("nope").is_err() && cmd("").is_err());
        assert_eq!(cmd("--help").unwrap_err().kind(), clap::error::ErrorKind::DisplayHelp);
        assert_eq!(cmd("-V").unwrap_err().kind(), clap::error::ErrorKind::DisplayVersion);
        assert!(parse(["glitu", "-q", "init", "--list-themes"].map(String::from)).unwrap().1);
    }

    #[test]
    fn config_actions() {
        let action = |line: &str| match cmd(line).unwrap() {
            Command::Config { action, .. } => action,
            other => panic!("{other:?}"),
        };
        assert_eq!(action("config"), ConfigAction::List);
        assert_eq!(action("config theme"), ConfigAction::Get("theme".into()));
        assert_eq!(action("config theme paper"), ConfigAction::Set("theme".into(), "paper".into()));
        assert_eq!(action("config --unset logo"), ConfigAction::Unset("logo".into()));
        assert!(matches!(cmd("config -f talk/ lang").unwrap(), Command::Config { file: Some(_), .. }));
        assert!(cmd("config --unset").is_err() && cmd("config --unset logo x").is_err());
    }

    #[test]
    fn add_kinds_and_options() {
        let req = |line: &str| match cmd(line).unwrap() {
            Command::Add { request, .. } => request,
            other => panic!("{other:?}"),
        };
        assert_eq!(req("add bullets"), add::Request::new(Kind::Bullets));
        let r = req("add code --lang python --layout large -n 2 --effect up -t Hi --transition zoom");
        assert_eq!(r.kind, Kind::Code { lang: Lang::Python, annotated: false });
        assert!(r.layout == Layout::Large && r.steps == 2 && r.effect == Some(Effect::Up));
        assert!(r.title.as_deref() == Some("Hi") && r.transition == Some(Transition::Zoom));
        // the common options before the kind too
        assert_eq!(req("add --no-notes asciicast").kind, Kind::Terminal { commands: vec![], speed: 12.0 });
        assert!(!req("add --no-notes cast").notes);
        let r = req("add terminal -c ls -c pwd --speed 20");
        assert_eq!(r.kind, Kind::Terminal { commands: vec!["ls".into(), "pwd".into()], speed: 20.0 });
        assert!(matches!(req("add skemo --dir down").kind, Kind::Diagram { dir: Dir::Down }));
        assert!(matches!(cmd("add toc --dry-run -f t/").unwrap(), Command::Add { dry_run: true, file: Some(_), .. }));
        assert!(cmd("add terminal --speed 0").is_err() && cmd("add bullets -n 12").is_err());
        assert!(cmd("add quote --steps 2").is_err() && cmd("add nope").is_err() && cmd("add").is_err());
        assert!(cmd("add image --layout large").is_err());
    }

    #[cfg(feature = "oracle")]
    #[test]
    fn oracle_commands() {
        assert_eq!(
            cmd("oracle capture http://x/ out").unwrap(),
            Command::OracleCapture { url: "http://x/".into(), out: "out".into() }
        );
        assert_eq!(
            cmd("oracle check http://x/").unwrap(),
            Command::OracleCheck { url: "http://x/".into(), reference: "tests/oracle".into(), threshold: 0.5 }
        );
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
