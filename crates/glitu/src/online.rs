//! Resources taken from the network at build time, then written into the site: icons (Iconify,
//! `:mdi:github:` in the text) and fonts (Google Fonts, `font:` in the front matter). Each download
//! is kept in `~/.cache/glitu`: once a presentation has been built, it builds again offline. The
//! transfer goes through `curl`, present almost everywhere, rather than through an HTTP library
//! that would make the binary heavier.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::build::Output;

/// Announced browser: Google Fonts only serves WOFF2 to browsers that read it
const AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("glitu")
}

/// Where the content of `url` is cached
fn cache_path(url: &str) -> PathBuf {
    let hash: String = Sha256::digest(url.as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect();
    cache_dir().join(hash)
}

/// The content of `url`, from the cache if it is there
pub fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let path = cache_path(url);
    if let Ok(bytes) = fs::read(&path) {
        return Ok(bytes);
    }
    let bytes = download(url)?;
    // a cache that cannot be written only costs a download next time
    let _ = fs::create_dir_all(cache_dir());
    let _ = fs::write(&path, &bytes);
    Ok(bytes)
}

/// The content of `url`, without cache
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", "-A", AGENT, url])
        .output()
        .map_err(|e| format!("curl: {e} (curl is needed to download {url})"))?;
    if !out.status.success() {
        return Err(format!("{url}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

/// Replaces the compiler's `<i class="icon" data-icon="mdi:github"></i>` with the icon's SVG
pub fn icons(html: &str) -> Result<String, String> {
    const MARK: &str = "<i class=\"icon\" data-icon=\"";
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    while let Some(at) = rest.find(MARK) {
        let start = at + MARK.len();
        let Some(len) = rest[start..].find("\"></i>") else { break };
        let id = &rest[start..start + len];
        if !seen.contains_key(id) {
            let (set, name) = id.split_once(':').unwrap_or((id, ""));
            let svg = fetch(&format!("https://api.iconify.design/{set}/{name}.svg?height=1em"))
                .map_err(|e| format!("icon {id} not found ({e})"))?;
            let svg = String::from_utf8_lossy(&svg).replacen("<svg ", "<svg class=\"icon\" aria-hidden=\"true\" ", 1);
            seen.insert(id.to_string(), svg);
        }
        out += &rest[..at];
        out += &seen[id];
        rest = &rest[start + len + 6..];
    }
    Ok(out + rest)
}

/// Stylesheet of the front matter's fonts, outside `glitu/`: it is not published with the engine
pub const FONTS_CSS: &str = "fonts/fonts.css";

/// The front matter's fonts: `font` (the text), `font-title` (the headings), `font-mono` (the
/// code), taken from Google Fonts. Returns the `FONTS_CSS` stylesheet and its files, or nothing.
pub fn fonts(meta: &BTreeMap<String, String>) -> Result<Vec<Output>, String> {
    let roles = [
        ("font", ".reveal", "--font"),
        ("font-title", ".reveal :is(h1, h2, h3, h4)", "--font-title"),
        ("font-mono", ".reveal :is(code, pre, kbd, samp)", "--mono"),
    ];
    let mut css = String::from("/* Fonts chosen in the presentation's front matter (Google Fonts) */\n");
    let mut files = Vec::new();
    for (key, selector, var) in roles {
        let Some(family) = meta.get(key).map(|f| f.trim().trim_matches(['"', '\''])).filter(|f| !f.is_empty()) else {
            continue;
        };
        let query = family.replace(' ', "+");
        let sheet = fetch(&format!("https://fonts.googleapis.com/css2?family={query}:wght@400;700&display=swap"))
            .or_else(|_| fetch(&format!("https://fonts.googleapis.com/css2?family={query}&display=swap")))
            .map_err(|e| format!("font « {family} » not found on Google Fonts ({e})"))?;
        css += &local_faces(&String::from_utf8_lossy(&sheet), family, &mut files)?;
        css += &format!("{selector} {{ {var}: \"{family}\"; font-family: \"{family}\", sans-serif !important; }}\n");
    }
    if files.is_empty() {
        return Ok(Vec::new());
    }
    files.push(Output { path: FONTS_CSS.into(), bytes: css.into_bytes() });
    Ok(files)
}

/// The Latin `@font-face` rules of Google's stylesheet, their files downloaded next to it
fn local_faces(sheet: &str, family: &str, files: &mut Vec<Output>) -> Result<String, String> {
    let mut css = String::new();
    // Each block is preceded by a comment that names its script: /* latin */
    for block in sheet.split("/* ").skip(1) {
        let (subset, face) = block.split_once(" */").unwrap_or(("", block));
        if subset != "latin" && subset != "latin-ext" {
            continue;
        }
        let Some(start) = face.find("url(") else { continue };
        let end = start + face[start..].find(')').ok_or("malformed Google Fonts stylesheet")?;
        let url = &face[start + 4..end];
        let name = format!("{}-{}.woff2", family.to_lowercase().replace(' ', "-"), files.len() + 1);
        files.push(Output { path: format!("fonts/{name}"), bytes: fetch(url)? });
        css += &format!("{}url({name}){}", &face[..start], &face[end + 1..]);
    }
    Ok(css)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Puts `bytes` in the cache for `url`, as if it had been downloaded
    fn cached(url: &str, bytes: &[u8]) {
        fs::create_dir_all(cache_dir()).unwrap();
        fs::write(cache_path(url), bytes).unwrap();
    }

    #[test]
    fn only_latin_faces_are_kept_and_relinked() {
        let sheet = "/* cyrillic */\n@font-face { src: url(https://x/c.woff2) format('woff2'); }\n\
                     /* latin */\n@font-face { font-family: 'Inter'; src: url(https://x/l.woff2) format('woff2'); }\n";
        cached("https://x/l.woff2", b"woff2");
        let mut files = Vec::new();
        let css = local_faces(sheet, "Inter", &mut files).unwrap();
        assert_eq!(css, "\n@font-face { font-family: 'Inter'; src: url(inter-1.woff2) format('woff2'); }\n");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "fonts/inter-1.woff2");
    }

    #[test]
    fn icons_come_from_the_cache() {
        cached("https://api.iconify.design/glitu-test/dot.svg?height=1em", b"<svg viewBox=\"0 0 1 1\"></svg>");
        let html = icons("a <i class=\"icon\" data-icon=\"glitu-test:dot\"></i> b <i class=\"icon\" data-icon=\"glitu-test:dot\"></i>")
            .unwrap();
        assert_eq!(html.matches("<svg class=\"icon\" aria-hidden=\"true\" viewBox").count(), 2);
        assert!(html.starts_with("a <svg") && html.contains("</svg> b <svg"));
        assert!(fonts(&BTreeMap::new()).unwrap().is_empty());
    }
}
