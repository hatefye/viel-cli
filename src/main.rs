use anyhow::Result;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use viel::api::{self, Api, ContinueRow, Kind, ProgressUpdate, Stream, Title};
use viel::config::{self, Config};
use viel::download::{self, Job};
use viel::screen::{self, Out, Row, Screen};
use viel::{deps, player, ui};

const HOME_HINT: &str = "type to search · ⏎ open · ^d download · ^a account · ^s settings · esc quit";
const TV_HINT: &str = "⏎ play · d download · D download more · esc back";
const ANIME_HINT: &str = "⏎ play · d download · D download more · a sub/dub · esc back";

#[derive(Clone)]
enum Item {
    Title(Title),
    Resume(ContinueRow),
}

fn item_row(item: &Item) -> Row {
    match item {
        Item::Title(t) => {
            let year = if t.year.is_empty() { String::new() } else { format!(" ({})", t.year) };
            let meta = if t.rating > 0.0 {
                format!("{}{} · ★ {:.1}", t.kind, year, t.rating)
            } else {
                format!("{}{}", t.kind, year)
            };
            let label = if t.seasons.len() > 1 {
                format!("{}{} · {} seasons · {}", t.name, year, t.seasons.len(), t.kind)
            } else {
                format!("{}{} · {}", t.name, year, t.kind)
            };
            Row {
                label,
                title: t.name.clone(),
                meta,
                body: t.overview.clone().unwrap_or_default(),
                poster: t.poster_path.clone(),
            }
        }
        Item::Resume(r) => Row {
            label: format!("▶ {r}"),
            title: r.show.clone(),
            meta: "continue watching".into(),
            body: String::new(),
            poster: r.poster.clone(),
        },
    }
}

#[derive(Clone)]
struct Ep {
    season: u32,
    number: f64,
    name: String,
    row: Row,
    sources: Vec<(String, String)>,
}

fn ep_row(ep: &Ep) -> Row {
    ep.row.clone()
}

enum Setting {
    Host,
    Player,
    Downloads,
    Subtitle,
    Audio,
    Parallel,
    Quality,
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
            Setting::Parallel => "parallel downloads",
            Setting::Quality => "max quality",
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

enum Scope {
    Season,
    Everything,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Scope::Season => "this season",
            Scope::Everything => "every episode",
        })
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
    screen::init_images();
    let mut cfg = Config::load()?;
    ensure_player(&mut cfg);
    viel::relay::set_max_height(cfg.max_height);
    viel::relay::set_audio_lang(&cfg.subtitle_lang);
    let mut api = Api::new(&cfg.host, config::session());
    let mut user = api.me().unwrap_or(None);
    let trending: Arc<Mutex<Option<Vec<Title>>>> = Arc::default();
    let mut query = String::new();
    let mut items: Vec<Item> = Vec::new();
    let mut status = None;

    loop {
        let mut home = Screen::new("browse", HOME_HINT, std::mem::take(&mut items), item_row);
        home.query = Some(query.clone());
        home.ctrl = vec!['d', 'a', 's'];
        home.status = status.take();
        home.search = Some(home_search(&api, user.is_some(), trending.clone()));
        let out = screen::run(&api, &mut home)?;
        query = home.query.take().unwrap_or_default();
        items = home.items;
        match out {
            Out::Back => break,
            Out::Pick(i) => status = open(&api, &cfg, user.is_some(), items[i].clone(), false).err().map(|e| e.to_string()),
            Out::Key('d', i) => status = open(&api, &cfg, user.is_some(), items[i].clone(), true).err().map(|e| e.to_string()),
            Out::Key('a', _) => {
                account(&mut api, &cfg, &mut user)?;
                items.clear();
            }
            Out::Key('s', _) => {
                settings(&mut cfg)?;
                api = Api::new(&cfg.host, config::session());
                items.clear();
            }
            Out::Key(..) => {}
        }
        if query.is_empty() {
            items.clear();
        }
    }
    Ok(())
}

