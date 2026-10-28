//! skemo blocks rendered by the skemo engine itself (the `skemo-engine` library) at every build:
//! no external binary, no cache. A skemo warning fails the build (like `--deny-warnings`), with
//! its line in the block.

use std::collections::BTreeMap;

use skemo_engine::{Error, Options};

/// skemo messages in English, like glitu's
pub fn speak_english() {
    skemo_lang::i18n::set_language(skemo_lang::i18n::Lang::En);
}

pub struct Skemo {
    /// The theme's skemo.css (its text)
    pub css: Option<String>,
    /// The theme's default text size factor (skemo's `--font-scale`)
    pub font_scale: Option<f64>,
}

/// Line (from 1) of a position in the source
fn line_of(src: &str, at: usize) -> usize {
    src[..at.min(src.len())].matches('\n').count() + 1
}

/// The highest text size factor a diagram accepts
const MAX_FONT_SCALE: f64 = 4.0;

impl Skemo {
    /// `font_scale`: the diagram's own factor, otherwise the theme's; `load` reads an image used
    /// as an icon (`icon="img/logo.svg"`, path relative to the presentation): skemo reads no file
    /// itself
    pub fn render(
        &self,
        source: &str,
        font_scale: Option<f64>,
        load: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    ) -> Result<String, String> {
        let scale = font_scale.or(self.font_scale);
        let mut css: Vec<String> = self.css.iter().cloned().collect();
        if let Some(k) = scale {
            if !(k.is_finite() && k > 0.0 && k <= MAX_FONT_SCALE) {
                return Err(format!("skemo: font-scale between 0 and {MAX_FONT_SCALE} expected, got {k}"));
            }
            css.push(format!(".stage {{ --font-scale: {k}; }}"));
        }
        let images = self.images(source, &css, load)?;
        // the source travels inside the SVG (like the files skemo writes, D-042): a diagram saved
        // from the page can be reopened with `skemo extract`
        let mut bundle = skemo_engine::source::Bundle::new(source);
        bundle.css = self
            .css
            .iter()
            .map(|c| skemo_engine::source::NamedText { name: "skemo.css".into(), content: c.clone() })
            .collect();
        bundle.font_scale = scale;
        bundle.images = images.iter().map(|(n, d)| skemo_engine::source::NamedBytes::new(n, d)).collect();
        let options = Options {
            css,
            theme: None,
            base_theme: None,
            high_contrast: false,
            transparent: true,
            images,
            fonts: Vec::new(),
        };
        let diag = |d: &skemo_lang::Diagnostic| {
            let help = d.help.as_deref().map(|h| format!(" ({h})")).unwrap_or_default();
            format!("line {} of the diagram: {} [{}]{help}", line_of(source, d.span.start), d.message, d.code)
        };
        match skemo_engine::from_source(source, &options) {
            Ok((d, warnings)) => {
                let all: Vec<String> = warnings.iter().map(diag).chain(d.warnings.iter().cloned()).collect();
                if !all.is_empty() {
                    return Err(format!("skemo:\n  {}", all.join("\n  ")));
                }
                Ok(skemo_engine::source::embed_text(&skemo_engine::svg::render(&d), &bundle))
            }
            Err(Error::Source(diags)) => {
                Err(format!("skemo:\n  {}", diags.iter().map(diag).collect::<Vec<_>>().join("\n  ")))
            }
            Err(Error::Internal(m)) => Err(format!("skemo (internal error): {m}")),
        }
    }

    /// The image files the diagram uses as icons, read through `load`
    fn images(
        &self,
        source: &str,
        css: &[String],
        load: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    ) -> Result<BTreeMap<String, Vec<u8>>, String> {
        let mut images = BTreeMap::new();
        for name in skemo_engine::images::referenced(&skemo_lang::parse(source).model, css) {
            skemo_engine::images::check_name(&name).map_err(|e| format!("skemo: {e}"))?;
            let data = load(&name)?;
            if data.len() > skemo_engine::images::MAX_BYTES {
                return Err(format!("skemo: image « {name} »: larger than 1 MB"));
            }
            images.insert(name, data);
        }
        Ok(images)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_files(name: &str) -> Result<Vec<u8>, String> {
        Err(format!("{name}: not found"))
    }

    #[test]
    fn a_diagram_becomes_an_svg_that_carries_its_source() {
        speak_english();
        let skemo = Skemo { css: Some(".web { --c: #2F7FD8; }".into()), font_scale: Some(1.2) };
        let svg = skemo.render("diagram\n  card#a.web A\n    to(b)\n  card#b B\n", None, &mut no_files).unwrap();
        assert!(svg.starts_with("<svg") && svg.contains("data-id=\"a\""), "{svg}");
        assert!(svg.contains("skemo-source"));
    }

    #[test]
    fn errors_name_their_line() {
        speak_english();
        let skemo = Skemo { css: None, font_scale: None };
        let err = skemo.render("diagram\n  card#a A\n    to(missing)\n", None, &mut no_files).unwrap_err();
        assert!(err.starts_with("skemo:") && err.contains("line 3 of the diagram"), "{err}");
        let err = skemo.render("diagram\n  card#a A\n", Some(9.0), &mut no_files).unwrap_err();
        assert!(err.contains("font-scale"), "{err}");
        let err = skemo.render("diagram\n  card#a(icon=\"logo.svg\") A\n", None, &mut no_files).unwrap_err();
        assert_eq!(err, "logo.svg: not found");
        assert_eq!(line_of("a\nb\nc", 3), 2);
    }
}
