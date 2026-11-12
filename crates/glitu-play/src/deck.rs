//! The file written by `glitu build play` (`.glituplay`): an uncompressed zip archive
//! of `deck.json` and one PNG per click.
//!
//!   { "glitu-play": 1, "title": "…", "width": 1024, "height": 576,
//!     "clicks": [ { "slide": 1, "image": "001.png", "title": "…", "notes": ["…"],
//!                   "draws": [ { "points": [x, y, x, y…], "width": 7.2, "delay": 0, "duration": 600 } ],
//!                   "fades": [ { "rect": [x, y, w, h], "delay": 560, "duration": 120 } ],
//!                   "base": "001-base.png", "drawn": "001-drawn.png",
//!                   "film": { "rect": [x, y, w, h], "frames": [ { "t": 62, "image": "001-film/0001.png" } ] } } ] }
//!
//! Under an animated backdrop (the template themes), the deck describes it once
//! (`"backdrop": { "scene", "seed", "rect": [x, y, w, h], "size": [w, h], "scale" }`: its place
//! in the images, its size in px of the slide) and, at each click, its pace (`"motion"`: still,
//! animated, chase), its dimming (`"dim"`), the areas that blur it (`"glass"`: `[x, y, w, h,
//! blur, radius]`) and the opacity of what covers it (`"alpha"`, a grey image).
//!
//! Coordinates are in pixels of the images. When arrows are drawn, two more images give the
//! states on the way: `base` without the arrows, `drawn` with them but without their heads and
//! the cards that wait for them. What the browser animates otherwise comes as a film, recorded at
//! export: the images of its area, with their time after the click in ms (a terminal typing,
//! animated code, the whole screen for an animated slide).

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The largest side of a slide, in pixels, and its largest area: what a screen shows, with room
/// to spare; above, the deck is damaged (or hostile) and no buffer is made for it
pub const MAX_SIDE: u64 = 16_384;
pub const MAX_AREA: u64 = 1 << 26;
/// The farthest an arrow's point is from the slide's corner (px), and its widest stroke: a
/// stroke is drawn by a rasteriser, that does not take any number
pub const MAX_REACH: f32 = 2.0 * MAX_SIDE as f32;

/// An arrow drawn along its path: the stroke that reveals it, of `width` pixels
#[derive(Clone, Debug, PartialEq)]
pub struct Draw {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
    pub delay: f64,
    pub duration: f64,
}

/// A rectangle that appears by fading, after the others
#[derive(Clone, Debug, PartialEq)]
pub struct Fade {
    pub rect: (f32, f32, f32, f32),
    pub delay: f64,
    pub duration: f64,
}

/// The images of an area (a terminal typing), at their time after the click (ms)
#[derive(Clone, Debug, PartialEq)]
pub struct Film {
    pub rect: (u32, u32, u32, u32),
    pub frames: Vec<(f64, String)>,
}

/// The animated backdrop, as the browser drew it
#[derive(Clone, Debug, PartialEq)]
pub struct BackdropInfo {
    pub scene: String,
    pub seed: i32,
    /// Its place in the images, pixels
    pub rect: (f32, f32, f32, f32),
    /// Its size in px of the slide
    pub size: (f64, f64),
}

