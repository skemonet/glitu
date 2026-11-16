//! The video export (`glitu build video`, see `docs/design/video-export.md`): the standalone page
//! is rendered frame by frame by Chrome's headless shell, on a virtual clock, so that the video does
//! not depend on the speed of the machine; ffmpeg encodes the frames. Each click state is a chapter
//! (`slide.click`) that starts with a hold, the state frozen: a player that pauses on entering a
//! chapter (glitu-play, through mpv) lands on it, whatever its delay.

use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use serde_json::{Value, json};

use crate::browser::{self, Browser};
use crate::build::{self, Project};
use crate::export::{HIDE_CONTROLS, STATE, Status, TempPage, wanted};

/// The tag that marks a glitu video (the players' scripts act only on it)
pub const TAG: &str = "GLITU_PLAY";
/// The longest a click may play (a looping video would record forever)
const LIMIT: f64 = 60.0;

#[derive(Debug, PartialEq)]
pub struct Options {
    /// Frames per second
    pub fps: u32,
    /// The hold at the start of each chapter, in seconds
    pub hold: f64,
    /// Numbers of the slides kept (from 1); empty: all of them
    pub range: Vec<(usize, usize)>,
    /// Width of the video, in pixels
    pub pixels: u32,
    /// Show the slide being rendered (on a terminal, without --quiet)
    pub progress: bool,
}

/// The page plays on the virtual clock (`__gv.advance`, once per frame):
/// - its videos: `play()` and `pause()` only mark them, each frame moves the marked ones forward,
///   waiting for the image of their new time;
/// - its CSS animations and transitions (they follow the real clock otherwise, and the frames take
///   longer to render than they last): each one is paused, then moved forward by the frame's time,
///   and finished at its end (its `finish` event, its promise).
const VIRTUAL_MEDIA: &str = "(() => {
  const P = HTMLMediaElement.prototype, playing = new Set();
  P.play = function () { playing.add(this); this.dispatchEvent(new Event('play')); return Promise.resolve(); };
  P.pause = function () {
    if (playing.delete(this)) this.dispatchEvent(new Event('pause'));
  };
  Object.defineProperty(P, 'paused', { get() { return !playing.has(this); } });
  // the animations moved by hand: their time, in ms
  const clock = new Map();
  const end = a => a.effect?.getComputedTiming().endTime ?? Infinity;
  window.__gv = {
    playing,
    // a finite animation still to play
    busy() {
      for (const a of clock.keys()) if (Number.isFinite(end(a))) return true;
      return false;
    },
    advance(dt) {
      for (const a of document.getAnimations()) {
        if (!clock.has(a)) {
          if (a.playState !== 'running') continue;
          // started since the last frame: from its start
          clock.set(a, Math.min(a.currentTime ?? 0, 0));
        }
      }
      for (const [a, t0] of [...clock]) {
        if (a.playState === 'idle' || a.playState === 'finished') { clock.delete(a); continue; }
        const t = t0 + dt * 1000, e = end(a);
        if (t >= e) { clock.delete(a); a.finish(); continue; }
        a.pause();
        a.currentTime = t;
        clock.set(a, t);
      }
      for (const v of [...playing]) {
        if (v.readyState < 1) continue;
        const end = v.duration || Infinity;
        let t = v.currentTime + dt;
        if (t >= end) {
          if (v.loop) t %= end;
          else { t = end; playing.delete(v); v.dispatchEvent(new Event('ended')); }
        }
        v.currentTime = t;
      }
    },
    seeking() { return [...document.querySelectorAll('video')].some(v => v.seeking); },
  };
})()";

/// The click has played: nothing left to move, but the animated backdrop (no finite animation
/// running, no terminal typing, no video playing)
const IDLE: &str = "(() => {
  if (document.querySelector('[data-playing]')) return false;
  if (window.__gv && (__gv.playing.size || __gv.busy())) return false;
  return !document.getAnimations().some(a => a.playState === 'running'
    && Number.isFinite(a.effect?.getComputedTiming().endTime ?? Infinity));
})()";

/// A chapter: its first frame, its last (excluded), its name and the slide's notes
#[derive(Debug, PartialEq)]
struct Chapter {
    start: u64,
    end: u64,
    name: String,
    notes: String,
}

/// ffmpeg, encoding the frames it receives as PNG on its input
struct Encoder {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: u64,
    last: Vec<u8>,
}

