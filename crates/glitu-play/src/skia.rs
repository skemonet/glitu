//! A native `Canvas` for the backdrops (`glitu_backdrop`), on tiny-skia: what the browser's 2D
//! context does with the calls the scenes make. The shadows (`shadowBlur`, in pixels of the
//! canvas, as in the browser) are a mask of the shape, blurred; the text is the outlines of the
//! embedded font.

use std::any::Any;
use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use ab_glyph::{Font, FontRef, OutlineCurve, PxScale, ScaleFont};
use glitu_backdrop::{Canvas, Gradient, GradientKind, Layer};
use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, GradientStop, IntSize, LineCap, LineJoin, LinearGradient, Mask, Paint,
    Path, PathBuilder, Pattern, Pixmap, Point, RadialGradient, Rect, Shader, SpreadMode, Stroke, StrokeDash, Transform,
};

use crate::text::LATO;

/// The monospaced font of the ascii and blueprint scenes
static PLEX_MONO: &[u8] = include_bytes!("../fonts/IBMPlexMono-Regular.ttf");

/// A CSS colour: `#rgb`, `#rrggbb`, `rgb()`, `rgba()`, `hsl()`, `hsla()`; transparent if unknown
pub fn css_color(s: &str) -> Color {
    let s = s.trim();
    let c = |r: f64, g: f64, b: f64, a: f64| {
        Color::from_rgba(
            (r / 255.0).clamp(0.0, 1.0) as f32,
            (g / 255.0).clamp(0.0, 1.0) as f32,
            (b / 255.0).clamp(0.0, 1.0) as f32,
            a.clamp(0.0, 1.0) as f32,
        )
        .unwrap_or(Color::TRANSPARENT)
    };
    if let Some(hex) = s.strip_prefix('#') {
        let n = u32::from_str_radix(hex, 16).unwrap_or(0);
        return match hex.len() {
            3 => {
                let d = |k: u32| f64::from(((n >> k) & 15) * 17);
                c(d(8), d(4), d(0), 1.0)
            }
            6 => c(f64::from(n >> 16), f64::from((n >> 8) & 255), f64::from(n & 255), 1.0),
            _ => Color::TRANSPARENT,
        };
    }
    let Some((name, args)) = s.strip_suffix(')').and_then(|s| s.split_once('(')) else { return Color::TRANSPARENT };
    let v: Vec<f64> = args.split(',').map(|a| a.trim().trim_end_matches('%').parse().unwrap_or(0.0)).collect();
    let alpha = v.get(3).copied().unwrap_or(1.0);
    match (name.trim(), v.len()) {
        ("rgb" | "rgba", 3 | 4) => c(v[0], v[1], v[2], alpha),
        ("hsl" | "hsla", 3 | 4) => {
            let (h, s, l) = (v[0].rem_euclid(360.0) / 360.0, v[1] / 100.0, v[2] / 100.0);
            let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
            let p = 2.0 * l - q;
            let hue = |t: f64| {
                let t = t.rem_euclid(1.0);
                255.0
                    * if t < 1.0 / 6.0 {
                        p + (q - p) * 6.0 * t
                    } else if t < 0.5 {
                        q
                    } else if t < 2.0 / 3.0 {
                        p + (q - p) * (2.0 / 3.0 - t) * 6.0
                    } else {
                        p
                    }
            };
            c(hue(h + 1.0 / 3.0), hue(h), hue(h - 1.0 / 3.0), alpha)
        }
        _ => Color::TRANSPARENT,
    }
}

#[derive(Clone)]
enum Style {
    Color(Color),
    Gradient(Gradient),
    Pattern(Rc<RefCell<Pixmap>>),
}

