//! Scaling of the images to the screen: a separable Catmull-Rom filter (bicubic, B = 0,
//! C = ½). A pass along the rows, then one along the columns, each a short sum
//! of weighted pixels on contiguous memory that the compiler vectorises; the weights are
//! computed once per image. Catmull-Rom keeps the text edges as sharp as the bicubic filter of
//! tiny-skia it replaces, at a fraction of its cost (a float pipeline per pixel: about a
//! second per slide at 1920 × 1080 on an Atom). When the image shrinks, the filter widens with
//! the scale, so that no detail aliases.

use wide::{i16x8, i32x4, i32x8, u8x16};

use crate::deck::Image;

/// Catmull-Rom at `x` (in source pixels)
fn catmull_rom(x: f32) -> f32 {
    let x = x.abs();
    if x < 1.0 {
        1.5 * x * x * x - 2.5 * x * x + 1.0
    } else if x < 2.0 {
        -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0
    } else {
        0.0
    }
}

/// For each output pixel along one axis: the first of its `n` source pixels and their weights
/// (sum 1)
struct Taps {
    n: usize,
    first: Vec<isize>,
    weight: Vec<f32>,
}

impl Taps {
    fn new(dst: usize, scale: f32) -> Taps {
        let widen = (1.0 / scale).max(1.0);
        let radius = 2.0 * widen;
        // the source pixels closer than `radius`: 4 when the image grows
        let n = (2.0 * radius).ceil() as usize;
        let (mut first, mut weight) = (Vec::with_capacity(dst), Vec::with_capacity(dst * n));
        let mut w = vec![0.0f32; n];
        for i in 0..dst {
            let center = (i as f32 + 0.5) / scale - 0.5;
            let start = (center - radius).floor() as isize + 1;
            for (j, w) in w.iter_mut().enumerate() {
                *w = catmull_rom(((start + j as isize) as f32 - center) / widen);
            }
            let sum: f32 = w.iter().sum();
            first.push(start);
            weight.extend(w.iter().map(|w| w / sum));
        }
        Taps { n, first, weight }
    }
}

/// A channel of the second pass (levels × 32) to 0..=255
fn level(acc: i16) -> u32 {
    ((i32::from(acc) + 16) >> 5).clamp(0, 255) as u32
}

/// The high half of `a × b`
fn high(a: i16, b: i16) -> i16 {
    ((i32::from(a) * i32::from(b)) >> 16) as i16
}

/// The weights of a pass along the rows, two taps per vector: (w0 w1) × 4, (w2 w3) × 4…, in
/// units of 2⁻¹⁴
fn pairs(taps: &Taps) -> Vec<i16x8> {
    let pairs = taps.n.div_ceil(2);
    taps.weight
        .chunks_exact(taps.n)
        .flat_map(|w| {
            let q = |j: usize| w.get(j).map_or(0, |&w| (w * 16384.0).round() as i16);
            (0..pairs).map(move |k| {
                let (a, b) = (q(2 * k), q(2 * k + 1));
                i16x8::from([a, b, a, b, a, b, a, b])
            })
        })
        .collect()
}

/// Along the rows: the source rows `rows` (of `sw` pixels) at the output width into `out`,
/// 4 channels per pixel (B, G, R, 0) in levels × 64. Two taps at a time: the source pixels by
/// pairs, their channels side by side (B B G G R R 0 0), multiplied by two weights and summed
/// by pair (`pmaddwd`)
fn across(rows: &[u32], sw: usize, taps: &Taps, weights: &[i16x8], out: &mut [i16]) {
    let pairs = taps.n.div_ceil(2);
    let dw = taps.first.len();
    // each source row extended by its edge pixels
    let pad = 2 * pairs + 1;
    let mut src = vec![0u32; sw + 2 * pad];
    let mut side = vec![i16x8::ZERO; sw + 2 * pad - 1];
    let half = i32x4::splat(1 << 7);
    let top = i16x8::splat(255 << 6);
    for (row, out) in rows.chunks_exact(sw).zip(out.chunks_exact_mut(dw * 4)) {
        src[..pad].fill(row[0]);
        src[pad + sw..].fill(row[sw - 1]);
        src[pad..pad + sw].copy_from_slice(row);
        for (s, p) in side.iter_mut().zip(src.windows(2)) {
            let (a, b) = (p[0].to_le_bytes().map(i16::from), p[1].to_le_bytes().map(i16::from));
            *s = i16x8::from([a[0], b[0], a[1], b[1], a[2], b[2], 0, 0]);
        }
        let pixel = |o: usize| {
            let at = (taps.first[o] + pad as isize).clamp(0, (sw + pad) as isize) as usize;
            let weight = &weights[o * pairs..][..pairs];
            let acc = weight.iter().enumerate().fold(half, |acc, (k, &w)| acc + side[at + 2 * k].dot(w));
            // in levels × 2¹⁴: to levels × 64
            acc >> 8_i32
        };
        // two pixels at a time, packed into 16 bits
        let (two, rest) = out.as_chunks_mut::<8>();
        for (i, out) in two.iter_mut().enumerate() {
            let both: i32x8 = bytemuck::cast([pixel(2 * i), pixel(2 * i + 1)]);
            *out = i16x8::from_i32x8_saturate(both).max(i16x8::ZERO).min(top).to_array();
        }
        if !rest.is_empty() {
            let last = pixel(dw - 1).max(i32x4::ZERO).min(i32x4::splat(255 << 6)).to_array();
            for (o, v) in rest.iter_mut().zip(last) {
                *o = v as i16;
            }
        }
    }
}

