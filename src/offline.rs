use crate::api::Stream;
use crate::fetch::{self, Progress};
use crate::{local, net};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;

#[derive(Clone)]
pub struct Entry {
    pub key: String,
    pub meta: Value,
    pub state: String,
    pub source: String,
    pub done: usize,
    pub total: usize,
    pub bytes: u64,
    pub error: Option<String>,
}

struct Job {
    key: String,
    meta: Value,
    streams: Vec<Stream>,
    max_height: u32,
    lang: String,
}

struct Queue {
    pending: VecDeque<Job>,
    entries: Vec<Entry>,
    cancelled: Vec<String>,
}

fn dir() -> &'static Mutex<Option<PathBuf>> {
    static DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    DIR.get_or_init(|| Mutex::new(None))
}

fn queue() -> &'static (Mutex<Queue>, Condvar) {
    static Q: OnceLock<(Mutex<Queue>, Condvar)> = OnceLock::new();
    Q.get_or_init(|| {
        thread::spawn(worker);
        (
            Mutex::new(Queue {
                pending: VecDeque::new(),
                entries: Vec::new(),
                cancelled: Vec::new(),
            }),
            Condvar::new(),
        )
    })
}

pub fn set_dir(path: &str) {
    let root = PathBuf::from(path);
    fs::create_dir_all(&root).ok();
    if let Ok(read) = fs::read_dir(&root) {
        for item in read.flatten() {
            let folder = item.path();
            if folder.is_dir() && !folder.join("meta.json").exists() {
                let _ = fs::remove_dir_all(&folder);
            }
        }
    }
    *dir().lock().unwrap() = Some(root);
}

fn root() -> Option<PathBuf> {
    dir().lock().unwrap().clone()
}

