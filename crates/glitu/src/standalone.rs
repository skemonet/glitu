//! Site → a single HTML page, in two ways:
//!
//! - offline: everything is written into the page (JS and wasm engine, stylesheets, fonts,
//!   images), which opens with a double click, without a network. The browser forbids a local page
//!   to read other files of the disk: that is why nothing is left next to it. The wasm, in base64,
//!   starts with `initSync` instead of being downloaded;
//! - online: the page only keeps the presentation (text, diagrams, terminals, images); the engine
//!   and the theme come from an address where what `glitu assets` writes has been published
//!   (`glitu/<version>/` and `themes/<name>/<version>/`). Each file there is checked against its
//!   fingerprint (SRI): a published file that changes, or that is tampered with, is refused by the
//!   browser instead of being run. A third-party theme, which is not published there, stays
//!   written in the page.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha384};

use crate::build::{ENGINE_BOOT as BOOT, ENGINE_JS_PATH, ENGINE_WASM_PATH, Output, engine_published};

/// Where the page takes the engine and the theme from
pub enum Mode {
    /// Everything in the page
    Offline,
    /// The engine from this address, and the theme if it is published there (`themes/<name>/<version>`)
    Online { base: String, theme: Option<String> },
}

/// Start of the module that starts the engine in the page built by `build::site`
const MODULE: &str = "<script type=\"module\">\n";
const LINK: &str = "<link rel=\"stylesheet\" href=\"";

/// The files of a built site, looked up by path, and where each one is published
struct Site<'a> {
    files: &'a [Output],
    mode: &'a Mode,
}

impl<'a> Site<'a> {
    fn file(&self, path: &str) -> Option<&'a Output> {
        self.files.iter().find(|f| f.path == path)
    }

    /// The published address of a site file: the engine in glitu/<version>/, the theme in
    /// themes/<name>/<version>/; none: it is written in the page
    fn published(&self, path: &str) -> Option<String> {
        let Mode::Online { base, theme } = self.mode else { return None };
        let base = base.trim_end_matches('/');
        if let Some(rest) = path.strip_prefix("glitu/") {
            return Some(format!("{base}/{}/{rest}", engine_published()));
        }
        Some(format!("{base}/{}/{}", theme.as_ref()?, path.strip_prefix("theme/")?))
    }

    /// Stylesheets: inline (their url() as data:), or linked to the address with their fingerprint
    fn stylesheets(&self, html: &str) -> Result<String, String> {
        let mut out = String::new();
        let mut rest = html;
        while let Some(at) = rest.find(LINK) {
            let start = at + LINK.len();
            let end = start + rest[start..].find('"').ok_or("malformed link tag")?;
            let close = end + rest[end..].find('>').ok_or("malformed link tag")? + 1;
            let href = &rest[start..end];
            let css = self.file(href).ok_or_else(|| format!("stylesheet not found: {href}"))?;
            out += &rest[..at];
            out += &match self.published(href) {
                None => format!(
                    "<style>\n{}\n</style>",
                    inline_urls(&String::from_utf8_lossy(&css.bytes), dir_of(href), &|p| self.file(p))
                        .replace("</style", "<\\/style")
                ),
                Some(url) => format!("{LINK}{url}\" integrity=\"{}\" crossorigin=\"anonymous\">", sri(&css.bytes)),
            };
            rest = &rest[close..];
        }
        Ok(out + rest)
    }

    /// The engine: written in the page, or imported from the address
    fn engine(&self) -> Result<String, String> {
        let js = self.file(ENGINE_JS_PATH).ok_or("engine glitu_web.js missing")?;
        let wasm = self.file(ENGINE_WASM_PATH).ok_or("engine glitu_web_bg.wasm missing")?;
        Ok(match self.published(ENGINE_JS_PATH).zip(self.published(ENGINE_WASM_PATH)) {
            None => format!(
                "{MODULE}{}\n  initSync({{ module: Uint8Array.from(atob('{}'), c => c.charCodeAt(0)) }});\n",
                String::from_utf8_lossy(&js.bytes).replace("</script", "<\\/script"),
                STANDARD.encode(&wasm.bytes)
            ),
            Some((js_url, wasm_url)) => format!(
                "<script type=\"importmap\">{{ \"imports\": {{ \"glitu-web\": \"{js_url}\" }}, \"integrity\": {{ \"{js_url}\": \"{}\" }} }}</script>\n\
                 {MODULE}  import init, {{ start }} from 'glitu-web';\n  \
                 await init({{ module_or_path: fetch('{wasm_url}', {{ integrity: '{}' }}) }});\n",
                sri(&js.bytes),
                sri(&wasm.bytes)
            ),
        })
    }

    /// The other files the page cites: the theme's (logo) at the address, the content as data:
    fn resources(&self, mut html: String) -> String {
        for f in self.files {
            if f.path == "index.html" || f.path.starts_with("glitu/") || mime(&f.path).is_none() {
                continue;
            }
            let target = self.published(&f.path).unwrap_or_else(|| data_uri(f));
            for q in ['"', '\''] {
                let quoted = format!("{q}{}{q}", f.path);
                if html.contains(&quoted) {
                    html = html.replace(&quoted, &format!("{q}{target}{q}"));
                }
            }
        }
        html
    }
}

