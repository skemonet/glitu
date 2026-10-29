//! Rendering oracle: captures every state of a presentation (slide × click) in Chromium (see
//! `browser`) and compares two series of screenshots.

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use serde_json::json;

use crate::browser::{self, Browser};
use crate::raster;

/// Goes through the presentation click by click; writes NN_<slide>-<click>.png and index.json
pub fn capture(url: &str, out: &Path) -> Result<Vec<String>, String> {
    clear_captures(out)?;
    let mut b = Browser::launch(9343 + (std::process::id() % 200) as u16, 1280, 720, 1.0)?;
    let base = url.split('#').next().unwrap_or(url);
    b.send("Page.navigate", json!({ "url": format!("{base}#/0") }))?;
    sleep(Duration::from_millis(2500));
    let mut names = Vec::new();
    let mut prev: Option<(String, Vec<u8>)> = None;
    for n in 0..500 {
        let img = b.stable_shot()?;
        let hash = b.eval("location.hash")?.as_str().unwrap_or("").to_string();
        if prev.as_ref().is_some_and(|(h, i)| *h == hash && *i == img) {
            break;
        }
        let (h, _, f) = browser::position(&hash);
        let name = format!("{n:02}_{h}-{}.png", f + 1);
        fs::write(out.join(&name), &img).map_err(|e| e.to_string())?;
        println!("{name}");
        names.push(name);
        prev = Some((hash, img));
        // N: next step, vertical slides included
        b.key("n", 78)?;
    }
    let index = serde_json::to_string_pretty(&names).map_err(|e| e.to_string())?;
    fs::write(out.join("index.json"), index + "\n").map_err(|e| e.to_string())?;
    Ok(names)
}

/// Removes a previous capture (PNG files and index.json) but keeps the rest of the folder (README)
fn clear_captures(out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    for entry in fs::read_dir(out).map_err(|e| format!("{}: {e}", out.display()))?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "index.json" || path.extension().is_some_and(|e| e == "png") {
            fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn decode(path: &Path) -> Result<raster::Rgb, String> {
    raster::decode(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
}

/// A channel further than this from the reference marks the pixel as different
const TOLERANCE: u8 = 24;

/// The comparison of two images: the share of different pixels, and an image of the differences
/// (red on the darkened reference; empty when the sizes differ)
struct Diff {
    percent: f64,
    image: Vec<u8>,
    width: u32,
    height: u32,
}

/// The share of pixels with a channel more than `TOLERANCE`/255 apart
fn diff(a: &raster::Rgb, b: &raster::Rgb) -> Diff {
    let (width, height) = (a.width, a.height);
    if (width, height) != (b.width, b.height) {
        return Diff { percent: 100.0, image: Vec::new(), width, height };
    }
    let mut n = 0usize;
    let mut image = Vec::with_capacity(a.pixels.len());
    for (p, q) in a.pixels.chunks(3).zip(b.pixels.chunks(3)) {
        let off = p.iter().zip(q).any(|(u, v)| u.abs_diff(*v) > TOLERANCE);
        n += off as usize;
        if off {
            image.extend([255, 0, 0])
        } else {
            image.extend(p.iter().map(|c| c / 4))
        }
    }
    Diff { percent: 100.0 * n as f64 / (width * height) as f64, image, width, height }
}

/// Captures the rendering of `url` and compares it with the oracle, state by state
pub fn check(url: &str, reference: &Path, limit: f64) -> Result<bool, String> {
    let cur = std::env::temp_dir().join("glitu-oracle-render");
    let names = capture(url, &cur)?;
    let index = fs::read_to_string(reference.join("index.json")).map_err(|e| format!("{}: {e}", reference.display()))?;
    let refs: Vec<String> = serde_json::from_str(&index).map_err(|e| e.to_string())?;
    let mut ok = refs.len() == names.len();
    if !ok {
        println!("number of states: oracle {}, rendering {}", refs.len(), names.len());
    }
    for (r, c) in refs.iter().zip(&names) {
        let d = diff(&decode(&reference.join(r))?, &decode(&cur.join(c))?);
        let bad = d.percent > limit;
        if bad && !d.image.is_empty() {
            let png = raster::encode(&raster::Rgb { width: d.width, height: d.height, pixels: d.image })?;
            fs::write(cur.join(format!("diff_{c}")), png).map_err(|e| e.to_string())?;
        }
        ok &= !bad;
        println!("{} {r} ↔ {c}  {:.2} %", if bad { "FAIL " } else { "ok   " }, d.percent);
    }
    println!("{}: rendering and differences", cur.display());
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differences_are_counted_and_drawn() {
        let a = raster::Rgb { width: 2, height: 1, pixels: vec![100, 100, 100, 0, 0, 0] };
        let b = raster::Rgb { width: 2, height: 1, pixels: vec![110, 100, 100, 0, 200, 0] };
        let d = diff(&a, &b);
        assert_eq!(d.percent, 50.0);
        assert_eq!(d.image, [25, 25, 25, 255, 0, 0]);
        let c = raster::Rgb { width: 1, height: 1, pixels: vec![0, 0, 0] };
        assert_eq!(diff(&a, &c).percent, 100.0);
    }

    #[test]
    fn a_new_capture_keeps_the_readme() {
        let dir = std::env::temp_dir().join(format!("glitu-oracle-clear-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for f in ["README.md", "index.json", "00_0-0.png"] {
            fs::write(dir.join(f), "x").unwrap();
        }
        clear_captures(&dir).unwrap();
        let left: Vec<_> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(left, ["README.md"]);
    }
}