fn home_search(api: &Api, signed_in: bool, trending: Arc<Mutex<Option<Vec<Title>>>>) -> screen::Search<Item> {
    let api = api.clone();
    Arc::new(move |q: &str| {
        let q = q.trim();
        if q.is_empty() {
            let mut rows: Vec<Item> = if signed_in {
                api.continue_rows().unwrap_or_default().into_iter().map(Item::Resume).collect()
            } else {
                Vec::new()
            };
            let mut cache = trending.lock().unwrap();
            if cache.is_none() {
                *cache = Some(api.trending().unwrap_or_default());
            }
            rows.extend(cache.clone().unwrap_or_default().into_iter().map(Item::Title));
            return rows;
        }
        let (media, anime) = std::thread::scope(|s| {
            let a = s.spawn(|| api.search(q).unwrap_or_default());
            let b = s.spawn(|| api.anime_search_merged(q));
            (a.join().unwrap_or_default(), b.join().unwrap_or_default())
        });
        let mut rows: Vec<Item> = anime.into_iter().take(6).map(Item::Title).collect();
        rows.extend(media.into_iter().take(14).map(Item::Title));
        rows
    })
}

fn ensure_player(cfg: &mut Config) {
    let mut found = deps::resolve_player(&cfg.player);
    if found.is_none() {
        ui::clear();
        deps::install_tool("mpv");
        found = deps::resolve_player(&cfg.player);
    }
    if found.is_none() {
        found = deps::prompt_player_path();
    }
    if let Some(path) = found {
        if path != cfg.player {
            cfg.player = path;
            let _ = cfg.save();
        }
    }
}

fn open(api: &Api, cfg: &Config, logged_in: bool, item: Item, download: bool) -> Result<()> {
    match item {
        Item::Title(t) => match t.kind {
            Kind::Movie if download => download::run_jobs(api, cfg, vec![download::movie_job(&t)]),
            Kind::Movie => play_movie(api, cfg, logged_in, &t, None).map(|_| ()),
            Kind::Anime if t.seasons.len() > 1 => seasons(api, cfg, logged_in, &t),
            _ => episodes(api, cfg, logged_in, &t, None),
        },
        Item::Resume(r) => {
            let title = Title {
                id: r.id,
                kind: r.kind,
                name: r.show.clone(),
                year: String::new(),
                rating: 0.0,
                overview: None,
                poster_path: r.poster.clone(),
                seasons: Vec::new(),
            };
            match r.kind {
                Kind::Movie => play_movie(api, cfg, true, &title, r.resume_at()).map(|_| ()),
                _ => episodes(api, cfg, true, &title, Some((r.season, r.episode, r.resume_at()))),
            }
        }
    }
}

fn season_row(t: &Title) -> Row {
    let year = if t.year.is_empty() { String::new() } else { format!(" ({})", t.year) };
    Row {
        label: format!("{}{} · {}", t.name, year, t.overview.clone().unwrap_or_default()),
        title: t.name.clone(),
        meta: format!("anime{year}"),
        body: t.overview.clone().unwrap_or_default(),
        poster: t.poster_path.clone(),
    }
}

fn seasons(api: &Api, cfg: &Config, logged_in: bool, title: &Title) -> Result<()> {
    let list: Vec<Title> = title.seasons.iter().cloned().map(|s| s.into_title(title.poster_path.clone())).collect();
    let mut cursor = 0;
    loop {
        let mut sc = Screen::new(&title.name, "⏎ open season · esc back", list.clone(), season_row);
        sc.cursor = cursor;
        match screen::run(api, &mut sc)? {
            Out::Pick(i) => {
                cursor = i;
                episodes(api, cfg, logged_in, &list[i], None)?;
            }
            _ => return Ok(()),
        }
    }
}

fn tv_episodes(api: &Api, title: &Title) -> Result<Vec<Ep>> {
    let seasons = api.seasons(title.id)?;
    let lists: Vec<_> = std::thread::scope(|s| {
        let jobs: Vec<_> = seasons.iter().map(|season| s.spawn(move || api.episodes(title.id, season.number).unwrap_or_default())).collect();
        jobs.into_iter().map(|j| j.join().unwrap_or_default()).collect()
    });
    let mut eps = Vec::new();
    for (season, list) in seasons.iter().zip(lists) {
        for e in list {
            eps.push(Ep {
                season: season.number,
                number: e.number as f64,
                name: e.name.clone(),
                row: Row {
                    label: format!("S{:02}E{:02} · {}", season.number, e.number, e.name),
                    title: e.name.clone(),
                    meta: format!("{} · season {} · episode {}", title.name, season.number, e.number),
                    body: e.overview,
                    poster: e.still.or_else(|| title.poster_path.clone()),
                },
                sources: Vec::new(),
            });
        }
    }
    Ok(eps)
}

