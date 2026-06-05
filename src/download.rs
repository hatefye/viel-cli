use crate::api::{AnimeProvider, Api, Kind, Title};
use crate::config::Config;
use crate::{deps, picker, ui};
use anyhow::{bail, Context, Result};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

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

struct Job {
    path: String,
    label: String,
    filename: String,
    anime: bool,
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
        Category::Anime => anime_jobs(api, query.trim())?,
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
        match run_ffmpeg(api, &ffmpeg, &dir, job) {
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
            path: title.id.to_string(),
            label: title.name.clone(),
            filename: sanitize(&movie_name(&title)),
            anime: false,
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

fn anime_jobs(api: &Api, query: &str) -> Result<Option<Vec<Job>>> {
    let results = api.anime_search(query)?;
    if results.is_empty() {
        println!("nothing found");
        ui::pause();
        return Ok(None);
    }
    let Some(title) = picker::pick_title(api, "download", results)? else {
        return Ok(None);
    };
    let mut providers = api.anime_providers(title.id)?;
    let has_sub = providers.iter().any(|p| !p.sub.is_empty());
    let has_dub = providers.iter().any(|p| !p.dub.is_empty());
    if !has_sub && !has_dub {
        println!("no episodes found");
        ui::pause();
        return Ok(None);
    }
    let dub = if has_sub && has_dub {
        match ui::select("audio", vec!["sub", "dub"])? {
            Some(c) => c == "dub",
            None => return Ok(None),
        }
    } else {
        has_dub
    };
    let Some(idx) = providers.iter().position(|p: &AnimeProvider| if dub { !p.dub.is_empty() } else { !p.sub.is_empty() }) else {
        return Ok(None);
    };
    let provider = providers.swap_remove(idx);
    let episodes = if dub { provider.dub } else { provider.sub };

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

    let jobs = selected
        .into_iter()
        .map(|ep| {
            let num = ep.number as u32;
            Job {
                path: ep.id,
                label: format!("{} E{num:02}", title.name),
                filename: sanitize(&format!("{} E{num:02}.mkv", title.name)),
                anime: true,
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
        path: format!("{}/{}/{}", title.id, season, episode),
        label: format!("{} S{season:02}E{episode:02} · {name}", title.name),
        filename: sanitize(&format!("{} S{season:02}E{episode:02} {name}.mkv", title.name)),
        anime: false,
    }
}

fn run_ffmpeg(api: &Api, ffmpeg: &str, dir: &Path, job: &Job) -> Result<PathBuf> {
    let stream = if job.anime {
        api.anime_stream(&job.path, &job.label)?
    } else {
        api.resolve(&job.path, &job.label, None)?
    };
    let out = dir.join(&job.filename);
    let status = Command::new(ffmpeg)
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "warning",
            "-stats",
            "-i",
            &stream.url,
            "-c",
            "copy",
            out.to_str().unwrap_or_default(),
        ])
        .status()
        .context("ffmpeg failed to run")?;
    if !status.success() {
        bail!("ffmpeg exited with an error");
    }
    Ok(out)
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