impl Encoder {
    fn start(ffmpeg: &Path, fps: u32, out: &Path) -> Result<Encoder, String> {
        let mut child = Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "image2pipe", "-c:v", "png", "-framerate"])
            .arg(fps.to_string())
            .args(["-i", "-", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", "-preset", "medium"])
            // a keyframe each second: going back a click lands quickly
            .args(["-g"])
            .arg(fps.to_string())
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            // its messages are read if it fails (`finish`)
            .stderr(fs::File::create(out.with_extension("log")).map(Stdio::from).unwrap_or_else(|_| Stdio::null()))
            .spawn()
            .map_err(|e| format!("{}: {e} (give its path: --tool ffmpeg=PATH)", ffmpeg.display()))?;
        let stdin = child.stdin.take();
        Ok(Encoder { child, stdin, frames: 0, last: Vec::new() })
    }

    /// A frame: a new image, or the previous one again (`None`: nothing changed)
    fn push(&mut self, png: Option<Vec<u8>>) -> Result<(), String> {
        if let Some(png) = png {
            self.last = png;
        }
        if self.last.is_empty() {
            return Ok(());
        }
        let stdin = self.stdin.as_mut().ok_or("ffmpeg stopped")?;
        stdin.write_all(&self.last).map_err(|e| format!("ffmpeg: {e}"))?;
        self.frames += 1;
        Ok(())
    }

    /// The end of the frames; `out`: the file being written, its messages beside it
    fn finish(mut self, out: &Path) -> Result<(), String> {
        drop(self.stdin.take());
        let status = self.child.wait().map_err(|e| format!("ffmpeg: {e}"));
        let log = out.with_extension("log");
        let said = fs::read_to_string(&log).unwrap_or_default();
        let _ = fs::remove_file(&log);
        let status = status?;
        if !status.success() {
            return Err(format!("ffmpeg failed ({status}): {}", said.trim()));
        }
        Ok(())
    }
}

/// ffmpeg: `--tool ffmpeg=PATH` (or `GLITU_TOOL_FFMPEG`, the configuration), otherwise the PATH
pub fn ffmpeg(project: &Project) -> Result<PathBuf, String> {
    project.settings.tools.get("ffmpeg").cloned().or_else(|| crate::chromium::in_path("ffmpeg")).ok_or_else(|| {
        "a video needs ffmpeg (https://ffmpeg.org/download.html); give its path: --tool ffmpeg=PATH".to_string()
    })
}

/// Exports the presentation as a video; returns the number of chapters (clicks)
pub fn export(project: &Project, dest: &Path, opts: &Options, browser: &Path) -> Result<usize, String> {
    let ffmpeg = ffmpeg(project)?;
    let header = glitu_core::header(&project.source()?);
    let config = project.theme(&header.theme)?.config;
    let size = |key, default| config[key].as_u64().map_or(default, |v| v as u32);
    let (width, height) = (size("width", 960), size("height", 700));
    let meta = |k: &str| header.meta.get(k).cloned().unwrap_or_default();
    // H.264 wants even sizes
    let pixels = opts.pixels & !1;
    let scale = f64::from(pixels) / f64::from(width);
    if (f64::from(height) * scale).round() as u32 % 2 == 1 {
        return Err(format!("--width {pixels}: the height of the video would be odd, choose another width"));
    }

    let status = Status::start(opts.progress);
    status.set("Preparing the page");
    let page = TempPage::write(project)?;
    let url = page.url()?;
    status.set("Starting the browser");
    let mut b = Browser::launch_frames(browser, 9743 + (std::process::id() % 200) as u16, width, height, scale)?;
    b.send("Page.addScriptToEvaluateOnNewDocument", json!({ "source": VIRTUAL_MEDIA }))?;

    let raw = dest.with_extension("frames.mkv");
    let mut enc = Encoder::start(&ffmpeg, opts.fps, &raw)?;
    let recorded = record(&mut b, &url, opts, &mut enc, &status);
    // ffmpeg's complaint about no frames would hide the reason of a failed recording
    let finished = enc.finish(&raw);
    let chapters = match recorded.and_then(|c| finished.map(|_| c)) {
        Ok(c) => c,
        Err(e) => {
            let _ = fs::remove_file(&raw);
            return Err(e);
        }
    };
    drop(b);
    if chapters.is_empty() {
        let _ = fs::remove_file(&raw);
        return Err("no slide in --range".into());
    }
    status.set(format!("Writing {}", dest.display()));
    let done = finish(&ffmpeg, &raw, dest, &chapters, opts.fps, &meta("title"), &header.lang);
    let _ = fs::remove_file(&raw);
    done?;
    Ok(chapters.len())
}

