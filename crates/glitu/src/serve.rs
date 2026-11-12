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
    /// The presentation, whose markdown receives the timed durations of the rehearsals
    project: Project,
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
            "port {port} is already in use (another glitu server?); stop it or choose another port with -p {}",
            port.saturating_add(1)
        ),
        std::io::ErrorKind::PermissionDenied => {
            format!("port {port} requires administrator privileges; choose a port above 1023, such as -p 8000")
        }
        _ => format!("cannot listen on port {port} ({e})"),
    }
}

/// `allow` empty: the machine alone. Otherwise the server listens on every interface, but only
/// answers the machine and the given ranges (phone, tablet on the same network).
pub fn serve(project: Project, port: u16, allow: Vec<Range>, quiet: bool) -> Result<(), String> {
    if let Some(missing) = project.files().iter().find(|f| !f.is_file()) {
        return Err(format!("{}: no such file or directory", missing.display()));
    }
    // a reload renders again only the diagrams that changed
    crate::engines::keep();
    let host = if allow.is_empty() { "127.0.0.1" } else { "0.0.0.0" };
    let listener = TcpListener::bind((host, port)).map_err(|e| listen_error(port, &e))?;
    let state = Arc::new(Mutex::new(State {
        site: build::site(&project, true),
        clients: Vec::new(),
        position: None,
        project: project.clone(),
        own_write: false,
    }));
    if let Err(e) = &lock(&state).site {
        eprintln!("error: {e}");
    }
    if !quiet {
        for note in project.notes() {
            println!("{note}");
        }
        println!("Serving {} at http://localhost:{port}/", project.name());
        for ip in local_ips(&allow) {
            println!("Also available at http://{ip}:{port}/");
        }
        println!("Press Ctrl+C to stop");
    }
    let watched = Arc::clone(&state);
    thread::spawn(move || watch(&project, &watched, quiet));
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
fn watch(project: &Project, state: &Mutex<State>, quiet: bool) {
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
            Ok(_) if !quiet => println!("Rebuilt {}", project.name()),
            Ok(_) => {}
            Err(e) => eprintln!("error: {e}"),
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
    match project.read() {
        Ok(src) => src.files.iter().for_each(|f| visit(&f.path)),
        Err(_) => project.files().iter().for_each(|f| visit(f)),
    }
    // a folder's markdown: a file added or removed changes the folder's date
    if let Some(dir) = &project.dir {
        visit(dir);
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
    /// `Range: bytes=…`, which a video asks for to seek and to play
    range: Option<String>,
    /// Sent by the page of another site (its `Origin` is not the address asked, `Host`): such a
    /// page must not move the presentation nor write durations in the markdown
    foreign: bool,
}

/// At most this many bytes in the request line or in a header, and this many headers: a client
/// cannot make the server keep an endless request in memory
const MAX_LINE: usize = 8192;
const MAX_HEADERS: usize = 100;

/// A line of the request, its end included; an error beyond `MAX_LINE`
fn read_limited_line(reader: &mut impl BufRead) -> std::io::Result<String> {
    let mut line = String::new();
    let n = reader.by_ref().take(MAX_LINE as u64).read_line(&mut line)?;
    if n == MAX_LINE && !line.ends_with('\n') {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "request line too long"));
    }
    Ok(line)
}

/// Reads the request line and the headers
fn read_request(reader: &mut impl BufRead) -> std::io::Result<Request> {
    let line = read_limited_line(reader)?;
    let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();
    let (mut length, mut range, mut host, mut origin) = (0, None, None, None);
    for n in 0.. {
        let h = read_limited_line(reader)?;
        if h.len() <= 2 {
            break;
        }
        if n >= MAX_HEADERS {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "too many headers"));
        }
        let lower = h.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = lower.strip_prefix("range:") {
            range = Some(v.trim().to_string());
        } else if let Some(v) = lower.strip_prefix("host:") {
            host = Some(v.trim().to_string());
        } else if let Some(v) = lower.strip_prefix("origin:") {
            origin = Some(v.trim().to_string());
        }
    }
    // `Origin: http://localhost:8000` with `Host: localhost:8000` is the presentation itself
    let foreign = origin.is_some_and(|o| o.split_once("://").map(|(_, address)| address.to_string()) != host);
    Ok(Request { post: line.starts_with("POST"), target, path, length, range, foreign })
}

