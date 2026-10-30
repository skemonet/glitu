//! Export: the standalone page is opened in headless Chromium and replayed click by click.
//!
//! - PowerPoint: each element on screen becomes an editable object (`assets/export.js` surveys the
//!   screen, `pptx` writes it); only the animated backdrop and what cannot be converted (canvas,
//!   video, formulas) stay images. `--images` keeps the old way, one screenshot per slide;
//! - PDF: the page printed by Chromium (`?print-pdf`, selectable text), or with `--clicks` one page
//!   per click, captured, and an outline of the titles;
//! - PNG: one image per slide (per click with `--clicks`) in a folder.
//!
//! The slides kept are those of `--range` (`1-3,5`, numbers of the slides in order); without
//! `--clicks`, each slide is taken once all its steps are done.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::browser::{self, Browser};
use crate::build::{self, Project};
use crate::pptx::{self, Background};
use crate::{pdf, raster, standalone};

/// Width of the images, in screen pixels: sharp on a 1080p projector
const PIXELS: f64 = 1920.0;
const COLLECTOR: &str = include_str!("../assets/export.js");

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Format {
    Pptx,
    Pdf,
    Png,
}

impl Format {
    /// According to the extension of the output file (none for a folder: it may be a website)
    pub fn of(path: &Path) -> Option<Format> {
        match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
            Some("pptx") => Some(Format::Pptx),
            Some("pdf") => Some(Format::Pdf),
            Some("png") => Some(Format::Png),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Result<Format, String> {
        match s {
            "pptx" => Ok(Format::Pptx),
            "pdf" => Ok(Format::Pdf),
            "png" => Ok(Format::Png),
            _ => Err(format!("unknown format: {s} (pptx, pdf or png)")),
        }
    }

    /// What a file of this format contains
    pub fn unit(self) -> &'static str {
        match self {
            Format::Pptx => "slides",
            Format::Pdf => "pages",
            Format::Png => "images",
        }
    }