/// The backdrop at a click
#[derive(Clone, Debug, PartialEq)]
pub struct Under {
    pub motion: String,
    pub dim: f64,
    /// [x, y, w, h, blur, radius], pixels of the images
    pub glass: Vec<[f32; 6]>,
    /// The grey image of the opacity of what covers it
    pub alpha: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Click {
    /// Number of the slide, from 1: the clicks of one slide share it
    pub slide: usize,
    pub image: String,
    /// Title of the slide
    pub title: String,
    /// The states on the way (`base`, `drawn`), when arrows are drawn
    pub stages: Option<(String, String)>,
    pub notes: Vec<String>,
    pub draws: Vec<Draw>,
    pub fades: Vec<Fade>,
    pub film: Option<Film>,
    pub under: Option<Under>,
}

/// The `.glituplay` file: each entry's offset and size, by name
#[derive(Clone, Default)]
pub struct Archive {
    path: PathBuf,
    entries: HashMap<String, (u64, usize)>,
}

pub struct Deck {
    pub archive: Archive,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub clicks: Vec<Click>,
    pub backdrop: Option<BackdropInfo>,
}

/// An image, in the pixels of softbuffer: 0x00RRGGBB
#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl Deck {
    /// `path`: the `.glituplay` file
    pub fn open(path: &Path) -> Result<Deck, String> {
        let at = |e: String| format!("{}: {e}", path.display());
        let archive = Archive { path: path.to_path_buf(), entries: zip_entries(path).map_err(at)? };
        let text = String::from_utf8(archive.read("deck.json").map_err(at)?).map_err(|e| at(e.to_string()))?;
        let deck = Deck::parse(&text, archive).map_err(at)?;
        deck.check_files().map_err(at)?;
        Ok(deck)
    }

    /// The images named by the clicks are in the archive, the slides' own size
    fn check_files(&self) -> Result<(), String> {
        let names = self.clicks.iter().flat_map(|c| {
            let film = c.film.iter().flat_map(|f| f.frames.iter().map(|f| f.1.as_str()));
            let alpha = c.under.iter().map(|u| u.alpha.as_str());
            let stages = c.stages.iter().flat_map(|(a, b)| [a.as_str(), b.as_str()]);
            [c.image.as_str()].into_iter().chain(stages).chain(alpha).chain(film)
        });
        for name in names {
            if !self.archive.entries.contains_key(name) {
                return Err(format!("{name}: missing from the archive"));
            }
        }
        // the slides are the size of the deck
        for c in &self.clicks {
            let slides =
                [c.image.as_str()].into_iter().chain(c.stages.iter().flat_map(|(a, b)| [a.as_str(), b.as_str()]));
            for name in slides {
                let (w, h) = self.archive.png_size(name)?;
                if (w, h) != (self.width, self.height) {
                    let (dw, dh) = (self.width, self.height);
                    return Err(format!("{name}: {w} × {h} pixels, the slides are {dw} × {dh}"));
                }
            }
        }
        Ok(())
    }

