//! Isométrique : un plateau de colonnes isométriques qui montent et descendent en vague, une
//! seule colonne corail au centre. Derrière le texte, le plateau rapetisse dans le coin droit.

use web_sys::CanvasRenderingContext2d;

use super::{Scene, clamp01, lerp};

pub struct Iso;

const N: usize = 9;

fn face(ctx: &CanvasRenderingContext2d, pts: [(f64, f64); 4], color: &str) {
    ctx.begin_path();
    ctx.move_to(pts[0].0, pts[0].1);
    for p in &pts[1..] {
        ctx.line_to(p.0, p.1);
    }
    ctx.close_path();
    ctx.set_fill_style_str(color);
    ctx.fill();
}

impl Scene for Iso {
    fn start(&self) -> f64 {
        5.3
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64) {
        let low = clamp01(dim / 0.6);
        ctx.set_fill_style_str("#e8ebf4");
        ctx.fill_rect(0.0, 0.0, w, h);
        let (a, cx, cy) = (w * lerp(0.034, 0.024, low), w * lerp(0.7, 0.82, low), h * lerp(0.46, 0.42, low));
        let mid = (N / 2) as f64;
        // Des colonnes du backdrop vers l'avant : chacune cache celles de derrière
        for s in 0..=2 * (N - 1) {
            for i in 0..N {
                let Some(j) = s.checked_sub(i).filter(|&j| j < N) else { continue };
                let (fi, fj) = (i as f64, j as f64);
                let dist = (fi - mid).hypot(fj - mid);
                let hgt = a * (0.6 + 1.5 * (0.5 + 0.5 * (dist * 0.9 - t * 1.6).sin()));
                let (x0, y0) = (cx + (fi - fj) * a, cy + (fi + fj) * a / 2.0 + a * 1.5);
                let accent = i == N / 2 && j == N / 2;
                let top = (x0, y0 - hgt);
                let right = (x0 + a, y0 + a / 2.0 - hgt);
                let bottom = (x0, y0 + a - hgt);
                let left = (x0 - a, y0 + a / 2.0 - hgt);
                face(ctx, [left, bottom, (x0, y0 + a), (x0 - a, y0 + a / 2.0)], if accent { "#e8664a" } else { "#b8c0ea" });
                face(ctx, [bottom, right, (x0 + a, y0 + a / 2.0), (x0, y0 + a)], if accent { "#c94e36" } else { "#8590d6" });
                face(ctx, [top, right, bottom, left], if accent { "#ff8a6b" } else { "#ffffff" });
            }
        }
    }
}
