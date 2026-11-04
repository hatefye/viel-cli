use crate::hls::{self, Part};
use crate::{local, net};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

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
    let fetching = current()
        .lock()
        .unwrap()
        .as_ref()
        .map(|(what, at)| (what.clone(), at.elapsed().as_secs_f32()));
    Activity {
        inflight: INFLIGHT.load(Ordering::Relaxed),
        served: SERVED.load(Ordering::Relaxed),
        fetching,
    }
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

pub fn port() -> u16 {
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
    let mut out = format!(
        "http://127.0.0.1:{}/{route}/{file}?u={}",
        port(),
        urlencoding::encode(url)
    );
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
    resp.headers()
        .get("content-length")?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body: Vec<u8>,
    range: Option<String>,
}

impl Request {
    fn param(&self, key: &str) -> Option<&str> {
        self.query
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }
}

fn parse(line: &str) -> Option<(String, String, HashMap<String, String>)> {
    let mut words = line.split_whitespace();
    let method = words.next()?.to_string();
    let target = words.next()?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = query
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            Some((k.to_string(), urlencoding::decode(v).ok()?.into_owned()))
        })
        .collect();
    Some((method, path.to_string(), query))
}

fn serve(conn: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(conn.try_clone()?);
    let mut out = conn;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let mut range = None;
        let mut length = 0usize;
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
            if let Some(v) = lower.strip_prefix("content-length:") {
                length = v.trim().parse().unwrap_or(0);
            }
            if lower.starts_with("connection:") && lower.contains("close") {
                close = true;
            }
        }
        let mut body = vec![0u8; length.min(1 << 20)];
        reader.read_exact(&mut body)?;
        let started = Instant::now();
        let target = line.split_whitespace().nth(1).unwrap_or("?").to_string();
        let keep = match parse(&line) {
            Some((method, path, query)) => respond(
                &mut out,
                Request {
                    method,
                    path,
                    query,
                    body,
                    range,
                },
            ),
            None => reply(&mut out, "400 Bad Request", "text/plain", b"bad request")
                .map(|_| true)
                .map_err(Into::into),
        };
        let took = started.elapsed().as_secs_f32();
        if took > 2.0 && !target.starts_with("/api/") {
            local::log("relay", format!("slow {took:.1}s {}", describe(&target)));
        }
        match keep {
            Ok(true) if !close => continue,
            Ok(_) => return Ok(()),
            Err(err) => {
                local::log("relay", format!("{err} ← {}", describe(&target)));
                let _ = reply(
                    &mut out,
                    "502 Bad Gateway",
                    "text/plain",
                    err.to_string().as_bytes(),
                );
                return Ok(());
            }
        }
    }
}

fn describe(target: &str) -> String {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let url = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("u="))
        .and_then(|v| urlencoding::decode(v).ok())
        .map(|u| u.into_owned())
        .unwrap_or_default();
    let host = net::host(&url);
    let tail: String = url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .chars()
        .take(40)
        .collect();
    format!("{path} {host}/…/{tail}")
}