/// At most `max` bytes of the request body
fn read_body(reader: &mut impl Read, req: &Request, max: usize) -> std::io::Result<Vec<u8>> {
    let mut body = vec![0; req.length.min(max)];
    reader.read_exact(&mut body)?;
    Ok(body)
}

const NO_CONTENT: &[u8] = b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";

fn handle(mut stream: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    // a client that sends its request too slowly is dropped (the timeout covers the request
    // only: the stream of events stays open as long as the page)
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let req = read_request(&mut reader)?;
    match (req.path.as_str(), req.post) {
        ("/__glitu/reload", _) => events(stream, &req, state),
        ("/__glitu/position" | "/__glitu/chrono", true) if req.foreign => {
            stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        }
        ("/__glitu/position", true) => {
            post_position(&String::from_utf8_lossy(&read_body(&mut reader, &req, 512)?), state);
            stream.write_all(NO_CONTENT)
        }
        ("/__glitu/chrono", true) => {
            let body = read_body(&mut reader, &req, 1 << 16)?;
            let project = {
                let mut s = lock(state);
                s.own_write = true;
                s.project.clone()
            };
            let answer = match crate::chrono::record(&project.files(), &project.reads(), &durations(&body)) {
                Ok(written) => {
                    println!("Rehearsal: {} durations written to {}", written.len(), project.name());
                    if written.is_empty() {
                        lock(state).own_write = false;
                    }
                    let json: BTreeMap<String, [u32; 2]> =
                        written.into_iter().map(|(i, (d, r))| (i.to_string(), [d, r])).collect();
                    serde_json::to_string(&json).unwrap_or_default()
                }
                Err(e) => {
                    eprintln!("error: rehearsal: {e}");
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
            // a part of the file, when asked (a video seeking)
            let (status, body, part) = match req.range.as_deref().filter(|_| status.starts_with("200")) {
                Some(range) => match byte_range(range, body.len()) {
                    Some((a, b)) => (
                        "206 Partial Content",
                        body[a..=b].to_vec(),
                        format!("Content-Range: bytes {a}-{b}/{}\r\n", body.len()),
                    ),
                    None => {
                        ("416 Range Not Satisfiable", Vec::new(), format!("Content-Range: bytes */{}\r\n", body.len()))
                    }
                },
                None => (status, body, String::new()),
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n{part}Accept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
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
    // a page that stops reading its events must not hold the others back (they are written
    // with the state locked): its stream fails after a while and is dropped
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
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
            let page = String::from_utf8_lossy(&f.bytes).replacen(
                "</head>",
                "<meta name=\"glitu-rehearse\" content=\"1\"></head>",
                1,
            );
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
        // by bytes: the two after a `%` may belong to a character of several bytes
        let hex = |k: usize| b.get(k).and_then(|c| (*c as char).to_digit(16));
        if b[i] == b'%'
            && let (Some(high), Some(low)) = (hex(i + 1), hex(i + 2))
        {
            out.push((high * 16 + low) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `bytes=0-99`, `bytes=100-`, `bytes=-50` over `len` bytes → first and last byte; `None`: outside
/// the file, or several ranges (not served)
fn byte_range(range: &str, len: usize) -> Option<(usize, usize)> {
    let (a, b) = range.strip_prefix("bytes=")?.trim().split_once('-')?;
    let num = |v: &str| v.trim().parse::<usize>().ok();
    let (first, last) = match (a.trim().is_empty(), b.trim().is_empty()) {
        // the last n bytes
        (true, false) => (len.checked_sub(num(b)?.min(len))?, len.checked_sub(1)?),
        (false, true) => (num(a)?, len.checked_sub(1)?),
        (false, false) => (num(a)?, num(b)?.min(len.checked_sub(1)?)),
        (true, true) => return None,
    };
    (first <= last && last < len).then_some((first, last))
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
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "woff2" => "font/woff2",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(site: Result<Vec<Output>, String>) -> State {
        State {
            site,
            clients: Vec::new(),
            position: None,
            project: Project::new(Path::new("slides.md")),
            own_write: false,
        }
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
    fn parts_of_a_file_for_the_videos() {
        assert_eq!(byte_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(byte_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(byte_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(byte_range("bytes=990-2000", 1000), Some((990, 999)));
        assert_eq!(byte_range("bytes=1000-", 1000), None);
        assert_eq!(byte_range("bytes=5-1", 1000), None);
        assert_eq!(byte_range("bytes=0-1, 5-6", 1000), None);
        assert_eq!(byte_range("bytes=0-", 0), None);
        assert_eq!(mime("img/demo.webm"), "video/webm");
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
        assert_eq!(
            req,
            Request {
                post: true,
                target: "/__glitu/position?x".into(),
                path: "/__glitu/position".into(),
                length: 12,
                range: None,
                foreign: false
            }
        );
        let from = |headers: &str| {
            read_request(&mut format!("POST /__glitu/chrono HTTP/1.1\r\n{headers}\r\n").as_bytes()).unwrap().foreign
        };
        assert!(!from("Host: localhost:8000\r\nOrigin: http://localhost:8000\r\n"));
        assert!(!from("Host: 192.168.1.2:8000\r\nORIGIN: http://192.168.1.2:8000\r\n"));
        assert!(from("Host: localhost:8000\r\nOrigin: https://example.org\r\n"));
        assert!(from("Host: localhost:8000\r\nOrigin: null\r\n") && from("Origin: http://localhost:8000\r\n"));
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
    fn a_request_is_read_within_limits() {
        let ok = read_request(&mut "GET /a?b HTTP/1.1\r\nRange: bytes=0-1\r\nContent-Length: 3\r\n\r\nabc".as_bytes());
        let ok = ok.unwrap();
        assert_eq!((ok.path.as_str(), ok.length, ok.range.as_deref()), ("/a", 3, Some("bytes=0-1")));
        // a line without end, a header without end, headers without end
        let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(MAX_LINE));
        assert!(read_request(&mut long.as_bytes()).is_err());
        let long = format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "a".repeat(MAX_LINE));
        assert!(read_request(&mut long.as_bytes()).is_err());
        let many = format!("GET / HTTP/1.1\r\n{}\r\n", "X: y\r\n".repeat(MAX_HEADERS + 1));
        assert!(read_request(&mut many.as_bytes()).is_err());
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
        // a `%` before a character of several bytes, or before a sign, is not an escape
        assert_eq!(percent_decode("/%aé"), "/%aé");
        assert_eq!(percent_decode("%+f%41"), "%+fA");
        assert_eq!(mime("x.unknown"), "application/octet-stream");
    }

    #[test]
    fn listen_errors_say_what_to_do() {
        let e = |kind| std::io::Error::from(kind);
        assert!(listen_error(8000, &e(std::io::ErrorKind::AddrInUse)).contains("-p 8001"));
        assert!(listen_error(80, &e(std::io::ErrorKind::PermissionDenied)).contains("above 1023"));
        assert!(listen_error(8000, &e(std::io::ErrorKind::Other)).starts_with("cannot listen on port 8000"));
    }

    #[test]
    fn the_stamp_follows_the_sources() {
        let root = std::env::temp_dir().join(format!("glitu-stamp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("img/sub")).unwrap();
        let project = Project::new(&root.join("slides.md"));
        assert_eq!(stamp(&project), SystemTime::UNIX_EPOCH);
        fs::write(&project.slides, "# A\n").unwrap();
        let first = stamp(&project);
        assert!(first > SystemTime::UNIX_EPOCH);
        let later = fs::File::create(root.join("img/sub/a.png")).unwrap();
        later.set_modified(first + Duration::from_secs(60)).unwrap();
        assert_eq!(stamp(&project), first + Duration::from_secs(60));
        let _ = fs::remove_dir_all(&root);
    }
}
