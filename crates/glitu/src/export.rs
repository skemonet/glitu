//! Export: the standalone page is opened in headless Chromium and replayed click by click.
//!
//! - PowerPoint: each element on screen becomes an editable object (`assets/export.js` surveys the
//!   screen, `pptx` writes it); only the animated backdrop and what cannot be converted (canvas,
//!   video, formulas) stay images. `--images` keeps the old way, one screenshot per slide;
//! - PDF: the page printed by Chromium (`?print-pdf`, selectable text), or with `--clicks` one page
//!   per click, captured, and an outline of the titles;
//! - PNG: one image per slide (per click with `--clicks`) in a folder;
//! - play: the native player's folder (`glitu-play`): one image per click, and what moves at
//!   each click (the arrows drawn by skemo, the elements that wait for them) with the states on
//!   the way, `deck.json`.
//!
//! The slides kept are those of `--range` (`1-3,5`, numbers of the slides in order); without
//! `--clicks`, each slide is taken once all its steps are done.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, sleep};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::browser::{self, Browser};
use crate::build::{self, Project};
use crate::pptx::{self, Background};
use crate::{pdf, raster, standalone};

/// Width of the images by default, in screen pixels: sharp on a 1080p projector
pub const PIXELS: u32 = 1920;
const COLLECTOR: &str = include_str!("../assets/export.js");

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Format {
    Pptx,
    Pdf,
    Png,
    Play,
}

impl Format {
    /// According to the extension of the output file (none for a folder: it may be a website)
    pub fn of(path: &Path) -> Option<Format> {
        match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
            Some("pptx") => Some(Format::Pptx),
            Some("pdf") => Some(Format::Pdf),
            Some("png") => Some(Format::Png),
            Some("glituplay") => Some(Format::Play),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Result<Format, String> {
        match s {
            "pptx" => Ok(Format::Pptx),
            "pdf" => Ok(Format::Pdf),
            "png" => Ok(Format::Png),
            "play" => Ok(Format::Play),
            _ => Err(format!("unknown format: {s} (pptx, pdf, png or play)")),
        }
    }

    /// What a file of this format contains
    pub fn unit(self) -> &'static str {
        match self {
            Format::Pptx => "slides",
            Format::Pdf => "pages",
            Format::Png => "images",
            Format::Play => "clicks",
        }
    }