    pub fn parse(text: &str, archive: Archive) -> Result<Deck, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if v["glitu-play"].as_u64() != Some(1) {
            return Err("not a presentation of `glitu build play` (or of another version)".into());
        }
        let num = |v: &Value| v.as_f64().unwrap_or(0.0);
        let side = |key: &str| v[key].as_u64().filter(|n| (1..=MAX_SIDE).contains(n));
        let (Some(width), Some(height)) = (side("width"), side("height")) else {
            return Err(format!("the size of the slides is missing, or not between 1 and {MAX_SIDE} pixels"));
        };
        if width * height > MAX_AREA {
            return Err(format!("the slides are larger than {MAX_AREA} pixels"));
        }
        for (i, c) in v["clicks"].as_array().into_iter().flatten().enumerate() {
            for p in c["draws"].as_array().into_iter().flatten() {
                let points = p["points"].as_array().into_iter().flatten().filter_map(Value::as_f64);
                let far = points.into_iter().any(|n| n.abs() > f64::from(MAX_REACH));
                if far || !(0.0..=MAX_SIDE as f64).contains(&num(&p["width"])) {
                    return Err(format!("click {}: an arrow is too far or too wide", i + 1));
                }
            }
            if c["film"].is_object() {
                // the area of the film is in the slide: x, y, width, height, whole pixels
                let r: Vec<u64> =
                    c["film"]["rect"].as_array().into_iter().flatten().filter_map(Value::as_u64).collect();
                let inside = r.len() == 4
                    && c["film"]["rect"].as_array().is_some_and(|a| a.len() == 4)
                    && r[2] >= 1
                    && r[3] >= 1
                    && r[0].saturating_add(r[2]) <= width
                    && r[1].saturating_add(r[3]) <= height;
                if !inside {
                    return Err(format!("click {}: the area of its film is not inside the slide", i + 1));
                }
            }
        }
        let clicks: Vec<Click> = v["clicks"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| Click {
                slide: c["slide"].as_u64().unwrap_or(0) as usize,
                image: c["image"].as_str().unwrap_or("").to_string(),
                title: c["title"].as_str().unwrap_or("").to_string(),
                stages: c["base"].as_str().zip(c["drawn"].as_str()).map(|(a, b)| (a.to_string(), b.to_string())),
                notes: c["notes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|n| n.as_str().map(String::from))
                    .collect(),
                draws: c["draws"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|d| {
                        let flat: Vec<f32> =
                            d["points"].as_array().into_iter().flatten().map(|p| num(p) as f32).collect();
                        Draw {
                            points: flat.as_chunks::<2>().0.iter().map(|p| (p[0], p[1])).collect(),
                            width: num(&d["width"]) as f32,
                            delay: num(&d["delay"]),
                            duration: num(&d["duration"]),
                        }
                    })
                    .collect(),
                fades: c["fades"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|f| {
                        let r = |i: usize| num(&f["rect"][i]) as f32;
                        Fade { rect: (r(0), r(1), r(2), r(3)), delay: num(&f["delay"]), duration: num(&f["duration"]) }
                    })
                    .collect(),
                under: c["alpha"].as_str().map(|alpha| Under {
                    motion: c["motion"].as_str().unwrap_or("still").to_string(),
                    dim: num(&c["dim"]),
                    glass: c["glass"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|g| std::array::from_fn(|i| num(&g[i]) as f32))
                        .collect(),
                    alpha: alpha.to_string(),
                }),
                film: c["film"].is_object().then(|| {
                    let r = |i: usize| c["film"]["rect"][i].as_u64().unwrap_or(0) as u32;
                    let frames = c["film"]["frames"].as_array().into_iter().flatten();
                    Film {
                        rect: (r(0), r(1), r(2), r(3)),
                        frames: frames.map(|f| (num(&f["t"]), f["image"].as_str().unwrap_or("").to_string())).collect(),
                    }
                }),
            })
            .collect();
        if clicks.is_empty() {
            return Err("no click in the deck".into());
        }
        let backdrop = v["backdrop"].is_object().then(|| {
            let b = &v["backdrop"];
            let r = |i: usize| num(&b["rect"][i]) as f32;
            BackdropInfo {
                scene: b["scene"].as_str().unwrap_or("crystal").to_string(),
                seed: b["seed"].as_i64().unwrap_or(0) as i32,
                rect: (r(0), r(1), r(2), r(3)),
                size: (num(&b["size"][0]), num(&b["size"][1])),
            }
        });
        if let Some(b) = &backdrop {
            // its place in the images, and its size: a canvas is made of them
            let side = |n: f64| (1.0..=MAX_SIDE as f64).contains(&n.round());
            let (w, h) = (f64::from(b.rect.2), f64::from(b.rect.3));
            let placed = b.rect.0.abs() <= MAX_SIDE as f32 && b.rect.1.abs() <= MAX_SIDE as f32;
            if !(placed
                && side(w)
                && side(h)
                && side(b.size.0)
                && side(b.size.1)
                && w.round() * h.round() <= MAX_AREA as f64)
            {
                return Err("the animated backdrop has a place or a size out of range".into());
            }
        }
        Ok(Deck {
            archive,
            title: v["title"].as_str().unwrap_or("").to_string(),
            width: width as u32,
            height: height as u32,
            clicks,
            backdrop,
        })
    }

    /// The click to start at for `--start`: the first of the slide (from 1); past the last slide,
    /// the last click (as the End key goes)
    pub fn start_click(&self, slide: usize) -> usize {
        self.clicks.iter().position(|c| c.slide >= slide).unwrap_or(self.clicks.len() - 1)
    }

