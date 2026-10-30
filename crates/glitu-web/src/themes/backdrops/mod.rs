//! Animated backdrops of the template themes, other than crystal: constellation, cut paper,
//! aurora, amber ASCII, blueprint, isometric. A shared driver (canvas the size of the slide,
//! slowing down, dimming) and one scene per backdrop, which only draws an image at a given
//! instant.
//!
//! The dimming (`dim`) goes from 0, the whole scene (cover, chapters), to 1, the scene erased
//! on the text side (full-width text slides); panel slides are in between. It glides from one
//! value to the other when the slide changes, even when the backdrop is still.

mod ascii;
mod aurora;
mod constellation;
mod iso;
mod paper;
mod blueprint;

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};

use super::crystal::{Motion, rng};
use crate::dom;

pub const TAU: f64 = PI * 2.0;

/// A scene: its drawing at instant `t`, in slide px
pub trait Scene {
    /// Instant of the first image (chosen so that it looks good)
    fn start(&self) -> f64;
    /// Frames per second when it runs
    fn fps(&self) -> f64 {
        30.0
    }
    /// Resolution of the image, relative to the screen's: a soft scene can draw fewer pixels
    fn resolution(&self) -> f64 {
        1.0
    }
    /// Prepares what depends only on the size (sprites, random positions)
    fn build(&mut self, _w: f64, _h: f64) {}
    fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, t: f64, dim: f64);
}

/// The backdrop named in glitu.json (`"backdrop"`), if it is one of these
pub fn scene(name: &str) -> Option<Box<dyn Scene>> {
    Some(match name {
        "constellation" => Box::new(constellation::Constellation::default()),
        "paper" => Box::new(paper::Paper::default()),
        "aurora" => Box::new(aurora::Aurora::default()),
        "ascii" => Box::new(ascii::Ascii::default()),
        "blueprint" => Box::new(blueprint::Blueprint),
        "iso" => Box::new(iso::Iso),
        _ => return None,
    })
}

struct Backdrop {
    scene: Box<dyn Scene>,
    host: Element,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    reduce: bool,
    w: f64, h: f64, scale: f64,
    t: f64, last: f64, raf: i32,
    /// Speed of the animation (0: still, 1: normal), which tends towards `target`
    rate: f64, target: f64,
    /// Dimming, which tends towards `dim_to`
    dim: f64, dim_to: f64,
}

impl Backdrop {
    fn draw(&self) {
        let _ = self.ctx.set_transform(self.scale, 0.0, 0.0, self.scale, 0.0, 0.0);
        let _ = self.ctx.set_global_composite_operation("source-over");
        self.ctx.set_global_alpha(1.0);
        self.scene.draw(&self.ctx, self.w, self.h, self.t, self.dim);
    }

    // As crystal: the scene in slide px, the image at the real resolution of the screen
    fn resize(&mut self) -> bool {
        let host = dom::html(&self.host);
        let (w, h) = (host.offset_width() as f64, host.offset_height() as f64);
        let rect = self.host.get_bounding_client_rect();
        if w == 0.0 || h == 0.0 { return false }
        let dpr = dom::window().device_pixel_ratio();
        let dpr = if dpr == 0.0 { 1.0 } else { dpr };
        self.scale = dpr.min(2.0) * rect.width() / w * self.scene.resolution();
        self.canvas.set_width((w * self.scale).round() as u32);
        self.canvas.set_height((h * self.scale).round() as u32);
        if w != self.w || h != self.h {
            self.w = w;
            self.h = h;
            self.scene.build(w, h);
        }
        true
    }

    fn settled(&self) -> bool {
        self.target == 0.0 && self.rate < 0.01 && (self.dim - self.dim_to).abs() < 0.005
    }
}

#[derive(Clone)]
pub struct Animated(Rc<RefCell<Backdrop>>);