    pub fn default_output(self) -> PathBuf {
        PathBuf::from(match self {
            Format::Pptx => "presentation.pptx",
            Format::Pdf => "presentation.pdf",
            Format::Png => "presentation",
            Format::Play => "presentation.glituplay",
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
    /// Width of the images, in pixels (`PIXELS` by default)
    pub pixels: u32,
    /// Show the slide being rendered (on a terminal, without --quiet)
    pub progress: bool,
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
    /// What moves at this click (`play`): `{ draws, fades }`
    motion: Value,
    /// The terminal's typing at this click (`play`)
    film: Option<Film>,
    /// The states on the way, when arrows are drawn (`play`): without the arrows, then with the
    /// arrows but without their heads and the cards that wait for them
    stages: Vec<Vec<u8>>,
    /// Under an animated backdrop (`play`): its pace and dimming on this click, the glass areas
    /// (`[x, y, w, h, blur, radius]`, pixels), the opacity of what covers it (grey PNG)
    backdrop: Option<(Value, Vec<u8>)>,
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
pub fn export(project: &Project, dest: &Path, opts: &Options, browser: &Path) -> Result<usize, String> {
    let header = glitu_core::header(&project.source()?);
    let config = project.theme(&header.theme)?.config;
    let size = |key, default| config[key].as_u64().map_or(default, |v| v as u32);
    let (width, height) = (size("width", 960), size("height", 700));
    let meta = |k: &str| header.meta.get(k).cloned().unwrap_or_default();

    let status = Status::start(opts.progress);
    status.set("Preparing the page");
    let page = TempPage::write(project)?;
    let url = page.url()?;
    status.set("Starting the browser");
    let mut b = Browser::launch(
        browser,
        9543 + (std::process::id() % 200) as u16,
        width,
        height,
        f64::from(opts.pixels) / f64::from(width),
    )?;
    if opts.format == Format::Pdf && !opts.clicks {
        status.set("Printing the slides to PDF");
        let (pdf, n) = print(&mut b, &url, opts)?;
        status.set(format!("Writing {}", dest.display()));
        build::write_file(dest, &pdf)?;
        return Ok(n);
    }
    status.set("Loading the slides");
    let shots =
        walk(&mut b, &url, opts, opts.format == Format::Pptx && !opts.images, opts.format == Format::Play, &status)?;
    if shots.is_empty() {
        return Err("no slide in --range".into());
    }
    let n = shots.len();
    status.set(format!("Writing {}", dest.display()));
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
            build::write_file(&dest.with_extension("pdfpc"), pdfpc(&shots).as_bytes())?;
            let pages: Vec<pdf::Page> =
                shots.into_iter().map(|s| pdf::Page { png: s.png, title: s.title, number: s.number }).collect();
            let points = |px: u32| f64::from(px) * 0.75;
            build::write_file(dest, &pdf::from_images(&pages, points(width), points(height), &meta("title"))?)?;
        }
        Format::Png => write_images(dest, &shots, opts.clicks)?,
        Format::Play => {
            // the markdown goes with the pictures (`glitu unpack` gets it back), unless `source: false`
            let copy =
                (meta("source") != "false").then(|| project.read().map(|s| crate::source::copy(&s))).transpose()?;
            write_play(dest, &shots, &meta("title"), copy)?
        }
    }
    Ok(n)
}

/// The native player's file (`.glituplay`): an uncompressed zip archive (its entries are PNG images,
/// already compressed), `deck.json` first, then one image per click, and the copy of the markdown
/// (`source.json`, see `source::copy`)
fn write_play(dest: &Path, shots: &[Shot], title: &str, copy: Option<String>) -> Result<(), String> {
    let first = raster::decode(&shots[0].png)?;
    let clicks: Vec<Value> = shots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut c = json!({
                "slide": s.number, "image": format!("{:03}.png", i + 1), "title": s.title, "notes": s.notes,
                "draws": s.motion["draws"], "fades": s.motion["fades"],
            });
            if !s.stages.is_empty() {
                c["base"] = format!("{:03}-base.png", i + 1).into();
                c["drawn"] = format!("{:03}-drawn.png", i + 1).into();
            }
            if let Some((state, _)) = &s.backdrop {
                c["motion"] = state["motion"].clone();
                c["dim"] = state["dim"].clone();
                c["glass"] = state["glass"].clone();
                c["alpha"] = format!("{:03}-alpha.png", i + 1).into();
            }
            if let Some(f) = &s.film {
                let frames: Vec<Value> = (0..f.frames.len())
                    .map(|k| json!({ "t": f.frames[k].0.round(), "image": format!("{:03}-film/{:04}.png", i + 1, k + 1) }))
                    .collect();
                c["film"] = json!({ "rect": f.rect, "frames": frames });
            }
            c
        })
        .collect();
    let mut deck =
        json!({ "glitu-play": 1, "title": title, "width": first.width, "height": first.height, "clicks": clicks });
    // the backdrop, the same on every slide
    if let Some((state, _)) = shots.iter().find_map(|s| s.backdrop.as_ref()) {
        deck["backdrop"] = json!({
            "scene": state["scene"], "seed": state["seed"], "rect": state["rect"], "size": state["size"],
            "scale": state["scale"],
        });
    }
    let mut zip = crate::zip::Zip::default();
    zip.add("deck.json", serde_json::to_string(&deck).unwrap_or_default().into_bytes());
    for (i, s) in shots.iter().enumerate() {
        zip.add(&format!("{:03}.png", i + 1), s.png.clone());
        if let Some((_, alpha)) = &s.backdrop {
            zip.add(&format!("{:03}-alpha.png", i + 1), alpha.clone());
        }
        for (name, png) in ["base", "drawn"].iter().zip(&s.stages) {
            zip.add(&format!("{:03}-{name}.png", i + 1), png.clone());
        }
        for (k, (_, png)) in s.film.iter().flat_map(|f| &f.frames).enumerate() {
            zip.add(&format!("{:03}-film/{:04}.png", i + 1, k + 1), png.clone());
        }
    }
    if let Some(copy) = copy {
        zip.add("source.json", copy.into_bytes());
    }
    build::write_file(dest, &zip.finish())
}

