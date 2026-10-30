//! Headless Chromium driven through the DevTools protocol: what the oracle (screenshots to compare)
//! and the export (PowerPoint, PDF, PNG) need. Reproducible rendering: `prefers-reduced-motion`
//! cuts the animations, `Math.random` is replayed identically, and a screenshot is taken when the
//! screen no longer moves.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use base64::Engine;
use serde_json::{Value, json};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

/// A fixed sequence for `Math.random`: the animated backdrop draws from it, identically each time
const SEEDED_RANDOM: &str =
    "{ let s = 42; Math.random = () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }; }";

pub struct Browser {
    child: Child,
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    id: u64,
    profile: PathBuf,
}

impl Browser {
    /// The browser `bin` (see `chromium`), headless, a window of `width` × `height` CSS px,
    /// `scale` screen pixels per px
    pub fn launch(bin: &Path, port: u16, width: u32, height: u32, scale: f64) -> Result<Self, String> {
        let profile = std::env::temp_dir().join(format!("glitu-browser-{}-{port}", std::process::id()));
        let child = Command::new(bin)
            .args(["--headless", "--no-sandbox", "--disable-gpu", "--hide-scrollbars"])
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("{}: {e} (set CHROMIUM to use another browser)", bin.display()))?;
        let url = debugger_url(port).ok_or("the browser does not answer")?;
        let (ws, _) = tungstenite::connect(url.as_str()).map_err(|e| e.to_string())?;
        let mut b = Browser { child, ws, id: 0, profile };
        b.send("Page.enable", json!({}))?;
        b.send(
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": width, "height": height, "deviceScaleFactor": scale, "mobile": false }),
        )?;
        b.send(
            "Emulation.setEmulatedMedia",
            json!({ "features": [{ "name": "prefers-reduced-motion", "value": "reduce" }] }),
        )?;
        b.send("Page.addScriptToEvaluateOnNewDocument", json!({ "source": SEEDED_RANDOM }))?;
        Ok(b)
    }

    pub fn send(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.id += 1;
        let id = self.id;
        self.ws
            .send(Message::text(json!({ "id": id, "method": method, "params": params }).to_string()))
            .map_err(|e| e.to_string())?;
        loop {
            let msg = self.ws.read().map_err(|e| e.to_string())?;
            let Message::Text(t) = msg else { continue };
            let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
            if v["id"].as_u64() == Some(id) {
                if let Some(err) = v.get("error") {
                    return Err(format!("{method}: {err}"));
                }
                return Ok(v["result"].clone());
            }
        }
    }

    pub fn eval(&mut self, expr: &str) -> Result<Value, String> {
        let r = self.send("Runtime.evaluate", json!({ "expression": expr, "awaitPromise": true, "returnByValue": true }))?;
        Ok(r["result"]["value"].clone())
    }

    fn shot(&mut self) -> Result<Vec<u8>, String> {
        self.screenshot(json!({ "format": "png" }))
    }

    /// `Page.captureScreenshot` with these parameters (`clip`…) → PNG
    pub fn screenshot(&mut self, params: Value) -> Result<Vec<u8>, String> {
        let r = self.send("Page.captureScreenshot", params)?;
        base64::engine::general_purpose::STANDARD.decode(r["data"].as_str().unwrap_or("")).map_err(|e| e.to_string())
    }

    /// A screenshot as soon as two successive images are identical, and no terminal is playing
    /// (its typing pauses: the screen then seems still)
    pub fn stable_shot(&mut self) -> Result<Vec<u8>, String> {
        let mut prev = self.shot()?;
        for _ in 0..40 {
            sleep(Duration::from_millis(400));
            let cur = self.shot()?;
            let playing = self.eval("!!document.querySelector('[data-playing]')")?.as_bool().unwrap_or(false);
            if cur == prev && !playing {
                return Ok(cur);
            }
            prev = cur;
        }
        Ok(prev)
    }

    pub fn key(&mut self, key: &str, code: u32) -> Result<(), String> {
        for kind in ["keyDown", "keyUp"] {
            self.send(
                "Input.dispatchKeyEvent",
                json!({ "type": kind, "key": key, "code": key, "windowsVirtualKeyCode": code }),
            )?;
        }
        Ok(())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.profile);
    }
}

/// The DevTools address of the browser's page, once it answers (10 s at most)
fn debugger_url(port: u16) -> Option<String> {
    for _ in 0..50 {
        sleep(Duration::from_millis(200));
        let Ok(list) = http_get(port, "/json") else { continue };
        let v: Value = serde_json::from_str(&list).unwrap_or(Value::Null);
        let page = v.as_array().and_then(|a| a.iter().find(|t| t["type"] == "page"));
        if let Some(url) = page.and_then(|t| t["webSocketDebuggerUrl"].as_str()) {
            return Some(url.to_string());
        }
    }
    None
}

fn http_get(port: u16, path: &str) -> std::io::Result<String> {
    let mut s = TcpStream::connect(("127.0.0.1", port))?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")?;
    let mut r = BufReader::new(s);
    let mut len = 0;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// `#/h/v/f` → (h, v, f); f is -1 before the first step
pub fn position(hash: &str) -> (u32, u32, i64) {
    let mut parts = hash.trim_start_matches('#').trim_start_matches('/').split('/').map(|p| p.parse::<i64>().ok());
    let mut next = || parts.next().flatten();
    let (h, v, f) = (next().unwrap_or(0), next().unwrap_or(0), next().unwrap_or(-1));
    (h as u32, v as u32, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_come_from_the_address() {
        assert_eq!(position("#/3/1/2"), (3, 1, 2));
        assert_eq!(position("#/3"), (3, 0, -1));
        assert_eq!(position(""), (0, 0, -1));
    }
}
