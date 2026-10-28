//! The command line: arguments → [`Command`], kept apart from their execution so that it can be
//! tested.

use std::path::PathBuf;

use crate::export::{self, Format};
use crate::serve::Range;

pub const USAGE: &str = "usage:
  glitu build [slides.md] [-o dist]
  glitu build [slides.md] --standalone [-o presentation.html]
  glitu build [slides.md] --web <url> [-o presentation.html]
  glitu serve [slides.md] [-p 8000] [--allow <CIDR range>]… [--rehearse]
  glitu export [slides.md] [-o presentation.pptx|.pdf|folder] [--format pptx|pdf|png] [--clicks] [--range 1-3,5] [--images]
  glitu unpack <page.html|url> [-o folder] [--force]
  glitu assets [-o static] [--themes themes]
  glitu oracle capture <url> <folder>
  glitu oracle check <url> [tests/oracle] [threshold %]
  glitu --version";

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
    Build { slides: PathBuf, mode: Build, out: Option<PathBuf> },
    Serve { slides: PathBuf, port: u16, allow: Vec<Range>, rehearse: bool },
    Export { slides: PathBuf, out: PathBuf, options: export::Options },
    Unpack { page: String, out: PathBuf, force: bool },
    /// The engine and the themes, one folder per version, for `build --web` pages
    Assets { out: PathBuf, themes: Option<PathBuf> },
    OracleCapture { url: String, out: PathBuf },
    OracleCheck { url: String, reference: PathBuf, threshold: f64 },
    Help,
    Version,
}

/// The arguments of a command, taken option by option; what remains is positional
struct Args(Vec<String>);

impl Args {
    /// Every value of a repeatable option (`--allow a --allow b`)
    fn values(&mut self, name: &str) -> Vec<String> {
        let (mut values, mut rest) = (Vec::new(), Vec::new());
        let mut it = std::mem::take(&mut self.0).into_iter();
        while let Some(a) = it.next() {
            if a == name {
                values.extend(it.next())
            } else {
                rest.push(a)
            }
        }
        self.0 = rest;
        values
    }

    /// The value of an option (`-o x`); repeated, the last one
    fn value(&mut self, name: &str) -> Option<String> {
        self.values(name).pop()
    }

    /// A flag without a value (`--standalone`)
    fn flag(&mut self, name: &str) -> bool {
        let before = self.0.len();
        self.0.retain(|a| a != name);
        self.0.len() != before
    }

    /// The presentation: the first positional argument, otherwise slides.md
    fn slides(&self) -> PathBuf {
        PathBuf::from(self.0.first().map_or("slides.md", String::as_str))
    }
}

/// The command written on the command line (without the program name)
pub fn parse(args: Vec<String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else { return Err(USAGE.into()) };
    let mut a = Args(args.collect());
    Ok(match command.as_str() {
        "build" => {
            let standalone = a.flag("--standalone");
            let web = a.value("--web");
            let out = a.value("-o").map(PathBuf::from);
            let mode = match web {
                Some(base) => Build::Web(base),
                None if standalone => Build::Standalone,
                None => Build::Site,
            };
            Command::Build { slides: a.slides(), mode, out }
        }
        "serve" => {
            let allow = a.values("--allow").iter().map(|r| r.parse::<Range>()).collect::<Result<Vec<_>, _>>()?;
            let port = a.value("-p").map(|p| p.parse().map_err(|_| format!("invalid port: {p}"))).transpose()?;
            let rehearse = a.flag("--rehearse");
            Command::Serve { slides: a.slides(), port: port.unwrap_or(8000), allow, rehearse }
        }
        "export" => export_command(a)?,
        "unpack" => {
            let force = a.flag("--force");
            let out = PathBuf::from(a.value("-o").unwrap_or_else(|| ".".into()));
            let page = a.0.first().cloned().ok_or(USAGE)?;
            Command::Unpack { page, out, force }
        }
        "assets" => {
            let out = PathBuf::from(a.value("-o").unwrap_or_else(|| "static".into()));
            Command::Assets { out, themes: a.value("--themes").map(PathBuf::from) }
        }
        "oracle" => match a.0.as_slice() {
            [cmd, url, dir, ..] if cmd == "capture" => Command::OracleCapture { url: url.clone(), out: dir.into() },
            [cmd, url, rest @ ..] if cmd == "check" => Command::OracleCheck {
                url: url.clone(),
                reference: PathBuf::from(rest.first().map_or("tests/oracle", String::as_str)),
                threshold: rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.5),
            },
            _ => return Err(USAGE.into()),
        },
        "-h" | "--help" | "help" => Command::Help,
        "-V" | "--version" | "version" => Command::Version,
        _ => return Err(USAGE.into()),
    })
}

fn export_command(mut a: Args) -> Result<Command, String> {
    let clicks = a.flag("--clicks");
    let images = a.flag("--images");
    let range = a.value("--range").as_deref().map(export::parse_range).transpose()?.unwrap_or_default();
    let format = a.value("--format");
    let out = a.value("-o");
    let format = match (&format, &out) {
        (Some(f), _) => Format::parse(f)?,
        (None, Some(o)) => Format::of(std::path::Path::new(o))
            .ok_or_else(|| format!("{o}: unknown extension (.pptx, .pdf, or a folder for PNG images)"))?,
        (None, None) => Format::Pptx,
    };
    let out = out.map_or_else(|| format.default_output(), PathBuf::from);
    Ok(Command::Export { slides: a.slides(), out, options: export::Options { format, clicks, images, range } })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(line: &str) -> Result<Command, String> {
        parse(line.split_whitespace().map(String::from).collect())
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
    }

    #[test]
    fn serve_options() {
        let Command::Serve { port, allow, rehearse, slides } =
            cmd("serve -p 9000 --allow 10.0.0.0/8 --allow 192.168.0.0/16 --rehearse d.md").unwrap()
        else {
            panic!()
        };
        assert_eq!((port, allow.len(), rehearse, slides), (9000, 2, true, PathBuf::from("d.md")));
        assert_eq!(cmd("serve -p x").unwrap_err(), "invalid port: x");
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
        assert_eq!(format("export -o shots"), (Format::Png, "shots".into()));
        assert_eq!(format("export --format png"), (Format::Png, "presentation".into()));
        assert!(cmd("export -o a.odp").unwrap_err().contains("unknown extension"));
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
        assert_eq!(cmd("assets").unwrap(), Command::Assets { out: "static".into(), themes: None });
        assert_eq!(cmd("assets -o pub --themes t").unwrap(), Command::Assets { out: "pub".into(), themes: Some("t".into()) });
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
        assert!(cmd("oracle").is_err() && cmd("nope").is_err() && parse(Vec::new()).is_err());
        assert_eq!(cmd("--help").unwrap(), Command::Help);
        assert_eq!(cmd("-V").unwrap(), Command::Version);
    }
}
