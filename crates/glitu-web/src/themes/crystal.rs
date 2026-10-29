//! "crystal" backdrop: almost black facets, touches of blue and purple, a turning light, a few
//! edges that pulse and, as an option, a chase running along the edges.
//!
//! The mesh is computed in slide coordinates (1280 × 720). Mathematical functions and random
//! numbers go through JavaScript (`Math`): same image as the original JavaScript version, for
//! the same seed.

use std::cell::RefCell;
use std::collections::HashSet;
use std::f64::consts::PI;
use std::rc::Rc;

use js_sys::Math;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};

use crate::dom;

#[derive(Clone, Copy)]
pub struct Options {
    pub cell: f64,
    pub speed: f64,
    pub accent: f64,
    pub glow: f64,
    pub chase: bool,
    pub chase_every: f64,
    pub chase_duration: f64,
    pub chase_length: usize,
    pub fps: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            cell: 45.0, speed: 1.0, accent: 0.15, glow: 0.03, chase: false,
            chase_every: 5.0, chase_duration: 1.5, chase_length: 10, fps: 30.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Pt { x: f64, y: f64, z: f64 }

struct Tri {
    v: [usize; 3],
    nx: f64, ny: f64, nz: f64,
    cx: f64, cy: f64,
    accent: bool, glow: bool, ph: f64,
}

struct Chase { path: Vec<usize>, cum: Vec<f64>, total: f64, time: f64 }

struct Backdrop {
    o: Options,
    host: Element,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    reduce: bool,
    seed: i32,
    w: f64, h: f64, scale: f64,
    pts: Vec<Pt>, tris: Vec<Tri>, vadj: Vec<Vec<usize>>,
    t: f64, last: f64, raf: i32, chase: Option<Chase>, wait: f64,
    /// Speed of the animation (0: still, 1: normal), which tends towards `target`
    rate: f64, target: f64,
}

/// mulberry32 generator, identical to the JavaScript version (Math.imul, >>>)
pub fn rng(seed: i32) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
        s = s.wrapping_add(0x6D2B79F5u32 as i32);
        let mut x = ((s as u32) ^ ((s as u32) >> 15)).wrapping_mul(1 | s as u32) as i32;
        x = (x.wrapping_add(((x as u32) ^ ((x as u32) >> 7)).wrapping_mul(61 | x as u32) as i32)) ^ x;
        (((x as u32) ^ ((x as u32) >> 14)) as f64) / 4294967296.0
    }
}

#[wasm_bindgen]
extern "C" {
    /// Math.hypot with three arguments
    #[wasm_bindgen(js_namespace = Math, js_name = hypot)]
    fn hypot3(a: f64, b: f64, c: f64) -> f64;
}

