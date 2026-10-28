//! LaTeX formulas → MathML, which browsers display on their own: `$…$` in the text, `$$…$$` as
//! a block, ` ```math ` as a visual. An unknown command is written in red in its place, as KaTeX
//! does; the original LaTeX stays attached (copy and paste, screen readers).

use pulldown_latex::config::DisplayMode;
use pulldown_latex::{Parser, RenderConfig, Storage, push_mathml};

pub fn mathml(tex: &str, block: bool) -> String {
    let storage = Storage::new();
    let config = RenderConfig {
        display_mode: if block { DisplayMode::Block } else { DisplayMode::Inline },
        annotation: Some(tex),
        ..RenderConfig::default()
    };
    let mut out = String::new();
    // Writing into a String cannot fail
    let _ = push_mathml(&mut out, Parser::new(tex, &storage), config);
    out
}

/// Does the page have formulas (and therefore need their font)?
pub fn used(html: &str) -> bool {
    html.contains("<math")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latex_becomes_mathml() {
        let m = mathml(r"\frac{a}{b}", true);
        assert!(m.starts_with("<math") && m.contains("display=\"block\"") && m.contains("<mfrac>"));
        assert!(used(&m) && !used("<p>x</p>"));
        assert!(!mathml("x", false).contains("display=\"block\""));
    }
}