    pub fn image(&self, i: usize) -> Result<Image, String> {
        self.file(&self.clicks[i].image)
    }

    /// A grey image (the opacity of a click): one byte per pixel
    pub fn gray(&self, name: &str) -> Result<(u32, u32, Vec<u8>), String> {
        let bytes = self.archive.read(name)?;
        let img = decode(&bytes).map_err(|e| format!("{name}: {e}"))?;
        Ok((img.width, img.height, img.pixels.iter().map(|p| (p & 0xff) as u8).collect()))
    }

    pub fn file(&self, name: &str) -> Result<Image, String> {
        let bytes = self.archive.read(name)?;
        decode(&bytes).map_err(|e| format!("{name}: {e}"))
    }
}

impl Archive {
    /// The bytes of the entry `name`
    pub fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        let (mut f, start, size) = self.locate(name)?;
        let mut bytes = vec![0; size];
        f.seek(SeekFrom::Start(start)).and_then(|_| f.read_exact(&mut bytes)).map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    /// The size in pixels of the PNG `name`, from its header alone
    fn png_size(&self, name: &str) -> Result<(u32, u32), String> {
        let (mut f, start, size) = self.locate(name)?;
        // the signature, then the header chunk: its length and name, the width and the height
        let mut head = [0u8; 24];
        if size < head.len() {
            return Err(format!("{name}: not a PNG image"));
        }
        f.seek(SeekFrom::Start(start)).and_then(|_| f.read_exact(&mut head)).map_err(|e| e.to_string())?;
        if head[..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] || &head[12..16] != b"IHDR" {
            return Err(format!("{name}: not a PNG image"));
        }
        let at = |i: usize| u32::from_be_bytes([head[i], head[i + 1], head[i + 2], head[i + 3]]);
        Ok((at(16), at(20)))
    }

    /// The file, where the bytes of the entry `name` start, and their number
    fn locate(&self, name: &str) -> Result<(fs::File, u64, usize), String> {
        let &(offset, size) = self.entries.get(name).ok_or_else(|| format!("{name}: missing from the archive"))?;
        let mut f = fs::File::open(&self.path).map_err(|e| e.to_string())?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        // the local header: 30 bytes, then the name and an extra field of their own lengths
        let mut head = [0u8; 30];
        f.seek(SeekFrom::Start(offset)).and_then(|_| f.read_exact(&mut head)).map_err(|e| e.to_string())?;
        if head[..4] != [0x50, 0x4b, 0x03, 0x04] {
            return Err(format!("{name}: damaged archive"));
        }
        let skip =
            u64::from(u16::from_le_bytes([head[26], head[27]])) + u64::from(u16::from_le_bytes([head[28], head[29]]));
        // the size comes from the file: it must fit in the file before a buffer is made for it
        let start = offset + 30 + skip;
        if start.checked_add(size as u64).is_none_or(|end| end > len) {
            return Err(format!("{name}: damaged archive"));
        }
        Ok((f, start, size))
    }
}