fn reply(out: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> io::Result<()> {
    write!(out, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n", body.len())?;
    out.write_all(body)?;
    out.flush()
}

fn respond(out: &mut TcpStream, req: Request) -> Result<bool> {
    let trimmed = req.path.trim_start_matches('/');
    let (route, file) = trimmed.split_once('/').unwrap_or((trimmed, ""));
    if route == "api" {
        let (status, kind, body) = local::handle(&req.method, file, &req.query, &req.body);
        reply(out, status, kind, &body)?;
        return Ok(true);
    }
    let url = req.param("u").ok_or_else(|| anyhow!("missing url"))?;
    let referer = req.param("r");
    let what = match route {
        "p" => format!("playlist · {}", net::host(url)),
        "t" => format!("subtitles · {}", net::host(url)),
        "v" => format!("video · {}", net::host(url)),
        _ => format!(
            "{} · {}",
            file.split('.').next().unwrap_or("segment"),
            net::host(url)
        ),
    };
    let _busy = begin(what);
    match route {
        "p" => {
            let text = net::send_with(url, referer, None, 2)?.text()?;
            if !text.contains("#EXTM3U") {
                return Err(anyhow!("not a playlist from {}", net::host(url)));
            }
            let text = if hls::is_master(&text) {
                hls::trim_master(
                    &text,
                    url,
                    MAX_HEIGHT.load(Ordering::Relaxed),
                    &lang().lock().unwrap(),
                )
                .unwrap_or(text)
            } else {
                text
            };
            reply(
                out,
                "200 OK",
                "application/vnd.apple.mpegurl",
                rewrite(&text, url, referer).as_bytes(),
            )?;
        }
        "t" => {
            let body = fetch_subtitle(url, referer)?;
            reply(out, "200 OK", "text/plain; charset=utf-8", &body)?;
        }
        "s" => return stream_segment(out, url, referer, file),
        "v" => return stream_video(out, url, referer, req.range.as_deref()),
        _ => reply(out, "404 Not Found", "text/plain", b"not found")?,
    }
    Ok(true)
}

fn fetch_subtitle(url: &str, referer: Option<&str>) -> Result<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    let mut pending = 1;
    spawn_fetch(tx.clone(), url.to_string(), referer.map(str::to_string));
    let mut wait = Duration::from_secs(3);
    let mut last = anyhow!("no subtitle route");
    loop {
        match rx.recv_timeout(wait) {
            Ok(Ok(body)) => return Ok(body),
            Ok(Err(err)) => {
                last = err;
                pending -= 1;
            }
            Err(mpsc::RecvTimeoutError::Timeout) if pending == 0 => return Err(last),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(last),
        }
        if wait.as_secs() == 3 {
            wait = Duration::from_secs(25);
            if let Some(via) = local::subtitle_proxy(url, referer) {
                pending += 1;
                spawn_fetch(tx.clone(), via, None);
            }
        }
        if pending == 0 {
            return Err(last);
        }
    }
}

fn spawn_fetch(tx: mpsc::Sender<Result<Vec<u8>>>, url: String, referer: Option<String>) {
    thread::spawn(move || {
        let result = net::send_with(&url, referer.as_deref(), None, 1)
            .and_then(|resp| Ok(resp.bytes()?.to_vec()));
        let _ = tx.send(result);
    });
}

fn stream_segment(
    out: &mut TcpStream,
    url: &str,
    referer: Option<&str>,
    file: &str,
) -> Result<bool> {
    let mut resp = net::send_with(url, referer, None, 2)?;
    let mut head = Vec::with_capacity(16384);
    let mut chunk = [0u8; 16384];
    while head.len() < 16384 {
        let n = resp.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..n]);
    }
    let skip = if file.ends_with(".ts") {
        net::ts_offset(&head)
    } else {
        0
    };
    let known = declared_len(&resp).filter(|_| skip == 0);
    write!(
        out,
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n"
    )?;
    match known {
        Some(n) => write!(out, "Content-Length: {n}\r\nConnection: keep-alive\r\n\r\n")?,
        None => write!(out, "Connection: close\r\n\r\n")?,
    }
    out.write_all(&head[skip..])?;
    io::copy(&mut resp, out)?;
    out.flush()?;
    Ok(known.is_some())
}

fn stream_video(
    out: &mut TcpStream,
    url: &str,
    referer: Option<&str>,
    range: Option<&str>,
) -> Result<bool> {
    let mut resp = net::send_with(url, referer, range, 2)?;
    let status = if resp.status().as_u16() == 206 {
        "206 Partial Content"
    } else {
        "200 OK"
    };
    let length = declared_len(&resp);
    write!(
        out,
        "HTTP/1.1 {status}\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\n"
    )?;
    if let Some(range) = resp
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok())
    {
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
