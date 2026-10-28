//! Constellation : peu de points qui dérivent, des liens très fins entre voisins proches,
//! quelques étoiles qui scintillent, sur un bleu nuit.

use web_sys::CanvasRenderingContext2d;

use super::{Scene, TAU, circle, lerp, seeded, veil};

struct Star { x: f64, y: f64, vx: f64, vy: f64, ph: f64, big: bool }

#[derive(Default)]
pub struct Constellation { stars: Vec<Star> }

impl Scene for Constellation {
    fn start(&self) -> f64 {
        12.0
    }

    fn build(&mut self, w: f64, _h: f64) {
        let mut r = seeded(17);
        let k = w / 480.0;
        self.stars = (0..90).map(|_| Star {
            x: r(), y: r(), vx: (r() - 0.5) * 9.0 * k, vy: (r() - 0.5) * 9.0 * k, ph: r() * TAU, big: r() < 0.12,
        }).collect();
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64) {
        let k = w / 480.0;
        let g = ctx.create_linear_gradient(0.0, 0.0, w, h);
        let _ = g.add_color_stop(0.0, "#060812");
        let _ = g.add_color_stop(1.0, "#101a38");
        ctx.set_fill_style_canvas_gradient(&g);
        ctx.fill_rect(0.0, 0.0, w, h);
        // Les points dérivent et reviennent par le bord opposé
        let (mw, mh) = (w + 20.0, h + 20.0);
        let pts: Vec<(f64, f64)> = self.stars.iter().map(|s| {
            ((s.x * w + s.vx * t).rem_euclid(mw) - 10.0, (s.y * h + s.vy * t).rem_euclid(mh) - 10.0)
        }).collect();
        let d0 = 80.0 * k;
        ctx.set_line_width(0.7 * k);
        for i in 0..pts.len() {
            for j in i + 1..pts.len() {
                let d = (pts[i].0 - pts[j].0).hypot(pts[i].1 - pts[j].1);
                if d < d0 {
                    ctx.set_stroke_style_str(&format!("rgba(159,180,255,{:.3})", 0.45 * (1.0 - d / d0)));
                    ctx.begin_path();
                    ctx.move_to(pts[i].0, pts[i].1);
                    ctx.line_to(pts[j].0, pts[j].1);
                    ctx.stroke();
                }
            }
        }
        for (s, &(x, y)) in self.stars.iter().zip(&pts) {
            let tw = 0.5 + 0.5 * (t * 2.2 + s.ph).sin();
            ctx.set_fill_style_str(&format!("rgba(235,240,255,{:.3})", if s.big { 0.6 + 0.4 * tw } else { 0.7 }));
            circle(ctx, x, y, k * if s.big { 1.4 + 1.2 * tw } else { 1.1 });
            ctx.fill();
        }
        veil(ctx, w, h, "#070a16", lerp(0.6, 0.82, dim), lerp(0.0, 0.35, dim));
    }
}
