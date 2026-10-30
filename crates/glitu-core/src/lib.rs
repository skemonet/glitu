//! glitu-core: the presentation compiler, markdown → HTML sections that the engine animates.
//!
//! Anything that comes from elsewhere (skemo diagrams, terminal recordings) goes through
//! [`Assets`].
//!
//! Two writing conventions are recognised automatically: glitu's own (`#` chapters, `##` slides,
//! `@n` click markers) and reveal-md's (slides separated by `---`). Their default themes are
//! `crystal` and `black` respectively.

pub mod highlight;
pub mod html;
pub mod markdown;
pub mod math;
pub mod render;
pub mod reveal_md;
pub mod skemo;

use std::collections::BTreeMap;

pub use render::Assets;

/// What a presentation declares before any rendering: enough to pick its engine and its theme
pub struct Header {
    pub meta: BTreeMap<String, String>,
    /// Written with reveal-md's conventions
    pub reveal_md: bool,
    /// Expected engine version (`glitu: 0.1.0`); none: the latest
    pub glitu: Option<String>,
    pub theme: Theme,
    pub lang: String,
}

/// The theme of a presentation (`theme:`)
#[derive(Debug, Clone, PartialEq)]
pub enum Theme {
    /// Official theme, `name` or `name:version`; without a version: the latest published one
    Official { name: String, version: Option<String> },
    /// Third-party theme: the address of its folder (`https://…/`, or a path relative to the presentation)
    At(String),
}

impl Theme {
    pub fn parse(s: &str) -> Theme {
        let s = s.trim();
        if s.contains("://") || s.starts_with('.') || s.starts_with('/') {
            return Theme::At(s.to_string());
        }
        match s.split_once(':') {
            Some((name, version)) => Theme::Official { name: name.trim().into(), version: latest_is_none(version) },
            None => Theme::Official { name: s.into(), version: None },
        }
    }

    /// The name the engine knows: the official theme's, otherwise the last segment of the address
    /// (a third-party theme may give its own in its `glitu.json`)
    pub fn name(&self) -> &str {
        match self {
            Theme::Official { name, .. } => name,
            Theme::At(at) => at.trim_end_matches('/').rsplit('/').next().unwrap_or(at),
        }
    }
}

/// `latest`, or nothing, means the latest version
fn latest_is_none(version: &str) -> Option<String> {
    Some(version.trim()).filter(|v| !v.is_empty() && *v != "latest").map(String::from)
}

pub fn header(src: &str) -> Header {
    let (meta, body) = markdown::front_matter(src);
    let reveal_md = reveal_md::detect(body);
    let theme = Theme::parse(meta.get("theme").map_or(if reveal_md { "black" } else { "crystal" }, String::as_str));
    let glitu = meta.get("glitu").and_then(|v| latest_is_none(v));
    let lang = meta.get("lang").cloned().unwrap_or_else(|| "en".into());
    Header { meta, reveal_md, glitu, theme, lang }
}

/// The compiled presentation: its title (escaped for HTML) and its sections
pub struct Compiled {
    pub title: String,
    pub sections: String,
}

pub fn compile(src: &str, header: &Header, assets: &mut dyn Assets) -> Result<Compiled, String> {
    let (_, body) = markdown::front_matter(src);
    if header.reveal_md {
        let title = header.meta.get("title").map_or_else(|| reveal_md::title(body), |t| html::esc(t));
        return Ok(Compiled { title, sections: reveal_md::render(body).sections });
    }
    let r = render::render(&markdown::parse(src), assets)?;
    Ok(Compiled { title: html::esc(&r.title), sections: r.sections })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_picks_the_theme_of_its_syntax() {
        let h = header("---\ntitle: T\n---\n\n# A\n\n## B\n");
        assert_eq!((h.reveal_md, h.theme.name(), h.lang.as_str(), h.glitu), (false, "crystal", "en", None));
        let h = header("# A\n\n---\n\n# B\n");
        assert_eq!((h.reveal_md, h.theme.name(), h.lang.as_str()), (true, "black", "en"));
        assert_eq!(header("---\nlang: fr\n---\n\n# A\n").lang, "fr");
    }

    #[test]
    fn versions_are_pinned_or_left_to_the_latest() {
        let official =
            |name: &str, version: Option<&str>| Theme::Official { name: name.into(), version: version.map(String::from) };
        assert_eq!(Theme::parse("crystal"), official("crystal", None));
        assert_eq!(Theme::parse("crystal:latest"), official("crystal", None));
        assert_eq!(Theme::parse("crystal:1.2.0"), official("crystal", Some("1.2.0")));
        assert_eq!(Theme::parse("https://example.net/home/1.2.0/"), Theme::At("https://example.net/home/1.2.0/".into()));
        assert_eq!(Theme::parse("./home").name(), "home");
        assert_eq!(header("---\nglitu: 0.1.0\n---\n\n# A\n").glitu.as_deref(), Some("0.1.0"));
        assert_eq!(header("---\nglitu: latest\n---\n\n# A\n").glitu, None);
    }

    #[test]
    fn compile_follows_the_syntax_of_the_presentation() {
        struct NoAssets;
        impl Assets for NoAssets {
            fn skemo(&mut self, _: &str, _: Option<f64>) -> Result<String, String> {
                unreachable!()
            }
            fn cast(&mut self, _: &str) -> Result<String, String> {
                unreachable!()
            }
        }
        let src = "---\ntitle: A & B\n---\n\n# One\n\n## Two\n\n- item\n";
        let c = compile(src, &header(src), &mut NoAssets).unwrap();
        assert_eq!(c.title, "A &amp; B");
        assert!(c.sections.contains("<li>item</li>"));
        let src = "# First <b>\n\n---\n\nSecond\n";
        let c = compile(src, &header(src), &mut NoAssets).unwrap();
        assert_eq!(c.title, "First &lt;b&gt;");
        assert_eq!(c.sections.matches("<section").count(), 2);
    }
}
