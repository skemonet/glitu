//! Aurora : rubans de lumière vert d'eau, cyan et magenta qui ondulent sur un ciel bleu nuit
//! étoilé. Derrière le texte, les rubans restent en place et pâlissent sous un voile.

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use super::{Noise, Scene, lerp, offscreen, rgba, seeded, veil};

const RIBBONS: [&str; 3] = ["#2de2c0", "#3aa8ff", "#c45cff"];

pub struct Aurora {
    noise: Noise,
    /// Un dégradé vertical par ruban, étiré colonne par colonne
    sprites: Vec<HtmlCanvasElement>,
    stars: Vec<(f64, f64, f64, f64)>,
    /// Calque des rubans, au tiers de la résolution : agrandi avec lissage, il ne montre pas les
    /// colonnes qui le composent
    layer: Option<(HtmlCanvasElement, CanvasRenderingContext2d)>,
}

impl Default for Aurora {
    fn default() -> Self {
        Aurora { noise: Noise::new(), sprites: Vec::new(), stars: Vec::new(), layer: None }
    }
}

impl Scene for Aurora {
    fn start(&self) -> f64 {
        31.0
    }

    fn build(&mut self, w: f64, h: f64) {
        self.layer = Some(offscreen((w / 3.0).ceil() as u32, (h / 3.0).ceil() as u32));
        if self.sprites.is_empty() {
            self.sprites = RIBBONS.iter().map(|c| {
                let (s, g) = offscreen(1, 128);
                let gr = g.create_linear_gradient(0.0, 0.0, 0.0, 128.0);
                let _ = gr.add_color_stop(0.0, &rgba(c, 0.0));
                let _ = gr.add_color_stop(0.78, &rgba(c, 0.9));
                let _ = gr.add_color_stop(0.86, "rgba(255,255,255,.8)");
                let _ = gr.add_color_stop(1.0, &rgba(c, 0.0));
                g.set_fill_style_canvas_gradient(&gr);
                g.fill_rect(0.0, 0.0, 1.0, 128.0);
                s
            }).collect();
        }
        let mut r = seeded(8);
        self.stars = (0..160).map(|_| (r(), r() * 0.8, 0.2 + 0.45 * r(), r() * 20.0)).collect();
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64) {
        let k = w / 480.0;
        let g = ctx.create_linear_gradient(0.0, 0.0, 0.0, h);
        let _ = g.add_color_stop(0.0, "#02040c");
        let _ = g.add_color_stop(1.0, "#0b1834");
        ctx.set_fill_style_canvas_gradient(&g);
        ctx.fill_rect(0.0, 0.0, w, h);
        for &(x, y, a, ph) in &self.stars {
            let tw = 0.75 + 0.25 * (t * 2.0 + ph).sin();
            ctx.set_fill_style_str(&format!("rgba(255,255,255,{:.3})", a * tw));
            ctx.fill_rect(x * w, y * h, k * 0.8, k * 0.8);
        }
        let Some((layer, lc)) = &self.layer else { return };
        let (lw, lh) = (layer.width() as f64, layer.height() as f64);
        let _ = lc.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let _ = lc.set_global_composite_operation("source-over");
        lc.clear_rect(0.0, 0.0, lw, lh);
        let _ = lc.set_global_composite_operation("lighter");
        let _ = lc.set_transform(lw / w, 0.0, 0.0, lh / h, 0.0, 0.0);
        // Colonnes jointives d'un pixel du calque : aucune ne double la lumière de sa voisine
        let step = w / lw;
        for (i, sprite) in self.sprites.iter().enumerate() {
            let fi = i as f64;
            let mut x = 0.0;
            while x < w {
                let u = x / w;
                let n = self.noise.n3(u * 2.2 + fi * 10.0, t * 0.07, fi);
                let y = h * (0.32 + 0.1 * fi) + (n - 0.5) * h * 0.45 + (u * 3.0 + t * 0.25 + fi).sin() * h * 0.06;
                let hh = h * (0.22 + 0.4 * self.noise.n3(u * 5.0, t * 0.15, fi + 5.0));
                // Opacité lentement variable le long du ruban : pas de stries verticales
                let a = 0.2 + 0.5 * self.noise.n3(u * 1.6 + fi, t * 0.2, 2.0).powf(1.5);
                lc.set_global_alpha(a * lerp(1.0, 0.6, dim));
                let _ = lc.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    sprite, 0.0, 0.0, 1.0, 128.0, x, y - hh, step, hh,
                );
                x += step;
            }
        }
        lc.set_global_alpha(1.0);
        let _ = ctx.set_global_composite_operation("lighter");
        ctx.set_image_smoothing_enabled(true);
        let _ = ctx.draw_image_with_html_canvas_element_and_dw_and_dh(layer, 0.0, 0.0, w, h);
        let _ = ctx.set_global_composite_operation("source-over");
        veil(ctx, w, h, "#03060f", 0.7 * dim, 0.2 * dim);
    }
}