/// Along the columns: the output rows from `y0` into `out`, each a weighted sum of whole rows of
/// `wide` (`sh` rows), in 16 bits: the high half of (levels × 64) × (weight × 2¹⁵) is in
/// levels × 32
fn down(wide: &[i16], sh: usize, taps: &Taps, y0: usize, out: &mut [u32]) {
    let dw = wide.len() / sh / 4;
    let line = dw * 4;
    let mut rows: Vec<(&[i16], i16)> = Vec::with_capacity(taps.n);
    let round = i16x8::splat(16);
    let lines = taps.first[y0..].iter().zip(taps.weight[y0 * taps.n..].chunks_exact(taps.n));
    for ((&first, weight), out) in lines.zip(out.chunks_exact_mut(dw)) {
        rows.clear();
        for (j, &w) in weight.iter().enumerate() {
            let y = (first + j as isize).clamp(0, sh as isize - 1) as usize;
            rows.push((&wide[y * line..][..line], (w * 32768.0).round().clamp(-32768.0, 32767.0) as i16));
        }
        // 4 pixels at a time: 16 channels
        let (quads, rest) = out.as_chunks_mut::<4>();
        for (q, out) in quads.iter_mut().enumerate() {
            let (mut lo, mut hi) = (round, round);
            for &(row, w) in &rows {
                let w = i16x8::splat(w);
                lo += i16x8::mul_keep_high(i16x8::from_slice_unaligned(&row[q * 16..]), w);
                hi += i16x8::mul_keep_high(i16x8::from_slice_unaligned(&row[q * 16 + 8..]), w);
            }
            let bytes = u8x16::narrow_i16x8(lo >> 5_i32, hi >> 5_i32).to_array();
            for (o, b) in out.iter_mut().zip(bytes.as_chunks::<4>().0) {
                *o = u32::from_le_bytes(*b);
            }
        }
        let done = quads.len() * 4;
        for (x, o) in rest.iter_mut().enumerate() {
            let c = |c: usize| {
                let i = (done + x) * 4 + c;
                level(rows.iter().fold(0i16, |a, &(row, w)| a.wrapping_add(high(row[i], w))))
            };
            *o = c(0) | (c(1) << 8) | (c(2) << 16);
        }
    }
}

/// `work` on the rows of `out` (`line` items per row), in two halves at once: the slow targets
/// have two cores, idle between two clicks
fn halves<T: Send>(out: &mut [T], line: usize, work: impl Fn(usize, &mut [T]) + Sync) {
    let rows = out.len() / line.max(1);
    if rows < 64 {
        return work(0, out);
    }
    let mid = rows / 2;
    let (a, b) = out.split_at_mut(mid * line);
    std::thread::scope(|scope| {
        scope.spawn(|| work(mid, b));
        work(0, a);
    });
}

/// `img` scaled by `scale`, into an image of `size`; scale 1: the image itself, untouched.
/// Both passes in 16-bit integers, 8 channels per vector (`wide`: SSE2, NEON; the compiler alone
/// does not vectorise them well without SSE4.1, that the Atoms lack); between them, the image in
/// levels × 64
pub fn resample(img: Image, size: (u32, u32), scale: f32) -> Image {
    if scale == 1.0 {
        return img;
    }
    let (sw, sh) = (img.width as usize, img.height as usize);
    let (dw, dh) = (size.0.max(1) as usize, size.1.max(1) as usize);
    if sw == 0 || sh == 0 {
        return Image { width: dw as u32, height: dh as u32, pixels: vec![0; dw * dh] };
    }
    let (along, columns) = (Taps::new(dw, scale), Taps::new(dh, scale));
    let weights = pairs(&along);
    let mut wide = vec![0i16; dw * 4 * sh];
    halves(&mut wide, dw * 4, |y, out| across(&img.pixels[y * sw..], sw, &along, &weights, out));
    let mut pixels = vec![0u32; dw * dh];
    halves(&mut pixels, dw, |y, out| down(&wide, sh, &columns, y, out));
    Image { width: dw as u32, height: dh as u32, pixels }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_image_stays_flat() {
        let img = Image { width: 4, height: 2, pixels: vec![0x808080; 8] };
        let small = resample(img.clone(), (2, 1), 0.5);
        assert_eq!((small.width, small.height, small.pixels[0]), (2, 1, 0x808080));
        let big = resample(img, (8, 4), 2.0);
        assert!(big.pixels.iter().all(|&p| p == 0x808080));
    }

    #[test]
    fn an_edge_stays_sharp_and_in_place() {
        // black then white, scaled ×1.875 (1024 → 1920)
        let pixels = (0..64).map(|x| if x < 32 { 0 } else { 0xffffff }).collect();
        let img = resample(Image { width: 64, height: 1, pixels }, (120, 1), 1.875);
        let g = |x: usize| img.pixels[x] & 0xff;
        assert_eq!((g(0), g(50), g(70), g(119)), (0, 0, 255, 255));
        // the edge at 32 × 1.875 = 60, over two pixels
        let edge: Vec<u32> = (57..63).map(g).collect();
        assert!(edge[1] < 10 && edge[4] > 245 && edge[2] < 128 && edge[3] > 128, "{edge:?}");
        // the channels are kept apart, the 4th byte stays 0
        let img = resample(Image { width: 3, height: 3, pixels: vec![0x10e080; 9] }, (5, 5), 1.7);
        assert!(img.pixels.iter().all(|&p| p == 0x10e080));
    }
}