impl Backdrop {
    fn add(&mut self, i1: usize, i2: usize, i3: usize, r: &mut impl FnMut() -> f64) {
        let (a, b, c) = (self.pts[i1], self.pts[i2], self.pts[i3]);
        let (ux, uy, uz) = (b.x - a.x, b.y - a.y, b.z - a.z);
        let (vx, vy, vz) = (c.x - a.x, c.y - a.y, c.z - a.z);
        let (mut nx, mut ny, mut nz) = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx);
        let l = hypot3(nx, ny, nz);
        let l = if l == 0.0 { 1.0 } else { l };
        if nz < 0.0 { nx = -nx; ny = -ny; nz = -nz; }
        let accent = r() < self.o.accent;
        let glow = r() < self.o.glow;
        let ph = r() * PI * 2.0;
        self.tris.push(Tri {
            v: [i1, i2, i3], nx: nx / l, ny: ny / l, nz: nz / l,
            cx: (a.x + b.x + c.x) / 3.0, cy: (a.y + b.y + c.y) / 3.0, accent, glow, ph,
        });
    }

    fn build(&mut self) {
        let mut r = rng(self.seed);
        let c = self.o.cell;
        let cols = (self.w / c).ceil() as usize + 3;
        let rows = (self.h / c).ceil() as usize + 3;
        self.pts.clear();
        for j in 0..rows {
            for i in 0..cols {
                let x = (i as f64 - 1.0) * c + (r() - 0.5) * c * 0.85;
                let y = (j as f64 - 1.0) * c + (r() - 0.5) * c * 0.85;
                let z = (r() * 2.0 - 1.0) * c * 0.55;
                self.pts.push(Pt { x, y, z });
            }
        }
        self.tris.clear();
        for j in 0..rows - 1 {
            for i in 0..cols - 1 {
                let a = j * cols + i;
                let (b, d, e) = (a + 1, a + cols, a + cols + 1);
                if r() < 0.5 {
                    self.add(a, b, e, &mut r);
                    self.add(a, e, d, &mut r);
                } else {
                    self.add(a, b, d, &mut r);
                    self.add(b, e, d, &mut r);
                }
            }
        }
        // Neighbours of the vertices, for the chase
        let mut edges = HashSet::new();
        self.vadj = vec![Vec::new(); self.pts.len()];
        for t in &self.tris {
            for (p, q) in [(t.v[0], t.v[1]), (t.v[1], t.v[2]), (t.v[2], t.v[0])] {
                if edges.insert((p.min(q), p.max(q))) {
                    self.vadj[p].push(q);
                    self.vadj[q].push(p);
                }
            }
        }
        self.chase = None;
    }

    // Path of about chase_length edges, in a roughly constant direction
    fn start_chase(&mut self) {
        let (w, h) = (self.w, self.h);
        let visible: Vec<usize> = self.pts.iter().enumerate()
            .filter(|(_, p)| p.x > w * 0.1 && p.x < w * 0.9 && p.y > h * 0.1 && p.y < h * 0.9)
            .map(|(k, _)| k).collect();
        if visible.is_empty() { return }
        let mut cur = visible[(Math::random() * visible.len() as f64) as usize];
        let mut ang = Math::random() * PI * 2.0;
        let (mut dx, mut dy) = (Math::cos(ang), Math::sin(ang));
        let mut path = vec![cur];
        let mut seen = HashSet::from([cur]);
        while path.len() <= self.o.chase_length {
            let (mut best, mut best_score) = (None, f64::NEG_INFINITY);
            let p0 = self.pts[cur];
            for &n in &self.vadj[cur] {
                if seen.contains(&n) { continue }
                let (ex, ey) = (self.pts[n].x - p0.x, self.pts[n].y - p0.y);
                let el = Math::hypot(ex, ey);
                let el = if el == 0.0 { 1.0 } else { el };
                let score = (ex * dx + ey * dy) / el + Math::random() * 0.5;
                if score > best_score { best_score = score; best = Some(n); }
            }
            let Some(b) = best else { break };
            ang = Math::atan2(self.pts[b].y - p0.y, self.pts[b].x - p0.x) + (Math::random() - 0.5) * 0.6;
            dx = Math::cos(ang);
            dy = Math::sin(ang);
            seen.insert(b);
            path.push(b);
            cur = b;
        }
        if path.len() < 2 { return }
        let mut cum = vec![0.0];
        for k in 1..path.len() {
            let (a, b) = (self.pts[path[k - 1]], self.pts[path[k]]);
            cum.push(cum[k - 1] + Math::hypot(b.x - a.x, b.y - a.y));
        }
        let total = *cum.last().unwrap();
        self.chase = Some(Chase { path, cum, total, time: 0.0 });
    }

    fn chase_at(&self, d: f64) -> (f64, f64) {
        let c = self.chase.as_ref().unwrap();
        let mut k = 1;
        while k < c.cum.len() - 1 && c.cum[k] < d { k += 1 }
        let (a, b) = (self.pts[c.path[k - 1]], self.pts[c.path[k]]);
        let seg = c.cum[k] - c.cum[k - 1];
        let seg = if seg == 0.0 { 1.0 } else { seg };
        let u = ((d - c.cum[k - 1]) / seg).clamp(0.0, 1.0);
        (a.x + (b.x - a.x) * u, a.y + (b.y - a.y) * u)
    }

    fn tri(&self, t: &Tri) {
        let (a, b, c) = (self.pts[t.v[0]], self.pts[t.v[1]], self.pts[t.v[2]]);
        let ctx = &self.ctx;
        ctx.begin_path();
        ctx.move_to(a.x, a.y);
        ctx.line_to(b.x, b.y);
        ctx.line_to(c.x, c.y);
        ctx.close_path();
    }

    fn draw(&self) {
        let ctx = &self.ctx;
        let t = self.t;
        let (mut lx, mut ly, mut lz) = (Math::cos(t * 0.23) * 0.55, Math::sin(t * 0.17) * 0.45 - 0.25, 0.75);
        let ll = hypot3(lx, ly, lz);
        lx /= ll; ly /= ll; lz /= ll;
        let (w, h) = (self.w, self.h);
        let _ = ctx.set_transform(self.scale, 0.0, 0.0, self.scale, 0.0, 0.0);
        let _ = ctx.set_global_composite_operation("source-over");
        ctx.set_shadow_blur(0.0);
        ctx.set_fill_style_str("#030308");
        ctx.fill_rect(0.0, 0.0, w, h);
        let (sx, sy, sr) = (w * 0.5, -h * 0.15, w.max(h) * 1.1);
        ctx.set_line_join("round");
        let mut hues = Vec::with_capacity(self.tris.len());
        for tr in &self.tris {
            let d = (tr.nx * lx + tr.ny * ly + tr.nz * lz).max(0.0);
            let shade = ((d - 0.55) / 0.45).max(0.0);
            let spec = Math::pow(d, 28.0);
            let mix = ((tr.cx / w) * 0.55 + (tr.cy / h) * 0.45).clamp(0.0, 1.0);
            let hue = 222.0 + mix * 58.0; // 222 = bleu, 280 = violet
            hues.push(hue);
            let near = (1.0 - Math::hypot(tr.cx - sx, tr.cy - sy) / sr).max(0.0);
            let (l, s) = if tr.accent {
                (6.0 + shade * 18.0 + near * 10.0 + spec * 14.0, 60.0 + near * 20.0)
            } else {
                (1.5 + shade * 4.0 + near * 2.5 + spec * 5.0, 25.0) // black, barely tinted
            };
            self.tri(tr);
            ctx.set_fill_style_str(&format!("hsl({},{}%,{}%)", fixed(hue, 1), fixed(s, 0), fixed(l, 1)));
            ctx.fill();
            let ea = if tr.accent { 0.08 + spec * 0.3 } else { 0.035 + spec * 0.12 };
            ctx.set_stroke_style_str(&format!("hsla({},80%,78%,{})", fixed(hue + 10.0, 1), fixed(ea, 3)));
            ctx.set_line_width(1.0);
            ctx.stroke();
        }
        let _ = ctx.set_global_composite_operation("lighter");
        ctx.set_line_width(1.2);
        for (k, tr) in self.tris.iter().enumerate() {
            if !tr.glow { continue }
            let p = 0.5 + 0.5 * Math::sin(t * 0.9 + tr.ph);
            ctx.set_shadow_blur(14.0);
            ctx.set_shadow_color(&format!("hsla({},100%,70%,{})", fixed(hues[k], 1), fixed(p, 2)));
            ctx.set_stroke_style_str(&format!("hsla({},100%,82%,{})", fixed(hues[k] + 15.0, 1), fixed(0.15 + 0.5 * p, 2)));
            self.tri(tr);
            ctx.stroke();
        }
        // Chase: a white point followed by a short trail
        if let Some(chase) = &self.chase {
            let trail = self.o.cell * 1.4;
            let s = (chase.time / self.o.chase_duration) * (chase.total + trail);
            let (head_d, tail_d) = (s.min(chase.total), (s - trail).max(0.0));
            ctx.set_line_cap("round");
            ctx.set_line_width(1.6);
            ctx.set_shadow_color("rgba(255,255,255,0.8)");
            ctx.set_shadow_blur(8.0);
            let mut prev = self.chase_at(tail_d);
            let mut dd = tail_d + 4.0;
            while dd < head_d + 4.0 {
                let dq = dd.min(head_d);
                let q = self.chase_at(dq);
                let fa = (dq - (s - trail)) / trail; // 0 at the tail, 1 at the head
                ctx.set_stroke_style_str(&format!("rgba(255,255,255,{})", fixed(0.85 * fa * fa, 3)));
                ctx.begin_path();
                ctx.move_to(prev.0, prev.1);
                ctx.line_to(q.0, q.1);
                ctx.stroke();
                prev = q;
                dd += 4.0;
            }
            if s <= chase.total {
                let hp = self.chase_at(head_d);
                if let Ok(g) = ctx.create_radial_gradient(hp.0, hp.1, 0.0, hp.0, hp.1, 14.0) {
                    let _ = g.add_color_stop(0.0, "rgba(255,255,255,1)");
                    let _ = g.add_color_stop(0.22, "rgba(255,255,255,0.75)");
                    let _ = g.add_color_stop(1.0, "rgba(255,255,255,0)");
                    ctx.set_shadow_blur(0.0);
                    ctx.set_fill_style_canvas_gradient(&g);
                    ctx.begin_path();
                    let _ = ctx.arc(hp.0, hp.1, 14.0, 0.0, PI * 2.0);
                    ctx.fill();
                }
            }
        }
        ctx.set_shadow_blur(0.0);
        let _ = ctx.set_global_composite_operation("source-over");
    }

    fn step(&mut self, dt: f64) {
        self.t += dt * self.o.speed;
        if !self.o.chase { return }
        if let Some(c) = &mut self.chase {
            c.time += dt;
            if c.time >= self.o.chase_duration {
                self.chase = None;
                self.wait = self.o.chase_every;
            }
        } else {
            self.wait -= dt;
            if self.wait <= 0.0 { self.start_chase() }
        }
    }

    // Mesh in slide px, image at the real resolution of the screen (slide scaled)
    fn resize(&mut self) -> bool {
        let host = dom::html(&self.host);
        let (w, h) = (host.offset_width() as f64, host.offset_height() as f64);
        let rect = self.host.get_bounding_client_rect();
        if w == 0.0 || h == 0.0 { return false }
        let dpr = dom::window().device_pixel_ratio();
        let dpr = if dpr == 0.0 { 1.0 } else { dpr };
        self.scale = dpr.min(2.0) * rect.width() / w;
        self.canvas.set_width(js_round(w * self.scale) as u32);
        self.canvas.set_height(js_round(h * self.scale) as u32);
        if w != self.w || h != self.h {
            self.w = w;
            self.h = h;
            self.build();
        }
        true
    }
}

