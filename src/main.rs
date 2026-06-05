use anyhow::Result;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;
use viel::api::{Api, Kind, ProgressUpdate, Title};
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
            Some(Home::Anime) => anime_browse(&api, &cfg),
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

fn anime_browse(api: &Api, cfg: &Config) -> Result<()> {
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
    anime_flow(api, cfg, &title)
}

fn anime_flow(api: &Api, cfg: &Config, title: &Title) -> Result<()> {
    let mut providers = api.anime_providers(title.id)?;
    let has_sub = providers.iter().any(|p| !p.sub.is_empty());
    let has_dub = providers.iter().any(|p| !p.dub.is_empty());
    if !has_sub && !has_dub {
        println!("no episodes found");
        ui::pause();
        return Ok(());
    }
    let want_dub = cfg.audio.eq_ignore_ascii_case("dub");
    let dub = if want_dub && has_dub {
        true
    } else if !want_dub && has_sub {
        false
    } else {
        has_dub
    };
    let Some(idx) = providers.iter().position(|p| if dub { !p.dub.is_empty() } else { !p.sub.is_empty() }) else {
        return Ok(());
    };
    let provider = providers.swap_remove(idx);
    let episodes = if dub { provider.dub } else { provider.sub };
    ui::clear();
    println!("via {} · {}", provider.name, if dub { "dub" } else { "sub" });
    let Some(episode) = ui::select("episode", episodes)? else {
        return Ok(());
    };
    let label = format!("{} · {}", title.name, episode);
    let stream = api.anime_stream(&episode.id, &label)?;
    player::play(&cfg.player, &stream, |_, _| {})
}

fn watch(api: &Api, cfg: &Config, logged_in: bool, title: &Title) -> Result<()> {
    ui::clear();
    if title.kind == Kind::Anime {
        return anime_flow(api, cfg, title);
    }
    detail(title);
    match title.kind {
        Kind::Anime => anime_flow(api, cfg, title),
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
        Kind::Tv | Kind::Anime => play_episode(api, cfg, true, &title, row.season, row.episode, "", start),
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
    let mut stream = api.resolve(&id, &title.name, start)?;
    attach_subs(api, cfg, &mut stream, &id);
    run(api, cfg, logged_in, "movie", title.id, &title.name, 0, 0, &stream)
}

fn attach_subs(api: &Api, cfg: &Config, stream: &mut viel::api::Stream, path: &str) {
    let mut subs = api.subtitles(path);
    let pref = cfg.subtitle_lang.to_lowercase();
    subs.sort_by_key(|s| !lang_matches(&s.lang, &pref));
    subs.truncate(10);
    stream.subs = subs;
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
) -> Result<()> {
    let path = format!("{}/{}/{}", title.id, season, episode);
    let label = if episode_name.is_empty() {
        format!("{} · S{season}E{episode}", title.name)
    } else {
        format!("{} · S{season}E{episode} · {episode_name}", title.name)
    };
    let mut stream = api.resolve(&path, &label, start)?;
    attach_subs(api, cfg, &mut stream, &path);
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
