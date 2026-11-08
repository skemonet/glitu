//! Text drawn by software for the speaker view: one embedded font (Lato, the font of the crystal
//! theme), lines wrapped at a width, glyphs blended into the window's pixels.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};

pub static LATO: &[u8] = include_bytes!("../fonts/Lato-Regular.ttf");

/// Pixels to draw into: 0x00RRGGBB, `width` per row, drawing cut at `clip` (x0, y0, x1, y1)
pub struct Canvas<'a> {
    pub pixels: &'a mut [u32],
    pub width: usize,
    pub clip: (i32, i32, i32, i32),
}

impl Canvas<'_> {
    pub fn fill(&mut self, (x, y, w, h): (i32, i32, i32, i32), color: u32) {
        let (x0, y0) = (x.max(self.clip.0), y.max(self.clip.1));
        let (x1, y1) = ((x + w).min(self.clip.2), (y + h).min(self.clip.3));
        for row in y0..y1 {
            for px in x0..x1 {
                self.pixels[row as usize * self.width + px as usize] = color;
            }
        }
    }
}

pub struct Text {
    font: FontRef<'static>,
}

/// `a` over `b` with the coverage `c` (0 to 1)
fn blend(a: u32, b: u32, c: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xff) as f32, ((b >> s) & 0xff) as f32);
        ((x * c + y * (1.0 - c)).round() as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

impl Text {
    pub fn new() -> Text {
        Text { font: FontRef::try_from_slice(LATO).expect("the embedded font") }
    }

    /// The distance from one line to the next, at `size` pixels (at least 1.2 times the size,
    /// for reading)
    pub fn line(&self, size: f32) -> f32 {
        let f = self.font.as_scaled(PxScale::from(size));
        (f.height() + f.line_gap()).max(size * 1.2)
    }

    pub fn width(&self, s: &str, size: f32) -> f32 {
        let f = self.font.as_scaled(PxScale::from(size));
        let mut last = None;
        let mut w = 0.0;
        for c in s.chars() {
            let id = f.glyph_id(c);
            if let Some(prev) = last {
                w += f.kern(prev, id);
            }
            w += f.h_advance(id);
            last = Some(id);
        }
        w
    }

    /// `text` cut into lines of `width` pixels at most, at its spaces (a longer word stays whole);
    /// its line breaks are the source's, not the speaker's: they are spaces too
    pub fn wrap(&self, text: &str, size: f32, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let longer = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && self.width(&longer, size) > width {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            } else {
                line = longer;
            }
        }
        lines.push(line);
        lines
    }

    /// `s` on one line, its top at `y`; returns its width
    pub fn draw(&self, canvas: &mut Canvas, s: &str, size: f32, color: u32, (x, y): (f32, f32)) -> f32 {
        let scale = PxScale::from(size);
        let f = self.font.as_scaled(scale);
        let baseline = y + f.ascent();
        let (mut pen, mut last) = (x, None);
        for c in s.chars() {
            let id = f.glyph_id(c);
            if let Some(prev) = last {
                pen += f.kern(prev, id);
            }
            if let Some(g) = self.font.outline_glyph(id.with_scale_and_position(scale, point(pen, baseline))) {
                let b = g.px_bounds();
                let (gx, gy) = (b.min.x as i32, b.min.y as i32);
                let clip = canvas.clip;
                g.draw(|dx, dy, cov| {
                    let (px, py) = (gx + dx as i32, gy + dy as i32);
                    if cov > 0.0 && px >= clip.0 && px < clip.2 && py >= clip.1 && py < clip.3 {
                        let i = py as usize * canvas.width + px as usize;
                        canvas.pixels[i] = blend(color, canvas.pixels[i], cov.min(1.0));
                    }
                });
            }
            pen += f.h_advance(id);
            last = Some(id);
        }
        pen - x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_and_draws() {
        let t = Text::new();
        let size = 20.0;
        assert!(t.width("Hello", size) > 0.0 && t.width("Hello world", size) > t.width("Hello", size));
        let w = t.width("Hello world", size);
        assert_eq!(t.wrap("Hello world", size, w + 1.0), ["Hello world"]);
        assert_eq!(t.wrap("Hello world", size, w - 1.0), ["Hello", "world"]);
        assert_eq!(t.wrap("a\nb", size, 100.0), ["a b"]);
        // a word longer than the line stays whole
        assert_eq!(t.wrap("unbreakable", size, 5.0), ["unbreakable"]);
        assert!(t.line(size) > size);
        let mut pixels = vec![0; 100 * 30];
        let mut c = Canvas { pixels: &mut pixels, width: 100, clip: (0, 0, 100, 30) };
        t.draw(&mut c, "Hé", size, 0xffffff, (2.0, 2.0));
        assert!(pixels.contains(&0xffffff));
        // nothing outside the clip
        let mut pixels = vec![0; 100 * 30];
        let mut c = Canvas { pixels: &mut pixels, width: 100, clip: (0, 0, 100, 30) };
        t.draw(&mut c, "Hé", size, 0xffffff, (200.0, 2.0));
        c.fill((90, 25, 50, 50), 7);
        assert_eq!(pixels.iter().filter(|&&p| p == 7).count(), 10 * 5);
        assert!(!pixels.contains(&0xffffff));
        assert_eq!(blend(0xffffff, 0, 0.5), 0x808080);
    }
}
