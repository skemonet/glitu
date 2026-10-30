//! Preview server: rebuilds the site when slides.md, a recording or the theme changes, and reloads
//! the open pages (Server-Sent Events).
//!
//! It also keeps the open windows in sync: a page that changes position posts it
//! (`POST /__glitu/position`), and the server sends it to all the others through the same stream
//! (`position` event). Projector, laptop and phone (`?remote`) stay together.
//!
//! Rehearsals: the speaker view times them; the durations kept arrive through
//! `POST /__glitu/chrono`, are written in the markdown (see `chrono`), and go back to the page in
//! the answer: the server does not reload the pages for its own writing (a reload would reopen
//! the speaker view).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};
use std::{fs, thread};

use crate::build::{self, Output, Project};

struct State {
    site: Result<Vec<Output>, String>,
    clients: Vec<TcpStream>,
    /// The last position posted, given to the pages that ask for it when they arrive
    position: Option<String>,
    /// The markdown where the timed durations of the rehearsals are written
    slides: PathBuf,
    /// The server has just written the timings: the next change of the sources reloads no page
    own_write: bool,
}

impl State {
    /// Sends an event to every open page, forgets the closed ones
    fn broadcast(&mut self, event: &str) {
        self.clients.retain_mut(|c| c.write_all(event.as_bytes()).and_then(|_| c.flush()).is_ok());
    }
}

/// The shared state; a thread that panicked while holding it leaves it usable
fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `{"id":"…","hash":"#/3/0/1"}`: nothing else goes through (the text is sent back to the pages as is)
fn valid_position(body: &str) -> bool {
    body.len() < 200
        && body.starts_with('{')
        && body.ends_with('}')
        && body.chars().all(|c| c.is_ascii_alphanumeric() || "{}\":,#/-_ ".contains(c))
}

/// An allowed address range, in CIDR notation: `192.168.0.0/16`, `10.1.2.3`, `fd00::/8`
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    net: IpAddr,
    bits: u8,
}

impl std::str::FromStr for Range {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let invalid = || format!("invalid range: {s}");
        let (ip, bits) = s.split_once('/').unwrap_or((s, ""));
        let net: IpAddr = ip.parse().map_err(|_| invalid())?;
        let max = if net.is_ipv4() { 32 } else { 128 };
        let bits = if bits.is_empty() { max } else { bits.parse().map_err(|_| invalid())? };
        if bits > max {
            return Err(invalid());
        }
        Ok(Range { net, bits })
    }
}

/// An IPv4 address mapped into IPv6 (`::ffff:192.168.1.2`) is seen as IPv4
fn unmapped(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    }
}

impl Range {
    pub fn contains(&self, ip: IpAddr) -> bool {
        let (a, b, len) = match (self.net, unmapped(ip)) {
            (IpAddr::V4(n), IpAddr::V4(i)) => (u32::from(n) as u128, u32::from(i) as u128, 32),
            (IpAddr::V6(n), IpAddr::V6(i)) => (u128::from(n), u128::from(i), 128),
            _ => return false,
        };
        let shift = len - self.bits as u32;
        shift >= len || (a >> shift) == (b >> shift)
    }
}

/// Why the server cannot listen, as a sentence
fn listen_error(port: u16, e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::AddrInUse => format!(
            "port {port} is already in use, maybe by another glitu serve: stop it, or choose another port with -p {}",
            port.saturating_add(1)
        ),
        std::io::ErrorKind::PermissionDenied => {
            format!("port {port} needs administrator rights: choose a port above 1023 with -p 8000")
        }
        _ => format!("cannot listen on port {port} ({e})"),
    }
}