/// The companion file of pdfpc (`talk.pdfpc` beside `talk.pdf`): the speaker notes, and the pages
/// of one slide as its overlays, so that pdfpc counts slides and not clicks
fn pdfpc(shots: &[Shot]) -> String {
    let mut overlay = 0;
    let pages: Vec<Value> = shots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            overlay = if i > 0 && shots[i - 1].number == s.number { overlay + 1 } else { 0 };
            json!({ "idx": i, "label": s.number.to_string(), "overlay": overlay, "note": s.notes.join("\n\n") })
        })
        .collect();
    // the notes are text already (no markdown left in them)
    let file = json!({ "pdfpcFormat": 2, "disableMarkdown": true, "pages": pages });
    serde_json::to_string_pretty(&file).unwrap_or_default()
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

/// Before a click (`play`): the slide, its arrows still to draw and its cards shown
const MOTION_BEFORE: &str = "(() => {
  const s = [...document.querySelectorAll('.slides section.present')].filter(e => !e.classList.contains('stack')).pop();
  window.__gpBefore = s && {
    slide: s,
    edges: [...s.querySelectorAll('svg.skemo .edge.sk-off[data-fx=\"draw\"]')],
    cards: [...s.querySelectorAll('svg.skemo .card:not(.sk-off)')].map(c => c.dataset.id),
  };
})()";

/// After the click (`play`), what moved, in pixels of the image: the arrows drawn (their path,
/// sampled, and the width of the stroke that reveals them, as skemo.rs draws them: 600 ms, ease),
/// their heads (appearing at 560 ms, in 120 ms) and the cards waiting for an arrow (at 600 ms).
/// Keeps these elements for `HIDE`: the lines (`__gpLines`), the heads and cards (`__gpLate`)
const MOTION_AFTER: &str = "(() => {
  const b = window.__gpBefore, out = { draws: [], fades: [] };
  window.__gpLines = []; window.__gpLate = [];
  const s = [...document.querySelectorAll('.slides section.present')].filter(e => !e.classList.contains('stack')).pop();
  if (!b || b.slide !== s) return out;
  const k = window.devicePixelRatio;
  const late = (el, delay, duration) => {
    const r = el.getBoundingClientRect();
    out.fades.push({ rect: [r.x * k - 2, r.y * k - 2, r.width * k + 4, r.height * k + 4], delay, duration });
    window.__gpLate.push(el);
  };
  for (const e of b.edges.filter(e => !e.classList.contains('sk-off'))) {
    const line = e.querySelector('.line');
    if (!line) continue;
    const m = line.getScreenCTM(), len = line.getTotalLength(), n = Math.max(2, Math.ceil(len / 2)), points = [];
    for (let i = 0; i <= n; i++) {
      const p = line.getPointAtLength(len * i / n), q = new DOMPoint(p.x, p.y).matrixTransform(m);
      points.push(q.x * k, q.y * k);
    }
    const cs = getComputedStyle(line), dashed = cs.strokeDasharray && cs.strokeDasharray !== 'none';
    const width = (dashed ? 12 : parseFloat(cs.strokeWidth) || 1) * Math.hypot(m.a, m.b) * k + 3 * k;
    out.draws.push({ points, width, delay: 0, duration: 600 });
    window.__gpLines.push(line);
    e.querySelectorAll('.head, .tail').forEach(h => late(h, 560, 120));
    const [from, to] = [e.dataset.from, e.dataset.to].map(id => b.cards.includes(id));
    const waiting = from && !to ? e.dataset.to : to && !from ? e.dataset.from : null;
    const card = waiting && e.closest('svg').querySelector(`.card[data-id=\"${waiting}\"]`);
    if (card) late(card, 600, 300);
  }
  return out;
})()";

/// Hides (`true`) or shows again (`false`) the elements kept by `MOTION_AFTER`: `__gpLate`, and
/// `__gpLines` with `lines`
fn hide(b: &mut Browser, on: bool, lines: bool) -> Result<(), String> {
    let lists = if lines { "[...__gpLate, ...__gpLines]" } else { "__gpLate" };
    let set = if on {
        "el.style.setProperty('transition', 'none', 'important'); el.style.setProperty('opacity', '0', 'important')"
    } else {
        "el.style.removeProperty('transition'); el.style.removeProperty('opacity')"
    };
    b.eval(&format!("{lists}.forEach(el => {{ {set} }})"))?;
    sleep(Duration::from_millis(60));
    Ok(())
}

