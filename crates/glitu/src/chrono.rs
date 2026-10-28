//! Rehearsal mode (`glitu serve --rehearse`): the speaker view times a rehearsal, and the duration
//! of each slide gone through is written in the markdown, after its heading:
//! `## Ingress {.large duration="95" runs="3"}`. The duration, in seconds, is the average of the
//! runs; a new measure is added to it. Included files are modified in place.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::markdown::tokens;

use crate::source;

/// Writes the measured durations (rank of the slide in the markdown → seconds); returns the number
/// of slides updated
pub fn record(slides: &Path, measured: &BTreeMap<usize, f64>) -> Result<usize, String> {
    let src = source::read(slides)?;
    if glitu_core::header(&src.text).reveal_md {
        return Err("rehearsal: the reveal-md syntax has no headings to write the durations in".into());
    }
    let deck = glitu_core::markdown::parse(&src.text);
    // The lines to rewrite, file by file
    let mut edits: BTreeMap<PathBuf, Vec<(usize, u32)>> = BTreeMap::new();
    for (&i, &secs) in measured {
        let Some(slide) = deck.slides.get(i) else { continue };
        let Some((path, line)) = src.origin(slide.at) else { continue };
        edits.entry(path.to_path_buf()).or_default().push((line, secs.round().max(1.0) as u32));
    }
    let mut done = 0;
    for (path, lines) in edits {
        done += rewrite(&path, &lines)?;
    }
    Ok(done)
}

/// Writes the new durations in the heading lines of one file; returns how many it wrote
fn rewrite(path: &Path, lines: &[(usize, u32)]) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    let mut done = 0;
    for &(n, secs) in lines {
        // Safeguard: the line must be a `#` or `##` heading
        let Some(line) = out.get_mut(n).filter(|l| l.starts_with('#')) else { continue };
        let bare = line.trim_end_matches(['\n', '\r']);
        let eol = line[bare.len()..].to_string();
        *line = with_duration(bare, secs) + &eol;
        done += 1;
    }
    fs::write(path, out.concat()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(done)
}

/// The heading with its new average duration: `## T {.large}` + 80 s → `## T {.large duration="80"
/// runs="1"}`; a duration already there counts for its runs
fn with_duration(line: &str, secs: u32) -> String {
    let (title, mut tokens) = match (line.rfind('{'), line.ends_with('}')) {
        (Some(open), true) => (line[..open].trim_end(), tokens(&line[open + 1..line.len() - 1])),
        _ => (line.trim_end(), Vec::new()),
    };
    let take = |tokens: &mut Vec<String>, key: &str| {
        let i = tokens.iter().position(|t| t.split_once('=').is_some_and(|(k, _)| k == key))?;
        tokens.remove(i).split_once('=').and_then(|(_, v)| v.trim_matches('"').parse::<f64>().ok())
    };
    let old = take(&mut tokens, "duration");
    let runs = take(&mut tokens, "runs").unwrap_or(1.0).max(1.0);
    let (duration, runs) = match old {
        Some(d) => (((d * runs + secs as f64) / (runs + 1.0)).round() as u32, runs as u32 + 1),
        None => (secs, 1),
    };
    tokens.push(format!("duration=\"{duration}\""));
    tokens.push(format!("runs=\"{runs}\""));
    format!("{title} {{{}}}", tokens.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_averaged_in_the_heading() {
        assert_eq!(with_duration("## Pods", 80), "## Pods {duration=\"80\" runs=\"1\"}");
        assert_eq!(
            with_duration("## Ingress {.large kicker=\"A b\"}", 95),
            "## Ingress {.large kicker=\"A b\" duration=\"95\" runs=\"1\"}"
        );
        // (100 × 3 + 60) / 4 = 90
        assert_eq!(with_duration("## X {duration=\"100\" runs=\"3\" .large}", 60), "## X {.large duration=\"90\" runs=\"4\"}");
        // a duration written by hand counts for one run
        assert_eq!(with_duration("# Chapter {- duration=\"30\"}", 50), "# Chapter {- duration=\"40\" runs=\"2\"}");
    }

    #[test]
    fn durations_land_in_included_files() {
        let dir = std::env::temp_dir().join(format!("glitu-chrono-{}", std::process::id()));
        fs::create_dir_all(dir.join("ch")).unwrap();
        fs::write(dir.join("slides.md"), "---\ntitle: T\n---\n\n## One\n\ntext\n\n<!-- include: ch/a.md -->\n\n## Three {.large}\n")
            .unwrap();
        fs::write(dir.join("ch/a.md"), "---\nx: y\n---\n# Chapter\n\n```md\n## not a slide\n```\n\n## Two\n").unwrap();
        let measured = BTreeMap::from([(0, 12.4), (1, 30.0), (2, 45.0), (3, 61.0), (9, 5.0)]);
        assert_eq!(record(&dir.join("slides.md"), &measured).unwrap(), 4);
        let main = fs::read_to_string(dir.join("slides.md")).unwrap();
        let inc = fs::read_to_string(dir.join("ch/a.md")).unwrap();
        assert!(main.contains("## One {duration=\"12\" runs=\"1\"}\n"));
        assert!(main.contains("## Three {.large duration=\"61\" runs=\"1\"}\n"));
        assert!(inc.contains("# Chapter {duration=\"30\" runs=\"1\"}\n"));
        assert!(inc.contains("## Two {duration=\"45\" runs=\"1\"}\n"));
        assert!(inc.contains("## not a slide\n"));
        fs::write(dir.join("slides.md"), "# A\n\n---\n\n# B\n").unwrap();
        assert!(record(&dir.join("slides.md"), &measured).unwrap_err().contains("reveal-md"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
