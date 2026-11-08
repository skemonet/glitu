//! "crystal" backdrop in the page: the mesh and its drawing are `glitu_backdrop::crystal`'s; here,
//! its canvas, its size and its cadence.

use std::cell::RefCell;
use std::rc::Rc;

use glitu_backdrop::crystal::Crystal as Mesh;
pub use glitu_backdrop::crystal::Options;
use glitu_backdrop::math::js_round;
pub use glitu_backdrop::{EASING, Motion};
use js_sys::Math;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlCanvasElement};

use crate::dom;
use crate::themes::canvas::Ctx;

struct Backdrop {
    mesh: Mesh,
    host: Element,
    canvas: HtmlCanvasElement,
    ctx: Ctx,
    reduce: bool,
    scale: f64,
    last: f64,
    raf: i32,
    /// Speed of the animation (0: still, 1: normal), which tends towards `target`
    rate: f64,
    target: f64,
}

impl Backdrop {
    fn draw(&self) {
        self.mesh.draw(&self.ctx, self.scale);
    }

    // Mesh in slide px, image at the real resolution of the screen (slide scaled)
    fn resize(&mut self) -> bool {
        let host = dom::html(&self.host);
        let (w, h) = (host.offset_width() as f64, host.offset_height() as f64);
        let rect = self.host.get_bounding_client_rect();
        if w == 0.0 || h == 0.0 {
            return false;
        }
        let dpr = dom::window().device_pixel_ratio();
        let dpr = if dpr == 0.0 { 1.0 } else { dpr };
        self.scale = dpr.min(2.0) * rect.width() / w;
        self.canvas.set_width(js_round(w * self.scale) as u32);
        self.canvas.set_height(js_round(h * self.scale) as u32);
        if (w, h) != self.mesh.size() {
            self.mesh.build(w, h);
        }
        true
    }
}

#[derive(Clone)]
pub struct Crystal(Rc<RefCell<Backdrop>>);

impl Crystal {
    pub fn new(host: &Element, o: Options) -> Crystal {
        let canvas: HtmlCanvasElement = dom::create("canvas").unchecked_into();
        canvas.set_class_name("backdrop-canvas");
        let _ = canvas.set_attribute("aria-hidden", "true");
        let _ = host.prepend_with_node_1(&canvas);
        let ctx = Ctx(canvas.get_context("2d").ok().flatten().expect("canvas 2d").unchecked_into());
        let reduce =
            dom::window().match_media("(prefers-reduced-motion: reduce)").ok().flatten().is_some_and(|m| m.matches());
        let seed = (Math::random() * 1e9) as i32;
        // for the export (glitu-play draws the same crystal): its seed
        let _ = host.set_attribute("data-seed", &seed.to_string());
        Crystal(Rc::new(RefCell::new(Backdrop {
            mesh: Mesh::new(o, seed),
            host: host.clone(),
            canvas,
            ctx,
            reduce,
            scale: 1.0,
            last: 0.0,
            raf: 0,
            rate: 0.0,
            target: 0.0,
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
        f.mesh.set_chase(matches!(motion, Motion::Animated { chase: true }));
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
        if now - f.last < 1000.0 / f.mesh.o.fps {
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
        f.mesh.step(dt * rate);
        f.last = now;
        f.draw();
        f.raf = dom::raf(move |t| me.frame(t));
    }
}