/// The states between two clicks that draw arrows (`play`): the arrows drawn but not their
/// heads nor the cards waiting for them, then without the arrows either
fn stages(b: &mut Browser) -> Result<Vec<Vec<u8>>, String> {
    hide(b, true, false)?;
    let drawn = b.screenshot(json!({ "format": "png" }))?;
    hide(b, true, true)?;
    let base = b.screenshot(json!({ "format": "png" }))?;
    hide(b, false, true)?;
    Ok(vec![base, drawn])
}

/// Before a click (`play`), the area that the click animates, to record (x, y, width, height and
/// the screen's size, in pixels, then whether the motions of the page must play: off for the
/// pictures): the terminals of the slide (their typing); the animated code whose next version
/// comes; the whole screen when the next slide is animated with this one (`{.animate}`). None if
/// nothing of this kind
const ANIMATED_AREA: &str = "(() => {
  const s = [...document.querySelectorAll('.slides section.present')].filter(e => !e.classList.contains('stack')).pop();
  if (!s) return null;
  const k = window.devicePixelRatio;
  const W = Math.round(innerWidth * k), H = Math.round(innerHeight * k);
  const last = [...s.querySelectorAll('.fragment')].every(f => f.classList.contains('visible'));
  if (last && s.hasAttribute('data-auto-animate')) {
    let n = s.nextElementSibling;
    if (!n && s.parentElement.classList.contains('stack')) n = s.parentElement.nextElementSibling;
    if (n && n.classList.contains('stack')) n = n.querySelector('section');
    if (n && n.hasAttribute('data-auto-animate')) return [0, 0, W, H, W, H, true];
  }
  const els = [...s.querySelectorAll('.cast-stage')];
  const code = last ? [] : [...s.querySelectorAll('pre.magic-move')].filter(p => p.querySelector(':scope > code.fragment:not(.visible)'));
  els.push(...code);
  const rs = els.map(e => e.getBoundingClientRect());
  if (!rs.length) return null;
  const x = Math.min(...rs.map(r => r.left)), y = Math.min(...rs.map(r => r.top));
  const r = Math.max(...rs.map(r => r.right)), b = Math.max(...rs.map(r => r.bottom));
  const X = Math.max(0, Math.floor(x * k)), Y = Math.max(0, Math.floor(y * k));
  return [X, Y, Math.min(W, Math.ceil(r * k)) - X, Math.min(H, Math.ceil(b * k)) - Y, W, H, code.length > 0];
})()";

/// An animation recorded (a terminal typing, animated code, an animated slide): its area (x, y, width, height) and its images, (ms after the click, PNG)
struct Film {
    rect: [u32; 4],
    frames: Vec<(f64, Vec<u8>)>,
}

