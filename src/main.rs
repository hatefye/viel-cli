use anyhow::Result;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;
use viel::api::{self, Api, Kind, ProgressUpdate, Stream, Title};
use viel::config::{self, Config};
use viel::{deps, download, picker, player, ui};

enum Home {
    Continue,
    Search,
    Anime,
    Random,
    Download,
    Account,
    Settings,
    Quit,
}

impl fmt::Display for Home {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Home::Continue => "continue",
            Home::Search => "search",
            Home::Anime => "anime",
            Home::Random => "random",
            Home::Download => "download",
            Home::Account => "account",
            Home::Settings => "settings",
            Home::Quit => "quit",
        };
        f.write_str(label)
    }
}

enum Setting {
    Host,
    Player,
    Downloads,
    Subtitle,
    Audio,
    Back,
}

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Setting::Host => "server host",
            Setting::Player => "player command",
            Setting::Downloads => "download folder",
            Setting::Subtitle => "default subtitle language",
            Setting::Audio => "default anime audio (sub/dub)",
            Setting::Back => "back",
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

fn main() {
    if let Err(err) = app() {
        eprintln!("\nerror: {err}");
        ui::pause();
    }
}

fn app() -> Result<()> {
    ui::theme();
    let mut cfg = Config::load()?;
    let mut api = Api::new(&cfg.host, config::session());

    let report = deps::check(&api, &cfg.player);
    if !report.player {
        ui::clear();
        let mut found = deps::resolve_player(&cfg.player);
        if found.is_none() {
            deps::install_tool("mpv");
            found = deps::resolve_player(&cfg.player);
        }
        if found.is_none() {
            found = deps::prompt_player_path();
        }
        match found {
            Some(path) => {
                if path != cfg.player {
                    println!("using mpv at {path}");
                    cfg.player = path;
                    let _ = cfg.save();
                }
            }
            None => sleep(Duration::from_millis(700)),
        }
    }
    if !report.host {
        ui::clear();
        println!("could not reach {}. check your connection or VIEL_HOST.", cfg.host);
        sleep(Duration::from_millis(1200));
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
        menu.push(Home::Anime);
        menu.push(Home::Random);
        menu.push(Home::Download);
        menu.push(Home::Account);
        menu.push(Home::Settings);
        menu.push(Home::Quit);

        let result = match ui::select(&greeting, menu)? {
            Some(Home::Continue) => resume(&api, &cfg),
            Some(Home::Search) => browse(&api, &cfg, user.is_some()),
            Some(Home::Anime) => anime_browse(&api, &cfg, user.is_some()),
            Some(Home::Random) => random(&api, &cfg, user.is_some()),
            Some(Home::Download) => download::flow(&api, &cfg),
            Some(Home::Account) => account(&mut api, &cfg, &mut user),
            Some(Home::Settings) => {
                let res = settings(&mut cfg);
                api = Api::new(&cfg.host, config::session());
                res
            }
            Some(Home::Quit) | None => break,
        };
        if let Err(err) = result {
            ui::clear();
            println!("something went wrong:\n  {err}");
            let message = err.to_string();
            if message.contains("exited") || message.contains("launch") {
                println!("\nthat usually means the configured player isn't real mpv or can't run.");
                if ui::confirm("point viel at a different mpv?").unwrap_or(false) {
                    if let Some(path) = deps::prompt_player_path() {
                        cfg.player = path;
                        let _ = cfg.save();
                        println!("saved.");
                    }
                }
            }
            ui::pause();
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

fn settings(cfg: &mut Config) -> Result<()> {
    loop {
        ui::clear();
        println!("server host:       {}", cfg.host);
        println!("player command:    {}", cfg.player);
        println!("download folder:   {}", cfg.download_dir);
        println!("subtitle language: {}", cfg.subtitle_lang);
        println!("anime audio:       {}\n", cfg.audio);
        let options = vec![
            Setting::Host,
            Setting::Player,
            Setting::Downloads,
            Setting::Subtitle,
            Setting::Audio,
            Setting::Back,
        ];
        match ui::select("settings", options)? {
            Some(Setting::Host) => edit_field(&mut cfg.host, "server host (e.g. https://viel.tv)")?,
            Some(Setting::Player) => edit_field(&mut cfg.player, "player command or full path")?,
            Some(Setting::Downloads) => edit_field(&mut cfg.download_dir, "download folder")?,
            Some(Setting::Subtitle) => edit_field(&mut cfg.subtitle_lang, "subtitle language code (e.g. en, es, ja)")?,
            Some(Setting::Audio) => {
                if let Some(choice) = ui::select("default anime audio", vec!["sub", "dub"])? {
                    cfg.audio = choice.to_string();
                }
            }
            Some(Setting::Back) | None => return Ok(()),
        }
        cfg.save()?;
    }
}

fn edit_field(field: &mut String, prompt: &str) -> Result<()> {
    if let Some(value) = ui::text(prompt)? {
        let value = value.trim();
        if !value.is_empty() {
            *field = value.to_string();
        }
    }
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
    let Some(title) = picker::pick_title(api, "results", results)? else {
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

fn anime_browse(api: &Api, cfg: &Config, logged_in: bool) -> Result<()> {
    ui::clear();
    let Some(query) = ui::text("search anime")? else {
        return Ok(());
    };
    if query.trim().is_empty() {
        return Ok(());
    }
    let results = api.anime_search(query.trim())?;
    if results.is_empty() {
        println!("nothing found");
        sleep(Duration::from_millis(900));
        return Ok(());
    }
    let Some(title) = picker::pick_title(api, "anime", results)? else {
        return Ok(());
    };
    anime_flow(api, cfg, logged_in, &title, None)
}

fn anime_flow(api: &Api, cfg: &Config, logged_in: bool, title: &Title, resume: Option<(u32, Option<f64>)>) -> Result<()> {
    let want_dub = cfg.audio.eq_ignore_ascii_case("dub");
    let mut dub = want_dub;
    let mut providers = api.anime_providers(title.id, dub)?;
    if providers.is_empty() {
        dub = !dub;
        providers = api.anime_providers(title.id, dub)?;
    }
    if providers.is_empty() {
        println!("no episodes found");
        ui::pause();
        return Ok(());
    }
    let episodes = api::merge_episodes(&providers);
    let mut index = match resume {
        Some((ep, _)) => episodes.iter().position(|e| e.number as u32 == ep).unwrap_or(0),
        None => {
            ui::clear();
            if dub != want_dub {
                println!("only {} is available for this one\n", if dub { "dub" } else { "sub" });
            }
            let names: Vec<String> = episodes.iter().map(|e| e.to_string()).collect();
            let Some(pick) = ui::select("episode", names)? else {
                return Ok(());
            };
            episodes.iter().position(|e| e.to_string() == pick).unwrap_or(0)
        }
    };
    let mut start = resume.and_then(|(_, s)| s);
    loop {
        let ep = &episodes[index];
        let label = format!("{} · {}{}", title.name, ep, if dub { " (dub)" } else { "" });
        ui::clear();
        println!("finding the best source for {label}");
        let sources = api::episode_sources(&providers, ep.number);
        let streams = sources.iter().filter_map(|(p, id)| api.anime_stream(p, id, &label)).map(|mut s| {
            s.start = start;
            s
        });
        let finished = play_any(api, cfg, logged_in, "anime", title.id, ep.title.as_deref().unwrap_or(&title.name), 0, ep.number as u32, streams)?;
        start = None;
        if !finished || index + 1 >= episodes.len() || !ui::confirm("next episode?")? {
            return Ok(());
        }
        index += 1;
    }
}

fn watch(api: &Api, cfg: &Config, logged_in: bool, title: &Title) -> Result<()> {
    ui::clear();
    if title.kind == Kind::Anime {
        return anime_flow(api, cfg, logged_in, title, None);
    }
    detail(title);
    match title.kind {
        Kind::Anime => anime_flow(api, cfg, logged_in, title, None),
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
            let catalog: Vec<(u32, String)> = episodes.iter().map(|e| (e.number, e.name.clone())).collect();
            let Some(episode) = ui::select("episode", episodes)? else {
                return Ok(());
            };
            let mut number = episode.number;
            let mut name = episode.name.clone();
            let last = catalog.iter().map(|(n, _)| *n).max().unwrap_or(number);
            while play_episode(api, cfg, logged_in, title, season.number, number, &name, None)? && number < last && ui::confirm("next episode?")? {
                number += 1;
                name = catalog.iter().find(|(n, _)| *n == number).map(|(_, t)| t.clone()).unwrap_or_default();
            }
            Ok(())
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
        Kind::Tv => play_episode(api, cfg, true, &title, row.season, row.episode, "", start).map(|_| ()),
        Kind::Anime => anime_flow(api, cfg, true, &title, Some((row.episode, start))),
    }
}

fn detail(title: &Title) {
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
    let id = title.id.to_string();
    ui::clear();
    println!("finding the best source for {}", title.name);
    let streams = with_subs(api, cfg, &id, api.resolve(&id, &title.name, start, true));
    play_any(api, cfg, logged_in, "movie", title.id, &title.name, 0, 0, streams.into_iter()).map(|_| ())
}

fn with_subs(api: &Api, cfg: &Config, path: &str, mut streams: Vec<Stream>) -> Vec<Stream> {
    let pref = cfg.subtitle_lang.to_lowercase();
    let mut subs: Vec<_> = api
        .subtitles(path)
        .into_iter()
        .filter(|s| lang_matches(&s.lang, &pref) || lang_matches(&s.lang, "en"))
        .collect();
    subs.sort_by_key(|s| !lang_matches(&s.lang, &pref));
    subs.truncate(3);
    for s in &mut streams {
        s.subs = subs.clone();
    }
    streams
}

fn lang_matches(lang: &str, pref: &str) -> bool {
    let l = lang.to_lowercase();
    if l.starts_with(pref) {
        return true;
    }
    let name = match pref {
        "en" => "english",
        "es" => "spanish",
        "fr" => "french",
        "pt" => "portuguese",
        "de" => "german",
        "ja" => "japanese",
        "ar" => "arabic",
        "it" => "italian",
        "ru" => "russian",
        "ko" => "korean",
        "zh" => "chinese",
        _ => "",
    };
    !name.is_empty() && l.starts_with(name)
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
) -> Result<bool> {
    let path = format!("{}/{}/{}", title.id, season, episode);
    let label = if episode_name.is_empty() {
        format!("{} · S{season}E{episode}", title.name)
    } else {
        format!("{} · S{season}E{episode} · {episode_name}", title.name)
    };
    ui::clear();
    println!("finding the best source for {label}");
    let streams = with_subs(api, cfg, &path, api.resolve(&path, &label, start, false));
    play_any(api, cfg, logged_in, "tv", title.id, &title.name, season, episode, streams.into_iter())
}

#[allow(clippy::too_many_arguments)]
fn play_any(
    api: &Api,
    cfg: &Config,
    logged_in: bool,
    kind: &str,
    id: u64,
    history_title: &str,
    season: u32,
    episode: u32,
    streams: impl Iterator<Item = Stream>,
) -> Result<bool> {
    let mut last_err = None;
    for stream in streams {
        println!("playing from {}", stream.source);
        let mut watched = (0.0, 0.0);
        let result = player::play(&cfg.player, &stream, |position, duration| {
            watched = (position, duration);
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
        });
        match result {
            Ok(()) => return Ok(watched.1 > 0.0 && watched.1 - watched.0 < 120.0),
            Err(err) => {
                println!("{} failed, trying the next source", stream.source);
                last_err = Some(err);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no working source found for this one")))
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit).collect();
    out.push('…');
    out
}
