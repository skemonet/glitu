//! Rehearsals (`glitu serve`): the speaker view times a rehearsal, and the duration of each slide
//! gone through is written in the markdown, at the end of its heading: `## Ingress ⏳95🏃3 {.large}`
//! (95 s, the average of 3 rehearsals). For each slide, the speaker chooses to average the new
//! measure with the former ones, or to replace them. Included files are modified in place.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::markdown::{timing, tokens};

use crate::source;

/// What to do with the measure of a slide
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Into the average of the former rehearsals (🏃 + 1)
    Average,
    /// Instead of them (🏃 1)
    Replace,
}

/// Writes the measured durations (rank of the slide in the markdown → seconds and mode); returns
/// the new timings of the slides updated (rank → seconds, rehearsals)
pub fn record(slides: &Path, measured: &BTreeMap<usize, (f64, Mode)>) -> Result<BTreeMap<usize, (u32, u32)>, String> {
    let src = source::read(slides)?;
    if glitu_core::header(&src.text).reveal_md {
        return Err("rehearsal: the reveal-md syntax has no headings to write the durations in".into());
    }
    let deck = glitu_core::markdown::parse(&src.text);
    // The lines to rewrite, file by file
    let mut edits: BTreeMap<PathBuf, Vec<(usize, usize, u32, Mode)>> = BTreeMap::new();
    for (&i, &(secs, mode)) in measured {
        let Some(slide) = deck.slides.get(i) else { continue };
        let Some((path, line)) = src.origin(slide.at) else { continue };
        edits.entry(path.to_path_buf()).or_default().push((i, line, secs.round().max(1.0) as u32, mode));
    }
    let mut done = BTreeMap::new();
    for (path, lines) in edits {
        done.extend(rewrite(&path, &lines)?);
    }
    Ok(done)
}

/// A slide's new timing: its rank, its seconds and its number of rehearsals
type Timing = (usize, (u32, u32));

/// Writes the new durations in the heading lines of one file; returns the new timings by rank
fn rewrite(path: &Path, lines: &[(usize, usize, u32, Mode)]) -> Result<Vec<Timing>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    let mut done = Vec::new();
    for &(rank, n, secs, mode) in lines {
        // Safeguard: the line must be a `#` or `##` heading
        let Some(line) = out.get_mut(n).filter(|l| l.starts_with('#')) else { continue };
        let bare = line.trim_end_matches(['\n', '\r']);
        let eol = line[bare.len()..].to_string();
        let (new, timing) = with_duration(bare, secs, mode);
        *line = new + &eol;
        done.push((rank, timing));
    }
    fs::write(path, out.concat()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(done)
}

