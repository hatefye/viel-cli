use anyhow::Result;
use std::fmt;
use viel::api::{Api, Kind, Title};
use viel::config::Config;
use viel::{player, ui};

enum Home {
    Movies,
    Tv,
    Random,
    Quit,
}

impl fmt::Display for Home {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Home::Movies => "movies",
            Home::Tv => "tv",
            Home::Random => "random",
            Home::Quit => "quit",
        };
        f.write_str(label)
    }
}

fn main() -> Result<()> {
    ui::theme();
    let cfg = Config::load()?;
    let api = Api::new(&cfg.host);
    let greeting = format!("welcome to viel, {}", whoami());

    loop {
        let menu = vec![Home::Movies, Home::Tv, Home::Random, Home::Quit];
        match ui::select(&greeting, menu)? {
            Some(Home::Movies) => browse(&api, &cfg, Kind::Movie)?,
            Some(Home::Tv) => browse(&api, &cfg, Kind::Tv)?,
            Some(Home::Random) => random(&api, &cfg)?,
            Some(Home::Quit) | None => break,
        }
    }
    Ok(())
}

fn browse(api: &Api, cfg: &Config, kind: Kind) -> Result<()> {
    let Some(query) = ui::text("search")? else {
        return Ok(());
    };
    if query.trim().is_empty() {
        return Ok(());
    }
    let results = api.search(kind, query.trim())?;
    if results.is_empty() {
        println!("nothing found");
        return Ok(());
    }
    let Some(title) = ui::select("results", results)? else {
        return Ok(());
    };
    watch(api, cfg, &title)
}

fn random(api: &Api, cfg: &Config) -> Result<()> {
    let trending = api.trending()?;
    if trending.is_empty() {
        return Ok(());
    }
    let pick = rand::random::<usize>() % trending.len();
    let title = &trending[pick];
    println!("rolling: {title}");
    watch(api, cfg, title)
}

fn watch(api: &Api, cfg: &Config, title: &Title) -> Result<()> {
    match title.kind {
        Kind::Movie => {
            let stream = api.resolve(&title.id.to_string(), &title.name)?;
            player::play(&cfg.player, &stream)
        }
        Kind::Tv => {
            let seasons = api.seasons(title.id)?;
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(());
            };
            let episodes = api.episodes(title.id, season.number)?;
            let Some(episode) = ui::select("episode", episodes)? else {
                return Ok(());
            };
            let path = format!("{}/{}/{}", title.id, season.number, episode.number);
            let label = format!("{} · S{}E{} · {}", title.name, season.number, episode.number, episode.name);
            let stream = api.resolve(&path, &label)?;
            player::play(&cfg.player, &stream)
        }
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "friend".into())
}