/// The screen's recording, cut to the terminal's area: the images that change it
struct Recording {
    rect: [u32; 4],
    frames: Vec<(f64, raster::Rgb)>,
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

/// The images of the screencast in the terminal's area; none if the terminal did not move (or
/// the images are not of the screenshots' size)
fn cut_film(frames: &[(f64, Vec<u8>)], area: &Value, pressed: f64) -> Option<Recording> {
    let n = |i: usize| area[i].as_u64().unwrap_or(0) as u32;
    let (rect, size) = ([n(0), n(1), n(2), n(3)], (n(4), n(5)));
    if rect[2] == 0 || rect[3] == 0 {
        return None;
    }
    let mut out: Vec<(f64, raster::Rgb)> = Vec::new();
    for (t, png) in frames {
        let img = raster::decode(png).ok().filter(|i| (i.width, i.height) == size)?;
        let crop = crop(&img, rect);
        if out.last().is_none_or(|(_, last)| last.pixels != crop.pixels) {
            out.push((((t - pressed) * 1000.0).max(0.0), crop));
        }
    }
    // the first image is the screen before the click: a film needs it and at least one more
    (out.len() > 2).then_some(Recording { rect, frames: out })
}

/// The screencast does not draw exactly as the screenshots (text smoothing, the backdrop seen
/// through). Around a pixel (a few pixels: the fringe of a character), once the recording no
/// longer changes, the pixel takes the click's screenshot (`after`); before it starts changing,
/// the previous click's (`before`); in between, the screencast: only what is being typed
fn resolve(rec: Recording, before: Option<&[u8]>, after: &[u8]) -> Option<Film> {
    let area = |png: &[u8]| raster::decode(png).ok().map(|img| crop(&img, rec.rect));
    let after = area(after)?;
    let before = before.and_then(area);
    let (w, h) = (rec.rect[2] as usize, rec.rect[3] as usize);
    let n = rec.frames.len();
    let px = |k: usize, p: usize| &rec.frames[k].1.pixels[p * 3..p * 3 + 3];
    // the first and the last image where each pixel changes
    let (mut first, mut last) = (vec![usize::MAX; w * h], vec![0; w * h]);
    for k in 1..n {
        for p in 0..w * h {
            if px(k, p) != px(k - 1, p) {
                first[p] = first[p].min(k);
                last[p] = k;
            }
        }
    }
    let (settled, untouched) = (spread(&last, w, h, usize::max), spread(&first, w, h, usize::min));
    let mut frames: Vec<(f64, Vec<u8>)> = Vec::new();
    let mut previous: Option<Vec<u8>> = None;
    for (k, (t, img)) in rec.frames.iter().enumerate() {
        let mut pixels = img.pixels.clone();
        for p in 0..w * h {
            let source = if k >= settled[p] {
                Some(&after)
            } else if k < untouched[p] {
                before.as_ref()
            } else {
                None
            };
            if let Some(src) = source {
                pixels[p * 3..p * 3 + 3].copy_from_slice(&src.pixels[p * 3..p * 3 + 3]);
            }
        }
        if previous.as_ref() == Some(&pixels) {
            continue;
        }
        let png = raster::encode(&raster::Rgb { width: w as u32, height: h as u32, pixels: pixels.clone() }).ok()?;
        frames.push((*t, png));
        previous = Some(pixels);
    }
    (frames.len() > 1).then_some(Film { rect: rec.rect, frames })
}

/// `values` combined (max or min) over the neighbourhood of each pixel: 4 pixels across, 6 down
fn spread(values: &[usize], w: usize, h: usize, f: fn(usize, usize) -> usize) -> Vec<usize> {
    let (rx, ry) = (4, 6);
    let mut across = values.to_vec();
    for y in 0..h {
        for x in 0..w {
            let row = &values[y * w..y * w + w];
            across[y * w + x] = row[x.saturating_sub(rx)..(x + rx + 1).min(w)].iter().copied().reduce(f).unwrap_or(0);
        }
    }
    let mut out = across.clone();
    for y in 0..h {
        for x in 0..w {
            out[y * w + x] =
                (y.saturating_sub(ry)..(y + ry + 1).min(h)).map(|yy| across[yy * w + x]).reduce(f).unwrap_or(0);
        }
    }
    out
}

fn crop(img: &raster::Rgb, [x, y, w, h]: [u32; 4]) -> raster::Rgb {
    let row = img.width as usize * 3;
    let pixels = (y..y + h)
        .flat_map(|r| {
            let at = r as usize * row + x as usize * 3;
            img.pixels[at..at + w as usize * 3].iter().copied()
        })
        .collect();
    raster::Rgb { width: w, height: h, pixels }
}

/// A line on the standard error saying what the export does, a spinner turning in front of it while
/// it works (also during the long waits: a stable screenshot, a terminal's recording); cleared when
/// dropped. Without `on` (not a terminal, `--quiet`), it says nothing.
struct Status {
    text: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Status {
    fn start(on: bool) -> Status {
        let (text, stop) = (Arc::new(Mutex::new(String::new())), Arc::new(AtomicBool::new(false)));
        let thread = on.then(|| {
            let (text, stop) = (text.clone(), stop.clone());
            std::thread::spawn(move || {
                for frame in ['|', '/', '-', '\\'].into_iter().cycle() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let line = text.lock().map(|t| t.clone()).unwrap_or_default();
                    if !line.is_empty() {
                        eprint!("\r{frame} {line}\x1b[K");
                        let _ = std::io::stderr().flush();
                    }
                    sleep(Duration::from_millis(100));
                }
            })
        });
        Status { text, stop, thread }
    }

    fn set(&self, text: impl Into<String>) {
        if let Ok(mut t) = self.text.lock() {
            *t = text.into();
        }
    }
}

impl Drop for Status {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
            eprint!("\r\x1b[K");
        }
    }
}