/// JavaScript's Math.round (half towards +∞)
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// JavaScript's Number.prototype.toFixed
fn fixed(x: f64, digits: u8) -> String {
    JsValue::from_f64(x)
        .unchecked_into::<js_sys::Number>()
        .to_fixed(digits).map_or_else(|_| format!("{x:.*}", digits as usize), String::from)
}

#[derive(Clone)]
pub struct Crystal(Rc<RefCell<Backdrop>>);

impl Crystal {
    pub fn new(host: &Element, o: Options) -> Crystal {
        let canvas: HtmlCanvasElement = dom::create("canvas").unchecked_into();
        canvas.set_class_name("backdrop-canvas");
        let _ = canvas.set_attribute("aria-hidden", "true");
        let _ = host.prepend_with_node_1(&canvas);
        let ctx: CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten().expect("canvas 2d").unchecked_into();
        let reduce = dom::window().match_media("(prefers-reduced-motion: reduce)").ok().flatten().is_some_and(|m| m.matches());
        let seed = (Math::random() * 1e9) as i32;
        Crystal(Rc::new(RefCell::new(Backdrop {
            o, host: host.clone(), canvas, ctx, reduce, seed,
            w: 0.0, h: 0.0, scale: 1.0, pts: vec![], tris: vec![], vadj: vec![],
            t: 0.0, last: 0.0, raf: 0, chase: None, wait: 1.2, rate: 0.0, target: 0.0,
        })))
    }