/// `allow` empty: the machine alone. Otherwise the server listens on every interface, but only
/// answers the machine and the given ranges (phone, tablet on the same network).
pub fn serve(project: Project, port: u16, allow: Vec<Range>) -> Result<(), String> {
    let host = if allow.is_empty() { "127.0.0.1" } else { "0.0.0.0" };
    let listener = TcpListener::bind((host, port)).map_err(|e| listen_error(port, &e))?;
    let slides = project.slides.clone();
    let state = Arc::new(Mutex::new(State {
        site: build::site(&project, true),
        clients: Vec::new(),
        position: None,
        slides,
        own_write: false,
    }));
    if let Err(e) = &lock(&state).site {
        eprintln!("{e}");
    }
    println!("http://localhost:{port}/");
    for ip in local_ips(&allow) {
        println!("http://{ip}:{port}/");
    }
    let watched = Arc::clone(&state);
    thread::spawn(move || watch(&project, &watched));
    for stream in listener.incoming().flatten() {
        if !stream.peer_addr().is_ok_and(|a| allowed(&allow, a.ip())) {
            continue;
        }
        let state = Arc::clone(&state);
        thread::spawn(move || {
            // a client that leaves halfway only affects its own request
            let _ = handle(stream, &state);
        });
    }
    Ok(())
}

/// Rebuilds the site whenever a source changes, and reloads the open pages
fn watch(project: &Project, state: &Mutex<State>) {
    let mut last = stamp(project);
    loop {
        thread::sleep(Duration::from_millis(250));
        let now = stamp(project);
        if now == last {
            continue;
        }
        last = now;
        let site = build::site(project, true);
        match &site {
            Ok(_) => println!("rebuilt"),
            Err(e) => eprintln!("{e}"),
        }
        let mut s = lock(state);
        s.site = site;
        // the timings written by the server: the pages already have them
        if std::mem::take(&mut s.own_write) {
            continue;
        }
        s.broadcast("data: reload\n\n");
    }
}

fn allowed(allow: &[Range], ip: IpAddr) -> bool {
    let ip = unmapped(ip);
    ip.is_loopback() || allow.iter().any(|r| r.contains(ip))
}

/// The machine's addresses in the allowed ranges: the source address it takes to reach each range
fn local_ips(allow: &[Range]) -> Vec<IpAddr> {
    let mut out = Vec::new();
    for r in allow {
        let any = if r.net.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let probe = std::net::UdpSocket::bind(any).and_then(|s| s.connect((r.net, 9)).map(|_| s));
        if let Ok(ip) = probe.and_then(|s| s.local_addr()).map(|a| a.ip())
            && r.contains(ip)
            && !ip.is_loopback()
            && !out.contains(&ip)
        {
            out.push(ip);
        }
    }
    out
}

/// The latest modification date of the sources
fn stamp(project: &Project) -> SystemTime {
    let mut latest = SystemTime::UNIX_EPOCH;
    let mut visit = |p: &Path| {
        if let Ok(t) = fs::metadata(p).and_then(|m| m.modified()) {
            latest = latest.max(t);
        }
    };
    // The markdown and the files it includes
    match crate::source::read(&project.slides) {
        Ok(src) => src.files.iter().for_each(|f| visit(&f.path)),
        Err(_) => visit(&project.slides),
    }
    // The presentation's own diagram types and styles
    visit(&project.root.join("skemo.css"));
    visit(&project.root.join("style.css"));
    let mut dirs: Vec<PathBuf> = ["themes", "casts", "img"].iter().map(|d| project.root.join(d)).collect();
    while let Some(d) = dirs.pop() {
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() { dirs.push(p) } else { visit(&p) }
        }
    }
    latest
}

/// What the server needs of an HTTP request
#[derive(Debug, PartialEq)]
struct Request {
    post: bool,
    /// Path and query, as written (`/__glitu/reload?follow`)
    target: String,
    /// Path alone
    path: String,
    length: usize,
}

/// Reads the request line and the headers
fn read_request(reader: &mut impl BufRead) -> std::io::Result<Request> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();
    let mut length = 0;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? <= 2 {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
    }
    Ok(Request { post: line.starts_with("POST"), target, path, length })
}

/// At most `max` bytes of the request body
fn read_body(reader: &mut impl Read, req: &Request, max: usize) -> std::io::Result<Vec<u8>> {
    let mut body = vec![0; req.length.min(max)];
    reader.read_exact(&mut body)?;
    Ok(body)
}

const NO_CONTENT: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";

