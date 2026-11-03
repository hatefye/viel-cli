use crate::api::{self, Api, Kind, Stream, Title};
use crate::config::Config;
use crate::hls::{self, Part};
use crate::{deps, net, picker, relay, ui};
use anyhow::{anyhow, bail, Context, Result};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

enum Category {
    Movies,
    Tv,
    Anime,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Category::Movies => "movies",
            Category::Tv => "tv shows",
            Category::Anime => "anime",
        };
        f.write_str(label)
    }
}

enum Scope {
    Episode,
    Range,
    Season,
    Everything,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Scope::Episode => "one episode",
            Scope::Range => "a range of episodes",
            Scope::Season => "a whole season",
            Scope::Everything => "everything",
        };
        f.write_str(label)
    }
}

enum Target {
    Media { path: String, movie: bool },
    Anime(Vec<(String, String)>),
}

struct Job {
    target: Target,
    label: String,
    filename: String,
}

pub fn flow(api: &Api, cfg: &Config) -> Result<()> {
    ui::clear();
    let category = match ui::select("download what?", vec![Category::Movies, Category::Tv, Category::Anime])? {
        Some(c) => c,
        None => return Ok(()),
    };
    ui::clear();
    let Some(query) = ui::text("search")? else {
        return Ok(());
    };
    if query.trim().is_empty() {
        return Ok(());
    }

    let ffmpeg = match ensure_ffmpeg() {
        Some(f) => f,
        None => return Ok(()),
    };

    let jobs = match category {
        Category::Anime => anime_jobs(api, cfg, query.trim())?,
        Category::Movies => media_jobs(api, query.trim(), Kind::Movie)?,
        Category::Tv => media_jobs(api, query.trim(), Kind::Tv)?,
    };
    let jobs = match jobs {
        Some(j) if !j.is_empty() => j,
        _ => return Ok(()),
    };

    let dir = PathBuf::from(&cfg.download_dir);
    std::fs::create_dir_all(&dir).ok();
    ui::clear();
    println!("saving to {}\n", dir.display());
    let total = jobs.len();
    for (i, job) in jobs.iter().enumerate() {
        println!("[{}/{}] {}", i + 1, total, job.label);
        match run_job(api, cfg, &ffmpeg, &dir, job) {
            Ok(path) => println!("saved {}\n", path.display()),
            Err(err) => println!("failed: {err}\n"),
        }
    }
    println!("done.");
    ui::pause();
    Ok(())
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

fn media_jobs(api: &Api, query: &str, kind: Kind) -> Result<Option<Vec<Job>>> {
    let results: Vec<Title> = api.search(query)?.into_iter().filter(|t| t.kind == kind).collect();
    if results.is_empty() {
        println!("nothing found");
        ui::pause();
        return Ok(None);
    }
    let Some(title) = picker::pick_title(api, "download", results)? else {
        return Ok(None);
    };

    if title.kind == Kind::Movie {
        return Ok(Some(vec![Job {
            target: Target::Media { path: title.id.to_string(), movie: true },
            label: title.name.clone(),
            filename: sanitize(&movie_name(&title)),
        }]));
    }

    let scope = match ui::select("what to download", vec![Scope::Episode, Scope::Range, Scope::Season, Scope::Everything])? {
        Some(s) => s,
        None => return Ok(None),
    };
    let seasons = api.seasons(title.id)?;
    if seasons.is_empty() {
        return Ok(None);
    }

    let mut jobs = Vec::new();
    match scope {
        Scope::Everything => {
            for season in &seasons {
                for ep in api.episodes(title.id, season.number)? {
                    jobs.push(tv_job(&title, season.number, ep.number, &ep.name));
                }
            }
        }
        Scope::Season => {
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(None);
            };
            for ep in api.episodes(title.id, season.number)? {
                jobs.push(tv_job(&title, season.number, ep.number, &ep.name));
            }
        }
        Scope::Episode => {
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(None);
            };
            let episodes = api.episodes(title.id, season.number)?;
            let Some(ep) = ui::select("episode", episodes)? else {
                return Ok(None);
            };
            jobs.push(tv_job(&title, season.number, ep.number, &ep.name));
        }
        Scope::Range => {
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(None);
            };
            let (lo, hi) = match ask_range()? {
                Some(r) => r,
                None => return Ok(None),
            };
            for ep in api.episodes(title.id, season.number)? {
                if ep.number >= lo && ep.number <= hi {
                    jobs.push(tv_job(&title, season.number, ep.number, &ep.name));
                }
            }
        }
    }
    Ok(Some(jobs))
}

