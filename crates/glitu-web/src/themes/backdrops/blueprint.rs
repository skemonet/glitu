//! Blueprint: architect's blue paper, a fine grid, a dimensioned mechanical part whose ring of
//! holes turns slowly, and a title block on the cover.

use js_sys::Array;
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

use super::{Scene, TAU, clamp01, lerp, veil};

pub struct Blueprint;

fn line(ctx: &CanvasRenderingContext2d, x1: f64, y1: f64, x2: f64, y2: f64) {
    ctx.begin_path();
    ctx.move_to(x1, y1);
    ctx.line_to(x2, y2);
    ctx.stroke();
}

fn ring(ctx: &CanvasRenderingContext2d, x: f64, y: f64, r: f64) {
    ctx.begin_path();
    let _ = ctx.arc(x, y, r, 0.0, TAU);
    ctx.stroke();
}

fn dash(ctx: &CanvasRenderingContext2d, d: &[f64]) {
    let a: Array = d.iter().map(|&v| JsValue::from_f64(v)).collect();
    let _ = ctx.set_line_dash(&a);
}

impl Scene for Blueprint {
    fn start(&self) -> f64 {
        9.0
    }

    fn fps(&self) -> f64 {
        24.0
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64) {
        let k = w / 480.0;
        let low = clamp01(dim / 0.6);
        ctx.set_fill_style_str("#0f3a73");
        ctx.fill_rect(0.0, 0.0, w, h);
        for (sp, a) in [(10.0 * k, 0.05), (50.0 * k, 0.13)] {
            ctx.set_stroke_style_str(&format!("rgba(255,255,255,{a})"));
            ctx.set_line_width((k * 0.3).max(1.0));
            ctx.begin_path();
            let mut x = 0.0;
            while x < w { ctx.move_to(x, 0.0); ctx.line_to(x, h); x += sp; }
            let mut y = 0.0;
            while y < h { ctx.move_to(0.0, y); ctx.line_to(w, y); y += sp; }
            ctx.stroke();
        }
        let (cx, cy, r) = (w * lerp(0.7, 0.8, low), h * lerp(0.48, 0.55, low), h * lerp(0.28, 0.24, low));
        let ink = format!("rgba(232,242,255,{:.3})", lerp(0.92, 0.55, low));
        ctx.set_stroke_style_str(&ink);
        ctx.set_fill_style_str(&ink);
        // Axes and circle of the holes in dash-dot line
        dash(ctx, &[8.0 * k, 3.0 * k, 1.5 * k, 3.0 * k]);
        ctx.set_line_width(0.7 * k);
        line(ctx, cx - r * 1.25, cy, cx + r * 1.25, cy);
        line(ctx, cx, cy - r * 1.25, cx, cy + r * 1.25);
        ring(ctx, cx, cy, r * 0.66);
        dash(ctx, &[]);
        ctx.set_line_width(1.3 * k);
        ring(ctx, cx, cy, r);
        ring(ctx, cx, cy, r * 0.3);
        let turn = t * 0.12;
        for i in 0..6 {
            let a = i as f64 * TAU / 6.0 - TAU / 4.0 + turn;
            ring(ctx, cx + a.cos() * r * 0.66, cy + a.sin() * r * 0.66, r * 0.09);
        }
        // Dimension of the diameter
        let dy = cy + r * 1.18;
        ctx.set_line_width(0.8 * k);
        line(ctx, cx - r, cy + r * 0.2, cx - r, dy + 6.0 * k);
        line(ctx, cx + r, cy + r * 0.2, cx + r, dy + 6.0 * k);
        line(ctx, cx - r, dy, cx + r, dy);
        for (x, s) in [(cx - r, 1.0), (cx + r, -1.0)] {
            ctx.begin_path();
            ctx.move_to(x, dy);
            ctx.line_to(x + s * 7.0 * k, dy - 2.5 * k);
            ctx.line_to(x + s * 7.0 * k, dy + 2.5 * k);
            ctx.close_path();
            ctx.fill();
        }
        ctx.set_font(&format!("{}px \"IBM Plex Mono\", monospace", 10.0 * k));
        ctx.set_text_align("center");
        ctx.set_text_baseline("bottom");
        let _ = ctx.fill_text("Ø 140", cx, dy - 3.0 * k);
        // Title block, which fades behind the text
        if low < 1.0 {
            ctx.set_global_alpha(1.0 - low);
            let (bw, bh) = (150.0 * k, 34.0 * k);
            let (bx, by) = (w - bw - 12.0 * k, h - bh - 12.0 * k);
            ctx.set_line_width(0.8 * k);
            ctx.stroke_rect(bx, by, bw, bh);
            line(ctx, bx, by + bh / 2.0, bx + bw, by + bh / 2.0);
            line(ctx, bx + bw * 0.6, by, bx + bw * 0.6, by + bh);
            ctx.set_text_align("left");
            ctx.set_text_baseline("middle");
            ctx.set_font(&format!("{}px \"IBM Plex Mono\", monospace", 7.0 * k));
            for (txt, x, y) in [
                ("GLITU", bx + 4.0 * k, by + bh / 4.0),
                ("SCALE 1:2", bx + bw * 0.6 + 4.0 * k, by + bh / 4.0),
                ("SHEET 01", bx + 4.0 * k, by + bh * 0.75),
                ("A3", bx + bw * 0.6 + 4.0 * k, by + bh * 0.75),
            ] {
                let _ = ctx.fill_text(txt, x, y);
            }
            ctx.set_global_alpha(1.0);
        }
        ctx.set_text_align("start");
        ctx.set_text_baseline("alphabetic");
        veil(ctx, w, h, "#0f3a73", 0.6 * low, 0.0);
    }
}