#[derive(Clone)]
struct State {
    transform: Transform,
    alpha: f32,
    blend: BlendMode,
    fill: Style,
    stroke: Style,
    line_width: f32,
    join: LineJoin,
    cap: LineCap,
    dash: Vec<f32>,
    shadow_blur: f64,
    shadow_color: Color,
    shadow_dy: f32,
    font_size: f32,
    /// The font asks for a monospaced one
    mono: bool,
    align: String,
    baseline: String,
    smooth: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            transform: Transform::identity(),
            alpha: 1.0,
            blend: BlendMode::SourceOver,
            fill: Style::Color(Color::BLACK),
            stroke: Style::Color(Color::BLACK),
            line_width: 1.0,
            join: LineJoin::Miter,
            cap: LineCap::Butt,
            dash: Vec::new(),
            shadow_blur: 0.0,
            shadow_color: Color::TRANSPARENT,
            shadow_dy: 0.0,
            font_size: 10.0,
            mono: false,
            align: "start".into(),
            baseline: "alphabetic".into(),
            smooth: true,
        }
    }
}

/// A canvas of tiny-skia: its pixels, its state, its path
pub struct Skia {
    pixmap: Rc<RefCell<Pixmap>>,
    state: RefCell<State>,
    stack: RefCell<Vec<State>>,
    path: RefCell<PathBuilder>,
    /// The current point of the path (for `arc`, which joins it)
    current: RefCell<Option<(f32, f32)>>,
}

impl Skia {
    pub fn new(w: u32, h: u32) -> Skia {
        Skia {
            pixmap: Rc::new(RefCell::new(Pixmap::new(w.max(1), h.max(1)).expect("canvas size"))),
            state: RefCell::default(),
            stack: RefCell::default(),
            path: RefCell::new(PathBuilder::new()),
            current: RefCell::new(None),
        }
    }

    pub fn width(&self) -> u32 {
        self.pixmap.borrow().width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.borrow().height()
    }

    /// The pixels as softbuffer takes them, 0x00RRGGBB (the backdrop is opaque)
    pub fn pixels(&self) -> Vec<u32> {
        self.pixmap
            .borrow()
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
            .collect()
    }

    fn built_path(&self) -> Option<Path> {
        self.path.borrow().clone().finish()
    }