/// The entries of a zip archive, from its central directory: name → (offset of the local header,
/// size). Only stored entries, as glitu writes them: images are already compressed
fn zip_entries(path: &Path) -> Result<HashMap<String, (u64, usize)>, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    // the end record: 22 bytes, after a comment of 64 KB at most
    let tail_len = len.min(22 + 65_535);
    let mut tail = vec![0; tail_len as usize];
    f.seek(SeekFrom::Start(len - tail_len)).and_then(|_| f.read_exact(&mut tail)).map_err(|e| e.to_string())?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| tail[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or("not a .glituplay file (see `glitu build play`)")?;
    let u16_at = |b: &[u8], i: usize| u16::from_le_bytes([b[i], b[i + 1]]) as usize;
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
    let (count, size, offset) = (u16_at(&tail, end + 10), u32_at(&tail, end + 12), u32_at(&tail, end + 16));
    // the directory lies in the file, and holds `count` entries of 46 bytes at least
    if offset as u64 + size as u64 > len || count * 46 > size {
        return Err("damaged archive".into());
    }
    let mut central = vec![0; size];
    f.seek(SeekFrom::Start(offset as u64)).and_then(|_| f.read_exact(&mut central)).map_err(|e| e.to_string())?;
    let mut entries = HashMap::new();
    let mut at = 0;
    for _ in 0..count {
        if central.get(at..at + 4) != Some(&[0x50, 0x4b, 0x01, 0x02][..]) || at + 46 > central.len() {
            return Err("damaged archive".into());
        }
        let (method, stored) = (u16_at(&central, at + 10), u32_at(&central, at + 20));
        let (name_len, extra, comment) =
            (u16_at(&central, at + 28), u16_at(&central, at + 30), u16_at(&central, at + 32));
        let local = u32_at(&central, at + 42) as u64;
        let name = central.get(at + 46..at + 46 + name_len).ok_or("damaged archive")?;
        let name = String::from_utf8_lossy(name).into_owned();
        if method != 0 {
            return Err(format!(
                "{name}: compressed entry; the player reads the archive as glitu writes it (export it again)"
            ));
        }
        // the entry (its header of 30 bytes at least, then its bytes) lies in the file
        if (local + 30).checked_add(stored as u64).is_none_or(|end| end > len) {
            return Err(format!("{name}: damaged archive"));
        }
        entries.insert(name, (local, stored));
        at += 46 + name_len + extra + comment;
    }
    Ok(entries)
}

