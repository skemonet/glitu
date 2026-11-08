//! The animation from one click to the next, between screenshots of Chromium: only the pixels
//! that differ change. When arrows are drawn, the export gives two states on the way: `base`
//! (the click without its arrows) and `drawn` (the arrows, without their heads nor the cards
//! that wait for them). Each pixel then adds three changes, each with its own timing:
//! - from the previous click to `base`: a cross-fade;
//! - from `base` to `drawn`: under the stroke of an arrow, cut at the length reached, as the
//!   browser draws it;
//! - from `drawn` to the click: the fade of the rectangle of a head or of a waiting card.
//!
//! The last frame is the click's image itself: what the browser shows, pixel for pixel.

use tiny_skia::{FillRule, LineCap, LineJoin, Mask, PathBuilder, Stroke, Transform};

use std::sync::Arc;

use crate::deck::{Draw, Fade, Image};

/// The `ease` curve of CSS: cubic-bezier(.25, .1, .25, 1)
pub fn ease(t: f64) -> f64 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0, t)
}

fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    if t <= 0.0 || t >= 1.0 {
        return t.clamp(0.0, 1.0);
    }
    let at = |a: f64, b: f64, s: f64| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s * s * s;
    // the parameter s for which x(s) = t, by bisection (x is monotonic)
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if at(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    at(y1, y2, (lo + hi) / 2.0)
}

/// Progress of a motion at `t` ms, from 0 to 255
fn progress(t: f64, delay: f64, duration: f64) -> u32 {
    let p = if duration <= 0.0 { f64::from(t >= delay) } else { (t - delay) / duration };
    (ease(p) * 255.0).round() as u32
}

/// A run of pixels of one row that follow the same motions: from `x`, `len` pixels of row `y`
/// (pixels of the slide); `reveal` the arrow that reveals them, `fade` the rectangle that fades
/// them (`NONE`: the cross-fade's timing). Unchanged pixels inside a run come out unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Span {
    y: u32,
    x: u32,
    len: u32,
    reveal: u16,
    fade: u16,
}

const NONE: u16 = u16::MAX;

/// Rows taller than this are shared between two threads (the slow targets have two cores)
const SPLIT: usize = 200_000;

/// An arrow: its path, and the mask of the part drawn, over the box of the path
struct Reveal {
    draw: Draw,
    length: f32,
    origin: (i32, i32),
    mask: Mask,
}

pub struct Transition {
    /// The previous click, `base`, `drawn`, the click
    images: [Arc<Image>; 4],
    spans: Vec<Span>,
    fades: Vec<Fade>,
    /// The progress of each fade, for the frame being rendered
    faded: Vec<u32>,
    reveals: Vec<Reveal>,
    /// Duration of the cross-fade, in ms
    global: f64,
    duration: f64,
    /// The box of the changed pixels: (x, y, width, height)
    pub area: Option<(u32, u32, u32, u32)>,
}

fn polyline(points: &[(f32, f32)], length: Option<f32>) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let (&(x0, y0), rest) = points.split_first()?;
    pb.move_to(x0, y0);
    let (mut left, mut prev) = (length.unwrap_or(f32::INFINITY), (x0, y0));
    for &(x, y) in rest {
        let d = (x - prev.0).hypot(y - prev.1);
        if d >= left {
            let k = if d > 0.0 { left / d } else { 0.0 };
            pb.line_to(prev.0 + (x - prev.0) * k, prev.1 + (y - prev.1) * k);
            break;
        }
        pb.line_to(x, y);
        left -= d;
        prev = (x, y);
    }
    pb.finish()
}

fn stroke(draw: &Draw) -> Stroke {
    Stroke { width: draw.width, line_cap: LineCap::Butt, line_join: LineJoin::Round, ..Stroke::default() }
}

/// Draws into `mask` the stroke of `draw` up to `length` (all of it: None)
fn paint(mask: &mut Mask, origin: (i32, i32), draw: &Draw, length: Option<f32>) {
    mask.clear();
    if length.is_some_and(|l| l <= 0.0) {
        return;
    }
    let Some(path) = polyline(&draw.points, length) else { return };
    let Some(outline) = path.stroke(&stroke(draw), 1.0) else { return };
    let shift = Transform::from_translate(-origin.0 as f32, -origin.1 as f32);
    mask.fill_path(&outline, FillRule::Winding, true, shift);
}