    /// The paint of `style`, at the global alpha
    fn paint(&self, style: &Style) -> Option<Paint<'static>> {
        let st = self.state.borrow();
        let shader = match style {
            Style::Color(c) => {
                let mut c = *c;
                c.apply_opacity(st.alpha);
                Shader::SolidColor(c)
            }
            Style::Gradient(g) => {
                let stops: Vec<GradientStop> = g
                    .stops
                    .borrow()
                    .iter()
                    .map(|(o, s)| {
                        let mut c = css_color(s);
                        c.apply_opacity(st.alpha);
                        GradientStop::new(*o as f32, c)
                    })
                    .collect();
                let pt = |x: f64, y: f64| Point::from_xy(x as f32, y as f32);
                match g.kind {
                    GradientKind::Linear { x0, y0, x1, y1 } => {
                        LinearGradient::new(pt(x0, y0), pt(x1, y1), stops, SpreadMode::Pad, Transform::identity())?
                    }
                    GradientKind::Radial { x0, y0, r0, x1, y1, r1 } => RadialGradient::new(
                        pt(x0, y0),
                        r0 as f32,
                        pt(x1, y1),
                        r1 as f32,
                        stops,
                        SpreadMode::Pad,
                        Transform::identity(),
                    )?,
                }
            }
            // a pattern borrows its pixels: see `with_pattern`
            Style::Pattern(_) => return None,
        };
        Some(Paint { shader, blend_mode: st.blend, anti_alias: true, ..Paint::default() })
    }

    /// Draws `path` (already in pixels of the canvas when `device`) with `style`: its shadow, then
    /// itself
    fn draw_path(&self, path: &Path, style: &Style, device: bool) {
        let st = self.state.borrow().clone();
        let transform = if device { Transform::identity() } else { st.transform };
        let Some(dev) = path.clone().transform(transform) else { return };
        if st.shadow_blur > 0.0 && st.shadow_color.alpha() > 0.0 {
            self.shadow(&dev, &st);
        }
        let mut pixmap = self.pixmap.borrow_mut();
        match style {
            Style::Pattern(src) => {
                let src = src.borrow();
                let quality = if st.smooth { FilterQuality::Bilinear } else { FilterQuality::Nearest };
                let shader = Pattern::new(src.as_ref(), SpreadMode::Repeat, quality, st.alpha, st.transform);
                let paint = Paint { shader, blend_mode: st.blend, anti_alias: true, ..Paint::default() };
                pixmap.fill_path(&dev, &paint, FillRule::Winding, Transform::identity(), None);
            }
            _ => {
                drop(pixmap);
                let Some(mut paint) = self.paint(style) else { return };
                paint.shader.transform(st.transform);
                self.pixmap.borrow_mut().fill_path(&dev, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
    }

    /// The shadow of `dev` (a path in pixels of the canvas): its mask blurred, of the shadow's
    /// colour, offset
    fn shadow(&self, dev: &Path, st: &State) {
        let (cw, ch) = (self.width() as i32, self.height() as i32);
        let sigma = st.shadow_blur / 2.0;
        let margin = (sigma * 3.0).ceil() as i32 + 1;
        let b = dev.bounds();
        let x0 = (b.left().floor() as i32 - margin).max(-margin);
        let y0 = (b.top().floor() as i32 - margin).max(-margin);
        let x1 = (b.right().ceil() as i32 + margin).min(cw + margin);
        let y1 = (b.bottom().ceil() as i32 + margin).min(ch + margin);
        let (w, h) = ((x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32);
        let Some(mut mask) = Mask::new(w, h) else { return };
        mask.fill_path(dev, FillRule::Winding, true, Transform::from_translate(-x0 as f32, -y0 as f32));
        blur(mask.data_mut(), w as usize, h as usize, sigma);
        // the mask, moved to its place in the canvas
        let mut full = vec![0u8; (cw * ch) as usize];
        let dy = st.shadow_dy.round() as i32;
        for y in 0..h as i32 {
            let cy = y0 + y + dy;
            if cy < 0 || cy >= ch {
                continue;
            }
            for x in 0..w as i32 {
                let cx = x0 + x;
                if cx >= 0 && cx < cw {
                    full[(cy * cw + cx) as usize] = mask.data()[(y as u32 * w + x as u32) as usize];
                }
            }
        }
        let Some(full) = Mask::from_vec(full, IntSize::from_wh(cw as u32, ch as u32).expect("size")) else { return };
        let mut c = st.shadow_color;
        c.apply_opacity(st.alpha);
        let paint = Paint { shader: Shader::SolidColor(c), blend_mode: st.blend, ..Paint::default() };
        let rect = Rect::from_xywh(0.0, 0.0, cw as f32, ch as f32).expect("rect");
        self.pixmap.borrow_mut().fill_rect(rect, &paint, Transform::identity(), Some(&full));
    }

    fn stroke_path(&self, path: &Path) {
        let st = self.state.borrow().clone();
        let mut stroke = Stroke { width: st.line_width, line_cap: st.cap, line_join: st.join, ..Stroke::default() };
        if st.dash.len() >= 2 {
            stroke.dash = StrokeDash::new(st.dash.clone(), 0.0);
        }
        // the outline of the stroke, in pixels of the canvas: the transform scales the width too
        let Some(dev) = path.clone().transform(st.transform) else { return };
        let res = (st.transform.sx.abs().max(st.transform.sy.abs())).max(0.01);
        let scaled = Stroke { width: st.line_width * res, ..stroke };
        let scaled = Stroke {
            dash: scaled.dash.as_ref().and_then(|_| StrokeDash::new(st.dash.iter().map(|d| d * res).collect(), 0.0)),
            ..scaled
        };
        if let Some(outline) = dev.stroke(&scaled, 1.0) {
            self.draw_path(&outline, &st.stroke, true);
        }
    }

    fn layer_pixmap(layer: &dyn Layer) -> Option<Rc<RefCell<Pixmap>>> {
        layer.as_any().downcast_ref::<SkiaLayer>().map(|l| Rc::clone(&l.0.pixmap))
    }
}

/// Gaussian blur of a mask, by three box blurs (as browsers do), `sigma` in pixels
fn blur(data: &mut [u8], w: usize, h: usize, sigma: f64) {
    if sigma < 0.5 {
        return;
    }
    // the size of the three boxes that make a Gaussian of `sigma`
    let r = ((sigma * 3.0 * (2.0 * PI).sqrt() / 4.0 + 0.5).floor() as usize).max(1) / 2;
    let mut buf = vec![0u8; data.len()];
    for _ in 0..3 {
        box_blur(data, &mut buf, w, h, r, true);
        box_blur(&buf, data, w, h, r, false);
    }
}

fn box_blur(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, across: bool) {
    let (n, lines) = if across { (w, h) } else { (h, w) };
    let at = |line: usize, i: usize| if across { line * w + i } else { i * w + line };
    let d = (2 * r + 1) as u32;
    for line in 0..lines {
        let mut sum: u32 = 0;
        for i in 0..=r.min(n - 1) {
            sum += u32::from(src[at(line, i)]);
        }
        for i in 0..n {
            dst[at(line, i)] = (sum / d) as u8;
            if i + r + 1 < n {
                sum += u32::from(src[at(line, i + r + 1)]);
            }
            if i >= r {
                sum -= u32::from(src[at(line, i - r)]);
            }
        }
    }
}

/// A canvas outside the page
pub struct SkiaLayer(Skia);

impl Layer for SkiaLayer {
    fn canvas(&self) -> &dyn Canvas {
        &self.0
    }
    fn width(&self) -> u32 {
        self.0.width()
    }
    fn height(&self) -> u32 {
        self.0.height()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn blend_mode(op: &str) -> BlendMode {
    match op {
        "lighter" => BlendMode::Plus,
        "overlay" => BlendMode::Overlay,
        "copy" => BlendMode::Source,
        "multiply" => BlendMode::Multiply,
        "screen" => BlendMode::Screen,
        _ => BlendMode::SourceOver,
    }
}

impl Canvas for Skia {
    fn set_transform(&self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.state.borrow_mut().transform =
            Transform::from_row(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32);
    }
    fn set_global_composite_operation(&self, op: &str) {
        self.state.borrow_mut().blend = blend_mode(op);
    }
    fn set_global_alpha(&self, alpha: f64) {
        self.state.borrow_mut().alpha = alpha.clamp(0.0, 1.0) as f32;
    }
    fn set_fill_style_str(&self, color: &str) {
        self.state.borrow_mut().fill = Style::Color(css_color(color));
    }
    fn set_stroke_style_str(&self, color: &str) {
        self.state.borrow_mut().stroke = Style::Color(css_color(color));
    }
    fn set_fill_gradient(&self, g: &Gradient) {
        self.state.borrow_mut().fill = Style::Gradient(g.clone());
    }
    fn set_fill_pattern(&self, layer: &dyn Layer) {
        if let Some(p) = Skia::layer_pixmap(layer) {
            self.state.borrow_mut().fill = Style::Pattern(p);
        }
    }
    fn fill_rect(&self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(r) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
            let fill = self.state.borrow().fill.clone();
            self.draw_path(&PathBuilder::from_rect(r), &fill, false);
        }
    }
    fn stroke_rect(&self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(r) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
            self.stroke_path(&PathBuilder::from_rect(r));
        }
    }
    fn clear_rect(&self, x: f64, y: f64, w: f64, h: f64) {
        let Some(r) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) else { return };
        let t = self.state.borrow().transform;
        let paint = Paint { blend_mode: BlendMode::Clear, ..Paint::default() };
        self.pixmap.borrow_mut().fill_rect(r, &paint, t, None);
    }
    fn begin_path(&self) {
        *self.path.borrow_mut() = PathBuilder::new();
        *self.current.borrow_mut() = None;
    }
    fn move_to(&self, x: f64, y: f64) {
        self.path.borrow_mut().move_to(x as f32, y as f32);
        *self.current.borrow_mut() = Some((x as f32, y as f32));
    }
    fn line_to(&self, x: f64, y: f64) {
        if self.current.borrow().is_none() {
            return self.move_to(x, y);
        }
        self.path.borrow_mut().line_to(x as f32, y as f32);
        *self.current.borrow_mut() = Some((x as f32, y as f32));
    }
    fn close_path(&self) {
        self.path.borrow_mut().close();
    }
    fn arc(&self, x: f64, y: f64, r: f64, start: f64, end: f64) {
        // clockwise from `start` to `end`, a full turn at most, in cubic curves of 90° at most
        let sweep = (end - start).clamp(-2.0 * PI, 2.0 * PI);
        let n = (sweep.abs() / (PI / 2.0)).ceil().max(1.0) as usize;
        let step = sweep / n as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let at = |a: f64| (x + r * a.cos(), y + r * a.sin());
        let (sx, sy) = at(start);
        self.line_to(sx, sy);
        let mut path = self.path.borrow_mut();
        let mut a = start;
        for _ in 0..n {
            let b = a + step;
            let (p0, p3) = (at(a), at(b));
            let c1 = (p0.0 - k * r * a.sin(), p0.1 + k * r * a.cos());
            let c2 = (p3.0 + k * r * b.sin(), p3.1 - k * r * b.cos());
            path.cubic_to(c1.0 as f32, c1.1 as f32, c2.0 as f32, c2.1 as f32, p3.0 as f32, p3.1 as f32);
            a = b;
        }
        let end = at(a);
        *self.current.borrow_mut() = Some((end.0 as f32, end.1 as f32));
    }
    fn fill(&self) {
        if let Some(p) = self.built_path() {
            let fill = self.state.borrow().fill.clone();
            self.draw_path(&p, &fill, false);
        }
    }
    fn stroke(&self) {
        if let Some(p) = self.built_path() {
            self.stroke_path(&p);
        }
    }
    fn save(&self) {
        let st = self.state.borrow().clone();
        self.stack.borrow_mut().push(st);
    }
    fn restore(&self) {
        if let Some(st) = self.stack.borrow_mut().pop() {
            *self.state.borrow_mut() = st;
        }
    }
    fn set_line_width(&self, w: f64) {
        if w > 0.0 {
            self.state.borrow_mut().line_width = w as f32;
        }
    }
    fn set_line_join(&self, join: &str) {
        self.state.borrow_mut().join = match join {
            "round" => LineJoin::Round,
            "bevel" => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
    }
    fn set_line_cap(&self, cap: &str) {
        self.state.borrow_mut().cap = match cap {
            "round" => LineCap::Round,
            "square" => LineCap::Square,
            _ => LineCap::Butt,
        };
    }
    fn set_line_dash(&self, dash: &[f64]) {
        let mut d: Vec<f32> = dash.iter().map(|&v| v as f32).collect();
        // as the canvas: an odd list is repeated
        if d.len() % 2 == 1 {
            d.extend(d.clone());
        }
        self.state.borrow_mut().dash = d;
    }
    fn set_shadow_blur(&self, blur: f64) {
        self.state.borrow_mut().shadow_blur = blur.max(0.0);
    }
    fn set_shadow_color(&self, color: &str) {
        self.state.borrow_mut().shadow_color = css_color(color);
    }
    fn set_shadow_offset_y(&self, dy: f64) {
        self.state.borrow_mut().shadow_dy = dy as f32;
    }
    fn set_font(&self, font: &str) {
        // "12.5px \"IBM Plex Mono\", monospace": the size only (one embedded font)
        let size = font.split_whitespace().find_map(|w| w.strip_suffix("px")?.parse::<f32>().ok());
        let mut st = self.state.borrow_mut();
        if let Some(size) = size {
            st.font_size = size;
        }
        st.mono = font.contains("Mono") || font.contains("monospace");
    }
    fn set_text_align(&self, align: &str) {
        self.state.borrow_mut().align = align.into();
    }
    fn set_text_baseline(&self, baseline: &str) {
        self.state.borrow_mut().baseline = baseline.into();
    }
    fn fill_text(&self, text: &str, x: f64, y: f64) {
        let st = self.state.borrow().clone();
        let font = FontRef::try_from_slice(if st.mono { PLEX_MONO } else { LATO }).expect("the embedded font");
        // a CSS size is the em square; ab_glyph's scale, the height from descent to ascent
        let em = font.units_per_em().unwrap_or(1000.0);
        let scaled = font.as_scaled(PxScale::from(st.font_size * font.height_unscaled() / em));
        let width: f32 = text.chars().map(|c| scaled.h_advance(scaled.glyph_id(c))).sum();
        let x = x as f32
            - match st.align.as_str() {
                "center" => width / 2.0,
                "right" | "end" => width,
                _ => 0.0,
            };
        // as Chromium: the em square shared between the ascent and the descent
        let up = st.font_size * font.ascent_unscaled() / font.height_unscaled();
        let y = y as f32
            + match st.baseline.as_str() {
                "top" | "hanging" => up,
                "middle" => up - st.font_size / 2.0,
                "bottom" | "ideographic" => up - st.font_size,
                _ => 0.0,
            };
        // the outlines, in font units (y up), placed and scaled
        let (sx, sy) = (scaled.h_scale_factor(), scaled.v_scale_factor());
        let mut pb = PathBuilder::new();
        let mut pen = x;
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            if let Some(outline) = font.outline(id) {
                let p = |q: ab_glyph::Point| (pen + q.x * sx, y - q.y * sy);
                let mut last: Option<(f32, f32)> = None;
                for curve in outline.curves {
                    let (start, end) = match curve {
                        OutlineCurve::Line(a, b) | OutlineCurve::Quad(a, _, b) | OutlineCurve::Cubic(a, _, _, b) => {
                            (p(a), p(b))
                        }
                    };
                    if last != Some(start) {
                        if last.is_some() {
                            pb.close();
                        }
                        pb.move_to(start.0, start.1);
                    }
                    match curve {
                        OutlineCurve::Line(..) => pb.line_to(end.0, end.1),
                        OutlineCurve::Quad(_, c, _) => {
                            let c = p(c);
                            pb.quad_to(c.0, c.1, end.0, end.1);
                        }
                        OutlineCurve::Cubic(_, c1, c2, _) => {
                            let (c1, c2) = (p(c1), p(c2));
                            pb.cubic_to(c1.0, c1.1, c2.0, c2.1, end.0, end.1);
                        }
                    }
                    last = Some(end);
                }
                if last.is_some() {
                    pb.close();
                }
            }
            pen += scaled.h_advance(id);
        }
        if let Some(path) = pb.finish() {
            self.draw_path(&path, &st.fill, false);
        }
    }
    fn set_image_smoothing_enabled(&self, on: bool) {
        self.state.borrow_mut().smooth = on;
    }
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
    ) {
        let Some(src) = Skia::layer_pixmap(layer) else { return };
        let Some(dst) = Rect::from_xywh(dx as f32, dy as f32, dw as f32, dh as f32) else { return };
        if sw <= 0.0 || sh <= 0.0 {
            return;
        }
        let st = self.state.borrow().clone();
        let (kx, ky) = ((dw / sw) as f32, (dh / sh) as f32);
        // the source rectangle onto the destination one, in user space
        let place = Transform::from_row(kx, 0.0, 0.0, ky, dx as f32 - sx as f32 * kx, dy as f32 - sy as f32 * ky);
        let src = src.borrow();
        let quality = if st.smooth { FilterQuality::Bilinear } else { FilterQuality::Nearest };
        let shader = Pattern::new(src.as_ref(), SpreadMode::Pad, quality, st.alpha, place);
        let paint = Paint { shader, blend_mode: st.blend, anti_alias: false, ..Paint::default() };
        self.pixmap.borrow_mut().fill_rect(dst, &paint, st.transform, None);
    }
    fn draw_layer(&self, layer: &dyn Layer, dx: f64, dy: f64, dw: f64, dh: f64) {
        let (w, h) = (f64::from(layer.width()), f64::from(layer.height()));
        self.draw_layer_part(layer, 0.0, 0.0, w, h, dx, dy, dw, dh);
    }
    fn layer(&self, w: u32, h: u32) -> Box<dyn Layer> {
        Box::new(SkiaLayer(Skia::new(w, h)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(c: Color) -> (u8, u8, u8, u8) {
        let c = c.to_color_u8();
        (c.red(), c.green(), c.blue(), c.alpha())
    }

    #[test]
    fn css_colors() {
        assert_eq!(rgb(css_color("#ff8000")), (255, 128, 0, 255));
        assert_eq!(rgb(css_color("#fff")), (255, 255, 255, 255));
        assert_eq!(rgb(css_color("rgba(255,255,255,.8)")), (255, 255, 255, 204));
        assert_eq!(rgb(css_color("rgb(10,20,30)")), (10, 20, 30, 255));
        assert_eq!(rgb(css_color("hsl(0,100%,50%)")), (255, 0, 0, 255));
        assert_eq!(rgb(css_color("hsla(240,100%,50%,0.5)")), (0, 0, 255, 128));
        assert_eq!(rgb(css_color("nonsense")).3, 0);
    }

    #[test]
    fn draws_shapes_shadows_text_layers() {
        let c = Skia::new(40, 20);
        c.set_fill_style_str("#000");
        c.fill_rect(0.0, 0.0, 40.0, 20.0);
        c.set_fill_style_str("#ff0000");
        c.begin_path();
        c.arc(10.0, 10.0, 5.0, 0.0, 2.0 * PI);
        c.fill();
        let px = c.pixels();
        assert_eq!(px[10 * 40 + 10], 0xff0000);
        assert_eq!(px[0], 0);
        // a shadow spreads beyond the shape
        c.set_shadow_blur(6.0);
        c.set_shadow_color("rgba(0,255,0,1)");
        c.fill_rect(30.0, 8.0, 4.0, 4.0);
        let px = c.pixels();
        assert!(px[10 * 40 + 36] & 0x00ff00 > 0);
        // text in its fill colour, a layer drawn enlarged
        let t = Skia::new(60, 30);
        t.set_fill_style_str("#ffffff");
        t.set_font("20px sans-serif");
        t.set_text_baseline("top");
        t.fill_text("Hi", 2.0, 2.0);
        assert!(t.pixels().iter().any(|&p| p == 0xffffff));
        let l = c.layer(1, 2);
        l.canvas().set_fill_style_str("#0000ff");
        l.canvas().fill_rect(0.0, 0.0, 1.0, 2.0);
        t.set_image_smoothing_enabled(false);
        t.draw_layer(l.as_ref(), 50.0, 0.0, 10.0, 30.0);
        assert_eq!(t.pixels()[15 * 60 + 55], 0x0000ff);
    }

    #[test]
    fn blurs_a_mask() {
        let mut m = vec![0u8; 21 * 21];
        m[10 * 21 + 10] = 255;
        blur(&mut m, 21, 21, 3.0);
        assert!(m[10 * 21 + 10] > 0 && m[10 * 21 + 10] < 255 && m[10 * 21 + 13] > 0);
        assert!(m[10 * 21 + 10] >= m[10 * 21 + 13]);
    }
}