/// PNG → 0x00RRGGBB pixels (an alpha channel is dropped: the screenshots are opaque)
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let channels = info.color_type.samples();
    let rgb = |p: &[u8]| match channels {
        1 | 2 => u32::from(p[0]) * 0x010101,
        _ => (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]),
    };
    let pixels = buf[..info.buffer_size()].chunks_exact(channels).map(rgb).collect();
    Ok(Image { width: info.width, height: info.height, pixels })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn reads_the_deck() {
        let text = r#"{ "glitu-play": 1, "title": "T", "width": 4, "height": 2, "clicks": [
            { "slide": 1, "image": "001.png", "title": "Intro", "notes": ["n"], "draws": [ { "points": [0, 1, 2, 3], "width": 5, "delay": 0, "duration": 600 } ],
              "fades": [ { "rect": [1, 2, 3, 4], "delay": 560, "duration": 120 } ], "base": "a.png", "drawn": "b.png",
              "film": { "rect": [1, 0, 3, 2], "frames": [ { "t": 62, "image": "f/1.png" } ] },
              "motion": "chase", "dim": 0.6, "glass": [[1, 2, 3, 4, 5, 6]], "alpha": "001-alpha.png" },
            { "slide": 2, "image": "002.png", "draws": null, "fades": null } ] }"#;
        let d = Deck::parse(text, Archive::default()).unwrap();
        assert_eq!((d.title.as_str(), d.width, d.clicks.len()), ("T", 4, 2));
        assert_eq!((d.clicks[0].title.as_str(), d.clicks[1].title.as_str()), ("Intro", ""));
        assert_eq!(d.clicks[0].draws[0].points, [(0.0, 1.0), (2.0, 3.0)]);
        assert_eq!(d.clicks[0].fades[0].rect, (1.0, 2.0, 3.0, 4.0));
        assert_eq!(d.clicks[0].stages, Some(("a.png".into(), "b.png".into())));
        assert_eq!(d.clicks[0].film, Some(Film { rect: (1, 0, 3, 2), frames: vec![(62.0, "f/1.png".into())] }));
        assert!(d.clicks[1].film.is_none() && d.clicks[1].under.is_none() && d.backdrop.is_none());
        let u = d.clicks[0].under.as_ref().unwrap();
        assert_eq!((u.motion.as_str(), u.dim, u.glass[0][4], u.alpha.as_str()), ("chase", 0.6, 5.0, "001-alpha.png"));
        let text = r#"{ "glitu-play": 1, "width": 4, "height": 2, "clicks": [ { "slide": 1, "image": "1.png" } ],
            "backdrop": { "scene": "aurora", "seed": 7, "rect": [0, 0, 4, 2], "size": [8, 4], "scale": 0.5 } }"#;
        let b = Deck::parse(text, Archive::default()).unwrap().backdrop.unwrap();
        assert_eq!((b.scene.as_str(), b.seed, b.rect.2, b.size), ("aurora", 7, 4.0, (8.0, 4.0)));
        assert!(d.clicks[1].draws.is_empty() && d.clicks[1].notes.is_empty() && d.clicks[1].stages.is_none());
        assert!(Deck::parse(r#"{ "glitu-play": 2 }"#, Archive::default()).is_err());
        assert!(Deck::parse(r#"{ "glitu-play": 1, "clicks": [] }"#, Archive::default()).is_err());
    }

    /// A stored zip archive, as `glitu build play` writes it (the CRC is not checked: left at zero)
    pub(crate) fn zip(files: &[(&str, &[u8])], comment: &[u8]) -> Vec<u8> {
        let (mut data, mut central) = (Vec::new(), Vec::new());
        for (name, bytes) in files {
            let offset = data.len() as u32;
            let common = |v: &mut Vec<u8>| {
                for x in [20u16, 0, 0, 0, 0x21] {
                    v.extend(x.to_le_bytes());
                }
                for x in [0, bytes.len() as u32, bytes.len() as u32] {
                    v.extend(x.to_le_bytes());
                }
                v.extend((name.len() as u16).to_le_bytes());
                v.extend(0u16.to_le_bytes());
            };
            data.extend(0x04034b50u32.to_le_bytes());
            common(&mut data);
            data.extend(name.as_bytes());
            data.extend(*bytes);
            central.extend(0x02014b50u32.to_le_bytes());
            central.extend(20u16.to_le_bytes());
            common(&mut central);
            central.extend([0u8; 6]);
            central.extend(0u32.to_le_bytes());
            central.extend(offset.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let (offset, size, n) = (data.len() as u32, central.len() as u32, files.len() as u16);
        data.extend(central);
        data.extend(0x06054b50u32.to_le_bytes());
        for x in [0u16, 0, n, n] {
            data.extend(x.to_le_bytes());
        }
        data.extend(size.to_le_bytes());
        data.extend(offset.to_le_bytes());
        data.extend((comment.len() as u16).to_le_bytes());
        data.extend(comment);
        data
    }

    #[test]
    fn reads_the_archive() {
        let dir = std::env::temp_dir().join(format!("glitu-play-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut png = Vec::new();
        let mut enc = png::Encoder::new(&mut png, 1, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.write_header().unwrap().write_image_data(&[1, 2, 3]).unwrap();
        let deck = br#"{ "glitu-play": 1, "title": "T", "width": 1, "height": 1, "clicks": [ { "slide": 1, "image": "001.png" } ] }"#;
        let file = dir.join("talk.glituplay");
        fs::write(&file, zip(&[("deck.json", deck), ("001.png", &png)], b"a comment")).unwrap();
        let d = Deck::open(&file).unwrap();
        assert_eq!(d.title, "T");
        assert_eq!(d.image(0).unwrap().pixels, [0x010203]);
        assert!(d.file("002.png").err().unwrap().contains("missing"));
        fs::write(&file, b"not an archive").unwrap();
        assert!(Deck::open(&file).err().unwrap().contains("not a .glituplay file"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_past_the_last_slide_opens_the_last_click() {
        let text = r#"{ "glitu-play": 1, "width": 4, "height": 2, "clicks": [ { "slide": 1, "image": "1.png" },
            { "slide": 2, "image": "2.png" }, { "slide": 2, "image": "3.png" }, { "slide": 3, "image": "4.png" } ] }"#;
        let d = Deck::parse(text, Archive::default()).unwrap();
        assert_eq!([1, 2, 3].map(|n| d.start_click(n)), [0, 1, 3]);
        assert_eq!((d.start_click(4), d.start_click(9999)), (3, 3));
    }

    #[test]
    fn a_deck_with_a_size_out_of_range_is_refused() {
        let deck = |size: &str, film: &str, backdrop: &str| {
            let text = format!(
                r#"{{ "glitu-play": 1, {size} "clicks": [ {{ "slide": 1, "image": "1.png" {film} }} ] {backdrop} }}"#
            );
            Deck::parse(&text, Archive::default()).err()
        };
        let ok = r#""width": 100, "height": 50,"#;
        assert_eq!(deck(ok, "", ""), None);
        // the slides: missing, null, huge, too wide for their area
        for size in [
            "",
            r#""width": 100,"#,
            r#""width": 0, "height": 50,"#,
            r#""width": 100, "height": 0,"#,
            r#""width": 4294967396, "height": 50,"#,
            r#""width": 16385, "height": 50,"#,
            r#""width": 16384, "height": 16384,"#,
            r#""width": -1, "height": 50,"#,
        ] {
            assert!(deck(size, "", "").is_some(), "{size}");
        }
        // the film: outside the slide, empty, huge, not whole
        for rect in [
            "[0, 0, 101, 50]",
            "[1, 0, 100, 50]",
            "[0, 0, 0, 10]",
            "[0, 0, 10, 0]",
            "[4294967297, 0, 10, 10]",
            "[0, 18446744073709551615, 10, 10]",
            "[0, 0, 10]",
            "[0, 0, 10, 10, 10]",
            "[0, 0, 1.5, 10]",
            "[0, 0, -1, 10]",
            "null",
        ] {
            let film = format!(r#", "film": {{ "rect": {rect}, "frames": [] }}"#);
            assert!(deck(ok, &film, "").is_some(), "{rect}");
        }
        assert_eq!(deck(ok, r#", "film": { "rect": [90, 40, 10, 10], "frames": [] }"#, ""), None);
        // the backdrop: its place and its size
        // an arrow: far from the slide, or wide
        for arrow in [
            r#"{ "points": [0, 0, 40000, 5], "width": 3 }"#,
            r#"{ "points": [0, 0, 1e300, 5], "width": 3 }"#,
            r#"{ "points": [0, -32769, 4, 5], "width": 3 }"#,
            r#"{ "points": [0, 0, 4, 5], "width": 1e9 }"#,
            r#"{ "points": [0, 0, 4, 5], "width": -1 }"#,
        ] {
            let draws = format!(r#", "draws": [ {arrow} ]"#);
            assert!(deck(ok, &draws, "").is_some(), "{arrow}");
        }
        assert_eq!(deck(ok, r#", "draws": [ { "points": [-32768, 0, 32768, 5], "width": 16384 } ]"#, ""), None);
        let backdrop = |rect: &str, size: &str| format!(r#", "backdrop": {{ "rect": {rect}, "size": {size} }}"#);
        assert_eq!(deck(ok, "", &backdrop("[0, 0, 100, 50]", "[1280, 720]")), None);
        for (rect, size) in [
            ("[0, 0, 0, 50]", "[1280, 720]"),
            ("[0, 0, 100, 50]", "[0, 720]"),
            ("[0, 0, 100, 50]", "[1280, 0]"),
            ("[0, 0, 100, 50]", "[1e300, 720]"),
            ("[0, 0, 1e30, 50]", "[1280, 720]"),
            ("[0, 0, 16384, 16384]", "[1280, 720]"),
            ("[1e30, 0, 100, 50]", "[1280, 720]"),
            ("[]", "[]"),
        ] {
            assert!(deck(ok, "", &backdrop(rect, size)).is_some(), "{rect} {size}");
        }
    }

    #[test]
    fn a_deck_naming_missing_images_or_another_size_is_refused() {
        let dir = folder("images");
        let png = |w: u32, h: u32| {
            let mut out = Vec::new();
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(png::ColorType::Rgb);
            enc.write_header().unwrap().write_image_data(&vec![0; (w * h * 3) as usize]).unwrap();
            out
        };
        let open = |deck: &str, files: &[(&str, &[u8])]| {
            let mut all = vec![("deck.json", deck.as_bytes())];
            all.extend_from_slice(files);
            let file = dir.join("talk.glituplay");
            fs::write(&file, zip(&all, b"")).unwrap();
            Deck::open(&file).err()
        };
        let head = r#"{ "glitu-play": 1, "width": 4, "height": 2, "clicks": [ { "slide": 1, "image": "1.png""#;
        let (ok, wide, not_png) = (png(4, 2), png(8, 2), b"short".to_vec());
        assert_eq!(open(&format!("{head} }} ] }}"), &[("1.png", &ok)]), None);
        let e = open(&format!("{head} }} ] }}"), &[]).unwrap();
        assert!(e.contains("1.png: missing"), "{e}");
        let e = open(&format!("{head} }} ] }}"), &[("1.png", &wide)]).unwrap();
        assert!(e.contains("1.png: 8 × 2 pixels, the slides are 4 × 2"), "{e}");
        let e = open(&format!("{head} }} ] }}"), &[("1.png", &not_png)]).unwrap();
        assert!(e.contains("not a PNG"), "{e}");
        let staged = format!(r#"{head}, "base": "b.png", "drawn": "d.png" }} ] }}"#);
        let e = open(&staged, &[("1.png", &ok), ("b.png", &ok)]).unwrap();
        assert!(e.contains("d.png: missing"), "{e}");
        let e = open(&staged, &[("1.png", &ok), ("b.png", &ok), ("d.png", &wide)]).unwrap();
        assert!(e.contains("d.png: 8 × 2"), "{e}");
        let film = format!(
            r#"{head}, "film": {{ "rect": [0, 0, 2, 2], "frames": [ {{ "t": 1, "image": "f.png" }} ] }} }} ] }}"#
        );
        let e = open(&film, &[("1.png", &ok)]).unwrap();
        assert!(e.contains("f.png: missing"), "{e}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// A fresh folder for a test
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("glitu-play-test-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(bytes: &mut [u8], at: usize, v: u32) {
        bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn an_archive_announcing_sizes_larger_than_itself_is_refused() {
        let dir = folder("sizes");
        let deck = br#"{ "glitu-play": 1, "width": 1, "height": 1, "clicks": [ { "slide": 1, "image": "1.png" } ] }"#;
        let good = zip(&[("deck.json", deck)], b"");
        let end = good.len() - 22;
        let central = u32::from_le_bytes(good[end + 16..end + 20].try_into().unwrap()) as usize;
        let mut bad = Vec::new();
        // the central directory, then an entry, larger than the file; more entries than it holds
        for (at, v) in [(end + 12, 0xffff_fff0), (central + 20, 0xffff_fff0), (end + 10, 9)] {
            let mut z = good.clone();
            put(&mut z, at, v);
            bad.push(z);
        }
        let mut z = good.clone();
        put(&mut z, end + 16, 0xffff_fff0);
        bad.push(z);
        for z in bad {
            let file = dir.join("talk.glituplay");
            fs::write(&file, &z).unwrap();
            let e = Deck::open(&file).err().unwrap_or_default();
            assert!(e.contains("damaged archive"), "{e}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn decodes_rgb_and_rgba() {
        let mut png = Vec::new();
        let mut enc = png::Encoder::new(&mut png, 2, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&[255, 0, 0, 255, 0, 0, 255, 128]).unwrap();
        let img = decode(&png).unwrap();
        assert_eq!((img.width, img.height, img.pixels), (2, 1, vec![0xff0000, 0x0000ff]));
    }
}
