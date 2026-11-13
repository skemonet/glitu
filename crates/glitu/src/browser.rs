//! Headless Chromium driven through the DevTools protocol: what the oracle (screenshots to compare)
//! and the export (PowerPoint, PDF, PNG) need. Reproducible rendering: `prefers-reduced-motion`
//! cuts the animations, `Math.random` is replayed identically, and a screenshot is taken when the
//! screen no longer moves.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
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
    /// The browser's profile: private and removed with the browser
    _profile: crate::engines::run::Scratch,
    /// Events received while waiting for a response, kept during a recording (`film`)
    events: Option<Vec<Value>>,
}

impl Browser {
    /// The browser `bin` (see `chromium`), headless, a window of `width` × `height` CSS px,
    /// `scale` screen pixels per px
    pub fn launch(bin: &Path, port: u16, width: u32, height: u32, scale: f64) -> Result<Self, String> {
        // a folder of its own (0700, a name nobody can foresee): in a temporary folder shared with
        // other users, none of them prepares the profile nor reads it
        let scratch = crate::engines::run::Scratch::new()?;
        let profile = scratch.0.clone();
        // what the browser writes goes to a file (a pipe nobody reads would block it), read if it fails
        let log = profile.join("glitu-stderr.log");
        let stderr = fs::File::create(&log).map(Stdio::from).unwrap_or_else(|_| Stdio::null());
        let mut child = Command::new(bin)
            .args(["--headless", "--no-sandbox", "--disable-gpu", "--hide-scrollbars"])
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .map_err(|e| format!("{}: {e} (set CHROMIUM to use another browser)", bin.display()))?;
        let url = match debugger_url(port, &mut child) {
            Ok(url) => url,
            Err(exit) => {
                let _ = child.kill();
                let _ = child.wait();
                let said = fs::read_to_string(&log).unwrap_or_default();
                return Err(crate::chromium::launch_failure(bin, exit.map(|s| s.to_string()), &said));
            }
        };
        let (ws, _) = tungstenite::connect(url.as_str()).map_err(|e| e.to_string())?;
        let mut b = Browser { child, ws, id: 0, _profile: scratch, events: None };
        b.send("Page.enable", json!({}))?;
        b.send(
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": width, "height": height, "deviceScaleFactor": scale, "mobile": false }),
        )?;
        b.reduce_motion(true)?;
        b.send("Page.addScriptToEvaluateOnNewDocument", json!({ "source": SEEDED_RANDOM }))?;
        Ok(b)
    }

    /// `prefers-reduced-motion: reduce`, for reproducible pictures; off while an animation is
    /// recorded (the backdrops read it once, at the start: they stay still)
    pub fn reduce_motion(&mut self, on: bool) -> Result<(), String> {
        let value = if on { "reduce" } else { "no-preference" };
        self.send(
            "Emulation.setEmulatedMedia",
            json!({ "features": [{ "name": "prefers-reduced-motion", "value": value }] }),
        )
        .map(drop)
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
            if let (Some(events), true) = (&mut self.events, v.get("method").is_some()) {
                events.push(v);
                continue;
            }
            if v["id"].as_u64() == Some(id) {
                if let Some(err) = v.get("error") {
                    return Err(format!("{method}: {err}"));
                }
                return Ok(v["result"].clone());
            }
        }
    }

    pub fn eval(&mut self, expr: &str) -> Result<Value, String> {
        let r =
            self.send("Runtime.evaluate", json!({ "expression": expr, "awaitPromise": true, "returnByValue": true }))?;
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

    /// Starts recording the screen (`Page.startScreencast`), in images of `width` × `height`
    pub fn film_start(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.events = Some(Vec::new());
        let params = json!({ "format": "png", "maxWidth": width, "maxHeight": height, "everyNthFrame": 1 });
        self.send("Page.startScreencast", params).map(drop)
    }

    /// The images recorded since `film_start`, (time in s since the epoch, PNG), until a terminal
    /// has played and stopped (`data-playing`), or nothing played within `wait`
    pub fn film(&mut self, wait: Duration, limit: Duration) -> Result<Vec<(f64, Vec<u8>)>, String> {
        let start = std::time::Instant::now();
        let (mut frames, mut played, mut end) = (Vec::new(), false, None);
        loop {
            let playing = self.eval("!!document.querySelector('[data-playing]')")?.as_bool().unwrap_or(false);
            played |= playing;
            for ev in self.events.as_mut().map(std::mem::take).unwrap_or_default() {
                if ev["method"] != "Page.screencastFrame" {
                    continue;
                }
                let p = &ev["params"];
                self.send("Page.screencastFrameAck", json!({ "sessionId": p["sessionId"] }))?;
                let png = base64::engine::general_purpose::STANDARD.decode(p["data"].as_str().unwrap_or(""));
                if let (Some(t), Ok(png)) = (p["metadata"]["timestamp"].as_f64(), png) {
                    frames.push((t, png));
                }
            }
            let waited = start.elapsed();
            if (played && !playing) || (!played && waited > wait) || waited > limit {
                // the last images of the screen, still on their way
                match end {
                    Some(at) if waited > at => break,
                    Some(_) => {}
                    None => end = Some(waited + Duration::from_millis(250)),
                }
            }
            sleep(Duration::from_millis(15));
        }
        let stop = self.send("Page.stopScreencast", json!({}));
        self.events = None;
        stop?;
        Ok(frames)
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
    }
}

/// The DevTools address of the browser's page, once it answers (10 s at most); otherwise its exit
/// status if it stopped with an error, `None` if it did not answer in time
fn debugger_url(port: u16, child: &mut Child) -> Result<String, Option<std::process::ExitStatus>> {
    for _ in 0..50 {
        sleep(Duration::from_millis(200));
        if let Ok(Some(status)) = child.try_wait()
            && !status.success()
        {
            return Err(Some(status));
        }
        let Ok(list) = http_get(port, "/json") else { continue };
        let v: Value = serde_json::from_str(&list).unwrap_or(Value::Null);
        let page = v.as_array().and_then(|a| a.iter().find(|t| t["type"] == "page"));
        if let Some(url) = page.and_then(|t| t["webSocketDebuggerUrl"].as_str()) {
            return Ok(url.to_string());
        }
    }
    Err(None)
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