fn handle(mut stream: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let req = read_request(&mut reader)?;
    match (req.path.as_str(), req.post) {
        ("/__glitu/reload", _) => events(stream, &req, state),
        ("/__glitu/position", true) => {
            post_position(&String::from_utf8_lossy(&read_body(&mut reader, &req, 512)?), state);
            stream.write_all(NO_CONTENT)
        }
        ("/__glitu/chrono", true) => {
            let body = read_body(&mut reader, &req, 1 << 16)?;
            let slides = {
                let mut s = lock(state);
                s.own_write = true;
                s.slides.clone()
            };
            let answer = match crate::chrono::record(&slides, &durations(&body)) {
                Ok(written) => {
                    println!("rehearsal: {} durations recorded", written.len());
                    if written.is_empty() {
                        lock(state).own_write = false;
                    }
                    let json: BTreeMap<String, [u32; 2]> =
                        written.into_iter().map(|(i, (d, r))| (i.to_string(), [d, r])).collect();
                    serde_json::to_string(&json).unwrap_or_default()
                }
                Err(e) => {
                    eprintln!("rehearsal: {e}");
                    lock(state).own_write = false;
                    serde_json::json!({ "error": e }).to_string()
                }
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            )
        }
        _ => {
            let (status, kind, body) = file(&req.path, &lock(state));
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(&body)
        }
    }
}

/// The event stream of a page: reloads and positions; it stays open
fn events(mut stream: TcpStream, req: &Request, state: &Mutex<State>) -> std::io::Result<()> {
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\r\n")?;
    let mut s = lock(state);
    // A page that follows (`?follow`, the remote) joins the presentation where it is
    if let Some(p) = s.position.as_ref().filter(|_| req.target.contains("follow")) {
        write!(stream, "event: position\ndata: {p}\n\n")?;
    }
    s.clients.push(stream);
    Ok(())
}

/// A position posted by a page, sent to all the others
fn post_position(body: &str, state: &Mutex<State>) {
    if valid_position(body) {
        let mut s = lock(state);
        s.broadcast(&format!("event: position\ndata: {body}\n\n"));
        s.position = Some(body.to_string());
    }
}

/// `{"7": [95.2, "average"], "8": [61, "replace"]}`: rank of the slide → seconds and what to do with
/// them; anything else is left out
fn durations(body: &[u8]) -> BTreeMap<usize, (f64, crate::chrono::Mode)> {
    use crate::chrono::Mode;
    serde_json::from_slice::<BTreeMap<String, (f64, String)>>(body)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, (secs, mode))| {
            let mode = match mode.as_str() {
                "average" => Mode::Average,
                "replace" => Mode::Replace,
                _ => return None,
            };
            Some((k.parse().ok()?, (secs, mode)))
        })
        .filter(|(_, (v, _))| v.is_finite() && *v > 0.0 && *v < 86400.0)
        .collect()
}

/// A file of the site: status, type and content
fn file(path: &str, s: &State) -> (&'static str, &'static str, Vec<u8>) {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() || rel.ends_with('/') { format!("{rel}index.html") } else { rel.to_string() };
    let rel = percent_decode(&rel);
    let files = match &s.site {
        Err(e) => return ("500 Internal Server Error", "text/html; charset=utf-8", error_page(e).into_bytes()),
        Ok(files) => files,
    };
    match files.iter().find(|f| f.path == rel) {
        // Served: the page knows it, the speaker view offers to record rehearsals
        Some(f) if rel == "index.html" => {
            let page = String::from_utf8_lossy(&f.bytes)
                .replacen("</head>", "<meta name=\"glitu-rehearse\" content=\"1\"></head>", 1);
            ("200 OK", mime(&rel), page.into_bytes())
        }
        Some(f) => ("200 OK", mime(&rel), f.bytes.clone()),
        None => ("404 Not Found", "text/plain; charset=utf-8", b"not found".to_vec()),
    }
}

