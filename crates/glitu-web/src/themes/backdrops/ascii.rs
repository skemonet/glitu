//! ASCII ambre : un champ de caractères dont la densité suit un bruit qui coule, avec des
//! anneaux qui s'éloignent d'un point, en ambre de vieux terminal.

use web_sys::CanvasRenderingContext2d;

use super::{Noise, Scene, clamp01, lerp, veil};

const RAMP: [&str; 11] = [" ", ".", "·", ":", "-", "=", "+", "*", "#", "%", "@"];

pub struct Ascii { noise: Noise }

impl Default for Ascii {
    fn default() -> Self {
        Ascii { noise: Noise::new() }
    }
}

impl Scene for Ascii {
    fn start(&self) -> f64 {
        14.0
    }

    fn fps(&self) -> f64 {
        20.0
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64) {
        let noise = &self.noise;
        let k = w / 480.0 * 0.75;
        ctx.set_fill_style_str("#110c06");
        ctx.fill_rect(0.0, 0.0, w, h);
        let (cw, ch) = (7.5 * k, 13.0 * k);
        ctx.set_font(&format!("{}px \"IBM Plex Mono\", monospace", 12.5 * k));
        ctx.set_text_baseline("top");
        let mut y = 0.0;
        while y < h {
            let mut x = 0.0;
            while x < w {
                let (dx, dy) = (x / w - 0.7, (y / h - 0.5) * 0.56);
                let ring = 0.5 + 0.5 * (dx.hypot(dy) * 26.0 - t * 2.2).sin();
                let mut v = clamp01(noise.fbm(x / w * 9.0, y / h * 7.0 - t * 0.25, t * 0.15) * 0.85 + ring * 0.3 - 0.12);
                // Derrière le texte : les caractères se raréfient vers la gauche
                v *= lerp(1.0, (0.25 + 0.75 * (x / w - 0.35) / 0.65).clamp(0.2, 1.0), dim);
                let c = RAMP[((v * RAMP.len() as f64) as usize).min(RAMP.len() - 1)];
                if c != " " {
                    ctx.set_fill_style_str(&format!("rgba(255,179,71,{:.2})", (0.2 + 0.8 * v) * lerp(1.0, 0.6, dim)));
                    let _ = ctx.fill_text(c, x, y);
                }
                x += cw;
            }
            y += ch;
        }
        if let Ok(g) = ctx.create_radial_gradient(w / 2.0, h / 2.0, h * 0.3, w / 2.0, h / 2.0, w * 0.75) {
            let _ = g.add_color_stop(0.0, "rgba(0,0,0,0)");
            let _ = g.add_color_stop(1.0, "rgba(0,0,0,.55)");
            ctx.set_fill_style_canvas_gradient(&g);
            ctx.fill_rect(0.0, 0.0, w, h);
        }
        veil(ctx, w, h, "#110c06", lerp(0.85, 0.75, dim), lerp(0.0, 0.1, dim));
    }
}