fn safe(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn path(key: &str, file: &str) -> Option<PathBuf> {
    if !safe(key) || file.contains("..") || file.contains('/') || file.is_empty() {
        return None;
    }
    let path = root()?.join(key).join(file);
    path.is_file().then_some(path)
}

pub fn enqueue(
    key: &str,
    meta: Value,
    streams: Vec<Stream>,
    max_height: u32,
    lang: &str,
) -> Result<()> {
    if !safe(key) {
        bail!("bad download key");
    }
    let Some(root) = root() else {
        bail!("downloads are not set up");
    };
    if root.join(key).join("meta.json").exists() {
        bail!("already downloaded");
    }
    if streams.is_empty() {
        bail!("no source has this one right now");
    }
    let (lock, signal) = queue();
    let mut q = lock.lock().unwrap();
    if q.entries
        .iter()
        .any(|e| e.key == key && e.state != "failed")
    {
        bail!("already downloading");
    }
    q.entries.retain(|e| e.key != key);
    q.entries.push(Entry {
        key: key.to_string(),
        meta: meta.clone(),
        state: "queued".into(),
        source: String::new(),
        done: 0,
        total: 0,
        bytes: 0,
        error: None,
    });
    q.pending.push_back(Job {
        key: key.to_string(),
        meta,
        streams,
        max_height,
        lang: lang.to_string(),
    });
    signal.notify_one();
    Ok(())
}

pub fn remove(key: &str) -> Result<()> {
    if !safe(key) {
        bail!("bad download key");
    }
    let (lock, _) = queue();
    {
        let mut q = lock.lock().unwrap();
        q.pending.retain(|j| j.key != key);
        q.entries.retain(|e| e.key != key);
        q.cancelled.push(key.to_string());
    }
    if let Some(root) = root() {
        let _ = fs::remove_dir_all(root.join(key));
    }
    Ok(())
}

pub fn list() -> Vec<Value> {
    let (lock, _) = queue();
    let live: Vec<Entry> = lock.lock().unwrap().entries.clone();
    let mut out: Vec<Value> = live.iter().map(entry_json).collect();
    if let Some(root) = root() {
        if let Ok(read) = fs::read_dir(&root) {
            let mut saved: Vec<(u64, Value)> = read
                .flatten()
                .filter_map(|item| {
                    let key = item.file_name().to_string_lossy().to_string();
                    if live.iter().any(|e| e.key == key) {
                        return None;
                    }
                    let meta: Value =
                        serde_json::from_slice(&fs::read(item.path().join("meta.json")).ok()?)
                            .ok()?;
                    let at = meta.get("saved_at").and_then(Value::as_u64).unwrap_or(0);
                    Some((
                        at,
                        json!({
                            "key": key,
                            "meta": meta,
                            "state": "done",
                            "source": meta.get("source").cloned().unwrap_or(Value::Null),
                            "done": 0,
                            "total": 0,
                            "bytes": meta.get("bytes").cloned().unwrap_or(json!(0)),
                            "error": Value::Null,
                        }),
                    ))
                })
                .collect();
            saved.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
            out.extend(saved.into_iter().map(|(_, v)| v));
        }
    }
    out
}

pub fn playback(key: &str) -> Option<Value> {
    let root = root()?;
    let meta: Value =
        serde_json::from_slice(&fs::read(root.join(key).join("meta.json")).ok()?).ok()?;
    let main = meta.get("main")?.as_str()?.to_string();
    let port = crate::relay::port();
    let sub = meta
        .get("sub")
        .and_then(Value::as_str)
        .map(|name| json!([{ "lang": meta.get("sub_lang").cloned().unwrap_or(json!("en")), "url": format!("http://127.0.0.1:{port}/d/{key}/{name}") }]))
        .unwrap_or(json!([]));
    Some(json!({
        "streams": [{
            "source": "offline",
            "url": format!("http://127.0.0.1:{port}/d/{key}/{main}"),
            "hls": main.ends_with(".m3u8"),
            "subs": sub,
            "intro": meta.get("intro").cloned().unwrap_or(Value::Null),
            "outro": meta.get("outro").cloned().unwrap_or(Value::Null),
            "thumbs": Value::Null,
        }]
    }))
}

fn entry_json(e: &Entry) -> Value {
    json!({
        "key": e.key,
        "meta": e.meta,
        "state": e.state,
        "source": e.source,
        "done": e.done,
        "total": e.total,
        "bytes": e.bytes,
        "error": e.error,
    })
}

fn update(key: &str, apply: impl FnOnce(&mut Entry)) {
    let (lock, _) = queue();
    if let Some(entry) = lock
        .lock()
        .unwrap()
        .entries
        .iter_mut()
        .find(|e| e.key == key)
    {
        apply(entry);
    }
}

fn cancelled(key: &str) -> bool {
    let (lock, _) = queue();
    lock.lock().unwrap().cancelled.iter().any(|k| k == key)
}

fn worker() {
    let (lock, signal) = queue();
    loop {
        let job = {
            let mut q = lock.lock().unwrap();
            loop {
                if let Some(job) = q.pending.pop_front() {
                    break job;
                }
                q = signal.wait(q).unwrap();
            }
        };
        {
            let mut q = lock.lock().unwrap();
            q.cancelled.retain(|k| k != &job.key);
        }
        let result = run(&job);
        let (lock, _) = queue();
        let mut q = lock.lock().unwrap();
        match result {
            Ok(()) => q.entries.retain(|e| e.key != job.key),
            Err(err) => {
                if let Some(entry) = q.entries.iter_mut().find(|e| e.key == job.key) {
                    entry.state = "failed".into();
                    entry.error = Some(err.to_string());
                }
                local::log("download", format!("{} failed: {err}", job.key));
            }
        }
    }
}

fn run(job: &Job) -> Result<()> {
    let root = root().ok_or_else(|| anyhow!("downloads are not set up"))?;
    let folder = root.join(&job.key);
    let mut last = None;
    for stream in &job.streams {
        if cancelled(&job.key) {
            bail!("cancelled");
        }
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder)?;
        let source = stream.source.clone();
        update(&job.key, |e| {
            e.state = "fetching".into();
            e.source = source.clone();
            e.done = 0;
            e.total = 0;
            e.bytes = 0;
        });
        local::log("download", format!("{} via {source}", job.key));
        let key = job.key.clone();
        let progress = move |p: Progress| {
            if let Progress::Parts(done, total, bytes) = p {
                update(&key, |e| {
                    e.done = done;
                    e.total = total;
                    e.bytes = bytes;
                });
            }
        };
        let key = job.key.clone();
        let stop = move || cancelled(&key);
        let fetched = if stream.hls {
            fetch::mirror(stream, &folder, job.max_height, &job.lang, &progress, &stop)
        } else {
            save_video(stream, &folder, &progress, &stop)
        };
        match fetched {
            Ok(main) => {
                let sub = fetch::save_subtitle(stream, &job.lang, &folder);
                let bytes = folder_size(&folder);
                let mut meta = job.meta.clone();
                if let Value::Object(map) = &mut meta {
                    map.insert("main".into(), json!(main));
                    map.insert("source".into(), json!(source));
                    map.insert("bytes".into(), json!(bytes));
                    map.insert("intro".into(), json!(stream.intro));
                    map.insert("outro".into(), json!(stream.outro));
                    map.insert(
                        "saved_at".into(),
                        json!(std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)),
                    );
                    if let Some(sub) = sub {
                        map.insert(
                            "sub".into(),
                            json!(sub.file_name().map(|n| n.to_string_lossy().to_string())),
                        );
                        map.insert("sub_lang".into(), json!(job.lang));
                    }
                }
                fs::write(folder.join("meta.json"), serde_json::to_vec(&meta)?)?;
                local::log(
                    "download",
                    format!(
                        "{} saved via {source}, {:.0} MB",
                        job.key,
                        bytes as f64 / 1_048_576.0
                    ),
                );
                return Ok(());
            }
            Err(err) => {
                if err.to_string() == "cancelled" {
                    let _ = fs::remove_dir_all(&folder);
                    bail!("cancelled");
                }
                last = Some(format!("{source}: {err}"));
            }
        }
    }
    let _ = fs::remove_dir_all(&folder);
    bail!(
        "{}",
        last.unwrap_or_else(|| "no source has this one".into())
    )
}

fn save_video(
    stream: &Stream,
    folder: &std::path::Path,
    progress: &(dyn Fn(Progress) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<String> {
    let mut resp = net::send_with(&stream.url, stream.referer.as_deref(), None, 2)?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let mut file = fs::File::create(folder.join("video.mp4"))?;
    let mut buf = [0u8; 65536];
    let mut got = 0u64;
    let mut tick = 0u64;
    loop {
        if stop() {
            bail!("cancelled");
        }
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        got += n as u64;
        if got - tick > 2_000_000 {
            tick = got;
            let parts = if total > 0 { 100 } else { 0 };
            let done = if total > 0 {
                ((got * 100) / total) as usize
            } else {
                0
            };
            progress(Progress::Parts(done, parts, got));
        }
    }
    progress(Progress::Parts(100, 100, got));
    Ok("video.mp4".into())
}

fn folder_size(folder: &std::path::Path) -> u64 {
    fs::read_dir(folder)
        .map(|read| {
            read.flatten()
                .filter_map(|f| f.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}