    pub fn default_output(self) -> PathBuf {
        PathBuf::from(match self {
            Format::Pptx => "presentation.pptx",
            Format::Pdf => "presentation.pdf",
            Format::Png => "presentation",
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Options {
    pub format: Format,
    /// One slide (or page, or image) per click
    pub clicks: bool,
    /// PowerPoint: one screenshot per slide instead of editable objects
    pub images: bool,
    /// Numbers of the slides kept (from 1); empty: all of them
    pub range: Vec<(usize, usize)>,
}

/// `1-3,5` → [(1, 3), (5, 5)]
pub fn parse_range(s: &str) -> Result<Vec<(usize, usize)>, String> {
    s.split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let n = |v: &str| v.trim().parse::<usize>().map_err(|_| format!("invalid --range: {s}"));
            match p.split_once('-') {
                Some((a, b)) => Ok((n(a)?, if b.trim().is_empty() { usize::MAX } else { n(b)? })),
                None => n(p).map(|v| (v, v)),
            }
        })
        .collect()
}

fn wanted(range: &[(usize, usize)], n: usize) -> bool {
    range.is_empty() || range.iter().any(|&(a, b)| (a..=b).contains(&n))
}

/// A slide (or a click) surveyed
struct Shot {
    /// Number of the slide, from 1
    number: usize,
    title: String,
    notes: Vec<String>,
    png: Vec<u8>,
    /// What `export.js` surveyed and the images captured (editable PowerPoint)
    page: Option<(Background, Vec<Value>)>,
}

/// The presentation's standalone page, written for the time of the export and removed afterwards
struct TempPage(PathBuf);

impl TempPage {
    fn write(project: &Project) -> Result<TempPage, String> {
        let page = standalone::bundle(&build::site(project, false)?, &standalone::Mode::Offline)?;
        let path = std::env::temp_dir().join(format!("glitu-export-{}.html", std::process::id()));
        fs::write(&path, page).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(TempPage(path))
    }

    fn url(&self) -> Result<String, String> {
        Ok(format!("file://{}", self.0.canonicalize().map_err(|e| e.to_string())?.display()))
    }
}

impl Drop for TempPage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Exports the presentation; returns the number of slides (pages, images) written
pub fn export(project: &Project, dest: &Path, opts: &Options) -> Result<usize, String> {
    let header = glitu_core::header(&project.source()?);
    let config = project.theme(&header.theme)?.config;
    let size = |key, default| config[key].as_u64().map_or(default, |v| v as u32);
    let (width, height) = (size("width", 960), size("height", 700));
    let meta = |k: &str| header.meta.get(k).cloned().unwrap_or_default();

    let page = TempPage::write(project)?;
    let url = page.url()?;
    let mut b = Browser::launch(9543 + (std::process::id() % 200) as u16, width, height, PIXELS / f64::from(width))?;
    if opts.format == Format::Pdf && !opts.clicks {
        let (pdf, n) = print(&mut b, &url, opts)?;
        build::write_file(dest, &pdf)?;
        return Ok(n);
    }
    let shots = walk(&mut b, &url, opts, opts.format == Format::Pptx && !opts.images)?;
    if shots.is_empty() {
        return Err("no slide in --range".into());
    }
    let n = shots.len();
    match opts.format {
        Format::Pptx => {
            let slides: Vec<pptx::Slide> = shots
                .into_iter()
                .map(|s| {
                    let (background, items) = s.page.unwrap_or((Background::Png(s.png), Vec::new()));
                    pptx::Slide { background, items, notes: s.notes }
                })
                .collect();
            let deck = pptx::Deck { width, height, title: &meta("title"), author: &meta("author"), lang: &header.lang };
            build::write_file(dest, &pptx::write(&slides, &deck))?;
        }
        Format::Pdf => {
            let pages: Vec<(Vec<u8>, String)> = shots.into_iter().map(|s| (s.png, s.title)).collect();
            let points = |px: u32| f64::from(px) * 0.75;
            build::write_file(dest, &pdf::from_images(&pages, points(width), points(height), &meta("title"))?)?;
        }
        Format::Png => write_images(dest, &shots, opts.clicks)?,
    }
    Ok(n)
}

/// The name of each image: `NN.png` per slide, or `NN-c.png` per click
fn image_names(numbers: &[usize], clicks: bool) -> Vec<String> {
    let mut click = 0;
    numbers
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            click = if i > 0 && numbers[i - 1] == n { click + 1 } else { 1 };
            if clicks { format!("{n:02}-{click}.png") } else { format!("{n:02}.png") }
        })
        .collect()
}

fn write_images(dir: &Path, shots: &[Shot], clicks: bool) -> Result<(), String> {
    let numbers: Vec<usize> = shots.iter().map(|s| s.number).collect();
    for (name, s) in image_names(&numbers, clicks).iter().zip(shots) {
        build::write_file(&dir.join(name), &s.png)?;
    }
    Ok(())
}

/// The PDF printed by Chromium: one page per slide, every step shown
fn print(b: &mut Browser, url: &str, opts: &Options) -> Result<(Vec<u8>, usize), String> {
    b.send("Page.navigate", json!({ "url": format!("{url}?print-pdf") }))?;
    sleep(Duration::from_millis(2500));
    b.eval(HIDE_CONTROLS)?;
    let pages = b.eval("document.querySelectorAll('.pdf-page').length")?.as_u64().unwrap_or(0) as usize;
    let mut params = json!({
        "printBackground": true, "preferCSSPageSize": true, "generateDocumentOutline": true, "generateTaggedPDF": true,
        "marginTop": 0, "marginBottom": 0, "marginLeft": 0, "marginRight": 0,
    });
    if let Some(ranges) = page_ranges(&opts.range, pages) {
        params["pageRanges"] = ranges.into();
    }
    let r = b.send("Page.printToPDF", params)?;
    let pdf = STANDARD.decode(r["data"].as_str().unwrap_or("")).map_err(|e| e.to_string())?;
    let n = (1..=pages).filter(|&n| wanted(&opts.range, n)).count();
    Ok((pdf, n))
}

