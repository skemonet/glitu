//! `mermaid.chromium`: Mermaid diagrams rendered by mermaid.js in the headless browser glitu drives
//! for the exports, without Node (SEC-13).
//!
//! mermaid.js is pinned: this version, this SHA-256, downloaded once over HTTPS into the user's
//! cache and checked each time it is used; never an address from the presentation. The page that
//! runs it holds the script and the diagram as data (JSON, never evaluated as code), under a
//! Content Security Policy that forbids any other script and any network request; mermaid runs with
//! `securityLevel: strict` and without HTML labels.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::browser::Browser;

pub const VERSION: &str = "12.2.0";
const SHA256: &str = "a8a6a6f805b074a8261a2825c86f99c8054acb3b35f50bb7e24a4f9f2c3c3b9f";
const URL: &str = "https://cdn.jsdelivr.net/npm/mermaid@12.2.0/dist/mermaid.min.js";

/// Where the pinned mermaid.js is kept
pub fn cached() -> PathBuf {
    crate::online::cache_dir().join(format!("mermaid-{VERSION}.min.js"))
}

/// mermaid.js, checked; downloaded if it is not in the cache yet
fn script() -> Result<String, String> {
    let path = cached();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Downloading mermaid.js {VERSION} (5 MB, once) into {}", path.display());
            let out = std::process::Command::new("curl")
                .args(["-fsSL", "--proto", "=https", "--proto-redir", "=https", "--max-time", "120", URL])
                .output()
                .map_err(|e| format!("curl: {e} (curl is needed to download mermaid.js)"))?;
            if !out.status.success() {
                return Err(format!("{URL}: {}", String::from_utf8_lossy(&out.stderr).trim()));
            }
            out.stdout
        }
    };
    let digest: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if digest != SHA256 {
        let _ = std::fs::remove_file(&path);
        return Err(format!("mermaid.js {VERSION}: wrong SHA-256 ({digest}); removed, it will be downloaded again"));
    }
    if !path.is_file() {
        let _ = std::fs::create_dir_all(crate::online::cache_dir());
        let _ = std::fs::write(&path, &bytes);
    }
    String::from_utf8(bytes).map_err(|_| "mermaid.js: not UTF-8".to_string())
}

/// A browser kept for the diagrams of a build
pub struct Session {
    browser: Browser,
    /// The page with mermaid.js, loaded once
    _page: super::run::Scratch,
}

impl Session {
    pub fn start(bin: &Path) -> Result<Session, String> {
        let script = script()?;
        let page = super::run::Scratch::new()?;
        let html = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\">\
             <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'unsafe-inline'; \
             style-src 'unsafe-inline'; img-src data:; font-src data:\">\
             <script>{}</script></head><body></body></html>",
            script.replace("</script", "<\\/script")
        );
        let file = page.put("mermaid.html", html.as_bytes())?;
        let port = std::net::TcpListener::bind(("127.0.0.1", 0)).and_then(|l| l.local_addr()).map(|a| a.port());
        let mut browser = Browser::launch(bin, port.map_err(|e| e.to_string())?, 1200, 800, 1.0)?;
        browser.send("Page.navigate", json!({ "url": format!("file://{}", file.display()) }))?;
        for _ in 0..200 {
            if browser.eval("typeof mermaid !== 'undefined'")?.as_bool() == Some(true) {
                return Ok(Session { browser, _page: page });
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        Err("mermaid.js did not load in the browser".into())
    }

    /// The SVG of a diagram
    pub fn render(&mut self, source: &str, values: &BTreeMap<String, String>) -> Result<Vec<u8>, String> {
        let theme = values.get("theme").map_or("default", String::as_str);
        let data = json!({ "source": source, "theme": theme }).to_string();
        // the diagram is data: a JSON value in the expression, never code
        let expr = format!(
            "(async (d) => {{
                mermaid.initialize({{ startOnLoad: false, securityLevel: 'strict', htmlLabels: false,
                    flowchart: {{ htmlLabels: false }}, theme: d.theme, deterministicIds: true }});
                try {{ const r = await mermaid.render('mmd', d.source); return {{ svg: r.svg }}; }}
                catch (e) {{ document.querySelectorAll('[id^=dmmd]').forEach((n) => n.remove());
                    return {{ error: String((e && e.message) || e) }}; }}
            }})({data})"
        );
        let r = self.browser.eval(&expr)?;
        match (r["svg"].as_str(), r["error"].as_str()) {
            (Some(svg), _) => Ok(svg.as_bytes().to_vec()),
            (_, Some(e)) => {
                Err(format!("```mermaid (mermaid.chromium):\n  {}", super::clean(e, 4096).replace('\n', "\n  ")))
            }
            _ => Err("mermaid.js gave no answer".into()),
        }
    }
}
