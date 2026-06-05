use anyhow::Result;
use std::fmt;
use viel::api::{Api, HistoryEntry, Kind, ProgressUpdate, Title};
use viel::config::{self, Config};
use viel::{player, ui};

enum Home {
    Continue,
    Movies,
    Tv,
    Random,
    Login,
    Logout,
    Quit,
}

impl fmt::Display for Home {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Home::Continue => "continue",
            Home::Movies => "movies",
            Home::Tv => "tv",
            Home::Random => "random",
            Home::Login => "log in",
            Home::Logout => "log out",
            Home::Quit => "quit",
        };
        f.write_str(label)
    }
}

fn main() -> Result<()> {
    ui::theme();
    let cfg = Config::load()?;
    let mut api = Api::new(&cfg.host, config::session());
    let mut user = api.me().unwrap_or(None);

    loop {
        let history = if user.is_some() { api.history().unwrap_or_default() } else { Vec::new() };
        let greeting = match &user {
            Some(name) => format!("welcome to viel, {name}"),
            None => "welcome to viel".into(),
        };

        let mut menu = Vec::new();
        if !history.is_empty() {
            menu.push(Home::Continue);
        }
        menu.push(Home::Movies);
        menu.push(Home::Tv);
        menu.push(Home::Random);
        menu.push(if user.is_some() { Home::Logout } else { Home::Login });
        menu.push(Home::Quit);

        match ui::select(&greeting, menu)? {
            Some(Home::Continue) => resume(&api, &cfg, history)?,
            Some(Home::Movies) => browse(&api, &cfg, user.is_some(), Kind::Movie)?,
            Some(Home::Tv) => browse(&api, &cfg, user.is_some(), Kind::Tv)?,
            Some(Home::Random) => random(&api, &cfg, user.is_some())?,
            Some(Home::Login) => {
                if let Some(token) = login(&api)? {
                    config::save_session(&token)?;
                    api = Api::new(&cfg.host, Some(token));
                    user = api.me().unwrap_or(None);
                }
            }
            Some(Home::Logout) => {
                config::clear_session();
                api = Api::new(&cfg.host, None);
                user = None;
            }
            Some(Home::Quit) | None => break,
        }
    }
    Ok(())
}

fn login(api: &Api) -> Result<Option<String>> {
    let Some(username) = ui::text("username")? else {
        return Ok(None);
    };
    let Some(password) = ui::password("password")? else {
        return Ok(None);
    };
    match api.login(username.trim(), &password) {
        Ok(token) => {
            println!("logged in");
            Ok(Some(token))
        }
        Err(err) => {
            println!("{err}");
            Ok(None)
        }
    }
}

fn browse(api: &Api, cfg: &Config, logged_in: bool, kind: Kind) -> Result<()> {
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
    watch(api, cfg, logged_in, &title)
}

fn random(api: &Api, cfg: &Config, logged_in: bool) -> Result<()> {
    let trending = api.trending()?;
    if trending.is_empty() {
        return Ok(());
    }
    let title = &trending[rand::random::<usize>() % trending.len()];
    println!("rolling: {title}");
    watch(api, cfg, logged_in, title)
}

fn watch(api: &Api, cfg: &Config, logged_in: bool, title: &Title) -> Result<()> {
    match title.kind {
        Kind::Movie => play_movie(api, cfg, logged_in, title, None),
        Kind::Tv => {
            let seasons = api.seasons(title.id)?;
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(());
            };
            let episodes = api.episodes(title.id, season.number)?;
            let Some(episode) = ui::select("episode", episodes)? else {
                return Ok(());
            };
            play_episode(api, cfg, logged_in, title, season.number, episode.number, &episode.name, None)
        }
    }
}

fn resume(api: &Api, cfg: &Config, history: Vec<HistoryEntry>) -> Result<()> {
    let Some(entry) = ui::select("continue", history)? else {
        return Ok(());
    };
    let id: u64 = match entry.content_id.parse() {
        Ok(id) => id,
        Err(_) => return Ok(()),
    };
    let start = (entry.position > 5.0 && entry.position < entry.duration - 30.0).then_some(entry.position);
    let title = Title {
        id,
        kind: if entry.kind == "tv" { Kind::Tv } else { Kind::Movie },
        name: entry.name().to_string(),
        year: String::new(),
        rating: 0.0,
    };
    match title.kind {
        Kind::Movie => play_movie(api, cfg, true, &title, start),
        Kind::Tv => play_episode(
            api,
            cfg,
            true,
            &title,
            entry.season as u32,
            entry.episode as u32,
            &title.name,
            start,
        ),
    }
}

fn play_movie(api: &Api, cfg: &Config, logged_in: bool, title: &Title, start: Option<f64>) -> Result<()> {
    let stream = api.resolve(&title.id.to_string(), &title.name, start)?;
    run(api, cfg, logged_in, "movie", title.id, &title.name, 0, 0, &stream)
}

#[allow(clippy::too_many_arguments)]
fn play_episode(
    api: &Api,
    cfg: &Config,
    logged_in: bool,
    title: &Title,
    season: u32,
    episode: u32,
    episode_name: &str,
    start: Option<f64>,
) -> Result<()> {
    let path = format!("{}/{}/{}", title.id, season, episode);
    let label = format!("{} · S{season}E{episode} · {episode_name}", title.name);
    let stream = api.resolve(&path, &label, start)?;
    run(api, cfg, logged_in, "tv", title.id, &title.name, season, episode, &stream)
}

#[allow(clippy::too_many_arguments)]
fn run(
    api: &Api,
    cfg: &Config,
    logged_in: bool,
    kind: &str,
    id: u64,
    history_title: &str,
    season: u32,
    episode: u32,
    stream: &viel::api::Stream,
) -> Result<()> {
    player::play(&cfg.player, stream, |position, duration| {
        if logged_in && duration > 0.0 {
            let _ = api.save_progress(&ProgressUpdate {
                kind,
                content_id: id.to_string(),
                season,
                episode,
                position,
                duration,
                title: history_title,
            });
        }
    })
}