impl Reveal {
    fn new(draw: &Draw) -> Option<Reveal> {
        let pad = draw.width / 2.0 + 2.0;
        let xs = draw.points.iter().map(|p| p.0);
        let ys = draw.points.iter().map(|p| p.1);
        let (x0, x1) = (xs.clone().fold(f32::INFINITY, f32::min), xs.fold(f32::NEG_INFINITY, f32::max));
        let (y0, y1) = (ys.clone().fold(f32::INFINITY, f32::min), ys.fold(f32::NEG_INFINITY, f32::max));
        if !x0.is_finite() || !y0.is_finite() {
            return None;
        }
        let origin = ((x0 - pad).floor() as i32, (y0 - pad).floor() as i32);
        let mask = Mask::new((x1 - x0 + 2.0 * pad).ceil() as u32 + 1, (y1 - y0 + 2.0 * pad).ceil() as u32 + 1)?;
        let length = draw.points.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
        Some(Reveal { draw: draw.clone(), length, origin, mask })
    }

    fn coverage(&self, x: u32, y: u32) -> u32 {
        let (mx, my) = (x as i32 - self.origin.0, y as i32 - self.origin.1);
        if mx < 0 || my < 0 || mx >= self.mask.width() as i32 || my >= self.mask.height() as i32 {
            return 0;
        }
        u32::from(self.mask.data()[my as usize * self.mask.width() as usize + mx as usize])
    }
}

/// The previous click plus the three changes, by `k` of 255 each, channel by channel
fn compose(v: &[u32; 4], k: [u32; 3]) -> u32 {
    let ch = |s: u32| {
        let c = |i: usize| ((v[i] >> s) & 0xff) as i32;
        let delta = (0..3).map(|i| (c(i + 1) - c(i)) * k[i] as i32).sum::<i32>();
        ((c(0) * 255 + delta + 127) / 255).clamp(0, 255) as u32
    };
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// The cross-fade from `a` to `b` by `k` of 255, byte by byte: the same rounding as `compose`
/// with three equal steps. Written on contiguous bytes in u16, so that it vectorises
fn blend(a: &[u32], b: &[u32], out: &mut [u32], k: u32) {
    let (k, j) = (k as u16, 255 - k as u16);
    let (a, b): (&[u8], &[u8]) = (bytemuck::cast_slice(a), bytemuck::cast_slice(b));
    let out: &mut [u8] = bytemuck::cast_slice_mut(out);
    for ((o, &a), &b) in out.iter_mut().zip(a).zip(b) {
        *o = ((u16::from(a) * j + u16::from(b) * k + 127) / 255) as u8;
    }
}

/// What a frame needs, shared by the threads that render it
struct Frame<'a> {
    images: &'a [Arc<Image>; 4],
    reveals: &'a [Reveal],
    faded: &'a [u32],
    global: u32,
    /// End of the transition: the click's image
    end: bool,
}

impl Frame<'_> {
    /// `spans` into `out`, whose first row is the slide's row `top` (`stride` pixels per row,
    /// the slide at `left`)
    fn render(&self, spans: &[Span], out: &mut [u32], top: u32, stride: usize, left: usize) {
        let w = self.images[0].width as usize;
        for s in spans {
            let (x, len) = (s.x as usize, s.len as usize);
            let at = s.y as usize * w + x;
            let dst = &mut out[(s.y - top) as usize * stride + left + x..][..len];
            let [from, base, drawn, to] = self.images.each_ref().map(|i| &i.pixels[at..at + len]);
            if self.end {
                dst.copy_from_slice(to);
            } else if s.reveal == NONE && s.fade == NONE {
                blend(from, to, dst, self.global);
            } else {
                let fade = if s.fade == NONE { self.global } else { self.faded[usize::from(s.fade)] };
                for (i, d) in dst.iter_mut().enumerate() {
                    let reveal = match s.reveal {
                        NONE => self.global,
                        r => self.reveals[usize::from(r)].coverage(s.x + i as u32, s.y),
                    };
                    *d = compose(&[from[i], base[i], drawn[i], to[i]], [self.global, reveal, fade]);
                }
            }
        }
    }
}

/// The columns of a row from the first to the last where the images differ
fn changed(row: &[&[u32]]) -> Option<(usize, usize)> {
    let differs = |x: &usize| row[1..].iter().any(|r| r[*x] != row[0][*x]);
    let first = (0..row[0].len()).find(differs)?;
    let last = (first..row[0].len()).rev().find(differs)?;
    Some((first, last + 1))
}

