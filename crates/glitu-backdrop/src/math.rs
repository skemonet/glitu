//! Mathematical functions and random numbers: those of JavaScript in the browser (crystal's image
//! is the original JavaScript version's, for the same seed), Rust's elsewhere.

#[cfg(target_arch = "wasm32")]
mod imp {
    use js_sys::Math;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::{JsCast, JsValue};

    #[wasm_bindgen]
    extern "C" {
        /// Math.hypot with three arguments
        #[wasm_bindgen(js_namespace = Math, js_name = hypot)]
        fn hypot3_js(a: f64, b: f64, c: f64) -> f64;
    }

    pub fn cos(x: f64) -> f64 {
        Math::cos(x)
    }
    pub fn sin(x: f64) -> f64 {
        Math::sin(x)
    }
    pub fn pow(x: f64, y: f64) -> f64 {
        Math::pow(x, y)
    }
    pub fn atan2(y: f64, x: f64) -> f64 {
        Math::atan2(y, x)
    }
    pub fn hypot(x: f64, y: f64) -> f64 {
        Math::hypot(x, y)
    }
    pub fn hypot3(a: f64, b: f64, c: f64) -> f64 {
        hypot3_js(a, b, c)
    }
    pub fn random() -> f64 {
        Math::random()
    }

    /// Number.prototype.toFixed
    pub fn fixed(x: f64, digits: u8) -> String {
        JsValue::from_f64(x)
            .unchecked_into::<js_sys::Number>()
            .to_fixed(digits)
            .map_or_else(|_| format!("{x:.*}", digits as usize), String::from)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::cell::Cell;

    pub fn cos(x: f64) -> f64 {
        x.cos()
    }
    pub fn sin(x: f64) -> f64 {
        x.sin()
    }
    pub fn pow(x: f64, y: f64) -> f64 {
        x.powf(y)
    }
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
    pub fn hypot(x: f64, y: f64) -> f64 {
        x.hypot(y)
    }
    pub fn hypot3(a: f64, b: f64, c: f64) -> f64 {
        (a * a + b * b + c * c).sqrt()
    }

    thread_local! {
        static STATE: Cell<u32> = const { Cell::new(42) };
    }

    /// The generator that `glitu export` puts in the page in place of Math.random (seed 42): a
    /// native player draws the same crystal as the pictures of the export
    pub fn random() -> f64 {
        STATE.with(|s| {
            let v = s.get().wrapping_mul(1664525).wrapping_add(1013904223);
            s.set(v);
            f64::from(v) / 4294967296.0
        })
    }

    pub fn fixed(x: f64, digits: u8) -> String {
        format!("{x:.*}", digits as usize)
    }
}

pub use imp::*;

/// JavaScript's Math.round (half towards +∞)
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_export_generator() {
        // the same numbers as the script of the export: s = (s * 1664525 + 1013904223) >>> 0
        let first = random();
        assert_eq!(first, f64::from(42u32.wrapping_mul(1664525).wrapping_add(1013904223)) / 4294967296.0);
        assert_eq!(js_round(-0.5), 0.0);
        let mut r = rng(7);
        assert!((0.0..1.0).contains(&r()));
    }
}
