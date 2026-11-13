//! The animated backdrop of the template themes, drawn natively (`glitu_backdrop` on `skia`), as
//! glitu-web drives it: the same canvas size, the same scene resolution, the same slowing down
//! and dimming from one slide to the next.

use glitu_backdrop::crystal::{Crystal, Options};
use glitu_backdrop::{Canvas, DIMMING, EASING, Motion, Scene};

use crate::deck::{BackdropInfo, Image, MAX_AREA, MAX_SIDE};
use crate::resample::resample;
use crate::skia::Skia;

enum Kind {
    Crystal(Box<Crystal>),
    Scene(Box<dyn Scene>),
}

pub struct Backdrop {
    kind: Kind,
    /// Size of the slide, px
    size: (f64, f64),
    /// Size of the image, pixels of the screen
    out: (u32, u32),
    /// Pixels of the canvas per px of the slide (crystal: the screen's; a scene: its resolution)
    scale: f64,
    canvas: Skia,
    /// Speed of the animation (0: still, 1: normal), which tends towards `target`
    #[allow(dead_code)] // the animated backdrop, not played yet (`advance`)
    rate: f64,
    target: f64,
    dim: f64,
    dim_to: f64,
    /// Instant of a scene
    t: f64,
}

impl Backdrop {
    /// The backdrop of the deck, for images of `out` pixels (the backdrop's rectangle in the
    /// window); at its first instant, still. A size that no canvas can have is refused
    pub fn new(info: &BackdropInfo, out: (u32, u32)) -> Result<Backdrop, String> {
        let (w, h) = info.size;
        let screen = f64::from(out.0) / w;
        let (kind, scale, t) = match glitu_backdrop::scene(&info.scene) {
            Some(scene) => {
                let (r, t) = (scene.resolution(), scene.start());
                (Kind::Scene(scene), screen * r, t)
            }
            None => (Kind::Crystal(Box::new(Crystal::new(Options::default(), info.seed))), screen, 0.0),
        };
        let (cw, ch) = ((w * scale + 0.5).floor(), (h * scale + 0.5).floor());
        let side = 1.0..=MAX_SIDE as f64;
        if !(side.contains(&cw)
            && side.contains(&ch)
            && cw * ch <= MAX_AREA as f64
            && side.contains(&w)
            && side.contains(&h))
        {
            return Err(format!("the animated backdrop of {w} × {h} px would be a canvas of {cw} × {ch} pixels"));
        }
        let canvas = Skia::new(cw as u32, ch as u32);
        let mut b =
            Backdrop { kind, size: (w, h), out, scale, canvas, rate: 0.0, target: 0.0, dim: 0.0, dim_to: 0.0, t };
        match &mut b.kind {
            Kind::Crystal(c) => c.build(w, h),
            Kind::Scene(s) => s.build(&b.canvas, w, h),
        }
        Ok(b)
    }

    /// Frames per second when it runs
    #[allow(dead_code)] // the animated backdrop, not played yet
    pub fn fps(&self) -> f64 {
        match &self.kind {
            Kind::Crystal(c) => c.o.fps,
            Kind::Scene(s) => s.fps(),
        }
    }

    /// The pace and the dimming of a slide; `at_once`: without gliding (the first slide)
    pub fn set(&mut self, motion: Motion, dim: f64, at_once: bool) {
        self.target = if matches!(motion, Motion::Animated { .. }) { 1.0 } else { 0.0 };
        self.dim_to = dim;
        if at_once {
            self.dim = dim;
        }
        if let Kind::Crystal(c) = &mut self.kind {
            c.set_chase(matches!(motion, Motion::Animated { chase: true }));
        }
    }

    /// Does it still move (running, slowing down, or dimming)?
    #[allow(dead_code)] // the animated backdrop, not played yet
    pub fn moving(&self) -> bool {
        !(self.target == 0.0 && self.rate < 0.01 && (self.dim - self.dim_to).abs() < 0.005)
    }