impl Transition {
    /// From `from` to `to` (images of the same size), by `stages` (`base`, `drawn`) if arrows
    /// are drawn; `global`: duration of the cross-fade
    pub fn new(
        from: Arc<Image>,
        stages: Option<(Arc<Image>, Arc<Image>)>,
        to: Arc<Image>,
        draws: &[Draw],
        fades: &[Fade],
        global: f64,
    ) -> Transition {
        let mut reveals: Vec<Reveal> = draws.iter().filter_map(Reveal::new).collect();
        for r in &mut reveals {
            paint(&mut r.mask, r.origin, &r.draw, None);
        }
        let simple = stages.is_none() && reveals.is_empty() && fades.is_empty();
        let (base, drawn) = stages.unwrap_or_else(|| (Arc::clone(&to), Arc::clone(&to)));
        let images = [from, base, drawn, to];
        let inside = |f: &Fade, x: f32, y: f32| {
            let (fx, fy, fw, fh) = f.rect;
            x >= fx && x < fx + fw && y >= fy && y < fy + fh
        };
        let w = images[0].width as usize;
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        let mut spans = Vec::new();
        for y in 0..images[0].height as usize {
            let rows = images.each_ref().map(|i| &i.pixels[y * w..(y + 1) * w]);
            // only the cross-fade: the two ends are enough
            let found = if simple { changed(&[rows[0], rows[3]]) } else { changed(&rows) };
            let Some((first, last)) = found else { continue };
            let y = y as u32;
            (x0, y0, x1, y1) = (x0.min(first as u32), y0.min(y), x1.max(last as u32), y1.max(y));
            if simple {
                spans.push(Span { y, x: first as u32, len: (last - first) as u32, reveal: NONE, fade: NONE });
                continue;
            }
            // the motions of each pixel: runs of the same ones
            for x in first..last {
                let v = rows.map(|r| r[x]);
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                let reveal = if v[1] == v[2] { None } else { reveals.iter().position(|r| r.coverage(x as u32, y) > 0) };
                let fade = if v[2] == v[3] { None } else { fades.iter().position(|f| inside(f, cx, cy)) };
                let index = |p: Option<usize>| p.map_or(NONE, |p| p as u16);
                let (reveal, fade) = (index(reveal), index(fade));
                match spans.last_mut() {
                    Some(s) if s.y == y && s.x + s.len == x as u32 && (s.reveal, s.fade) == (reveal, fade) => {
                        s.len += 1;
                    }
                    _ => spans.push(Span { y, x: x as u32, len: 1, reveal, fade }),
                }
            }
        }
        let ends = fades.iter().map(|f| f.delay + f.duration).chain(draws.iter().map(|d| d.delay + d.duration));
        let duration = ends.fold(global, f64::max);
        let area = (!spans.is_empty()).then(|| (x0, y0, x1 - x0, y1 - y0 + 1));
        Transition {
            images,
            spans,
            fades: fades.to_vec(),
            faded: vec![0; fades.len()],
            reveals,
            global,
            duration,
            area,
        }
    }