impl Animated {
    pub fn new(host: &Element, scene: Box<dyn Scene>) -> Animated {
        let canvas: HtmlCanvasElement = dom::create("canvas").unchecked_into();
        canvas.set_class_name("backdrop-canvas");
        let _ = canvas.set_attribute("aria-hidden", "true");
        let _ = host.prepend_with_node_1(&canvas);
        let ctx: CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten().expect("canvas 2d").unchecked_into();
        let reduce = dom::window().match_media("(prefers-reduced-motion: reduce)").ok().flatten().is_some_and(|m| m.matches());
        let t = scene.start();
        Animated(Rc::new(RefCell::new(Backdrop {
            scene, host: host.clone(), canvas, ctx, reduce,
            w: 0.0, h: 0.0, scale: 1.0, t, last: 0.0, raf: 0,
            rate: 0.0, target: 0.0, dim: 0.0, dim_to: 0.0,
        })))
    }

    pub fn redraw(&self) {
        let mut f = self.0.borrow_mut();
        if f.resize() {
            f.draw();
        }
    }

    pub fn snapshot(&self) -> Option<String> {
        self.0.borrow().canvas.to_data_url().ok()
    }

    /// Pace (runs or stands still, without changing image) and dimming wanted for the slide
    pub fn set_motion(&self, motion: Motion, dim: f64) {
        let mut f = self.0.borrow_mut();
        let animated = matches!(motion, Motion::Animated { .. }) && !f.reduce;
        f.target = if animated { 1.0 } else { 0.0 };
        f.dim_to = dim;
        if f.reduce {
            // Without animation: the new dimming at once
            f.dim = dim;
            f.draw();
            return;
        }
        if !f.settled() && f.raf == 0 {
            f.last = dom::now();
            let me = self.clone();
            f.raf = dom::raf(move |t| me.frame(t));
        }
    }

    fn frame(&self, now: f64) {
        let me = self.clone();
        let mut f = self.0.borrow_mut();
        // a few ms of slack: a screen frame that comes a little early is not skipped (an uneven
        // cadence shows more than a slower one)
        if now - f.last < 1000.0 / f.scene.fps() - 4.0 {
            f.raf = dom::raf(move |t| me.frame(t));
            return;
        }
        let dt = ((now - f.last) / 1000.0).min(0.1);
        f.last = now;
        f.rate += (f.target - f.rate) * (dt * EASING).min(1.0);
        f.dim += (f.dim_to - f.dim) * (dt * DIMMING).min(1.0);
        if f.settled() {
            f.rate = 0.0;
            f.dim = f.dim_to;
            f.raf = 0;
            f.draw();
            return;
        }
        f.t += dt * f.rate;
        f.draw();
        f.raf = dom::raf(move |t| me.frame(t));
    }
}

/// Speed of slowing down and resuming (per second), as crystal
const EASING: f64 = 3.0;
/// Speed of the dimming: about half a second
const DIMMING: f64 = 6.0;

// ----- Scene tools ------------------------------------------------------------------------

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

/// `#rrggbb` → `rgba(r,g,b,a)`
pub fn rgba(hex: &str, a: f64) -> String {
    let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    format!("rgba({},{},{},{:.3})", n >> 16, (n >> 8) & 255, n & 255, a)
}

/// Random with a fixed seed: each backdrop keeps the same drawing from one showing to the next
pub fn seeded(seed: i32) -> impl FnMut() -> f64 {
    rng(seed)
}

/// 3D value noise, smoothed (permutation table drawn once and for all)
pub struct Noise([u8; 512]);

impl Noise {
    pub fn new() -> Noise {
        let mut p: Vec<u8> = (0..=255).collect();
        let mut r = rng(11);
        for i in (1..256).rev() {
            let j = (r() * (i + 1) as f64) as usize;
            p.swap(i, j);
        }
        let mut t = [0u8; 512];
        for i in 0..512 {
            t[i] = p[i & 255];
        }
        Noise(t)
    }

