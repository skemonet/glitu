//! Animated backdrops of the template themes, other than crystal: constellation, cut paper,
//! aurora, amber ASCII, blueprint, isometric. A shared driver (canvas the size of the slide,
//! slowing down, dimming) and one scene per backdrop, which only draws an image at a given
//! instant.
//!
//! The dimming (`dim`) goes from 0, the whole scene (cover, chapters), to 1, the scene erased
//! on the text side (full-width text slides); panel slides are in between. It glides from one
//! value to the other when the slide changes, even when the backdrop is still.

use std::cell::RefCell;
use std::rc::Rc;

pub use glitu_backdrop::Scene;
use glitu_backdrop::{Canvas, DIMMING, EASING, Motion};
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlCanvasElement};

use crate::dom;
use crate::themes::canvas::Ctx;

/// The backdrop named in glitu.yaml (`backdrop`), if it is one of the scenes
pub fn scene(name: &str) -> Option<Box<dyn Scene>> {
    glitu_backdrop::scene(name)
}

struct Backdrop {
    scene: Box<dyn Scene>,
    host: Element,
    canvas: HtmlCanvasElement,
    ctx: Ctx,
    reduce: bool,
    w: f64,
    h: f64,
    scale: f64,
    t: f64,
    last: f64,
    raf: i32,
    /// Speed of the animation (0: still, 1: normal), which tends towards `target`
    rate: f64,
    target: f64,
    /// Dimming, which tends towards `dim_to`
    dim: f64,
    dim_to: f64,
}

impl Backdrop {
    fn draw(&self) {
        self.ctx.set_transform(self.scale, 0.0, 0.0, self.scale, 0.0, 0.0);
        self.ctx.set_global_composite_operation("source-over");
        self.ctx.set_global_alpha(1.0);
        self.scene.draw(&self.ctx, self.w, self.h, self.t, self.dim);
    }

    // As crystal: the scene in slide px, the image at the real resolution of the screen
    fn resize(&mut self) -> bool {
        let host = dom::html(&self.host);
        let (w, h) = (host.offset_width() as f64, host.offset_height() as f64);
        let rect = self.host.get_bounding_client_rect();
        if w == 0.0 || h == 0.0 {
            return false;
        }
        let dpr = dom::window().device_pixel_ratio();
        let dpr = if dpr == 0.0 { 1.0 } else { dpr };
        self.scale = dpr.min(2.0) * rect.width() / w * self.scene.resolution();
        self.canvas.set_width((w * self.scale).round() as u32);
        self.canvas.set_height((h * self.scale).round() as u32);
        if w != self.w || h != self.h {
            self.w = w;
            self.h = h;
            self.scene.build(&self.ctx, w, h);
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
        let ctx = Ctx(canvas.get_context("2d").ok().flatten().expect("canvas 2d").unchecked_into());
        let reduce =
            dom::window().match_media("(prefers-reduced-motion: reduce)").ok().flatten().is_some_and(|m| m.matches());
        let t = scene.start();
        Animated(Rc::new(RefCell::new(Backdrop {
            scene,
            host: host.clone(),
            canvas,
            ctx,
            reduce,
            w: 0.0,
            h: 0.0,
            scale: 1.0,
            t,
            last: 0.0,
            raf: 0,
            rate: 0.0,
            target: 0.0,
            dim: 0.0,
            dim_to: 0.0,
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