/// "Rendering slide 3/12", the slide counted among those of `--range`; `step` (from 2), the click
/// within the slide; `what`, the long work under way
fn rendering(opts: &Options, number: usize, total: usize, step: usize, what: &str) -> String {
    let rank = (1..=number).filter(|&n| wanted(&opts.range, n)).count().max(1);
    let all = (1..=total.max(number)).filter(|&n| wanted(&opts.range, n)).count().max(rank);
    let mut s = format!("Rendering slide {rank}/{all}");
    if step > 1 {
        s += &format!(", step {step}");
    }
    if !what.is_empty() {
        s += &format!(" ({what})");
    }
    s
}

fn walk(
    b: &mut Browser,
    url: &str,
    opts: &Options,
    editable: bool,
    play: bool,
    status: &Status,
) -> Result<Vec<Shot>, String> {
    b.send("Page.navigate", json!({ "url": format!("{url}#/") }))?;
    sleep(Duration::from_millis(2500));
    b.eval(HIDE_CONTROLS)?;
    // the engine has removed the hidden slides: the others, stacks left out
    let total = b.eval("document.querySelectorAll('.reveal .slides section:not(.stack)').length")?.as_u64().unwrap_or(0)
        as usize;
    if editable {
        if std::env::var("GLITU_EXPORT_DUMP").is_ok() {
            b.eval("window.__gxDebug = true")?;
        }
        b.eval(COLLECTOR)?;
    }
    let mut shots: Vec<Shot> = Vec::new();
    let mut film = None;
    let (mut number, mut slide) = (0, None);
    for _ in 0..5000 {
        let hash = b.eval("location.hash")?.as_str().unwrap_or("").to_string();
        let (h, v, _) = browser::position(&hash);
        if slide != Some((h, v)) {
            slide = Some((h, v));
            number += 1;
        }
        let step = 1 + shots.iter().rev().take_while(|s| s.number == number).count();
        if wanted(&opts.range, number) {
            status.set(rendering(opts, number, total, step, ""));
        }
        let state = b.eval(STATE)?;
        if wanted(&opts.range, number) && (opts.clicks || state["last"].as_bool() == Some(true)) {
            let png = b.stable_shot()?;
            let page = if editable { Some(collect(b)?) } else { None };
            let notes =
                state["notes"].as_array().into_iter().flatten().filter_map(|n| n.as_str().map(String::from)).collect();
            let title = state["title"].as_str().unwrap_or("").to_string();
            let motion = if play { b.eval(MOTION_AFTER)? } else { Value::Null };
            let stages =
                if motion["draws"].as_array().is_some_and(|d| !d.is_empty()) { stages(b)? } else { Vec::new() };
            // the click before, on this slide or the previous one (an animated slide)
            let before = shots.last().filter(|s| s.number + 1 >= number).map(|s| s.png.as_slice());
            let film = film.take().and_then(|rec| resolve(rec, before, &png));
            let backdrop = if play { layered(b)? } else { None };
            shots.push(Shot { number, title, notes, png, page, motion, stages, film, backdrop });
        }
        if !opts.range.is_empty() && opts.range.iter().all(|&(_, z)| z < number) {
            break;
        }
        if play {
            b.eval(MOTION_BEFORE)?;
        }
        // a terminal on the slide, animated code, an animated slide next: recorded, image by image
        let stage = if play { b.eval(ANIMATED_AREA)? } else { Value::Null };
        if stage.is_array() {
            status.set(rendering(opts, number, total, step, "recording the animation"));
            b.film_start(stage[4].as_u64().unwrap_or(0) as u32, stage[5].as_u64().unwrap_or(0) as u32)?;
        }
        let motions = stage[6].as_bool() == Some(true);
        if motions {
            b.reduce_motion(false)?;
        }
        let pressed = now();
        b.key("n", 78)?;
        if stage.is_array() {
            let frames = b.film(Duration::from_millis(700), Duration::from_secs(300))?;
            film = cut_film(&frames, &stage, pressed);
        }
        if motions {
            b.reduce_motion(true)?;
        }
        sleep(Duration::from_millis(350));
        if b.eval("location.hash")?.as_str().unwrap_or("") == hash {
            break;
        }
    }
    Ok(shots)
}

