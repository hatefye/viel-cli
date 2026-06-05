use crate::api::{Api, Kind, Title};
use crate::config::Config;
use crate::{deps, ui};
use anyhow::{bail, Context, Result};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Target {
    season: u32,
    episode: u32,
    name: String,
    label: String,
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
            Scope::Everything => "every season",
        };
        f.write_str(label)
    }
}

pub fn flow(api: &Api, cfg: &Config) -> Result<()> {
    ui::clear();
    let Some(query) = ui::text("search to download")? else {
        return Ok(());
    };
    if query.trim().is_empty() {
        return Ok(());
    }
    let results = api.search(query.trim())?;
    if results.is_empty() {
        println!("nothing found");
        ui::pause();
        return Ok(());
    }
    let Some(title) = ui::select("results", results)? else {
        return Ok(());
    };

    let ffmpeg = match deps::resolve_ffmpeg() {
        Some(f) => f,
        None => {
            deps::install_tool("ffmpeg");
            match deps::resolve_ffmpeg() {
                Some(f) => f,
                None => {
                    println!("ffmpeg is required to download.");
                    ui::pause();
                    return Ok(());
                }
            }
        }
    };

    let targets = match collect_targets(api, &title)? {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()),
    };

    let dir = PathBuf::from(&cfg.download_dir);
    std::fs::create_dir_all(&dir).ok();
    ui::clear();
    println!("saving to {}", dir.display());
    let total = targets.len();
    for (i, target) in targets.iter().enumerate() {
        println!("\n[{}/{}] {}", i + 1, total, target.label);
        match download_one(api, &ffmpeg, &dir, &title, target) {
            Ok(path) => println!("saved {}", path.display()),
            Err(err) => println!("failed: {err}"),
        }
    }
    println!("\ndone.");
    ui::pause();
    Ok(())
}

fn collect_targets(api: &Api, title: &Title) -> Result<Option<Vec<Target>>> {
    if title.kind == Kind::Movie {
        return Ok(Some(vec![Target {
            season: 0,
            episode: 0,
            name: title.name.clone(),
            label: title.name.clone(),
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

    match scope {
        Scope::Everything => {
            let mut out = Vec::new();
            for season in &seasons {
                for ep in api.episodes(title.id, season.number)? {
                    out.push(make_target(title, season.number, ep.number, &ep.name));
                }
            }
            Ok(Some(out))
        }
        Scope::Season => {
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(None);
            };
            let out = api
                .episodes(title.id, season.number)?
                .into_iter()
                .map(|ep| make_target(title, season.number, ep.number, &ep.name))
                .collect();
            Ok(Some(out))
        }
        Scope::Episode => {
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(None);
            };
            let episodes = api.episodes(title.id, season.number)?;
            let Some(ep) = ui::select("episode", episodes)? else {
                return Ok(None);
            };
            Ok(Some(vec![make_target(title, season.number, ep.number, &ep.name)]))
        }
        Scope::Range => {
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(None);
            };
            let episodes = api.episodes(title.id, season.number)?;
            let Some(first) = ui::text("first episode number")? else {
                return Ok(None);
            };
            let Some(last) = ui::text("last episode number")? else {
                return Ok(None);
            };
            let lo: u32 = first.trim().parse().unwrap_or(0);
            let hi: u32 = last.trim().parse().unwrap_or(0);
            let out = episodes
                .into_iter()
                .filter(|ep| ep.number >= lo && ep.number <= hi)
                .map(|ep| make_target(title, season.number, ep.number, &ep.name))
                .collect();
            Ok(Some(out))
        }
    }
}

fn make_target(title: &Title, season: u32, episode: u32, name: &str) -> Target {
    Target {
        season,
        episode,
        name: name.to_string(),
        label: format!("{} S{season:02}E{episode:02} · {name}", title.name),
    }
}

fn download_one(api: &Api, ffmpeg: &str, dir: &Path, title: &Title, target: &Target) -> Result<PathBuf> {
    let (path, label) = if target.season == 0 {
        (title.id.to_string(), title.name.clone())
    } else {
        (
            format!("{}/{}/{}", title.id, target.season, target.episode),
            target.label.clone(),
        )
    };
    let stream = api.resolve(&path, &label, None)?;
    let out = dir.join(filename(title, target));
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

fn filename(title: &Title, target: &Target) -> String {
    let raw = if target.season == 0 {
        let year = if title.year.is_empty() {
            String::new()
        } else {
            format!(" ({})", title.year)
        };
        format!("{}{year}.mkv", title.name)
    } else {
        format!("{} S{:02}E{:02} {}.mkv", title.name, target.season, target.episode, target.name)
    };
    sanitize(&raw)
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}
