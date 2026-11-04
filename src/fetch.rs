use crate::api::Stream;
use crate::hls::{self, Part};
use crate::net;
use anyhow::{anyhow, bail, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;

const CONNECTIONS: usize = 12;

pub enum Progress {
    Parts(usize, usize, u64),
    Remux,
}

pub struct Task {
    pub url: String,
    pub file: PathBuf,
    pub ts: bool,
}

pub fn mirror(
    stream: &Stream,
    work: &Path,
    max_height: u32,
    lang: &str,
    progress: &(dyn Fn(Progress) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<String> {
    let referer = stream.referer.as_deref();
    let mut tasks = Vec::new();
    let text = net::text(&stream.url, referer)?;
    let main = if hls::is_master(&text) {
        let variant = hls::best(hls::variants(&text, &stream.url), max_height)
            .ok_or_else(|| anyhow!("no video variant"))?;
        let video = localize(&variant.url, referer, work, "v", &mut tasks)?;
        let audio = variant
            .audio
            .as_deref()
            .and_then(|g| hls::pick_audio(hls::audio_tracks(&text, &stream.url, g), lang));
        match audio {
            Some(track) => {
                let audio = localize(&track.url, referer, work, "a", &mut tasks)?;
                let media = track.line.replace(
                    &format!("URI=\"{}\"", hls::raw_uri(&track.line)),
                    &format!("URI=\"{audio}\""),
                );
                let inf = strip_attrs(&variant.inf);
                fs::write(
                    work.join("master.m3u8"),
                    format!("#EXTM3U\n#EXT-X-VERSION:3\n{media}\n{inf}\n{video}\n"),
                )?;
                "master.m3u8".to_string()
            }
            None => video,
        }
    } else {
        localize_text(&text, &stream.url, work, "v", &mut tasks)?
    };
    fetch_all(&tasks, referer, progress, stop)?;
    Ok(main)
}

pub fn save_subtitle(stream: &Stream, lang: &str, work: &Path) -> Option<PathBuf> {
    let sub = stream.subs.iter().find(|s| {
        s.lang.to_lowercase().starts_with(&lang.to_lowercase()) || lang_name(&s.lang, lang)
    })?;
    let body = net::bytes(&sub.url, sub.referer.as_deref()).ok()?;
    let ext = if body.starts_with(b"WEBVTT") || body.starts_with(b"\xEF\xBB\xBFWEBVTT") {
        "vtt"
    } else if body.windows(13).take(64).any(|w| w == b"[Script Info]") {
        "ass"
    } else {
        "srt"
    };
    let path = work.join(format!("sub.{ext}"));
    fs::write(&path, body).ok()?;
    Some(path)
}

pub fn strip_attrs(inf: &str) -> String {
    inf.split(',')
        .filter(|kv| !kv.starts_with("SUBTITLES=") && !kv.starts_with("CLOSED-CAPTIONS="))
        .collect::<Vec<_>>()
        .join(",")
}

fn localize(
    url: &str,
    referer: Option<&str>,
    work: &Path,
    tag: &str,
    tasks: &mut Vec<Task>,
) -> Result<String> {
    let text = net::text(url, referer)?;
    localize_text(&text, url, work, tag, tasks)
}

fn localize_text(
    text: &str,
    base: &str,
    work: &Path,
    tag: &str,
    tasks: &mut Vec<Task>,
) -> Result<String> {
    if hls::is_master(text) {
        bail!("nested master playlist");
    }
    let mut n = 0;
    let local = hls::rewrite(text, base, |part, abs| {
        n += 1;
        let (name, ts) = match part {
            Part::Segment(ext) => (format!("{tag}{n:05}.{ext}"), ext == "ts"),
            Part::Key => (format!("{tag}{n:05}.key"), false),
            Part::Init => (format!("{tag}{n:05}-init.mp4"), false),
            Part::Playlist => return abs,
        };
        tasks.push(Task {
            url: abs,
            file: work.join(&name),
            ts,
        });
        name
    });
    let name = format!("{tag}.m3u8");
    fs::write(work.join(&name), local)?;
    Ok(name)
}

struct Gate {
    free: Mutex<usize>,
    ready: Condvar,
}

fn gate() -> &'static Gate {
    static GATE: OnceLock<Gate> = OnceLock::new();
    GATE.get_or_init(|| Gate {
        free: Mutex::new(CONNECTIONS),
        ready: Condvar::new(),
    })
}

struct Permit;

impl Gate {
    fn acquire(&self) -> Permit {
        let mut free = self.free.lock().unwrap();
        while *free == 0 {
            free = self.ready.wait(free).unwrap();
        }
        *free -= 1;
        Permit
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let gate = gate();
        *gate.free.lock().unwrap() += 1;
        gate.ready.notify_one();
    }
}

pub fn fetch_all(
    tasks: &[Task],
    referer: Option<&str>,
    progress: &(dyn Fn(Progress) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<()> {
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let bytes = AtomicU64::new(0);
    let failed = AtomicBool::new(false);
    let error = Mutex::new(None);
    let total = tasks.len();
    progress(Progress::Parts(0, total, 0));
    thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                while !failed.load(Ordering::Relaxed) {
                    if stop() {
                        *error.lock().unwrap() = Some(anyhow!("cancelled"));
                        failed.store(true, Ordering::Relaxed);
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(i) else { break };
                    let fetched = {
                        let _permit = gate().acquire();
                        net::bytes(&task.url, referer)
                    };
                    match fetched {
                        Ok(body) => {
                            let body = if task.ts {
                                net::unmask_ts(&body)
                            } else {
                                &body[..]
                            };
                            if let Err(err) = fs::write(&task.file, body) {
                                *error.lock().unwrap() = Some(anyhow!(err));
                                failed.store(true, Ordering::Relaxed);
                            }
                            let b = bytes.fetch_add(body.len() as u64, Ordering::Relaxed)
                                + body.len() as u64;
                            let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                            progress(Progress::Parts(d, total, b));
                        }
                        Err(err) => {
                            *error.lock().unwrap() = Some(err);
                            failed.store(true, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });
    match error.into_inner().unwrap() {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

pub fn lang_name(lang: &str, pref: &str) -> bool {
    let names = [
        ("en", "english"),
        ("es", "spanish"),
        ("fr", "french"),
        ("pt", "portuguese"),
        ("de", "german"),
        ("it", "italian"),
        ("ja", "japanese"),
        ("ar", "arabic"),
        ("ru", "russian"),
    ];
    names
        .iter()
        .find(|(code, _)| *code == pref.to_lowercase())
        .is_some_and(|(_, name)| lang.to_lowercase().starts_with(name))
}