/// The heading with its new duration: `## T {.large}` + 80 s → `## T ⏳80🏃1 {.large}`. Averaged,
/// a duration already there counts for its runs (one if written by hand); replaced, it goes
fn with_duration(line: &str, secs: u32, mode: Mode) -> (String, (u32, u32)) {
    let (text, mut tokens) = match (line.rfind('{'), line.ends_with('}')) {
        (Some(open), true) => (line[..open].trim_end(), tokens(&line[open + 1..line.len() - 1])),
        _ => (line.trim_end(), Vec::new()),
    };
    // the former timing: at the end of the title, in the braces, or the former attributes
    let mut old: Option<(f64, f64)> = None;
    let title = match text.rsplit_once(char::is_whitespace) {
        Some((before, last)) if timing(last).is_some() => {
            let (d, r) = timing(last).unwrap_or_default();
            old = Some((d as f64, r.unwrap_or(1) as f64));
            before.trim_end()
        }
        _ => text,
    };
    tokens.retain(|t| match timing(t) {
        Some((d, r)) => {
            old = Some((d as f64, r.unwrap_or(1) as f64));
            false
        }
        None => true,
    });
    let attr = |tokens: &mut Vec<String>, key: &str| {
        let i = tokens.iter().position(|t| t.split_once('=').is_some_and(|(k, _)| k == key))?;
        tokens.remove(i).split_once('=').and_then(|(_, v)| v.trim_matches('"').parse::<f64>().ok())
    };
    if let Some(d) = attr(&mut tokens, "duration") {
        old = Some((d, attr(&mut tokens, "runs").unwrap_or(1.0).max(1.0)));
    }
    tokens.retain(|t| !t.starts_with("runs="));
    let (duration, runs) = match (old, mode) {
        (Some((d, runs)), Mode::Average) => (((d * runs + secs as f64) / (runs + 1.0)).round() as u32, runs as u32 + 1),
        _ => (secs, 1),
    };
    let braces = if tokens.is_empty() { String::new() } else { format!(" {{{}}}", tokens.join(" ")) };
    (format!("{title} ⏳{duration}🏃{runs}{braces}"), (duration, runs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_averaged_or_replaced_in_the_heading() {
        use Mode::*;
        assert_eq!(with_duration("## Pods", 80, Average).0, "## Pods ⏳80🏃1");
        assert_eq!(
            with_duration("## Ingress {.large kicker=\"A b\"}", 95, Average).0,
            "## Ingress ⏳95🏃1 {.large kicker=\"A b\"}"
        );
        // (100 × 3 + 60) / 4 = 90
        assert_eq!(with_duration("## X ⏳100🏃3 {.large}", 60, Average).0, "## X ⏳90🏃4 {.large}");
        assert_eq!(with_duration("## X ⏳100🏃3 {.large}", 60, Replace).0, "## X ⏳60🏃1 {.large}");
        // in the braces, or the former attributes: moved to the title
        assert_eq!(with_duration("## X {.large ⏳100🏃3}", 60, Average).0, "## X ⏳90🏃4 {.large}");
        assert_eq!(with_duration("## X {duration=\"100\" runs=\"3\" .large}", 60, Average).0, "## X ⏳90🏃4 {.large}");
        // a duration written by hand counts for one run
        assert_eq!(with_duration("# Chapter ⏳30 {-}", 50, Average).0, "# Chapter ⏳40🏃2 {-}");
    }

    #[test]
    fn durations_land_in_included_files() {
        let dir = std::env::temp_dir().join(format!("glitu-chrono-{}", std::process::id()));
        fs::create_dir_all(dir.join("ch")).unwrap();
        fs::write(dir.join("slides.md"), "---\ntitle: T\n---\n\n## One\n\ntext\n\n<!-- include: ch/a.md -->\n\n## Three {.large}\n")
            .unwrap();
        fs::write(dir.join("ch/a.md"), "---\nx: y\n---\n# Chapter\n\n```md\n## not a slide\n```\n\n## Two\n").unwrap();
        let measured: BTreeMap<usize, (f64, Mode)> =
            [(0, 12.4), (1, 30.0), (2, 45.0), (3, 61.0), (9, 5.0)].into_iter().map(|(i, s)| (i, (s, Mode::Average))).collect();
        let written = record(&dir.join("slides.md"), &measured).unwrap();
        assert_eq!(written.into_iter().collect::<Vec<_>>(), [(0, (12, 1)), (1, (30, 1)), (2, (45, 1)), (3, (61, 1))]);
        let main = fs::read_to_string(dir.join("slides.md")).unwrap();
        let inc = fs::read_to_string(dir.join("ch/a.md")).unwrap();
        assert!(main.contains("## One ⏳12🏃1\n"));
        assert!(main.contains("## Three ⏳61🏃1 {.large}\n"));
        assert!(inc.contains("# Chapter ⏳30🏃1\n"));
        assert!(inc.contains("## Two ⏳45🏃1\n"));
        assert!(inc.contains("## not a slide\n"));
        fs::write(dir.join("slides.md"), "# A\n\n---\n\n# B\n").unwrap();
        assert!(record(&dir.join("slides.md"), &measured).unwrap_err().contains("reveal-md"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