fn anime_jobs(api: &Api, cfg: &Config, query: &str) -> Result<Option<Vec<Job>>> {
    let results = api.anime_search(query)?;
    if results.is_empty() {
        println!("nothing found");
        ui::pause();
        return Ok(None);
    }
    let Some(title) = picker::pick_title(api, "download", results)? else {
        return Ok(None);
    };
    let sub = api.anime_providers(title.id, false)?;
    let dub = api.anime_providers(title.id, true)?;
    if sub.is_empty() && dub.is_empty() {
        println!("no episodes found");
        ui::pause();
        return Ok(None);
    }
    let want_dub = if !sub.is_empty() && !dub.is_empty() {
        let default = if cfg.audio.eq_ignore_ascii_case("dub") { 1 } else { 0 };
        match ui::select_from("audio", vec!["sub", "dub"], default)? {
            Some(c) => c == "dub",
            None => return Ok(None),
        }
    } else {
        sub.is_empty()
    };
    let providers = if want_dub { dub } else { sub };
    let episodes = api::merge_episodes(&providers);

    let scope = match ui::select("what to download", vec![Scope::Episode, Scope::Range, Scope::Everything])? {
        Some(s) => s,
        None => return Ok(None),
    };
    let selected: Vec<_> = match scope {
        Scope::Everything => episodes,
        Scope::Range => {
            let (lo, hi) = match ask_range()? {
                Some(r) => r,
                None => return Ok(None),
            };
            episodes.into_iter().filter(|e| e.number as u32 >= lo && e.number as u32 <= hi).collect()
        }
        Scope::Episode | Scope::Season => {
            let Some(ep) = ui::select("episode", episodes)? else {
                return Ok(None);
            };
            vec![ep]
        }
    };

    let tag = if want_dub { " (dub)" } else { "" };
    let jobs = selected
        .into_iter()
        .map(|ep| {
            let num = ep.number as u32;
            Job {
                target: Target::Anime(api::episode_sources(&providers, ep.number)),
                label: format!("{} E{num:02}{tag}", title.name),
                filename: sanitize(&format!("{} E{num:02}{tag}.mkv", title.name)),
            }
        })
        .collect();
    Ok(Some(jobs))
}

fn ask_range() -> Result<Option<(u32, u32)>> {
    let Some(first) = ui::text("first episode number")? else {
        return Ok(None);
    };
    let Some(last) = ui::text("last episode number")? else {
        return Ok(None);
    };
    Ok(Some((first.trim().parse().unwrap_or(0), last.trim().parse().unwrap_or(0))))
}

fn tv_job(title: &Title, season: u32, episode: u32, name: &str) -> Job {
    Job {
        target: Target::Media { path: format!("{}/{}/{}", title.id, season, episode), movie: false },
        label: format!("{} S{season:02}E{episode:02} · {name}", title.name),
        filename: sanitize(&format!("{} S{season:02}E{episode:02} {name}.mkv", title.name)),
    }
}

fn run_job(api: &Api, cfg: &Config, ffmpeg: &str, dir: &Path, job: &Job) -> Result<PathBuf> {
    let out = dir.join(&job.filename);
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
    for stream in streams {
        println!("  from {}", stream.source);
        match grab(&stream, ffmpeg, &out, cfg) {
            Ok(()) => return Ok(out),
            Err(err) => println!("  {}: {err}", stream.source),
        }
    }
    bail!("no source could be downloaded")
}

struct Task {
    url: String,
    file: PathBuf,
    ts: bool,
}

pub fn grab(stream: &Stream, ffmpeg: &str, out: &Path, cfg: &Config) -> Result<()> {
    let work = std::env::temp_dir().join(format!("viel-dl-{}-{}", std::process::id(), rand::random::<u32>()));
    fs::create_dir_all(&work)?;
    let result = grab_into(stream, ffmpeg, out, &work, cfg);
    let _ = fs::remove_dir_all(&work);
    result
}

fn grab_into(stream: &Stream, ffmpeg: &str, out: &Path, work: &Path, cfg: &Config) -> Result<()> {
    let referer = stream.referer.as_deref();
    let input = if stream.hls {
        let mut tasks = Vec::new();
        let text = net::text(&stream.url, referer)?;
        let main = if hls::is_master(&text) {
            let variant = hls::best(hls::variants(&text, &stream.url), 1080).ok_or_else(|| anyhow!("no video variant"))?;
            let video = localize(&variant.url, referer, work, "v", &mut tasks)?;
            let audio = variant
                .audio
                .as_deref()
                .and_then(|g| hls::pick_audio(hls::audio_tracks(&text, &stream.url, g), "en"));
            match audio {
                Some(track) => {
                    let audio = localize(&track.url, referer, work, "a", &mut tasks)?;
                    let media = track.line.replace(&format!("URI=\"{}\"", raw_uri(&track.line)), &format!("URI=\"{audio}\""));
                    let inf = strip_attrs(&variant.inf);
                    fs::write(work.join("master.m3u8"), format!("#EXTM3U\n#EXT-X-VERSION:3\n{media}\n{inf}\n{video}\n"))?;
                    "master.m3u8".to_string()
                }
                None => video,
            }
        } else {
            localize_text(&text, &stream.url, work, "v", &mut tasks)?
        };
        fetch_all(&tasks, referer)?;
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

fn raw_uri(line: &str) -> &str {
    line.split("URI=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("")
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

fn fetch_all(tasks: &[Task], referer: Option<&str>) -> Result<()> {
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let bytes = AtomicU64::new(0);
    let failed = AtomicBool::new(false);
    let error = std::sync::Mutex::new(None);
    thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                while !failed.load(Ordering::Relaxed) {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(i) else { break };
                    match net::bytes(&task.url, referer) {
                        Ok(body) => {
                            let body = if task.ts { net::unmask_ts(&body) } else { &body[..] };
                            if let Err(err) = fs::write(&task.file, body) {
                                *error.lock().unwrap() = Some(anyhow!(err));
                                failed.store(true, Ordering::Relaxed);
                            }
                            bytes.fetch_add(body.len() as u64, Ordering::Relaxed);
                            done.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(err) => {
                            *error.lock().unwrap() = Some(err);
                            failed.store(true, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
        let total = tasks.len();
        loop {
            let d = done.load(Ordering::Relaxed);
            let mb = bytes.load(Ordering::Relaxed) as f64 / 1_048_576.0;
            print!("\r  {d}/{total} parts · {mb:.0} MB   ");
            let _ = std::io::stdout().flush();
            if d >= total || failed.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(300));
        }
        println!();
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
