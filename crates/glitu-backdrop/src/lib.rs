//! The animated backdrops of the template themes: crystal (`crystal`) and the scenes of
//! `scenes` (constellation, cut paper, aurora, amber ASCII, blueprint, isometric). They draw on a
//! `Canvas`, the subset of the HTML canvas 2D context they need: the browser's (glitu-web) or a
//! native one (glitu-play). The driver around them (size, cadence, slowing down, dimming) is
//! the host's.

pub mod crystal;
pub mod math;
pub mod scenes;

use std::any::Any;
use std::cell::RefCell;
use std::f64::consts::PI;

pub use math::rng;

pub const TAU: f64 = PI * 2.0;

/// A gradient, as the canvas makes them: its stops are added after
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub kind: GradientKind,
    pub stops: RefCell<Vec<(f64, String)>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientKind {
    Linear { x0: f64, y0: f64, x1: f64, y1: f64 },
    Radial { x0: f64, y0: f64, r0: f64, x1: f64, y1: f64, r1: f64 },
}

impl Gradient {
    /// `color`: a CSS colour
    #[allow(clippy::result_unit_err)] // as the canvas of the browser, whose errors carry nothing
    pub fn add_color_stop(&self, offset: f64, color: &str) -> Result<(), ()> {
        self.stops.borrow_mut().push((offset, color.to_string()));
        Ok(())
    }
}

/// The 2D context of a canvas, as the backdrops use it (the names and the arguments of the
/// browser's: colours, fonts, modes are CSS text)
pub trait Canvas {
    fn set_transform(&self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64);
    /// "source-over", "lighter", "overlay"
    fn set_global_composite_operation(&self, op: &str);
    fn set_global_alpha(&self, alpha: f64);
    fn set_fill_style_str(&self, color: &str);
    fn set_stroke_style_str(&self, color: &str);
    fn set_fill_gradient(&self, g: &Gradient);
    /// The image of `layer`, repeated
    fn set_fill_pattern(&self, layer: &dyn Layer);
    fn fill_rect(&self, x: f64, y: f64, w: f64, h: f64);
    fn stroke_rect(&self, x: f64, y: f64, w: f64, h: f64);
    fn clear_rect(&self, x: f64, y: f64, w: f64, h: f64);
    fn begin_path(&self);
    fn move_to(&self, x: f64, y: f64);
    fn line_to(&self, x: f64, y: f64);
    fn close_path(&self);
    fn arc(&self, x: f64, y: f64, r: f64, start: f64, end: f64);
    fn fill(&self);
    fn stroke(&self);
    fn save(&self);
    fn restore(&self);
    fn set_line_width(&self, w: f64);
    fn set_line_join(&self, join: &str);
    fn set_line_cap(&self, cap: &str);
    fn set_line_dash(&self, dash: &[f64]);
    fn set_shadow_blur(&self, blur: f64);
    fn set_shadow_color(&self, color: &str);
    fn set_shadow_offset_y(&self, dy: f64);
    /// "12px \"IBM Plex Mono\", monospace"
    fn set_font(&self, font: &str);
    fn set_text_align(&self, align: &str);
    fn set_text_baseline(&self, baseline: &str);
    fn fill_text(&self, text: &str, x: f64, y: f64);
    fn set_image_smoothing_enabled(&self, on: bool);
    /// The part (sx, sy, sw, sh) of `layer`, drawn into (dx, dy, dw, dh)
    #[allow(clippy::too_many_arguments)]
    fn draw_layer_part(
        &self,
        layer: &dyn Layer,
        sx: f64,
        sy: f64,
        sw: f64,
        sh: f64,
        dx: f64,
        dy: f64,
        dw: f64,
        dh: f64,
    );
    fn draw_layer(&self, layer: &dyn Layer, dx: f64, dy: f64, dw: f64, dh: f64);
    /// A canvas outside the page, of `w` × `h` pixels
    fn layer(&self, w: u32, h: u32) -> Box<dyn Layer>;

