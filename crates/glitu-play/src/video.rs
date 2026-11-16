//! A glitu video (`glitu build video`): presented through mpv, with the script that pauses at each
//! click (`glitu.lua`, carried inside this binary). See `docs/design/video-export.md`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Options;

/// The script given to mpv
pub const SCRIPT: &str = include_str!("glitu.lua");

/// The tag `glitu build video` writes into the file
const TAG: &[u8] = b"GLITU_PLAY";

/// A Matroska file carrying the glitu tag (the tags come first: ffmpeg writes them in the header)
pub fn is_video(path: &Path) -> bool {
    let mut head = Vec::new();
    let read = fs::File::open(path).and_then(|f| f.take(256 * 1024).read_to_end(&mut head));
    read.is_ok() && marked(&head)
}

fn marked(head: &[u8]) -> bool {
    head.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) && head.windows(TAG.len()).any(|w| w == TAG)
}

/// mpv: `GLITU_PLAY_MPV`, otherwise the PATH
fn mpv() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GLITU_PLAY_MPV").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let file = if cfg!(windows) { "mpv.exe" } else { "mpv" };
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(file)).find(|p| p.is_file())
}

/// mpv's command line for this video
fn args(options: &Options, script: &Path) -> Vec<String> {
    let mut a: Vec<String> = [
        // the user's own setup and scripts cannot change the keys or the display of the show
        "--no-config",
        "--load-scripts=no",
        "--keep-open=yes",
        "--no-osc",
        "--osd-on-seek=no",
        "--hwdec=auto-safe",
        // the notes stay off the screen
        "--sid=no",
        "--force-window=yes",
        "--title=glitu-play",
    ]
    .map(String::from)
    .to_vec();
    a.push(format!("--script={}", script.display()));
    if !options.window {
        a.push("--fs".into());
    }
    if options.start > 1 {
        a.push(format!("--script-opts=glitu-start={}", options.start));
    }
    a.push("--".into());
    a.push(options.deck.display().to_string());
    a
}

/// Presents the video in mpv, until it quits
pub fn run(options: &Options) -> Result<(), String> {
    if options.speaker {
        return Err("--speaker: a video has no speaker view yet; its notes are its subtitle track `notes`".into());
    }
    let mpv = mpv().ok_or(
        "a glitu video is presented with mpv, which is not installed (https://mpv.io/installation/); \
         any video player can still show it by chapters, one per click",
    )?;
    let script = std::env::temp_dir().join(format!("glitu-play-{}.lua", std::process::id()));
    fs::write(&script, SCRIPT).map_err(|e| format!("{}: {e}", script.display()))?;
    let status = Command::new(&mpv).args(args(options, &script)).status();
    let _ = fs::remove_file(&script);
    let status = status.map_err(|e| format!("{}: {e}", mpv.display()))?;
    if !status.success() {
        return Err(format!("mpv stopped with an error ({status})"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glitu_video_is_recognised() {
        let mut head = vec![0x1a, 0x45, 0xdf, 0xa3, 0, 0];
        assert!(!marked(&head));
        head.extend_from_slice(b"..GLITU_PLAY..");
        assert!(marked(&head));
        assert!(!marked(b"PK\x03\x04GLITU_PLAY"));
    }

    #[test]
    fn options_carry_over_to_mpv() {
        let o =
            |window, start| Options { deck: PathBuf::from("talk.mkv"), window, speaker: false, check: false, start };
        let a = args(&o(false, 1), Path::new("/tmp/g.lua"));
        assert!(a.contains(&"--fs".to_string()) && a.contains(&"--script=/tmp/g.lua".to_string()));
        assert_eq!(a.last().map(String::as_str), Some("talk.mkv"));
        let a = args(&o(true, 3), Path::new("/tmp/g.lua"));
        assert!(!a.contains(&"--fs".to_string()) && a.contains(&"--script-opts=glitu-start=3".to_string()));
        assert!(SCRIPT.contains("GLITU_PLAY"));
    }
}