fn error_page(e: &str) -> String {
    format!(
        "<!doctype html><meta charset=utf-8><title>Error</title><body style=\"font:16px monospace;background:#200;color:#fdd;padding:2em\"><pre>{}</pre>\
         <script>new EventSource('/__glitu/reload').onmessage = () => location.reload();</script>",
        glitu_core::html::esc(e)
    )
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(site: Result<Vec<Output>, String>) -> State {
        State { site, clients: Vec::new(), position: None, slides: PathBuf::from("slides.md"), own_write: false }
    }

    #[test]
    fn positions_cannot_inject_events() {
        assert!(valid_position(r##"{"id":"k3x9","hash":"#/3/0/1"}"##));
        assert!(!valid_position("{\"hash\":\"x\"}\n\ndata: reload"));
        assert!(!valid_position("<script>"));
        let s = Mutex::new(state(Ok(Vec::new())));
        post_position("<script>", &s);
        assert_eq!(lock(&s).position, None);
        post_position(r##"{"hash":"#/1"}"##, &s);
        assert_eq!(lock(&s).position.as_deref(), Some(r##"{"hash":"#/1"}"##));
    }

    #[test]
    fn ranges() {
        let r: Range = "192.168.0.0/16".parse().unwrap();
        assert!(r.contains("192.168.1.42".parse().unwrap()));
        assert!(r.contains("::ffff:192.168.1.42".parse().unwrap()));
        assert!(!r.contains("10.0.0.1".parse().unwrap()));
        assert!(!r.contains("fd00::1".parse().unwrap()));
        let one: Range = "10.1.2.3".parse().unwrap();
        assert!(one.contains("10.1.2.3".parse().unwrap()) && !one.contains("10.1.2.4".parse().unwrap()));
        assert!("0.0.0.0/0".parse::<Range>().unwrap().contains("8.8.8.8".parse().unwrap()));
        assert!("fd00::/8".parse::<Range>().unwrap().contains("fd12::1".parse().unwrap()));
        assert!("192.168.0.0/33".parse::<Range>().is_err() && "not an ip".parse::<Range>().is_err());
        assert!(allowed(&[], "127.0.0.1".parse().unwrap()) && !allowed(&[], "192.168.1.2".parse().unwrap()));
        assert!(local_ips(&["127.0.0.0/8".parse().unwrap()]).is_empty());
    }

    #[test]
    fn requests_are_read_with_their_length() {
        let raw = "POST /__glitu/position?x HTTP/1.1\r\nHost: a\r\nContent-Length: 12\r\n\r\n{\"hash\":\"\"}x";
        let mut reader = std::io::Cursor::new(raw);
        let req = read_request(&mut reader).unwrap();
        assert_eq!(req, Request { post: true, target: "/__glitu/position?x".into(), path: "/__glitu/position".into(), length: 12 });
        assert_eq!(read_body(&mut reader, &req, 512).unwrap(), b"{\"hash\":\"\"}x");
        assert_eq!(read_body(&mut std::io::Cursor::new("abc"), &req, 2).unwrap(), b"ab");
    }

    #[test]
    fn durations_keep_sensible_values_only() {
        use crate::chrono::Mode;
        let d = durations(
            br#"{"1": [12.5, "average"], "x": [3, "average"], "2": [-1, "replace"], "3": [1e9, "average"], "4": [30, "replace"], "5": [8, "?"]}"#,
        );
        assert_eq!(d, BTreeMap::from([(1, (12.5, Mode::Average)), (4, (30.0, Mode::Replace))]));
        assert!(durations(b"not json").is_empty());
    }

    #[test]
    fn files_are_served_from_the_site_in_memory() {
        let site = vec![
            Output { path: "index.html".into(), bytes: b"<head></head>".to_vec() },
            Output { path: "img/a b.png".into(), bytes: b"P".to_vec() },
        ];
        let s = state(Ok(site));
        let (status, kind, body) = file("/", &s);
        assert_eq!((status, kind), ("200 OK", "text/html; charset=utf-8"));
        assert!(String::from_utf8_lossy(&body).contains("glitu-rehearse"));
        assert_eq!(file("/img/a%20b.png", &s), ("200 OK", "image/png", b"P".to_vec()));
        assert_eq!(file("/absent.js", &s).0, "404 Not Found");
        let broken = state(Err("line 3: <oops>".into()));
        let (status, _, body) = file("/", &broken);
        assert_eq!(status, "500 Internal Server Error");
        assert!(String::from_utf8_lossy(&body).contains("line 3: &lt;oops&gt;"));
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(mime("x.unknown"), "application/octet-stream");
    }
}
