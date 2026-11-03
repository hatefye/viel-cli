use crate::api::{Api, Stream, Title};
use crate::config::Config;
use crate::hls::{self, Part};
use crate::{deps, net, relay, ui};
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

pub enum Target {
    Media { path: String, movie: bool },
    Anime(Vec<(String, String)>),
}

pub struct Job {
    pub target: Target,
    pub label: String,
    pub filename: String,
}

pub fn movie_job(title: &Title) -> Job {
    Job {
        target: Target::Media { path: title.id.to_string(), movie: true },
        label: title.name.clone(),
        filename: sanitize(&movie_name(title)),
    }
}

pub fn tv_job(title: &Title, season: u32, episode: u32, name: &str) -> Job {
    Job {
        target: Target::Media { path: format!("{}/{}/{}", title.id, season, episode), movie: false },
        label: format!("{} S{season:02}E{episode:02} · {name}", title.name),
        filename: sanitize(&format!("{} S{season:02}E{episode:02} {name}.mkv", title.name)),
    }
}

pub fn anime_job(title: &Title, number: f64, dub: bool, sources: Vec<(String, String)>) -> Job {
    let num = number as u32;
    let tag = if dub { " (dub)" } else { "" };
    Job {
        target: Target::Anime(sources),
        label: format!("{} E{num:02}{tag}", title.name),
        filename: sanitize(&format!("{} E{num:02}{tag}.mkv", title.name)),
    }
}

const CONNECTIONS: usize = 12;

pub enum Status {
    Queued,
    Resolving,
    Fetching { source: String, done: usize, total: usize, bytes: u64 },
    Remuxing { source: String },
    Saved(PathBuf),
    Failed(String),
}

pub enum Progress {
    Parts(usize, usize, u64),
    Remux,
}

