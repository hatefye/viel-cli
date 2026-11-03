use crate::hls::{self, Part};
use crate::net;
use anyhow::{anyhow, Result};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Instant;

static MAX_HEIGHT: AtomicU32 = AtomicU32::new(1080);
static INFLIGHT: AtomicUsize = AtomicUsize::new(0);
static SERVED: AtomicUsize = AtomicUsize::new(0);

pub fn set_max_height(h: u32) {
    MAX_HEIGHT.store(h.max(240), Ordering::Relaxed);
}

fn lang() -> &'static Mutex<String> {
    static LANG: OnceLock<Mutex<String>> = OnceLock::new();
    LANG.get_or_init(|| Mutex::new("en".into()))
}

pub fn set_audio_lang(l: &str) {
    *lang().lock().unwrap() = l.to_string();
}

fn current() -> &'static Mutex<Option<(String, Instant)>> {
    static CUR: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();
    CUR.get_or_init(|| Mutex::new(None))
}

pub struct Activity {
    pub inflight: usize,
    pub served: usize,
    pub fetching: Option<(String, f32)>,
}

pub fn activity() -> Activity {
    let fetching = current().lock().unwrap().as_ref().map(|(what, at)| (what.clone(), at.elapsed().as_secs_f32()));
    Activity { inflight: INFLIGHT.load(Ordering::Relaxed), served: SERVED.load(Ordering::Relaxed), fetching }
}

pub fn reset_activity() {
    SERVED.store(0, Ordering::Relaxed);
    *current().lock().unwrap() = None;
}

struct Busy;

fn begin(what: String) -> Busy {
    INFLIGHT.fetch_add(1, Ordering::Relaxed);
    *current().lock().unwrap() = Some((what, Instant::now()));
    Busy
}

impl Drop for Busy {
    fn drop(&mut self) {
        INFLIGHT.fetch_sub(1, Ordering::Relaxed);
        SERVED.fetch_add(1, Ordering::Relaxed);
        if INFLIGHT.load(Ordering::Relaxed) == 0 {
            *current().lock().unwrap() = None;
        }
    }
}

fn port() -> u16 {
    static PORT: OnceLock<u16> = OnceLock::new();
    *PORT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind relay");
        let port = listener.local_addr().expect("relay addr").port();
        thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                thread::spawn(move || {
                    let _ = serve(conn);
                });
            }
        });
        port
    })
}

fn link(route: &str, file: &str, url: &str, referer: Option<&str>) -> String {
    let mut out = format!("http://127.0.0.1:{}/{route}/{file}?u={}", port(), urlencoding::encode(url));
    if let Some(r) = referer.filter(|r| !r.is_empty()) {
        out.push_str("&r=");
        out.push_str(&urlencoding::encode(r));
    }
    out
}

pub fn playlist(url: &str, referer: Option<&str>) -> String {
    link("p", "index.m3u8", url, referer)
}

pub fn video(url: &str, referer: Option<&str>) -> String {
    link("v", "video.mp4", url, referer)
}

pub fn subtitle(url: &str, referer: Option<&str>) -> String {
    link("t", "sub", url, referer)
}

fn rewrite(text: &str, base: &str, referer: Option<&str>) -> String {
    hls::rewrite(text, base, |part, abs| match part {
        Part::Playlist => playlist(&abs, referer),
        Part::Segment(ext) => link("s", &format!("seg.{ext}"), &abs, referer),
        Part::Key => link("s", "key.bin", &abs, referer),
        Part::Init => link("s", "init.mp4", &abs, referer),
    })
}

fn declared_len(resp: &reqwest::blocking::Response) -> Option<u64> {
    resp.headers().get("content-length")?.to_str().ok()?.trim().parse().ok()
}

struct Request {
    route: String,
    file: String,
    url: String,
    referer: Option<String>,
    range: Option<String>,
}

fn parse(target: &str, range: Option<String>) -> Option<Request> {
    let (path, query) = target.split_once('?')?;
    let mut parts = path.trim_start_matches('/').splitn(2, '/');
    let route = parts.next()?.to_string();
    let file = parts.next().unwrap_or("").to_string();
    let mut url = None;
    let mut referer = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = urlencoding::decode(v).ok()?.into_owned();
        match k {
            "u" => url = Some(v),
            "r" => referer = Some(v),
            _ => {}
        }
    }
    Some(Request { route, file, url: url?, referer, range })
}

