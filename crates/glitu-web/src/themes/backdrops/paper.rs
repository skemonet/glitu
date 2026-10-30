//! Cut paper: wavy layers of paper, from aqua to petrol blue, with real drop shadows that
//! breathe. Behind the text, the layers go down and flatten.

use web_sys::CanvasRenderingContext2d;

use super::{Grain, Noise, Scene, TAU, clamp01, lerp};

const LAYERS: [&str; 6] = ["#dcebe8", "#b7dbd4", "#86c2b8", "#4fa39a", "#2c7a75", "#1b4e52"];

pub struct Paper { noise: Noise, grain: Option<Grain> }

impl Default for Paper {
    fn default() -> Self {
        Paper { noise: Noise::new(), grain: None }
    }
}

impl Scene for Paper {
    fn start(&self) -> f64 {
        8.0
    }

    fn fps(&self) -> f64 {
        30.0
    }

    // soft shapes and blurred shadows: half the pixels, which the blur makes cheap enough to keep
    // a steady cadence
    fn resolution(&self) -> f64 {
        0.5
    }

    fn build(&mut self, _w: f64, _h: f64) {
        self.grain.get_or_insert_with(Grain::new);
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64) {
        let k = w / 480.0;
        let low = clamp01(dim / 0.6); // already low on the panel slides
        ctx.set_fill_style_str("#f2f6f5");
        ctx.fill_rect(0.0, 0.0, w, h);
        let (b0, step, amp) = (lerp(0.4, 0.74, low), lerp(0.1, 0.045, low), lerp(1.0, 0.6, low));
        ctx.save();
        ctx.set_shadow_color("rgba(15,40,40,.32)");
        ctx.set_shadow_blur(14.0 * k);
        ctx.set_shadow_offset_y(-3.0 * k);
        for (i, c) in LAYERS.iter().enumerate() {
            let fi = i as f64;
            let base = h * (b0 + fi * step);
            let dir = if i % 2 == 1 { 1.0 } else { -1.0 };
            ctx.begin_path();
            ctx.move_to(-10.0, h + 10.0);
            let mut x = -10.0;
            while x <= w + 10.0 {
                let u = x / w;
                let wave = (u * TAU * (0.8 + fi * 0.25) + t * 0.3 * dir + fi).sin() * h * 0.03
                    + (self.noise.n3(u * 2.0 + fi, t * 0.1, fi) - 0.5) * h * 0.1;
                ctx.line_to(x, base + amp * wave);
                x += w / 90.0;
            }
            ctx.line_to(w + 10.0, h + 10.0);
            ctx.close_path();
            ctx.set_fill_style_str(c);
            ctx.fill();
        }
        ctx.restore();
        if let Some(g) = &self.grain {
            g.draw(ctx, w, h, 0.12);
        }
    }
}