pub fn bundle(files: &[Output], mode: &Mode) -> Result<String, String> {
    let site = Site { files, mode };
    let index = site.file("index.html").ok_or("index.html missing from the site")?;
    let html = String::from_utf8(index.bytes.clone()).map_err(|e| e.to_string())?;
    let html = site.stylesheets(&html)?;
    let boot = format!("{MODULE}{BOOT}");
    if !html.contains(&boot) {
        return Err("the engine loading was not found in index.html".into());
    }
    let html = html.replace(&boot, &site.engine()?);
    Ok(site.resources(html))
}

/// Integrity fingerprint (Subresource Integrity), as the browser expects it
fn sri(bytes: &[u8]) -> String {
    format!("sha384-{}", STANDARD.encode(Sha384::digest(bytes)))
}

/// `url(…)` of a stylesheet located in `dir` → data:, for the site's files
fn inline_urls<'a>(css: &str, dir: &str, file: &dyn Fn(&str) -> Option<&'a Output>) -> String {
    let mut out = String::new();
    let mut rest = css;
    while let Some(at) = rest.find("url(") {
        let start = at + 4;
        let Some(len) = rest[start..].find(')') else { break };
        let raw = rest[start..start + len].trim().trim_matches(['"', '\'']);
        let path = raw.split(['?', '#']).next().unwrap_or(raw);
        out += &rest[..start];
        match file(&join(dir, path)).filter(|_| !raw.starts_with("data:") && !raw.contains("://")) {
            Some(f) => out += &format!("\"{}\"", data_uri(f)),
            None => out += &rest[start..start + len],
        }
        out += ")";
        rest = &rest[start + len + 1..];
    }
    out + rest
}

fn dir_of(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..i])
}

/// A relative path resolved from `dir` (`../`, `./`)
fn join(dir: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for p in rel.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

fn mime(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        _ => return None,
    })
}