/// `--range` as Chromium's `pageRanges` for a document of `pages` pages; none: every page
fn page_ranges(range: &[(usize, usize)], pages: usize) -> Option<String> {
    let ranges: Vec<String> = range
        .iter()
        .filter(|(a, _)| *a <= pages)
        .map(|&(a, z)| if a == z { a.to_string() } else { format!("{a}-{}", z.min(pages)) })
        .collect();
    (!ranges.is_empty()).then(|| ranges.join(","))
}

/// Hides the navigation interface (reveal.js arrows): it has no place outside the screen
const HIDE_CONTROLS: &str =
    "document.head.insertAdjacentHTML('beforeend', '<style>.reveal .controls{display:none!important}</style>')";

/// Title and notes of the slide shown; are all its fragments already visible?
const STATE: &str = "(() => {
  const s = [...document.querySelectorAll('.slides section.present')].filter(e => !e.classList.contains('stack')).pop();
  if (!s) return { last: true, notes: [], title: '' };
  const a = s.querySelector('aside.notes');
  let notes = [];
  if (a) {
    notes = [...a.querySelectorAll('p')].map(p => p.textContent.trim()).filter(t => t);
    if (!notes.length) notes = a.textContent.split(/\\n\\s*\\n/).map(t => t.trim()).filter(t => t);
  }
  const h = ['h1', '.chapter h2', '.panel h2', '.caption h2', 'h2', 'h3'].map(q => s.querySelector(q)).find(e => e);
  const title = h ? h.textContent.trim() : (s.dataset.header || '').replace('|', ' / ');
  const last = [...s.querySelectorAll('.fragment')].every(f => f.classList.contains('visible'));
  return { last, notes, title };
})()";

fn walk(b: &mut Browser, url: &str, opts: &Options, editable: bool) -> Result<Vec<Shot>, String> {
    b.send("Page.navigate", json!({ "url": format!("{url}#/") }))?;
    sleep(Duration::from_millis(2500));
    b.eval(HIDE_CONTROLS)?;
    if editable {
        if std::env::var("GLITU_EXPORT_DUMP").is_ok() {
            b.eval("window.__gxDebug = true")?;
        }
        b.eval(COLLECTOR)?;
    }
    let mut shots: Vec<Shot> = Vec::new();
    let (mut number, mut slide) = (0, None);
    for _ in 0..5000 {
        let hash = b.eval("location.hash")?.as_str().unwrap_or("").to_string();
        let (h, v, _) = browser::position(&hash);
        if slide != Some((h, v)) {
            slide = Some((h, v));
            number += 1;
        }
        let state = b.eval(STATE)?;
        if wanted(&opts.range, number) && (opts.clicks || state["last"].as_bool() == Some(true)) {
            let png = b.stable_shot()?;
            let page = if editable { Some(collect(b)?) } else { None };
            let notes =
                state["notes"].as_array().into_iter().flatten().filter_map(|n| n.as_str().map(String::from)).collect();
            let title = state["title"].as_str().unwrap_or("").to_string();
            shots.push(Shot { number, title, notes, png, page });
            eprint!("\r{} slides", shots.len());
        }
        if !opts.range.is_empty() && opts.range.iter().all(|&(_, z)| z < number) {
            break;
        }
        b.key("n", 78)?;
        sleep(Duration::from_millis(350));
        if b.eval("location.hash")?.as_str().unwrap_or("") == hash {
            break;
        }
    }
    eprintln!();
    Ok(shots)
}

