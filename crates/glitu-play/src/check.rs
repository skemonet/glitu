//! `glitu-play --check`: the fidelity of the native backdrop. For each click under an animated
//! backdrop, the backdrop drawn natively at the instant of the export is compared with Chromium's
//! screenshot, where nothing covers it (opacity 0, outside the glass areas). The player shows the
//! screenshots themselves and adds only the backdrop's motion: what is compared here is what an
//! animated backdrop changes.

use std::path::Path;

use crate::backdrop::Backdrop;
use crate::deck::Deck;

/// Above, a pixel differs (out of 255, on its most different channel)
const TOLERANCE: u32 = 24;
/// Above, a click fails: the share of the backdrop's visible pixels that differ
pub const LIMIT: f64 = 2.0;

/// Share of the pixels compared that differ (in %), and their number
pub struct Score {
    pub differ: f64,
    pub compared: usize,
    /// The mean difference, out of 255
    pub mean: f64,
}

/// The pixels of `shot` against those of `drawn` (both of the backdrop's rectangle), where
/// `alpha` is 0 and outside `glass`
fn compare(shot: &[u32], drawn: &[u32], alpha: &[u8], width: usize, glass: &[(i32, i32, i32, i32)]) -> Score {
    let (mut compared, mut differ, mut total) = (0usize, 0usize, 0u64);
    for (i, (&a, &b)) in shot.iter().zip(drawn).enumerate() {
        let (x, y) = ((i % width) as i32, (i / width) as i32);
        if alpha.get(i) != Some(&0)
            || glass.iter().any(|&(gx, gy, gw, gh)| x >= gx && x < gx + gw && y >= gy && y < gy + gh)
        {
            continue;
        }
        compared += 1;
        let d = [16, 8, 0].iter().map(|s| ((a >> s) & 255).abs_diff((b >> s) & 255)).max().unwrap_or(0);
        total += u64::from(d);
        if d > TOLERANCE {
            differ += 1;
        }
    }
    let share = |n: f64| if compared == 0 { 0.0 } else { n / compared as f64 };
    Score { differ: share(differ as f64 * 100.0), compared, mean: share(total as f64) }
}

/// Checks each click; true if all pass
pub fn run(path: &Path) -> Result<bool, String> {
    let deck = Deck::open(path)?;
    let Some(info) = &deck.backdrop else {
        println!("no animated backdrop: glitu-play shows the screenshots as they are");
        return Ok(true);
    };
    let (rx, ry, rw, rh) = info.rect;
    let (ox, oy, w, h) = (rx.round() as i64, ry.round() as i64, rw.round() as u32, rh.round() as u32);
    let mut ok = true;
    let mut references: Vec<(f64, Vec<u32>)> = Vec::new();
    for (i, c) in deck.clicks.iter().enumerate() {
        let Some(under) = &c.under else { continue };
        let drawn = match references.iter().find(|(d, _)| *d == under.dim) {
            Some((_, px)) => px.clone(),
            None => {
                let px = Backdrop::reference(info, (w, h), under.dim).pixels;
                references.push((under.dim, px.clone()));
                px
            }
        };
        let img = deck.image(i)?;
        let (_, _, alpha) = deck.gray(&under.alpha)?;
        // the screenshot and its opacity, cut to the backdrop's rectangle
        let cut = |px: &dyn Fn(usize) -> u32| -> Vec<u32> {
            (0..h as i64)
                .flat_map(|y| (0..w as i64).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let (sx, sy) = (x + ox, y + oy);
                    if sx < 0 || sy < 0 || sx >= i64::from(img.width) || sy >= i64::from(img.height) {
                        return u32::MAX;
                    }
                    px((sy as u32 * img.width + sx as u32) as usize)
                })
                .collect()
        };
        let shot = cut(&|k| img.pixels[k]);
        let opacity: Vec<u8> =
            cut(&|k| u32::from(alpha.get(k).copied().unwrap_or(255))).iter().map(|&v| v.min(255) as u8).collect();
        // the glass areas and the blur that spreads around them
        let glass: Vec<(i32, i32, i32, i32)> = under
            .glass
            .iter()
            .map(|g| {
                let m = g[4] * 1.5;
                ((g[0] - m - rx) as i32, (g[1] - m - ry) as i32, (g[2] + 2.0 * m) as i32, (g[3] + 2.0 * m) as i32)
            })
            .collect();
        let s = compare(&shot, &drawn, &opacity, w as usize, &glass);
        let pass = s.differ <= LIMIT;
        ok &= pass;
        println!(
            "{}  click {:3} (slide {:2})  {:5.2} % of {} pixels of the backdrop differ (mean {:.1})",
            if pass { "ok  " } else { "FAIL" },
            i + 1,
            c.slide,
            s.differ,
            s.compared,
            s.mean
        );
        if let Ok(dir) = std::env::var("GLITU_PLAY_CHECK_DUMP") {
            let png =
                |px: &[u32]| px.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect::<Vec<u8>>();
            let write = |name: String, px: &[u32]| {
                let mut out = Vec::new();
                let mut enc = png::Encoder::new(&mut out, w, h);
                enc.set_color(png::ColorType::Rgb);
                if enc.write_header().and_then(|mut wr| wr.write_image_data(&png(px))).is_ok() {
                    let _ = std::fs::write(Path::new(&dir).join(name), out);
                }
            };
            write(format!("{:03}-native.png", i + 1), &drawn);
            write(format!("{:03}-chromium.png", i + 1), &shot);
        }
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_the_visible_backdrop() {
        let shot = [0x000000, 0x101010, 0xffffff, 0x000000];
        let drawn = [0x000000, 0x000000, 0x000000, 0x000000];
        // the 3rd pixel is covered, the 4th under glass: two compared, none differs
        let s = compare(&shot, &drawn, &[0, 0, 255, 0], 2, &[(1, 1, 1, 1)]);
        assert_eq!((s.compared, s.differ, s.mean), (2, 0.0, 8.0));
        let s = compare(&shot, &drawn, &[0, 0, 0, 0], 2, &[]);
        assert_eq!((s.compared, s.differ), (4, 25.0));
    }
}