    fn create_linear_gradient(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Gradient {
        Gradient { kind: GradientKind::Linear { x0, y0, x1, y1 }, stops: RefCell::default() }
    }

    #[allow(clippy::result_unit_err)] // as the canvas of the browser
    fn create_radial_gradient(&self, x0: f64, y0: f64, r0: f64, x1: f64, y1: f64, r1: f64) -> Result<Gradient, ()> {
        if r0 < 0.0 || r1 < 0.0 {
            return Err(());
        }
        Ok(Gradient { kind: GradientKind::Radial { x0, y0, r0, x1, y1, r1 }, stops: RefCell::default() })
    }
}

/// A canvas outside the page: its context, its size
pub trait Layer {
    fn canvas(&self) -> &dyn Canvas;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    /// The host's own type (to draw it into another of its canvases)
    fn as_any(&self) -> &dyn Any;
}

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
    /// Prepares what depends only on the size (sprites, random positions); `ctx` makes the
    /// canvases outside the page
    fn build(&mut self, _ctx: &dyn Canvas, _w: f64, _h: f64) {}
    /// `dim`: from 0, the whole scene (cover, chapters), to 1, the scene erased on the text side
    fn draw(&self, ctx: &dyn Canvas, w: f64, h: f64, t: f64, dim: f64);
}

/// The backdrop named in glitu.yaml (`backdrop`), if it is one of the scenes (crystal is not one)
pub fn scene(name: &str) -> Option<Box<dyn Scene>> {
    Some(match name {
        "constellation" => Box::new(scenes::constellation::Constellation::default()),
        "paper" => Box::new(scenes::paper::Paper::default()),
        "aurora" => Box::new(scenes::aurora::Aurora::default()),
        "ascii" => Box::new(scenes::ascii::Ascii::default()),
        "blueprint" => Box::new(scenes::blueprint::Blueprint),
        "iso" => Box::new(scenes::iso::Iso),
        _ => return None,
    })
}

/// Pace of the backdrop on a slide
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Motion {
    Still,
    Animated { chase: bool },
}

impl Motion {
    /// As the page writes it for the export (`data-motion`)
    pub fn name(self) -> &'static str {
        match self {
            Motion::Still => "still",
            Motion::Animated { chase: false } => "animated",
            Motion::Animated { chase: true } => "chase",
        }
    }

    pub fn parse(s: &str) -> Motion {
        match s {
            "animated" => Motion::Animated { chase: false },
            "chase" => Motion::Animated { chase: true },
            _ => Motion::Still,
        }
    }
}

/// Speed of slowing down and resuming (per second): about one second to stand still
pub const EASING: f64 = 3.0;
/// Speed of the dimming: about half a second
pub const DIMMING: f64 = 6.0;

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

impl Default for Noise {
    fn default() -> Self {
        Noise::new()
    }
}

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
pub fn veil(ctx: &dyn Canvas, w: f64, h: f64, hex: &str, a0: f64, a1: f64) {
    if a0 <= 0.0 && a1 <= 0.0 {
        return;
    }
    let g = ctx.create_linear_gradient(0.0, 0.0, w, 0.0);
    let _ = g.add_color_stop(0.0, &rgba(hex, a0));
    let _ = g.add_color_stop(0.62, &rgba(hex, a1 + (a0 - a1) * 0.4));
    let _ = g.add_color_stop(1.0, &rgba(hex, a1));
    ctx.set_fill_gradient(&g);
    ctx.fill_rect(0.0, 0.0, w, h);
}

/// Paper grain: a tile of noise repeated, blended as an overlay
pub struct Grain(Box<dyn Layer>);

impl Grain {
    pub fn new(ctx: &dyn Canvas) -> Grain {
        const N: u32 = 160;
        let layer = ctx.layer(N, N);
        let g = layer.canvas();
        let mut r = rng(4);
        for y in 0..N {
            for x in 0..N {
                let v = (r() * 255.0) as u8;
                g.set_fill_style_str(&format!("rgb({v},{v},{v})"));
                g.fill_rect(x as f64, y as f64, 1.0, 1.0);
            }
        }
        Grain(layer)
    }

    pub fn draw(&self, ctx: &dyn Canvas, w: f64, h: f64, alpha: f64) {
        ctx.save();
        ctx.set_global_alpha(alpha);
        ctx.set_global_composite_operation("overlay");
        ctx.set_fill_pattern(self.0.as_ref());
        ctx.fill_rect(0.0, 0.0, w, h);
        ctx.restore();
    }
}

pub fn circle(ctx: &dyn Canvas, x: f64, y: f64, r: f64) {
    ctx.begin_path();
    ctx.arc(x, y, r.max(0.0), 0.0, TAU);
}