/// Surveys the screen (`export.js`) and captures what stays an image: the backdrop, then each
/// element that cannot be converted, alone, on a transparent background
fn collect(b: &mut Browser) -> Result<(Background, Vec<Value>), String> {
    let mut page = b.eval("__gx.collect()")?;
    if let Ok(js) = std::env::var("GLITU_EXPORT_EVAL") {
        eprintln!("{}", b.eval(&js)?);
    }
    // GLITU_EXPORT_DUMP=folder: keeps each survey, to tune export.js
    if let Ok(dir) = std::env::var("GLITU_EXPORT_DUMP") {
        let n = fs::read_dir(&dir).map_or(0, Iterator::count);
        let _ = fs::write(
            Path::new(&dir).join(format!("{:02}.json", n + 1)),
            serde_json::to_string_pretty(&page).unwrap_or_default(),
        );
    }
    let bg_ids = page["background"].clone();
    let background = if bg_ids.as_array().is_some_and(|a| !a.is_empty()) {
        b.eval(&format!("__gx.isolate({bg_ids}, false)"))?;
        let png = b.screenshot(json!({ "format": "png" }))?;
        raster::decode(&png).ok().and_then(|img| raster::uniform(&img)).map(Background::Color).unwrap_or(Background::Png(png))
    } else {
        let c = &page["color"];
        let ch = |i: usize| c[i].as_f64().unwrap_or(255.0) as u8;
        Background::Color([ch(0), ch(1), ch(2)])
    };
    b.send("Emulation.setDefaultBackgroundColorOverride", json!({ "color": { "r": 0, "g": 0, "b": 0, "a": 0 } }))?;
    let mut items = page["items"].take();
    let captured = isolated(b, &mut items);
    b.send("Emulation.setDefaultBackgroundColorOverride", json!({}))?;
    b.eval("__gx.clear()")?;
    captured?;
    Ok((background, items.as_array().cloned().unwrap_or_default()))
}

/// The screen area of an item, on whole pixels: (x, y, width, height)
fn pixel_box(it: &Value) -> (f64, f64, f64, f64) {
    let n = |k: &str| it[k].as_f64().unwrap_or(0.0);
    let (x, y) = (n("x").floor(), n("y").floor());
    (x, y, (n("x") + n("w")).ceil() - x, (n("y") + n("h")).ceil() - y)
}

fn isolated(b: &mut Browser, items: &mut Value) -> Result<(), String> {
    for it in items.as_array_mut().into_iter().flatten() {
        if it["t"] == "group" {
            isolated(b, &mut it["items"])?;
        }
        let Some(id) = it["iso"].as_u64() else { continue };
        b.eval(&format!("__gx.isolate([{id}], true)"))?;
        let (x, y, w, h) = pixel_box(it);
        if w < 1.0 || h < 1.0 {
            continue;
        }
        let png = b.screenshot(json!({ "format": "png", "clip": { "x": x, "y": y, "width": w, "height": h, "scale": 1 } }))?;
        it["src"] = format!("data:image/png;base64,{}", STANDARD.encode(png)).into();
        (it["x"], it["y"], it["w"], it["h"]) = (x.into(), y.into(), w.into(), h.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_formats() {
        assert_eq!(parse_range("1-3,5").unwrap(), vec![(1, 3), (5, 5)]);
        assert_eq!(parse_range("4-").unwrap(), vec![(4, usize::MAX)]);
        assert!(parse_range("a").is_err());
        assert!(wanted(&[(1, 3)], 2) && !wanted(&[(1, 3)], 4) && wanted(&[], 9));
        assert_eq!(Format::of(Path::new("x.PDF")), Some(Format::Pdf));
        assert_eq!(Format::of(Path::new("shots")), None);
        assert_eq!(Format::of(Path::new("x.odp")), None);
        assert!(Format::parse("odp").is_err());
        assert_eq!((Format::Pdf.unit(), Format::Png.default_output()), ("pages", PathBuf::from("presentation")));
    }

    #[test]
    fn pages_and_images_follow_the_range() {
        assert_eq!(page_ranges(&[(1, 3), (5, 5), (40, 50)], 10).as_deref(), Some("1-3,5"));
        assert_eq!(page_ranges(&[(4, usize::MAX)], 10).as_deref(), Some("4-10"));
        assert_eq!(page_ranges(&[], 10), None);
        assert_eq!(image_names(&[1, 2, 2, 2, 3], true), ["01-1.png", "02-1.png", "02-2.png", "02-3.png", "03-1.png"]);
        assert_eq!(image_names(&[1, 2], false), ["01.png", "02.png"]);
        assert_eq!(pixel_box(&json!({ "x": 1.5, "y": 2.2, "w": 3.0, "h": 1.0 })), (1.0, 2.0, 4.0, 2.0));
    }
}