fn anime_episodes(api: &Api, title: &Title, dub: bool) -> Result<Vec<Ep>> {
    let providers = api.anime_providers(title.id, dub)?;
    let meta = api.anime_episode_meta(title.id);
    let tag = if dub { "dub" } else { "sub" };
    Ok(api::merge_episodes(&providers)
        .into_iter()
        .map(|e| {
            let m = meta.iter().find(|m| m.num == e.number);
            let name = m.and_then(|m| m.title.clone()).or(e.title.clone()).unwrap_or_default();
            let num = if e.number.fract() == 0.0 { format!("{}", e.number as i64) } else { e.number.to_string() };
            Ep {
                season: 0,
                number: e.number,
                name: name.clone(),
                row: Row {
                    label: if name.is_empty() { format!("E{num}") } else { format!("E{num} · {name}") },
                    title: if name.is_empty() { format!("Episode {num}") } else { name },
                    meta: format!("{} · episode {num} · {tag}", title.name),
                    body: m.and_then(|m| m.desc.clone()).unwrap_or_default(),
                    poster: m.and_then(|m| m.still.clone()).or_else(|| title.poster_path.clone()),
                },
                sources: api::episode_sources(&providers, e.number),
            }
        })
        .collect())
}

fn episodes(api: &Api, cfg: &Config, logged_in: bool, title: &Title, resume: Option<(u32, u32, Option<f64>)>) -> Result<()> {
    let anime = title.kind == Kind::Anime;
    let mut dub = cfg.audio.eq_ignore_ascii_case("dub");
    let mut eps = if anime { anime_episodes(api, title, dub)? } else { tv_episodes(api, title)? };
    if anime && eps.is_empty() {
        dub = !dub;
        eps = anime_episodes(api, title, dub)?;
    }
    if eps.is_empty() {
        anyhow::bail!("no episodes found for {}", title.name);
    }
    let mut cursor = resume
        .and_then(|(s, e, _)| eps.iter().position(|ep| (anime || ep.season == s) && ep.number as u32 == e))
        .unwrap_or(0);
    let mut start = resume.and_then(|(_, _, at)| at);
    let mut status = None;
    loop {
        let heading = if anime { format!("{} · {}", title.name, if dub { "dub" } else { "sub" }) } else { title.name.clone() };
        let mut sc = Screen::new(&heading, if anime { ANIME_HINT } else { TV_HINT }, eps.clone(), ep_row);
        sc.cursor = cursor;
        sc.keys = if anime { vec!['d', 'D', 'a'] } else { vec!['d', 'D'] };
        sc.status = status.take();
        match screen::run(api, &mut sc)? {
            Out::Back => return Ok(()),
            Out::Pick(i) => match play_from(api, cfg, logged_in, title, &eps, i, dub, start.take()) {
                Ok(next) => cursor = next,
                Err(err) => {
                    cursor = i;
                    status = Some(err.to_string());
                }
            },
            Out::Key('d', i) => {
                download::run_jobs(api, cfg, vec![job(title, &eps[i], dub)])?;
                cursor = i;
            }
            Out::Key('D', i) => {
                cursor = i;
                ui::clear();
                let scope = if anime { vec![Scope::Everything] } else { vec![Scope::Season, Scope::Everything] };
                let Some(scope) = ui::select("download", scope)? else { continue };
                let jobs: Vec<Job> = eps
                    .iter()
                    .filter(|e| matches!(scope, Scope::Everything) || e.season == eps[i].season)
                    .map(|e| job(title, e, dub))
                    .collect();
                download::run_jobs(api, cfg, jobs)?;
            }
            Out::Key('a', i) if anime => {
                let flipped = anime_episodes(api, title, !dub)?;
                if flipped.is_empty() {
                    status = Some(format!("no {} for this one", if dub { "sub" } else { "dub" }));
                    cursor = i;
                } else {
                    dub = !dub;
                    eps = flipped;
                    cursor = i.min(eps.len() - 1);
                }
            }
            Out::Key(..) => {}
        }
    }
}