    /// In ms
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// The frame at `t` ms into `frame`, an image of `stride` pixels per line where the slide
    /// starts at `offset` (x, y)
    pub fn render(&mut self, t: f64, frame: &mut [u32], stride: usize, offset: (usize, usize)) {
        // the end: the click's image, even where a stroke only partly covers a pixel
        let end = t >= self.duration;
        if !end {
            for (k, f) in self.faded.iter_mut().zip(&self.fades) {
                *k = progress(t, f.delay, f.duration);
            }
            for r in &mut self.reveals {
                let p = ease((t - r.draw.delay) / r.draw.duration.max(1.0)) as f32;
                paint(&mut r.mask, r.origin, &r.draw, (p < 1.0).then_some(r.length * p));
            }
        }
        let f = Frame {
            images: &self.images,
            reveals: &self.reveals,
            faded: &self.faded,
            global: progress(t, 0.0, self.global),
            end,
        };
        let Some((_, y0, _, h)) = self.area else { return };
        let frame = &mut frame[(offset.1 + y0 as usize) * stride..];
        let pixels: usize = self.spans.iter().map(|s| s.len as usize).sum();
        if pixels < SPLIT || h < 2 {
            return f.render(&self.spans, frame, y0, stride, offset.0);
        }
        // two halves of rows, one per thread
        let mid = y0 + h / 2;
        let cut = self.spans.partition_point(|s| s.y < mid);
        let (top, bottom) = self.spans.split_at(cut);
        let (a, b) = frame.split_at_mut((mid - y0) as usize * stride);
        std::thread::scope(|scope| {
            scope.spawn(|| f.render(bottom, b, mid, stride, offset.0));
            f.render(top, a, y0, stride, offset.0);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: u32, h: u32, px: u32) -> Image {
        Image { width: w, height: h, pixels: vec![px; (w * h) as usize] }
    }

    #[test]
    fn the_ease_curve_of_css() {
        assert_eq!((ease(0.0), ease(1.0)), (0.0, 1.0));
        // cubic-bezier(.25,.1,.25,1) at half time is at about 80 %
        assert!((ease(0.5) - 0.8024).abs() < 1e-3, "{}", ease(0.5));
        assert_eq!((progress(-5.0, 0.0, 100.0), progress(500.0, 0.0, 100.0)), (0, 255));
        let (a, b) = (0x000000, 0xffffff);
        assert_eq!(compose(&[a, b, b, b], [255, 0, 0]), b);
        assert_eq!(compose(&[a, a, a, b], [255, 255, 0]), a);
        assert_eq!(compose(&[0x102030; 4], [77, 3, 200]), 0x102030);
        // a head over a box that appears: the box first, the head later
        assert_eq!(compose(&[a, 0x808080, 0x808080, b], [255, 0, 0]), 0x808080);
    }

    #[test]
    fn a_cross_fade_ends_on_the_next_image() {
        let (a, mut b) = (image(4, 3, 0x000000), image(4, 3, 0x000000));
        b.pixels[5] = 0xffffff;
        let mut tr = Transition::new(a.clone().into(), None, b.clone().into(), &[], &[], 300.0);
        assert_eq!((tr.area, tr.duration()), (Some((1, 1, 1, 1)), 300.0));
        let mut frame = a.pixels.clone();
        tr.render(150.0, &mut frame, 4, (0, 0));
        assert!(frame[5] > 0 && frame[5] < 0xffffff);
        tr.render(300.0, &mut frame, 4, (0, 0));
        assert_eq!(frame, b.pixels);
    }

    #[test]
    fn an_arrow_is_revealed_from_its_start() {
        // a horizontal arrow from x = 2 to x = 38 on line 5
        let (a, mut b) = (image(40, 10, 0), image(40, 10, 0));
        for x in 2..38 {
            b.pixels[5 * 40 + x] = 0xffffff;
        }
        let draw = Draw { points: vec![(2.0, 5.5), (38.0, 5.5)], width: 3.0, delay: 0.0, duration: 600.0 };
        let mut tr = Transition::new(
            a.clone().into(),
            Some((a.clone().into(), b.clone().into())),
            b.clone().into(),
            std::slice::from_ref(&draw),
            &[],
            300.0,
        );
        assert!(tr.spans.iter().all(|s| s.reveal == 0 && s.fade == NONE));
        let mut frame = a.pixels.clone();
        tr.render(200.0, &mut frame, 40, (0, 0));
        // the start is drawn, the end not yet
        assert_eq!((frame[5 * 40 + 3], frame[5 * 40 + 36]), (0xffffff, 0));
        tr.render(600.0, &mut frame, 40, (0, 0));
        assert_eq!(frame, b.pixels);
        // a pixel at the edge of the stroke, only partly covered by it, ends on the next image
        b.pixels[3 * 40 + 10] = 0x808080;
        let mut tr = Transition::new(
            a.clone().into(),
            Some((a.clone().into(), b.clone().into())),
            b.clone().into(),
            std::slice::from_ref(&draw),
            &[],
            300.0,
        );
        tr.render(600.0, &mut frame, 40, (0, 0));
        assert_eq!(frame, b.pixels);
    }

    #[test]
    fn a_fade_waits_for_its_delay() {
        let (a, b) = (image(4, 4, 0), image(4, 4, 0xffffff));
        let fade = Fade { rect: (0.0, 0.0, 2.0, 4.0), delay: 560.0, duration: 120.0 };
        let mut tr = Transition::new(
            a.clone().into(),
            Some((a.clone().into(), a.clone().into())),
            b.clone().into(),
            &[],
            std::slice::from_ref(&fade),
            300.0,
        );
        assert_eq!(tr.duration(), 680.0);
        // in a frame of 6 × 5 pixels, the slide at (1, 1)
        let mut frame = vec![0; 30];
        tr.render(400.0, &mut frame, 6, (1, 1));
        assert_eq!((frame[6 + 1], frame[6 + 4]), (0, 0xffffff));
    }

    /// Images of `w` × `h` where about one pixel in `odds` differs from `a`
    fn noisy(a: &Image, seed: &mut u64, odds: u64) -> Image {
        let mut next = || {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *seed >> 33
        };
        let pixels = a.pixels.iter().map(|&p| if next() % odds == 0 { next() as u32 & 0xffffff } else { p }).collect();
        Image { pixels, ..a.clone() }
    }

    /// The frame at `t` pixel by pixel, as the transition was first written: `compose` on every
    /// pixel that differs
    fn reference(images: &[&Image; 4], tr: &Transition, t: f64) -> Vec<u32> {
        let w = images[0].width;
        let global = progress(t, 0.0, tr.global);
        let mut out = images[0].pixels.clone();
        for (i, o) in out.iter_mut().enumerate() {
            let v = images.map(|img| img.pixels[i]);
            if t >= tr.duration {
                *o = v[3];
                continue;
            }
            let (x, y) = (i as u32 % w, i as u32 / w);
            let span = tr.spans.iter().find(|s| s.y == y && x >= s.x && x < s.x + s.len);
            let (reveal, fade) = span.map_or((NONE, NONE), |s| (s.reveal, s.fade));
            let r = if reveal == NONE { global } else { tr.reveals[usize::from(reveal)].coverage(x, y) };
            let f = if fade == NONE { global } else { tr.faded[usize::from(fade)] };
            *o = compose(&v, [global, r, f]);
        }
        out
    }

    #[test]
    fn the_fast_cross_fade_is_the_general_one() {
        let mut seed = 7;
        // large enough to be rendered by two threads
        let a = noisy(&image(700, 400, 0x204060), &mut seed, 5);
        let b = noisy(&a, &mut seed, 3);
        let (fa, fb) = (Arc::new(a.clone()), Arc::new(b.clone()));
        let mut fast = Transition::new(Arc::clone(&fa), None, Arc::clone(&fb), &[], &[], 300.0);
        let mut general =
            Transition::new(Arc::clone(&fa), Some((Arc::clone(&fb), Arc::clone(&fb))), fb, &[], &[], 300.0);
        assert!(general.spans.len() > fast.spans.len() || general.spans == fast.spans);
        for t in [0.0, 40.0, 150.0, 299.0, 300.0] {
            let (mut x, mut y) = (a.pixels.clone(), a.pixels.clone());
            fast.render(t, &mut x, 700, (0, 0));
            general.render(t, &mut y, 700, (0, 0));
            assert!(x == y, "at {t} ms");
            assert!(x == reference(&[&a, &b, &b, &b], &fast, t), "at {t} ms");
        }
        // every 8-bit value and step, as `compose`
        let a: Vec<u32> = (0..=255).map(|v| v * 0x010101).collect();
        for b in [0, 1, 128, 254, 255] {
            let bs = vec![b * 0x010101; 256];
            for k in 0..=255 {
                let mut out = vec![0; 256];
                blend(&a, &bs, &mut out, k);
                assert!(
                    out.iter()
                        .zip(&a)
                        .all(|(&o, &a)| o == compose(&[a, b * 0x010101, b * 0x010101, b * 0x010101], [k; 3]))
                );
            }
        }
    }

    #[test]
    fn arrows_and_fades_on_random_images_as_pixel_by_pixel() {
        let mut seed = 11;
        let a = noisy(&image(120, 60, 0xffffff), &mut seed, 4);
        let base = noisy(&a, &mut seed, 3);
        let drawn = noisy(&base, &mut seed, 3);
        let to = noisy(&drawn, &mut seed, 3);
        let draw =
            Draw { points: vec![(5.0, 5.0), (100.0, 50.0), (110.0, 10.0)], width: 6.0, delay: 50.0, duration: 400.0 };
        let fade = Fade { rect: (20.0, 10.0, 60.0, 30.0), delay: 300.0, duration: 200.0 };
        let arc = |i: &Image| Arc::new(i.clone());
        let mut tr = Transition::new(
            arc(&a),
            Some((arc(&base), arc(&drawn))),
            arc(&to),
            std::slice::from_ref(&draw),
            std::slice::from_ref(&fade),
            300.0,
        );
        for t in [0.0, 120.0, 260.0, 400.0, 499.0, 500.0] {
            // in a larger frame that shows the previous click, the slide at (3, 2)
            let mut frame = vec![0x123456; 130 * 70];
            crate::show::blit(&mut frame, (130, 70), &a, (3, 2));
            let mut outside = frame.clone();
            tr.render(t, &mut frame, 130, (3, 2));
            let want = reference(&[&a, &base, &drawn, &to], &tr, t);
            for y in 0..60 {
                let row = (y + 2) * 130 + 3;
                assert!(frame[row..row + 120] == want[y * 120..][..120], "row {y} at {t} ms");
                outside[row..row + 120].copy_from_slice(&frame[row..row + 120]);
            }
            assert!(frame == outside, "only the slide is drawn");
        }
    }
}