pub fn run_jobs(api: &Api, cfg: &Config, jobs: Vec<Job>) -> Result<()> {
    let Some(ffmpeg) = ensure_ffmpeg() else {
        return Ok(());
    };
    let dir = PathBuf::from(&cfg.download_dir);
    fs::create_dir_all(&dir).ok();
    ui::clear();
    println!("saving to {}\n", dir.display());
    let states: Vec<Mutex<Status>> = jobs.iter().map(|_| Mutex::new(Status::Queued)).collect();
    let next = AtomicUsize::new(0);
    let parallel = cfg.parallel_downloads.clamp(1, 6).min(jobs.len().max(1));
    thread::scope(|scope| {
        for _ in 0..parallel {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(job) = jobs.get(i) else { break };
                let set = |st: Status| *states[i].lock().unwrap() = st;
                let result = run_job(api, cfg, &ffmpeg, &dir, job, &set);
                set(match result {
                    Ok(path) => Status::Saved(path),
                    Err(err) => Status::Failed(err.to_string()),
                });
            });
        }
        let mut view = View::default();
        loop {
            let finished = view.draw(&jobs, &states);
            if finished {
                break;
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
    let failed = states.iter().filter(|s| matches!(*s.lock().unwrap(), Status::Failed(_))).count();
    if failed > 0 {
        ui::pause();
    } else {
        thread::sleep(Duration::from_millis(1200));
    }
    Ok(())
}

#[derive(Default)]
struct View {
    drawn: u16,
    logged: Vec<bool>,
}

impl View {
    fn draw(&mut self, jobs: &[Job], states: &[Mutex<Status>]) -> bool {
        use crossterm::cursor::MoveUp;
        use crossterm::terminal::{size, Clear, ClearType};
        let width = size().map(|(c, _)| c as usize).unwrap_or(100).saturating_sub(1).max(20);
        let mut out = std::io::stdout();
        if self.drawn > 0 {
            let _ = crossterm::execute!(out, MoveUp(self.drawn), Clear(ClearType::FromCursorDown));
        }
        self.logged.resize(jobs.len(), false);
        let total = jobs.len();
        let (mut saved, mut failed, mut queued, mut lines) = (0, 0, 0, Vec::new());
        for (i, (job, state)) in jobs.iter().zip(states).enumerate() {
            let tag = format!("[{}/{total}] {}", i + 1, job.label);
            match &*state.lock().unwrap() {
                Status::Queued => queued += 1,
                Status::Saved(path) => {
                    saved += 1;
                    if !self.logged[i] {
                        self.logged[i] = true;
                        println!("{}", clip(&format!("✓ {tag} · {}", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()), width));
                    }
                }
                Status::Failed(err) => {
                    failed += 1;
                    if !self.logged[i] {
                        self.logged[i] = true;
                        println!("{}", clip(&format!("✗ {tag} · {err}"), width));
                    }
                }
                Status::Resolving => lines.push(format!("  {tag} · finding source")),
                Status::Fetching { source, done, total, bytes } => {
                    lines.push(format!("  {tag} · {source} · {done}/{total} parts · {:.0} MB", *bytes as f64 / 1_048_576.0))
                }
                Status::Remuxing { source } => lines.push(format!("  {tag} · {source} · remuxing")),
            }
        }
        lines.push(format!("{saved} saved · {failed} failed · {queued} queued"));
        for line in &lines {
            println!("{}", clip(line, width));
        }
        let _ = out.flush();
        self.drawn = lines.len() as u16;
        saved + failed == total
    }
}

fn clip(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

fn ensure_ffmpeg() -> Option<String> {
    if let Some(f) = deps::resolve_ffmpeg() {
        return Some(f);
    }
    deps::install_tool("ffmpeg");
    let found = deps::resolve_ffmpeg();
    if found.is_none() {
        println!("ffmpeg is required to download.");
        ui::pause();
    }
    found
}

fn run_job(api: &Api, cfg: &Config, ffmpeg: &str, dir: &Path, job: &Job, set: &(dyn Fn(Status) + Sync)) -> Result<PathBuf> {
    let out = dir.join(&job.filename);
    set(Status::Resolving);
    let streams: Box<dyn Iterator<Item = Stream> + '_> = match &job.target {
        Target::Media { path, movie } => {
            let subs = api.subtitles(path);
            Box::new(api.resolve(path, &job.label, None, *movie).into_iter().map(move |mut s| {
                s.subs = subs.clone();
                s
            }))
        }
        Target::Anime(list) => Box::new(list.iter().filter_map(|(p, id)| api.anime_stream(p, id, &job.label))),
    };
    let mut last = None;
    for stream in streams {
        let source = stream.source.clone();
        set(Status::Fetching { source: source.clone(), done: 0, total: 0, bytes: 0 });
        let progress = |p: Progress| {
            set(match p {
                Progress::Parts(done, total, bytes) => Status::Fetching { source: source.clone(), done, total, bytes },
                Progress::Remux => Status::Remuxing { source: source.clone() },
            })
        };
        match grab(&stream, ffmpeg, &out, cfg, &progress) {
            Ok(()) => return Ok(out),
            Err(err) => last = Some(format!("{source}: {err}")),
        }
    }
    bail!("{}", last.unwrap_or_else(|| "no source has this one".into()))
}

struct Task {
    url: String,
    file: PathBuf,
    ts: bool,
}

pub fn grab(stream: &Stream, ffmpeg: &str, out: &Path, cfg: &Config, progress: &(dyn Fn(Progress) + Sync)) -> Result<()> {
    let work = std::env::temp_dir().join(format!("viel-dl-{}-{}", std::process::id(), rand::random::<u32>()));
    fs::create_dir_all(&work)?;
    let result = grab_into(stream, ffmpeg, out, &work, cfg, progress);
    let _ = fs::remove_dir_all(&work);
    result
}

fn grab_into(stream: &Stream, ffmpeg: &str, out: &Path, work: &Path, cfg: &Config, progress: &(dyn Fn(Progress) + Sync)) -> Result<()> {
    let referer = stream.referer.as_deref();
    let input = if stream.hls {
        let mut tasks = Vec::new();
        let text = net::text(&stream.url, referer)?;
        let main = if hls::is_master(&text) {
            let variant = hls::best(hls::variants(&text, &stream.url), cfg.max_height).ok_or_else(|| anyhow!("no video variant"))?;
            let video = localize(&variant.url, referer, work, "v", &mut tasks)?;
            let audio = variant
                .audio
                .as_deref()
                .and_then(|g| hls::pick_audio(hls::audio_tracks(&text, &stream.url, g), "en"));
            match audio {
                Some(track) => {
                    let audio = localize(&track.url, referer, work, "a", &mut tasks)?;
                    let media = track.line.replace(&format!("URI=\"{}\"", hls::raw_uri(&track.line)), &format!("URI=\"{audio}\""));
                    let inf = strip_attrs(&variant.inf);
                    fs::write(work.join("master.m3u8"), format!("#EXTM3U\n#EXT-X-VERSION:3\n{media}\n{inf}\n{video}\n"))?;
                    "master.m3u8".to_string()
                }
                None => video,
            }
        } else {
            localize_text(&text, &stream.url, work, "v", &mut tasks)?
        };
        fetch_all(&tasks, referer, progress)?;
        work.join(main).display().to_string()
    } else {
        relay::video(&stream.url, referer)
    };

    let sub = stream
        .subs
        .iter()
        .find(|s| s.lang.to_lowercase().starts_with(&cfg.subtitle_lang.to_lowercase()) || lang_name(&s.lang, &cfg.subtitle_lang))
        .and_then(|s| {
            let body = net::bytes(&s.url, s.referer.as_deref()).ok()?;
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
        });

    progress(Progress::Remux);
    if remux(ffmpeg, &input, sub.as_deref(), &cfg.subtitle_lang, out).is_err() && sub.is_some() {
        println!("  subtitle could not be embedded, saving without it");
        return remux(ffmpeg, &input, None, "", out);
    }
    if !out.exists() {
        bail!("ffmpeg produced no file");
    }
    Ok(())
}

fn remux(ffmpeg: &str, input: &str, sub: Option<&Path>, lang: &str, out: &Path) -> Result<()> {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-y", "-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL"]);
    cmd.args(["-protocol_whitelist", "file,crypto,data,http,tcp", "-i", input]);
    if let Some(sub) = sub {
        cmd.arg("-i").arg(sub);
    }
    cmd.args(["-map", "0:v:0?", "-map", "0:a:0?"]);
    if sub.is_some() {
        cmd.args(["-map", "1:0", "-metadata:s:s:0"]).arg(format!("language={lang}"));
    }
    cmd.args(["-c", "copy"]).arg(out);
    let status = cmd.status().context("ffmpeg failed to run")?;
    if !status.success() {
        let _ = fs::remove_file(out);
        bail!("ffmpeg exited with an error");
    }
    Ok(())
}

fn strip_attrs(inf: &str) -> String {
    inf.split(',')
        .filter(|kv| !kv.starts_with("SUBTITLES=") && !kv.starts_with("CLOSED-CAPTIONS="))
        .collect::<Vec<_>>()
        .join(",")
}

fn localize(url: &str, referer: Option<&str>, work: &Path, tag: &str, tasks: &mut Vec<Task>) -> Result<String> {
    let text = net::text(url, referer)?;
    localize_text(&text, url, work, tag, tasks)
}

fn localize_text(text: &str, base: &str, work: &Path, tag: &str, tasks: &mut Vec<Task>) -> Result<String> {
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
        tasks.push(Task { url: abs, file: work.join(&name), ts });
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
    GATE.get_or_init(|| Gate { free: Mutex::new(CONNECTIONS), ready: Condvar::new() })
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

fn fetch_all(tasks: &[Task], referer: Option<&str>, progress: &(dyn Fn(Progress) + Sync)) -> Result<()> {
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
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(i) else { break };
                    let fetched = {
                        let _permit = gate().acquire();
                        net::bytes(&task.url, referer)
                    };
                    match fetched {
                        Ok(body) => {
                            let body = if task.ts { net::unmask_ts(&body) } else { &body[..] };
                            if let Err(err) = fs::write(&task.file, body) {
                                *error.lock().unwrap() = Some(anyhow!(err));
                                failed.store(true, Ordering::Relaxed);
                            }
                            let b = bytes.fetch_add(body.len() as u64, Ordering::Relaxed) + body.len() as u64;
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

fn lang_name(lang: &str, pref: &str) -> bool {
    let names = [("en", "english"), ("es", "spanish"), ("fr", "french"), ("pt", "portuguese"), ("de", "german"), ("it", "italian"), ("ja", "japanese"), ("ar", "arabic"), ("ru", "russian")];
    names
        .iter()
        .find(|(code, _)| *code == pref.to_lowercase())
        .is_some_and(|(_, name)| lang.to_lowercase().starts_with(name))
}

fn movie_name(title: &Title) -> String {
    if title.year.is_empty() {
        format!("{}.mkv", title.name)
    } else {
        format!("{} ({}).mkv", title.name, title.year)
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}