fn job(title: &Title, ep: &Ep, dub: bool) -> Job {
    if title.kind == Kind::Anime {
        download::anime_job(title, ep.number, dub, ep.sources.clone())
    } else {
        download::tv_job(title, ep.season, ep.number as u32, &ep.name)
    }
}

#[allow(clippy::too_many_arguments)]
fn play_from(api: &Api, cfg: &Config, logged_in: bool, title: &Title, eps: &[Ep], mut i: usize, dub: bool, mut start: Option<f64>) -> Result<usize> {
    loop {
        let ep = &eps[i];
        let label = if title.kind == Kind::Anime {
            format!("{} · {}{}", title.name, ep.row.label, if dub { " (dub)" } else { "" })
        } else {
            format!("{} · {}", title.name, ep.row.label)
        };
        ui::clear();
        println!("finding sources for {label}");
        let (kind, season, number, history_title): (&str, u32, u32, &str) = if title.kind == Kind::Anime {
            ("anime", 0, ep.number as u32, if ep.name.is_empty() { &title.name } else { &ep.name })
        } else {
            ("tv", ep.season, ep.number as u32, &title.name)
        };
        let streams: Vec<Stream> = if title.kind == Kind::Anime {
            ep.sources.iter().filter_map(|(p, id)| api.anime_stream(p, id, &label)).collect()
        } else {
            let path = format!("{}/{}/{}", title.id, ep.season, ep.number as u32);
            with_subs(api, cfg, &path, api.resolve(&path, &label, None, false))
        };
        let streams = streams.into_iter().map(|mut s| {
            s.start = start;
            s
        });
        let watched = play_any(api, cfg, logged_in, kind, title.id, history_title, season, number, streams)?;
        start = None;
        if !watched.finished() {
            return Ok(i);
        }
        let Some(next) = eps.get(i + 1) else { return Ok(i) };
        if !screen::countdown(&next.row.label, 6)? {
            return Ok(i + 1);
        }
        i += 1;
    }
}

fn play_movie(api: &Api, cfg: &Config, logged_in: bool, title: &Title, start: Option<f64>) -> Result<player::Watched> {
    let id = title.id.to_string();
    ui::clear();
    println!("finding sources for {}", title.name);
    let streams = with_subs(api, cfg, &id, api.resolve(&id, &title.name, start, true));
    play_any(api, cfg, logged_in, "movie", title.id, &title.name, 0, 0, streams.into_iter())
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
) -> Result<player::Watched> {
    let mut tried = 0;
    for stream in streams {
        tried += 1;
        println!("▶ playing from {}", stream.source);
        let result = player::play(&cfg.player, &stream, |position, duration| {
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
            Ok(watched) => return Ok(watched),
            Err(err) => println!("✗ {}: {err}", stream.source),
        }
    }
    if tried == 0 {
        anyhow::bail!("no source has this one right now");
    }
    anyhow::bail!("every source failed to play")
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
    let password = if signup { ui::new_password("password")? } else { ui::password("password")? };
    let Some(password) = password else {
        return Ok(());
    };
    let attempt = if signup { api.register(username.trim(), &password) } else { api.login(username.trim(), &password) };
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
        println!("anime audio:       {}", cfg.audio);
        println!("parallel downloads: {}", cfg.parallel_downloads);
        println!("max quality:       {}p\n", cfg.max_height);
        let options = vec![Setting::Host, Setting::Player, Setting::Downloads, Setting::Subtitle, Setting::Audio, Setting::Parallel, Setting::Quality, Setting::Back];
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
            Some(Setting::Parallel) => {
                if let Some(n) = ui::select("episodes at once", vec![1usize, 2, 3, 4, 6])? {
                    cfg.parallel_downloads = n;
                }
            }
            Some(Setting::Quality) => {
                if let Some(h) = ui::select("max quality", vec![720u32, 1080, 1440, 2160])? {
                    cfg.max_height = h;
                    viel::relay::set_max_height(h);
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