/// Plays the presentation click by click into the encoder; returns the chapters
fn record(
    b: &mut Browser,
    url: &str,
    opts: &Options,
    enc: &mut Encoder,
    status: &Status,
) -> Result<Vec<Chapter>, String> {
    let dt = 1000.0 / f64::from(opts.fps);
    b.send("Page.navigate", json!({ "url": format!("{url}#/") }))?;
    // the page loads on the real clock, its frames drawn on the way
    for _ in 0..50 {
        b.frame(50.0, false)?;
    }
    b.eval(HIDE_CONTROLS)?;
    b.send("Emulation.setVirtualTimePolicy", json!({ "policy": "pause" }))?;
    let mut current = b.frame(dt, true)?;
    if current.is_none() {
        // nothing changed since the last frame drawn: one more, with its image
        current = b.frame(dt, true)?;
    }
    enc.last = current.unwrap_or_default();

    let mut chapters: Vec<Chapter> = Vec::new();
    let (mut number, mut slide, mut step) = (0, None, 0);
    let hold = (opts.hold * f64::from(opts.fps)).round().max(1.0) as u64;
    for _ in 0..5000 {
        let hash = b.eval("location.hash")?.as_str().unwrap_or("").to_string();
        let (h, v, _) = browser::position(&hash);
        if slide != Some((h, v)) {
            slide = Some((h, v));
            number += 1;
            step = 0;
        } else {
            step += 1;
        }
        let keep = wanted(&opts.range, number);
        if !opts.range.is_empty() && opts.range.iter().all(|&(_, z)| z < number) {
            break;
        }
        let state = b.eval(STATE)?;
        if keep {
            status.set(format!(
                "Recording slide {number}{}",
                if step > 0 { format!(", step {}", step + 1) } else { String::new() }
            ));
            if let Some(c) = chapters.last_mut() {
                c.end = enc.frames;
            }
            // a slide without a title of its own has its header's (`/ Title`)
            let title = state["title"].as_str().unwrap_or("").trim().trim_start_matches('/').trim();
            let notes: Vec<&str> = state["notes"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            chapters.push(Chapter {
                start: enc.frames,
                end: enc.frames,
                name: format!("{number}.{step} {title}").trim_end().to_string(),
                notes: notes.join("\n\n"),
            });
            for _ in 0..hold {
                enc.push(None)?;
            }
        }
        b.key("n", 78)?;
        // the click plays until nothing moves any more (a few frames, to be sure), within LIMIT
        let (mut still, mut moved) = (0, false);
        for i in 0..(LIMIT * f64::from(opts.fps)) as u32 {
            b.eval(&format!("__gv.advance({})", dt / 1000.0))?;
            for _ in 0..100 {
                if b.eval("__gv.seeking()")?.as_bool() != Some(true) {
                    break;
                }
                sleep(Duration::from_millis(5));
            }
            let png = b.frame(dt, true)?;
            if i == 0 {
                let now = b.eval("location.hash")?.as_str().unwrap_or("").to_string();
                let (nh, nv, _) = browser::position(&now);
                let leaving = Some((nh, nv)) != slide;
                let past = !opts.range.is_empty() && opts.range.iter().all(|&(_, z)| z <= number);
                // the last click (nothing came), or the next slide is past --range
                if now == hash || (leaving && past) {
                    if let Some(c) = chapters.last_mut() {
                        c.end = enc.frames;
                    }
                    return Ok(chapters);
                }
            }
            moved |= png.is_some();
            if keep {
                enc.push(png)?;
            } else if png.is_some() {
                enc.last = png.unwrap_or_default();
            }
            let idle = b.eval(IDLE)?.as_bool() == Some(true);
            still = if idle { still + 1 } else { 0 };
            if still >= opts.fps / 5 && (moved || i > opts.fps) {
                break;
            }
        }
    }
    if let Some(c) = chapters.last_mut() {
        c.end = enc.frames;
    }
    Ok(chapters)
}

/// The video with its chapters, its tags and the notes as a subtitle track, disabled
fn finish(
    ffmpeg: &Path,
    raw: &Path,
    dest: &Path,
    chapters: &[Chapter],
    fps: u32,
    title: &str,
    lang: &str,
) -> Result<(), String> {
    let meta = dest.with_extension("chapters.txt");
    let srt = dest.with_extension("notes.srt");
    fs::write(&meta, metadata(chapters, fps, title)).map_err(|e| format!("{}: {e}", meta.display()))?;
    let notes = subtitles(chapters, fps);
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-y", "-i"]).arg(raw).arg("-i").arg(&meta);
    if !notes.is_empty() {
        fs::write(&srt, notes).map_err(|e| format!("{}: {e}", srt.display()))?;
        cmd.arg("-i").arg(&srt);
    }
    cmd.args(["-map", "0:v", "-map_metadata", "1", "-map_chapters", "1", "-c:v", "copy"]);
    if srt.exists() {
        cmd.args(["-map", "2:s", "-c:s", "srt", "-disposition:s:0", "0", "-metadata:s:s:0", "title=notes"]);
        let lang = iso639_2(lang);
        cmd.arg("-metadata:s:s:0").arg(format!("language={lang}"));
    }
    let out = cmd.arg(dest).stdout(Stdio::null()).output().map_err(|e| format!("ffmpeg: {e}"));
    let _ = fs::remove_file(&meta);
    let _ = fs::remove_file(&srt);
    let out = out?;
    if !out.status.success() {
        return Err(format!("ffmpeg failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(())
}

/// ffmpeg's metadata file: the tags, then one chapter per click
fn metadata(chapters: &[Chapter], fps: u32, title: &str) -> String {
    let mut s = format!(";FFMETADATA1\n{TAG}=1\nGLITU_VERSION={}\n", escape(build::VERSION));
    if !title.is_empty() {
        let _ = writeln!(s, "title={}", escape(title));
    }
    for c in chapters {
        let _ = write!(s, "[CHAPTER]\nTIMEBASE=1/{fps}\nSTART={}\nEND={}\ntitle={}\n", c.start, c.end, escape(&c.name));
    }
    s
}

/// `=`, `;`, `#`, `\` and line ends are escaped in ffmpeg's metadata
fn escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if matches!(ch, '=' | ';' | '#' | '\\' | '\n') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// The notes of each chapter, over its time (SRT)
fn subtitles(chapters: &[Chapter], fps: u32) -> String {
    let time = |frame: u64| {
        let ms = frame * 1000 / u64::from(fps);
        format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
    };
    let mut s = String::new();
    for (i, c) in chapters.iter().filter(|c| !c.notes.is_empty() && c.end > c.start).enumerate() {
        let _ = write!(s, "{}\n{} --> {}\n{}\n\n", i + 1, time(c.start), time(c.end), c.notes.replace("\n\n", "\n"));
    }
    s
}

/// The language of the notes, as Matroska wants it (ISO 639-2); `und` when unknown
fn iso639_2(lang: &str) -> &'static str {
    match lang.split(['-', '_']).next().unwrap_or("") {
        "en" => "eng",
        "fr" => "fre",
        "de" => "ger",
        "es" => "spa",
        "it" => "ita",
        "pt" => "por",
        "nl" => "dut",
        _ => "und",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter(start: u64, end: u64, name: &str, notes: &str) -> Chapter {
        Chapter { start, end, name: name.into(), notes: notes.into() }
    }

    #[test]
    fn chapters_and_tags() {
        let m = metadata(&[chapter(0, 30, "1.0 glitu", ""), chapter(30, 75, "2.0 a=b", "")], 30, "My talk");
        assert!(m.starts_with(";FFMETADATA1\nGLITU_PLAY=1\n"));
        assert!(m.contains("title=My talk\n"));
        assert!(m.contains("[CHAPTER]\nTIMEBASE=1/30\nSTART=30\nEND=75\ntitle=2.0 a\\=b\n"));
    }

    #[test]
    fn notes_follow_their_chapters() {
        let s = subtitles(
            &[chapter(0, 30, "1.0", "Hello\n\nthere"), chapter(30, 30, "1.1", "empty"), chapter(30, 3630, "2.0", "")],
            30,
        );
        assert_eq!(s, "1\n00:00:00,000 --> 00:00:01,000\nHello\nthere\n\n");
        assert_eq!(subtitles(&[chapter(108_000, 108_030, "x", "n")], 30), "1\n01:00:00,000 --> 01:00:01,000\nn\n\n");
        assert_eq!((iso639_2("fr-FR"), iso639_2("")), ("fre", "und"));
    }
}