    /// Redraws the backdrop at the size of its host: first image, resize
    pub fn redraw(&self) {
        let mut f = self.0.borrow_mut();
        if f.resize() {
            f.draw();
        }
    }

    /// The current image of the backdrop (PNG as data:), for printing
    pub fn snapshot(&self) -> Option<String> {
        self.0.borrow().canvas.to_data_url().ok()
    }

    /// Changes pace without changing image: the animation slows down until it stands still, or
    /// resumes gently; the chase stops at once
    pub fn set_motion(&self, motion: Motion) {
        let mut f = self.0.borrow_mut();
        let animated = matches!(motion, Motion::Animated { .. }) && !f.reduce;
        f.o.chase = matches!(motion, Motion::Animated { chase: true });
        if !f.o.chase {
            f.chase = None;
        }
        f.target = if animated { 1.0 } else { 0.0 };
        if animated && f.raf == 0 {
            f.last = dom::now();
            let me = self.clone();
            f.raf = dom::raf(move |t| me.frame(t));
        }
    }

    fn frame(&self, now: f64) {
        let me = self.clone();
        let mut f = self.0.borrow_mut();
        if now - f.last < 1000.0 / f.o.fps {
            f.raf = dom::raf(move |t| me.frame(t));
            return;
        }
        let dt = ((now - f.last) / 1000.0).min(0.1);
        f.rate += (f.target - f.rate) * (dt * EASING).min(1.0);
        if f.target == 0.0 && f.rate < 0.01 {
            f.rate = 0.0;
            f.raf = 0;
            f.draw();
            return;
        }
        let rate = f.rate;
        f.step(dt * rate);
        f.last = now;
        f.draw();
        f.raf = dom::raf(move |t| me.frame(t));
    }
}

/// Pace of the backdrop
#[derive(Clone, Copy, PartialEq)]
pub enum Motion {
    Still,
    Animated { chase: bool },
}

/// Speed of slowing down and resuming (per second): about one second to stand still
const EASING: f64 = 3.0;
