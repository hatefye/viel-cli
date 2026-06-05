use anyhow::Result;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;
use viel::api::{Api, Kind, ProgressUpdate, Title};
use viel::config::{self, Config};
use viel::{deps, player, ui};

enum Home {
    Continue,
    Search,
    Random,
    Account,
    Quit,
}

impl fmt::Display for Home {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Home::Continue => "continue",
            Home::Search => "search",
            Home::Random => "random",
            Home::Account => "account",
            Home::Quit => "quit",
        };
        f.write_str(label)
    }
}

enum Account {
    SignIn,
    SignUp,
    LogOut,
    Back,
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Account::SignIn => "sign in",
            Account::SignUp => "sign up",
            Account::LogOut => "log out",
            Account::Back => "back",
        };
        f.write_str(label)
    }
}

fn main() -> Result<()> {
    ui::theme();
    let cfg = Config::load()?;
    let mut api = Api::new(&cfg.host, config::session());

    ui::clear();
    let report = deps::check(&api, &cfg.player);
    if !report.player {
        println!("{} is needed to play video and was not found on your PATH.", cfg.player);
        if ui::confirm("open the install page?")? {
            deps::open_install_page();
        }
    }
    if !report.host {
        println!("could not reach {}. check your connection or VIEL_HOST.", cfg.host);
    }

    let mut user = api.me().unwrap_or(None);

    loop {
        ui::clear();
        let has_history = user.is_some() && !api.history().unwrap_or_default().is_empty();
        let greeting = match &user {
            Some(name) => format!("welcome to viel, {name}"),
            None => "welcome to viel".into(),
        };

        let mut menu = Vec::new();
        if has_history {
            menu.push(Home::Continue);
        }
        menu.push(Home::Search);
        menu.push(Home::Random);
        menu.push(Home::Account);
        menu.push(Home::Quit);

        match ui::select(&greeting, menu)? {
            Some(Home::Continue) => resume(&api, &cfg)?,
            Some(Home::Search) => browse(&api, &cfg, user.is_some())?,
            Some(Home::Random) => random(&api, &cfg, user.is_some())?,
            Some(Home::Account) => account(&mut api, &cfg, &mut user)?,
            Some(Home::Quit) | None => break,
        }
    }
    Ok(())
}

fn account(api: &mut Api, cfg: &Config, user: &mut Option<String>) -> Result<()> {
    ui::clear();
    let options = match user {
        Some(name) => {
            println!("signed in as {name}\n");
            vec![Account::LogOut, Account::Back]
        }
        None => vec![Account::SignIn, Account::SignUp, Account::Back],
    };
    match ui::select("account", options)? {
        Some(Account::SignIn) => authenticate(api, cfg, user, false),
        Some(Account::SignUp) => authenticate(api, cfg, user, true),
        Some(Account::LogOut) => {
            config::clear_session();
            *api = Api::new(&cfg.host, None);
            *user = None;
            Ok(())
        }
        Some(Account::Back) | None => Ok(()),
    }
}

fn authenticate(api: &mut Api, cfg: &Config, user: &mut Option<String>, signup: bool) -> Result<()> {
    let Some(username) = ui::text("username")? else {
        return Ok(());
    };
    let password = if signup {
        ui::new_password("password")?
    } else {
        ui::password("password")?
    };
    let Some(password) = password else {
        return Ok(());
    };
    let attempt = if signup {
        api.register(username.trim(), &password)
    } else {
        api.login(username.trim(), &password)
    };
    match attempt {
        Ok(token) => {
            config::save_session(&token)?;
            *api = Api::new(&cfg.host, Some(token));
            *user = api.me().unwrap_or(None);
            println!("{}", if signup { "account created" } else { "signed in" });
        }
        Err(err) => println!("{err}"),
    }
    sleep(Duration::from_millis(900));
    Ok(())
}

fn browse(api: &Api, cfg: &Config, logged_in: bool) -> Result<()> {
    ui::clear();
    let Some(query) = ui::text("search")? else {
        return Ok(());
    };
    if query.trim().is_empty() {
        return Ok(());
    }
    let results = api.search(query.trim())?;
    if results.is_empty() {
        println!("nothing found");
        sleep(Duration::from_millis(900));
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
    let index = rand::random::<usize>() % trending.len();
    watch(api, cfg, logged_in, &trending[index])
}

fn watch(api: &Api, cfg: &Config, logged_in: bool, title: &Title) -> Result<()> {
    ui::clear();
    detail(api, title);
    match title.kind {
        Kind::Movie => {
            if ui::confirm("play this?")? {
                play_movie(api, cfg, logged_in, title, None)?;
            }
            Ok(())
        }
        Kind::Tv => {
            let seasons = api.seasons(title.id)?;
            let Some(season) = ui::select("season", seasons)? else {
                return Ok(());
            };
            ui::clear();
            let episodes = api.episodes(title.id, season.number)?;
            let Some(episode) = ui::select("episode", episodes)? else {
                return Ok(());
            };
            play_episode(api, cfg, logged_in, title, season.number, episode.number, &episode.name, None)
        }
    }
}

fn resume(api: &Api, cfg: &Config) -> Result<()> {
    let rows = api.continue_rows()?;
    if rows.is_empty() {
        return Ok(());
    }
    ui::clear();
    let Some(row) = ui::select("continue", rows)? else {
        return Ok(());
    };
    let start = row.resume_at();
    let title = Title {
        id: row.id,
        kind: row.kind,
        name: row.show.clone(),
        year: String::new(),
        rating: 0.0,
        overview: None,
        poster_path: None,
    };
    match row.kind {
        Kind::Movie => play_movie(api, cfg, true, &title, start),
        Kind::Tv => play_episode(api, cfg, true, &title, row.season, row.episode, "", start),
    }
}

fn detail(api: &Api, title: &Title) {
    if let Some(path) = &title.poster_path {
        if let Some(bytes) = api.poster(path) {
            ui::poster(&bytes);
        }
    }
    let year = if title.year.is_empty() {
        String::new()
    } else {
        format!(" ({})", title.year)
    };
    println!("\x1b[1m{}\x1b[0m{}", title.name, year);
    if title.rating > 0.0 {
        println!("★ {:.1} · {}", title.rating, title.kind);
    } else {
        println!("{}", title.kind);
    }
    if let Some(overview) = &title.overview {
        println!("\n{}", truncate(overview, 320));
    }
    println!();
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
    let label = if episode_name.is_empty() {
        format!("{} · S{season}E{episode}", title.name)
    } else {
        format!("{} · S{season}E{episode} · {episode_name}", title.name)
    };
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

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit).collect();
    out.push('…');
    out
}
