//! The canvas of the page as the backdrops draw on it (`glitu_backdrop::Canvas`): each call is the
//! context's own, gradients and patterns made when they are set.

use std::any::Any;

use glitu_backdrop::{Canvas, Gradient, GradientKind, Layer};
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasGradient, CanvasRenderingContext2d, HtmlCanvasElement};

use crate::dom;

/// The 2D context of a canvas of the page
pub struct Ctx(pub CanvasRenderingContext2d);

/// A canvas outside the page
pub struct Offscreen {
    canvas: HtmlCanvasElement,
    ctx: Ctx,
}

impl Offscreen {
    pub fn new(w: u32, h: u32) -> Offscreen {
        let canvas: HtmlCanvasElement = dom::create("canvas").unchecked_into();
        canvas.set_width(w.max(1));
        canvas.set_height(h.max(1));
        let ctx = Ctx(canvas.get_context("2d").ok().flatten().expect("canvas 2d").unchecked_into());
        Offscreen { canvas, ctx }
    }
}

impl Layer for Offscreen {
    fn canvas(&self) -> &dyn Canvas {
        &self.ctx
    }
    fn width(&self) -> u32 {
        self.canvas.width()
    }
    fn height(&self) -> u32 {
        self.canvas.height()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn element(layer: &dyn Layer) -> Option<&HtmlCanvasElement> {
    layer.as_any().downcast_ref::<Offscreen>().map(|o| &o.canvas)
}

fn gradient(ctx: &CanvasRenderingContext2d, g: &Gradient) -> Option<CanvasGradient> {
    let cg = match g.kind {
        GradientKind::Linear { x0, y0, x1, y1 } => ctx.create_linear_gradient(x0, y0, x1, y1),
        GradientKind::Radial { x0, y0, r0, x1, y1, r1 } => ctx.create_radial_gradient(x0, y0, r0, x1, y1, r1).ok()?,
    };
    for (offset, color) in g.stops.borrow().iter() {
        let _ = cg.add_color_stop(*offset as f32, color);
    }
    Some(cg)
}

impl Canvas for Ctx {
    fn set_transform(&self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        let _ = self.0.set_transform(a, b, c, d, e, f);
    }
    fn set_global_composite_operation(&self, op: &str) {
        let _ = self.0.set_global_composite_operation(op);
    }
    fn set_global_alpha(&self, alpha: f64) {
        self.0.set_global_alpha(alpha);
    }
    fn set_fill_style_str(&self, color: &str) {
        self.0.set_fill_style_str(color);
    }
    fn set_stroke_style_str(&self, color: &str) {
        self.0.set_stroke_style_str(color);
    }
    fn set_fill_gradient(&self, g: &Gradient) {
        if let Some(cg) = gradient(&self.0, g) {
            self.0.set_fill_style_canvas_gradient(&cg);
        }
    }
    fn set_fill_pattern(&self, layer: &dyn Layer) {
        if let Some(el) = element(layer)
            && let Ok(Some(p)) = self.0.create_pattern_with_html_canvas_element(el, "repeat")
        {
            self.0.set_fill_style_canvas_pattern(&p);
        }
    }
    fn fill_rect(&self, x: f64, y: f64, w: f64, h: f64) {
        self.0.fill_rect(x, y, w, h);
    }
    fn stroke_rect(&self, x: f64, y: f64, w: f64, h: f64) {
        self.0.stroke_rect(x, y, w, h);
    }
    fn clear_rect(&self, x: f64, y: f64, w: f64, h: f64) {
        self.0.clear_rect(x, y, w, h);
    }
    fn begin_path(&self) {
        self.0.begin_path();
    }
    fn move_to(&self, x: f64, y: f64) {
        self.0.move_to(x, y);
    }
    fn line_to(&self, x: f64, y: f64) {
        self.0.line_to(x, y);
    }
    fn close_path(&self) {
        self.0.close_path();
    }
    fn arc(&self, x: f64, y: f64, r: f64, start: f64, end: f64) {
        let _ = self.0.arc(x, y, r, start, end);
    }
    fn fill(&self) {
        self.0.fill();
    }
    fn stroke(&self) {
        self.0.stroke();
    }
    fn save(&self) {
        self.0.save();
    }
    fn restore(&self) {
        self.0.restore();
    }
    fn set_line_width(&self, w: f64) {
        self.0.set_line_width(w);
    }
    fn set_line_join(&self, join: &str) {
        self.0.set_line_join(join);
    }
    fn set_line_cap(&self, cap: &str) {
        self.0.set_line_cap(cap);
    }
    fn set_line_dash(&self, dash: &[f64]) {
        let a: Array = dash.iter().map(|&v| JsValue::from_f64(v)).collect();
        let _ = self.0.set_line_dash(&a);
    }
    fn set_shadow_blur(&self, blur: f64) {
        self.0.set_shadow_blur(blur);
    }
    fn set_shadow_color(&self, color: &str) {
        self.0.set_shadow_color(color);
    }
    fn set_shadow_offset_y(&self, dy: f64) {
        self.0.set_shadow_offset_y(dy);
    }
    fn set_font(&self, font: &str) {
        self.0.set_font(font);
    }
    fn set_text_align(&self, align: &str) {
        self.0.set_text_align(align);
    }
    fn set_text_baseline(&self, baseline: &str) {
        self.0.set_text_baseline(baseline);
    }
    fn fill_text(&self, text: &str, x: f64, y: f64) {
        let _ = self.0.fill_text(text, x, y);
    }
    fn set_image_smoothing_enabled(&self, on: bool) {
        self.0.set_image_smoothing_enabled(on);
    }
    fn draw_layer_part(&self, layer: &dyn Layer, sx: f64, sy: f64, sw: f64, sh: f64, dx: f64, dy: f64, dw: f64, dh: f64) {
        if let Some(el) = element(layer) {
            let _ = self
                .0
                .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(el, sx, sy, sw, sh, dx, dy, dw, dh);
        }
    }
    fn draw_layer(&self, layer: &dyn Layer, dx: f64, dy: f64, dw: f64, dh: f64) {
        if let Some(el) = element(layer) {
            let _ = self.0.draw_image_with_html_canvas_element_and_dw_and_dh(el, dx, dy, dw, dh);
        }
    }
    fn layer(&self, w: u32, h: u32) -> Box<dyn Layer> {
        Box::new(Offscreen::new(w, h))
    }
}