    /// Advances by `dt` seconds (at most 0.1, as the page)
    #[allow(dead_code)] // the animated backdrop, not played yet
    pub fn advance(&mut self, dt: f64) {
        let dt = dt.min(0.1);
        self.rate += (self.target - self.rate) * (dt * EASING).min(1.0);
        self.dim += (self.dim_to - self.dim) * (dt * DIMMING).min(1.0);
        if !self.moving() {
            self.rate = 0.0;
            self.dim = self.dim_to;
            return;
        }
        match &mut self.kind {
            Kind::Crystal(c) => c.step(dt * self.rate),
            Kind::Scene(_) => self.t += dt * self.rate,
        }
    }

    /// The image now, at the size asked
    pub fn render(&self) -> Image {
        self.render_at(self.t, self.dim)
    }

    /// The image that the browser drew under the click's screenshot: the first instant, the
    /// click's dimming (the export keeps the backdrop still)
    pub fn reference(info: &BackdropInfo, out: (u32, u32), dim: f64) -> Result<Image, String> {
        let mut b = Backdrop::new(info, out)?;
        b.set(Motion::Still, dim, true);
        Ok(b.render())
    }

    fn render_at(&self, t: f64, dim: f64) -> Image {
        let c = &self.canvas;
        c.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        c.clear_rect(0.0, 0.0, f64::from(c.width()), f64::from(c.height()));
        match &self.kind {
            Kind::Crystal(m) => m.draw(c, self.scale),
            Kind::Scene(s) => {
                c.set_transform(self.scale, 0.0, 0.0, self.scale, 0.0, 0.0);
                c.set_global_composite_operation("source-over");
                c.set_global_alpha(1.0);
                s.draw(c, self.size.0, self.size.1, t, dim);
            }
        }
        let img = Image { width: c.width(), height: c.height(), pixels: c.pixels() };
        // the canvas stretched over the backdrop's rectangle, as the page's CSS does
        if (img.width, img.height) == self.out {
            img
        } else {
            let k = self.out.0 as f32 / img.width as f32;
            resample(img, self.out, k)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backdrop_of_an_impossible_size_is_refused() {
        let info = |scene: &str, size: (f64, f64)| BackdropInfo {
            scene: scene.into(),
            seed: 1,
            rect: (0.0, 0.0, 100.0, 50.0),
            size,
        };
        for scene in ["crystal", "aurora"] {
            assert!(Backdrop::new(&info(scene, (200.0, 100.0)), (100, 50)).is_ok());
            for size in [(0.0, 100.0), (200.0, 0.0), (f64::NAN, 100.0), (1.0, 1e9), (1e300, 1e300), (-5.0, 100.0)] {
                assert!(Backdrop::new(&info(scene, size), (100, 50)).is_err(), "{scene} {size:?}");
            }
            assert!(Backdrop::new(&info(scene, (200.0, 100.0)), (u32::MAX, 50)).is_err());
            assert!(Backdrop::reference(&info(scene, (0.0, 0.0)), (100, 50), 0.0).is_err());
        }
    }

    /// `cargo test --release -p glitu-play frame_time -- --ignored --nocapture`: the time of one frame
    #[test]
    #[ignore]
    fn frame_time() {
        for scene in ["crystal", "constellation", "paper", "aurora", "ascii", "blueprint", "iso"] {
            let info =
                BackdropInfo { scene: scene.into(), seed: 1, rect: (0.0, 0.0, 1024.0, 576.0), size: (1280.0, 720.0) };
            for out in [(1024, 576), (512, 288)] {
                let mut b = Backdrop::new(&info, out).unwrap();
                b.set(Motion::Animated { chase: true }, 0.0, true);
                let n = 20;
                let start = std::time::Instant::now();
                for _ in 0..n {
                    b.advance(0.033);
                    let _ = b.render();
                }
                println!("{scene:14} {out:?}: {:.1} ms", start.elapsed().as_secs_f64() * 1000.0 / n as f64);
            }
        }
    }
}
