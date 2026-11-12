//! Graphite: charcoal, a purple glow in the top-left corner, a thin purple line along the top. One
//! backdrop for the whole screen: no veil on the text side, which has no panel to see.

use crate::{Canvas, Scene};

pub struct Graphite;

impl Scene for Graphite {
    fn start(&self) -> f64 {
        4.0
    }

    fn fps(&self) -> f64 {
        30.0
    }

    fn draw(&self, ctx: &dyn Canvas, w: f64, h: f64, t: f64, _dim: f64) {
        let k = w / 960.0;
        let g = ctx.create_linear_gradient(0.0, 0.0, 0.0, h);
        g.add_color_stop(0.0, "#1b1c21");
        g.add_color_stop(1.0, "#121317");
        ctx.set_fill_gradient(&g);
        ctx.fill_rect(0.0, 0.0, w, h);
        // The purple glow, which breathes slowly
        let breath = 1.0 + 0.06 * (t * 0.5).sin();
        let (gx, gy) = (w * 0.06, h * 0.04);
        if let Ok(g) = ctx.create_radial_gradient(gx, gy, 0.0, gx, gy, w * 0.55 * breath) {
            g.add_color_stop(0.0, "rgba(147,64,230,0.34)");
            g.add_color_stop(0.45, "rgba(120,50,200,0.12)");
            g.add_color_stop(1.0, "rgba(100,40,170,0)");
            ctx.set_fill_gradient(&g);
            ctx.fill_rect(0.0, 0.0, w, h);
        }
        // The line along the top, purple fading to the right
        let g = ctx.create_linear_gradient(0.0, 0.0, w, 0.0);
        g.add_color_stop(0.0, "rgba(168,85,247,0.95)");
        g.add_color_stop(0.6, "rgba(168,85,247,0.55)");
        g.add_color_stop(1.0, "rgba(168,85,247,0.15)");
        ctx.set_fill_gradient(&g);
        ctx.fill_rect(0.0, 0.0, w, (3.0 * k).max(2.0));
    }
}