    pub fn n3(&self, x: f64, y: f64, z: f64) -> f64 {
        let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
        let (x, y, z) = (x - fx, y - fy, z - fz);
        let (xi, yi, zi) = ((fx as i64 & 255) as usize, (fy as i64 & 255) as usize, (fz as i64 & 255) as usize);
        let (u, v, w) = (x * x * (3.0 - 2.0 * x), y * y * (3.0 - 2.0 * y), z * z * (3.0 - 2.0 * z));
        let p = &self.0;
        let h = |i: usize, j: usize, k: usize| {
            p[p[p[(xi + i) & 255] as usize + ((yi + j) & 255)] as usize + ((zi + k) & 255)] as f64 / 255.0
        };
        lerp(
            lerp(lerp(h(0, 0, 0), h(1, 0, 0), u), lerp(h(0, 1, 0), h(1, 1, 0), u), v),
            lerp(lerp(h(0, 0, 1), h(1, 0, 1), u), lerp(h(0, 1, 1), h(1, 1, 1), u), v),
            w,
        )
    }

    /// Two octaves
    pub fn fbm(&self, x: f64, y: f64, z: f64) -> f64 {
        (self.n3(x, y, z) + 0.5 * self.n3(x * 2.03, y * 2.03, z + 3.1)) / 1.5
    }
}

/// Horizontal veil of a colour, of opacity `a0` on the left (text side) to `a1` on the right
pub fn veil(ctx: &CanvasRenderingContext2d, w: f64, h: f64, hex: &str, a0: f64, a1: f64) {
    if a0 <= 0.0 && a1 <= 0.0 { return }
    let g = ctx.create_linear_gradient(0.0, 0.0, w, 0.0);
    let _ = g.add_color_stop(0.0, &rgba(hex, a0));
    let _ = g.add_color_stop(0.62, &rgba(hex, a1 + (a0 - a1) * 0.4));
    let _ = g.add_color_stop(1.0, &rgba(hex, a1));
    ctx.set_fill_style_canvas_gradient(&g);
    ctx.fill_rect(0.0, 0.0, w, h);
}

/// A canvas outside the page
pub fn offscreen(w: u32, h: u32) -> (HtmlCanvasElement, CanvasRenderingContext2d) {
    let c: HtmlCanvasElement = dom::create("canvas").unchecked_into();
    c.set_width(w.max(1));
    c.set_height(h.max(1));
    let ctx = c.get_context("2d").ok().flatten().expect("canvas 2d").unchecked_into();
    (c, ctx)
}

/// Paper grain: a tile of noise repeated, blended as an overlay
pub struct Grain(HtmlCanvasElement);

impl Grain {
    pub fn new() -> Grain {
        const N: u32 = 160;
        let (c, g) = offscreen(N, N);
        let mut r = rng(4);
        for y in 0..N {
            for x in 0..N {
                let v = (r() * 255.0) as u8;
                g.set_fill_style_str(&format!("rgb({v},{v},{v})"));
                g.fill_rect(x as f64, y as f64, 1.0, 1.0);
            }
        }
        Grain(c)
    }

    pub fn draw(&self, ctx: &CanvasRenderingContext2d, w: f64, h: f64, alpha: f64) {
        let Ok(Some(pattern)) = ctx.create_pattern_with_html_canvas_element(&self.0, "repeat") else { return };
        ctx.save();
        ctx.set_global_alpha(alpha);
        let _ = ctx.set_global_composite_operation("overlay");
        ctx.set_fill_style_canvas_pattern(&pattern);
        ctx.fill_rect(0.0, 0.0, w, h);
        ctx.restore();
    }
}

pub fn circle(ctx: &CanvasRenderingContext2d, x: f64, y: f64, r: f64) {
    ctx.begin_path();
    let _ = ctx.arc(x, y, r.max(0.0), 0.0, TAU);
}