/// The animated backdrop of the template themes, for glitu-play: its scene and seed, its place in
/// the image (`rect`, pixels) and its size in px of the slide (`size`); on this click its pace
/// (`motion`), its dimming (`dim`) and the elements that blur it (`glass`, pixels). Null without
const BACKDROP_STATE: &str = "(() => {
  const host = document.querySelector('.tpl-backdrop[data-scene]');
  const canvas = host && host.querySelector('canvas.backdrop-canvas');
  if (!canvas) return null;
  const k = window.devicePixelRatio, r = host.getBoundingClientRect();
  const zoom = r.width / host.offsetWidth;
  const s = [...document.querySelectorAll('.slides section.present')].filter(e => !e.classList.contains('stack')).pop();
  const scope = [...(s ? s.querySelectorAll('*') : []), ...document.querySelectorAll('.tpl-progress, .tpl-progress *')];
  const glass = [];
  for (const e of scope) {
    const f = getComputedStyle(e).backdropFilter || '';
    const m = f.match(/blur\\(([\\d.]+)px\\)/);
    if (!m || !e.checkVisibility({ opacityProperty: true, visibilityProperty: true })) continue;
    const b = e.getBoundingClientRect();
    if (b.width < 1 || b.height < 1) continue;
    const z = b.width / (e.offsetWidth || b.width);
    glass.push([b.left * k, b.top * k, b.width * k, b.height * k, parseFloat(m[1]) * z * k,
      (parseFloat(getComputedStyle(e).borderTopLeftRadius) || 0) * z * k]);
  }
  return {
    scene: host.dataset.scene, seed: +(host.dataset.seed || 0),
    rect: [r.left * k, r.top * k, r.width * k, r.height * k], size: [host.offsetWidth, host.offsetHeight],
    scale: zoom * k, motion: host.dataset.motion || 'still', dim: +(host.dataset.dim || 0), glass,
  };
})()";

/// Hides the backdrop and the backgrounds behind it (`true`), or shows them again
const BACKDROP_HIDE: &str = "((on) => {
  document.getElementById('__gp_bare')?.remove();
  if (!on) return;
  const st = document.createElement('style');
  st.id = '__gp_bare';
  st.textContent = 'html, body, .reveal, .reveal-viewport, .reveal .slides, .tpl-backdrop '
    + '{ background: transparent !important; } canvas.backdrop-canvas { visibility: hidden !important; }';
  document.head.append(st);
})";

/// Under an animated backdrop: its state at this click, and the opacity of what covers it, from a
/// screenshot without it on a transparent page
fn layered(b: &mut Browser) -> Result<Option<(Value, Vec<u8>)>, String> {
    let state = b.eval(BACKDROP_STATE)?;
    if !state.is_object() {
        return Ok(None);
    }
    b.eval(&format!("{BACKDROP_HIDE}(true)"))?;
    b.send("Emulation.setDefaultBackgroundColorOverride", json!({ "color": { "r": 0, "g": 0, "b": 0, "a": 0 } }))?;
    let shot = b.screenshot(json!({ "format": "png" }));
    b.send("Emulation.setDefaultBackgroundColorOverride", json!({}))?;
    b.eval(&format!("{BACKDROP_HIDE}(false)"))?;
    let (w, h, alpha) = raster::alpha(&shot?)?;
    Ok(Some((state, raster::encode_gray(w, h, &alpha)?)))
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
        raster::decode(&png)
            .ok()
            .and_then(|img| raster::uniform(&img))
            .map(Background::Color)
            .unwrap_or(Background::Png(png))
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
        let png =
            b.screenshot(json!({ "format": "png", "clip": { "x": x, "y": y, "width": w, "height": h, "scale": 1 } }))?;
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
        let shot = |number, notes: &[&str]| Shot {
            number,
            title: String::new(),
            notes: notes.iter().map(|n| n.to_string()).collect(),
            png: Vec::new(),
            page: None,
            motion: Value::Null,
            stages: Vec::new(),
            film: None,
            backdrop: None,
        };
        let file: Value = serde_json::from_str(&pdfpc(&[shot(1, &["a", "b"]), shot(1, &[]), shot(2, &[])])).unwrap();
        let p = &file["pages"];
        assert_eq!(
            (p[0]["note"].as_str(), p[1]["overlay"].as_u64(), p[2]["label"].as_str()),
            (Some("a\n\nb"), Some(1), Some("2"))
        );
        assert_eq!(p[2]["overlay"], 0);
        assert_eq!(pixel_box(&json!({ "x": 1.5, "y": 2.2, "w": 3.0, "h": 1.0 })), (1.0, 2.0, 4.0, 2.0));
    }
}