fn data_uri(f: &Output) -> String {
    format!("data:{};base64,{}", mime(&f.path).unwrap_or("application/octet-stream"), STANDARD.encode(&f.bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(path: &str, bytes: &[u8]) -> Output {
        Output { path: path.into(), bytes: bytes.to_vec() }
    }

    fn page() -> String {
        format!(
            "<head><link rel=\"stylesheet\" href=\"theme/t.css\"></head><body><img src=\"img/a.png\">\
             <script type=\"module\">\n{BOOT}  start({{\"logo\":\"theme/logo.svg\"}});\n</script></body>"
        )
    }

    #[test]
    fn everything_inline() {
        let site = vec![
            out("index.html", page().as_bytes()),
            out("theme/t.css", b"@font-face{src:url('fonts/f.woff2')} .x{background:url(../img/a.png)} .y{background:url(data:x)}"),
            out("theme/fonts/f.woff2", b"F"),
            out("theme/logo.svg", b"<svg/>"),
            out("img/a.png", b"P"),
            out("glitu/glitu_web.js", b"export function start(o) {}"),
            out("glitu/glitu_web_bg.wasm", b"\0asm"),
        ];
        let page = bundle(&site, &Mode::Offline).unwrap();
        assert!(!page.contains("<link"));
        assert!(page.contains("url(\"data:font/woff2;base64,Rg==\")"));
        assert!(page.contains("url(\"data:image/png;base64,UA==\")"));
        assert!(page.contains("url(data:x)"));
        assert!(page.contains("<img src=\"data:image/png;base64,UA==\">"));
        assert!(page.contains("\"logo\":\"data:image/svg+xml;base64,"));
        assert!(page.contains("export function start(o) {}\n  initSync({ module: Uint8Array.from(atob('AGFzbQ=='"));
        assert!(!page.contains("import init"));
    }

    #[test]
    fn online_links_the_engine_and_theme_with_their_fingerprints() {
        let site = vec![
            out("index.html", page().as_bytes()),
            out("theme/t.css", b"x"),
            out("theme/logo.svg", b"<svg/>"),
            out("img/a.png", b"P"),
            out("glitu/glitu_web.js", b"export function start(o) {}"),
            out("glitu/glitu_web_bg.wasm", b"\0asm"),
        ];
        let mode = Mode::Online { base: "https://glitu.net/".into(), theme: Some("themes/crystal/1.2.0".into()) };
        let page = bundle(&site, &mode).unwrap();
        let x = sri(b"x");
        let engine = format!("https://glitu.net/{}", engine_published());
        assert!(page.contains(&format!(
            "<link rel=\"stylesheet\" href=\"https://glitu.net/themes/crystal/1.2.0/t.css\" integrity=\"{x}\" crossorigin=\"anonymous\">"
        )));
        assert!(page.contains(&format!("\"glitu-web\": \"{engine}/glitu_web.js\"")));
        assert!(page.contains(&format!("fetch('{engine}/glitu_web_bg.wasm', {{ integrity: 'sha384-")));
        assert!(page.contains("\"logo\":\"https://glitu.net/themes/crystal/1.2.0/logo.svg\""));
        assert!(page.contains("<img src=\"data:image/png;base64,UA==\">"));
        assert!(!page.contains("export function"));
        // A third-party theme, not published, stays in the page; the engine still comes from the address
        let page = bundle(&site, &Mode::Online { base: "https://glitu.net".into(), theme: None }).unwrap();
        assert!(page.contains("<style>\nx\n</style>"));
        assert!(page.contains("\"logo\":\"data:image/svg+xml;base64,"));
        assert!(page.contains(&format!("\"glitu-web\": \"{engine}/glitu_web.js\"")));
        // Reference: printf x | openssl dgst -sha384 -binary | base64
        assert_eq!(x, "sha384-11LCxR+6DimqGQVwqdQlPkQHegWNMpf6OlYw1b0BJiL5fCisrtMTtcg7uZDKp9qF");
    }

    #[test]
    fn a_broken_site_is_refused() {
        assert!(bundle(&[], &Mode::Offline).unwrap_err().contains("index.html missing"));
        let site = vec![out("index.html", b"<link rel=\"stylesheet\" href=\"missing.css\">")];
        assert!(bundle(&site, &Mode::Offline).unwrap_err().contains("stylesheet not found"));
        let site = vec![out("index.html", b"<p>no engine</p>")];
        assert!(bundle(&site, &Mode::Offline).unwrap_err().contains("engine loading"));
    }

    #[test]
    fn paths() {
        assert_eq!(join("theme", "fonts/a.woff2"), "theme/fonts/a.woff2");
        assert_eq!(join("theme/css", "../img/a.png"), "theme/img/a.png");
        assert_eq!(join("", "./img/a.png"), "img/a.png");
        assert_eq!(mime("a.MP4"), Some("video/mp4"));
        assert_eq!(mime("a.txt"), None);
    }
}