fn serve(conn: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(conn.try_clone()?);
    let mut out = conn;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
        let mut range = None;
        let mut close = line.contains("HTTP/1.0");
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
                break;
            }
            let lower = header.to_ascii_lowercase();
            if let Some(v) = lower.strip_prefix("range:") {
                range = Some(v.trim().to_string());
            }
            if lower.starts_with("connection:") && lower.contains("close") {
                close = true;
            }
        }
        let keep = match parse(&target, range) {
            Some(req) => respond(&mut out, req),
            None => reply(&mut out, "400 Bad Request", "text/plain", b"bad request").map(|_| true).map_err(Into::into),
        };
        match keep {
            Ok(true) if !close => continue,
            Ok(_) => return Ok(()),
            Err(err) => {
                let _ = reply(&mut out, "502 Bad Gateway", "text/plain", err.to_string().as_bytes());
                return Ok(());
            }
        }
    }
}

fn reply(out: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> io::Result<()> {
    write!(out, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n", body.len())?;
    out.write_all(body)?;
    out.flush()
}

fn respond(out: &mut TcpStream, req: Request) -> Result<bool> {
    let referer = req.referer.as_deref();
    let what = match req.route.as_str() {
        "p" => format!("playlist · {}", net::host(&req.url)),
        "t" => format!("subtitles · {}", net::host(&req.url)),
        "v" => format!("video · {}", net::host(&req.url)),
        _ => format!("{} · {}", req.file.split('.').next().unwrap_or("segment"), net::host(&req.url)),
    };
    let _busy = begin(what);
    match req.route.as_str() {
        "p" => {
            let text = net::send_with(&req.url, referer, None, 2)?.text()?;
            if !text.contains("#EXTM3U") {
                return Err(anyhow!("not a playlist from {}", net::host(&req.url)));
            }
            let text = if hls::is_master(&text) {
                hls::trim_master(&text, &req.url, MAX_HEIGHT.load(Ordering::Relaxed), &lang().lock().unwrap()).unwrap_or(text)
            } else {
                text
            };
            reply(out, "200 OK", "application/vnd.apple.mpegurl", rewrite(&text, &req.url, referer).as_bytes())?;
        }
        "t" => {
            let body = net::send_with(&req.url, referer, None, 2)?.bytes()?;
            reply(out, "200 OK", "text/plain; charset=utf-8", &body)?;
        }
        "s" => return stream_segment(out, &req),
        "v" => return stream_video(out, &req),
        _ => reply(out, "404 Not Found", "text/plain", b"not found")?,
    }
    Ok(true)
}

fn stream_segment(out: &mut TcpStream, req: &Request) -> Result<bool> {
    let mut resp = net::send_with(&req.url, req.referer.as_deref(), None, 2)?;
    let mut head = Vec::with_capacity(16384);
    let mut chunk = [0u8; 16384];
    while head.len() < 16384 {
        let n = resp.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..n]);
    }
    let skip = if req.file.ends_with(".ts") { net::ts_offset(&head) } else { 0 };
    let known = declared_len(&resp).filter(|_| skip == 0);
    write!(out, "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n")?;
    match known {
        Some(n) => write!(out, "Content-Length: {n}\r\nConnection: keep-alive\r\n\r\n")?,
        None => write!(out, "Connection: close\r\n\r\n")?,
    }
    out.write_all(&head[skip..])?;
    io::copy(&mut resp, out)?;
    out.flush()?;
    Ok(known.is_some())
}

fn stream_video(out: &mut TcpStream, req: &Request) -> Result<bool> {
    let mut resp = net::send_with(&req.url, req.referer.as_deref(), req.range.as_deref(), 2)?;
    let status = if resp.status().as_u16() == 206 { "206 Partial Content" } else { "200 OK" };
    let length = declared_len(&resp);
    write!(out, "HTTP/1.1 {status}\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\n")?;
    if let Some(range) = resp.headers().get("content-range").and_then(|v| v.to_str().ok()) {
        write!(out, "Content-Range: {range}\r\n")?;
    }
    match length {
        Some(n) => write!(out, "Content-Length: {n}\r\nConnection: keep-alive\r\n\r\n")?,
        None => write!(out, "Connection: close\r\n\r\n")?,
    }
    io::copy(&mut resp, out)?;
    out.flush()?;
    Ok(length.is_some())
}
