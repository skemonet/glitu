//! glitu-play: presents the file of `glitu build play` (`.glituplay`) without a browser, for
//! computers too slow for one. The browser has laid out every click at export; the player shows
//! these images and animates, by software, only what moves (see `scene`).

mod backdrop;
mod check;
mod deck;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
mod keymap;
mod loader;
mod player;
mod resample;
mod scene;
mod show;
mod skia;
mod speaker;
mod text;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
mod wayland;

use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "Presents a glitu presentation exported for slow computers, without a browser

Usage: glitu-play [OPTIONS] <FILE>

  <FILE>  presentation of `glitu build play` (.glituplay)

Options:
  --window           Start in a window instead of full screen
  --speaker          Open the speaker view (full screen on the other screen when there are two)
  --check            Compare the native backdrop with the export's screenshots, then quit
  --start <SLIDE>    Start at this slide (from 1)
  -h, --help         Print help
  -V, --version      Print version

Keys:
  → ↓ Space Enter PageDown n, click   next
  ← ↑ Backspace PageUp p              previous
  Home End                            first, last
  f F11                               full screen
  Esc                                 leave full screen
  s                                   speaker view: open, close
  t, r                                timer: pause or resume, back to zero
  + -                                 size of the notes
  q                                   quit";

/// The released binaries get it from the release tag (`GLITU_BUILD_VERSION`, set by the CI);
/// other builds say the workspace's
const VERSION: &str = match option_env!("GLITU_BUILD_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

pub struct Options {
    pub deck: PathBuf,
    pub window: bool,
    pub speaker: bool,
    pub check: bool,
    /// Slide to start at, from 1
    pub start: usize,
}

fn parse(args: &[String]) -> Result<Option<Options>, String> {
    let (mut deck, mut window, mut speaker, mut check, mut start) = (None, false, false, false, 1);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("glitu-play {VERSION}");
                return Ok(None);
            }
            "--window" => window = true,
            "--speaker" => speaker = true,
            "--check" => check = true,
            "--start" => {
                let v = it.next().ok_or("--start: a slide number is expected")?;
                start = v.parse().ok().filter(|&n| n >= 1).ok_or(format!("--start: invalid slide number: {v}"))?;
            }
            s if s.starts_with('-') => return Err(format!("unknown option: {s} (see --help)")),
            s if deck.is_none() => deck = Some(PathBuf::from(s)),
            s => return Err(format!("unexpected argument: {s}")),
        }
    }
    let deck = deck.ok_or("the .glituplay file of the presentation is expected (see --help)")?;
    Ok(Some(Options { deck, window, speaker, check, start }))
}

/// On Wayland, our own windows when the compositor can enlarge a buffer (`wayland`); else winit
fn play(options: Options) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    if let Some(done) = wayland::run(&options) {
        return done;
    }
    player::run(options)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let run = parse(&args).and_then(|o| match o {
        Some(o) if o.check => match check::run(&o.deck) {
            Ok(true) => Ok(()),
            Ok(false) => Err(format!("the backdrop differs by more than {} % on some clicks", check::LIMIT)),
            Err(e) => Err(e),
        },
        Some(o) => play(o),
        None => Ok(()),
    });
    match run {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("glitu-play: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn options() {
        let o = parse(&args("talk --window --start 3")).unwrap().unwrap();
        assert_eq!((o.deck, o.window, o.speaker, o.start), (PathBuf::from("talk"), true, false, 3));
        assert!(parse(&args("--speaker talk")).unwrap().unwrap().speaker);
        assert!(parse(&args("")).is_err());
        assert!(parse(&args("talk --start 0")).is_err());
        assert!(parse(&args("talk --gpu")).is_err());
        assert!(parse(&args("a b")).is_err());
    }
}
