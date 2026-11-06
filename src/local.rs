use crate::api::{self, Api, ContinueRow, Details, Kind, ProgressUpdate, Stream, Title};
use crate::{offline, relay};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FIRST_HIT_GRACE: Duration = Duration::from_secs(6);
const RESOLVE_LIMIT: Duration = Duration::from_secs(25);

#[derive(Clone, Serialize)]
struct Probe {
    name: String,
    state: &'static str,
}

fn probes() -> &'static Mutex<Vec<Probe>> {
    static PROBES: OnceLock<Mutex<Vec<Probe>>> = OnceLock::new();
    PROBES.get_or_init(|| Mutex::new(Vec::new()))
}

fn mark(name: &str, state: &'static str) {
    let mut list = probes().lock().unwrap();
    if let Some(p) = list.iter_mut().find(|p| p.name == name) {
        p.state = state;
    }
}

fn resolve_all<F>(names: Vec<String>, job: F) -> Vec<Stream>
where
    F: Fn(&str) -> Option<Stream> + Send + Sync + 'static,
{
    *probes().lock().unwrap() = names
        .iter()
        .map(|n| Probe {
            name: n.clone(),
            state: "checking",
        })
        .collect();
    let job = std::sync::Arc::new(job);
    let (tx, rx) = mpsc::channel();
    for (index, name) in names.iter().cloned().enumerate() {
        let tx = tx.clone();
        let job = job.clone();
        thread::spawn(move || {
            let found = job(&name);
            mark(&name, if found.is_some() { "found" } else { "none" });
            let _ = tx.send((index, found));
        });
    }
    drop(tx);
    let started = Instant::now();
    let mut first_hit: Option<Instant> = None;
    let mut slots: Vec<Option<Stream>> = names.iter().map(|_| None).collect();
    let mut pending = names.len();
    while pending > 0 {
        let deadline = match first_hit {
            Some(at) => (at + FIRST_HIT_GRACE).min(started + RESOLVE_LIMIT),
            None => started + RESOLVE_LIMIT,
        };
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match rx.recv_timeout(deadline - now) {
            Ok((index, found)) => {
                pending -= 1;
                if found.is_some() && first_hit.is_none() {
                    first_hit = Some(Instant::now());
                }
                slots[index] = found;
            }
            Err(_) => break,
        }
    }
    for (index, slot) in slots.iter().enumerate() {
        if slot.is_none() {
            let state = probes()
                .lock()
                .unwrap()
                .get(index)
                .map(|p| p.state)
                .unwrap_or("none");
            if state == "checking" {
                mark(&names[index], "skipped");
            }
        }
    }
    slots.into_iter().flatten().collect()
}

struct State {
    api: Api,
    host: String,
    subtitle_lang: String,
    audio_lang: Option<String>,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(State {
            api: Api::new("https://viel.tv", None),
            host: "https://viel.tv".into(),
            subtitle_lang: "en".into(),
            audio_lang: None,
        })
    })
}

pub fn configure(host: &str, token: Option<String>) {
    let mut s = state().write().unwrap();
    s.host = host.trim_end_matches('/').to_string();
    s.api = Api::new(&s.host, token);
}

pub fn subtitle_proxy(url: &str, referer: Option<&str>) -> Option<String> {
    let host = state().read().unwrap().host.clone();
    if host.is_empty() || url.starts_with(&host) {
        return None;
    }
    let mut out = format!("{host}/proxy/subtitle?url={}", urlencoding::encode(url));
    if let Some(r) = referer.filter(|r| !r.is_empty()) {
        out.push_str("&ref=");
        out.push_str(&urlencoding::encode(r));
    }
    Some(out)
}

pub fn set_token(token: Option<String>) {
    let mut s = state().write().unwrap();
    s.api = Api::new(&s.host, token);
}

fn pending() -> &'static Mutex<Vec<Value>> {
    static LINES: OnceLock<Mutex<Vec<Value>>> = OnceLock::new();
    LINES.get_or_init(|| {
        thread::spawn(|| loop {
            thread::sleep(Duration::from_secs(5));
            flush();
        });
        Mutex::new(Vec::new())
    })
}

fn flush() {
    let lines: Vec<Value> = std::mem::take(&mut *pending().lock().unwrap());
    if lines.is_empty() {
        return;
    }
    let api = state().read().unwrap().api.clone();
    for chunk in lines.chunks(40) {
        api.log_event(&json!({ "type": "app", "detail": { "lines": chunk }, "url": "app://ios" }));
    }
}

pub fn log(tag: &str, message: impl Into<String>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    pending()
        .lock()
        .unwrap()
        .push(json!({ "t": now, "tag": tag, "msg": message.into() }));
}

type Reply = (&'static str, &'static str, Vec<u8>);

pub fn handle(method: &str, path: &str, query: &HashMap<String, String>, body: &[u8]) -> Reply {
    match route(method, path, query, body) {
        Ok(Some(v)) => ("200 OK", "application/json", v.to_string().into_bytes()),
        Ok(None) => (
            "404 Not Found",
            "application/json",
            br#"{"error":"not found"}"#.to_vec(),
        ),
        Err(e) => {
            log("api", format!("{method} {path}: {e}"));
            (
                "500 Internal Server Error",
                "application/json",
                json!({ "error": e.to_string() }).to_string().into_bytes(),
            )
        }
    }
}

fn route(
    method: &str,
    path: &str,
    query: &HashMap<String, String>,
    body: &[u8],
) -> Result<Option<Value>> {
    let (api, host, lang) = {
        let s = state().read().unwrap();
        (s.api.clone(), s.host.clone(), s.subtitle_lang.clone())
    };
    let q = |k: &str| query.get(k).cloned().unwrap_or_default();
    let value = match (method, path) {
        ("GET", "home") => home(&api, &host),
        ("GET", "search") => search(&api, &host, &q("q")),
        ("GET", "list") if q("path").starts_with("anime") => {
            let page: u32 = q("page").parse().unwrap_or(1);
            let rows: Vec<TitleOut> = api
                .anime_trending_page(page)?
                .into_iter()
                .map(|t| title_out(&host, t))
                .collect();
            json!(rows)
        }
        ("GET", "list") => {
            let page = q("page");
            let mut path = q("path");
            if !page.is_empty() {
                path.push(if path.contains('?') { '&' } else { '?' });
                path.push_str("page=");
                path.push_str(&page);
            }
            let rows: Vec<TitleOut> = api
                .list(&path)?
                .into_iter()
                .map(|t| title_out(&host, t))
                .collect();
            json!(rows)
        }
        ("GET", "details") => {
            let kind = parse_kind(&q("kind"))?;
            json!(details_out(
                &host,
                api.details(kind, q("id").parse().unwrap_or(0))?
            ))
        }
        ("GET", "title") => title(
            &api,
            &host,
            &q("kind"),
            q("id").parse().unwrap_or(0),
            q("dub") == "1",
        )?,
        ("POST", "play") => play(&api, &lang, serde_json::from_slice(body)?)?,
        ("GET", "history") => history(&api, &host),
        ("GET", "downloads") => json!(offline::list()),
        ("GET", "offline") => {
            offline::playback(&q("key")).ok_or_else(|| anyhow::anyhow!("download not found"))?
        }
        ("POST", "downloads/plan") => {
            let d: DownloadIn = serde_json::from_slice(body)?;
            let height = d.max_height.unwrap_or_else(relay::max_height);
            let streams = resolve_streams(&api, &lang, d.play)?;
            offline::plan(&d.key, d.meta, &streams, height, &lang)?
        }
        ("POST", "downloads/finish") => {
            let d: KeyIn = serde_json::from_slice(body)?;
            offline::finish(&d.key)?
        }
        ("POST", "downloads/fail") => {
            let d: KeyIn = serde_json::from_slice(body)?;
            offline::fail(&d.key, d.error.as_deref().unwrap_or("download failed"))?;
            json!({ "ok": true })
        }
        ("POST", "downloads/remove") => {
            let d: KeyIn = serde_json::from_slice(body)?;
            offline::remove(&d.key)?;
            json!({ "ok": true })
        }
        ("GET", "audio") => audio_tracks(&q("url"))?,
        ("GET", "watchlist") => {
            let rows: Vec<Value> = api
                .watchlist()?
                .into_iter()
                .filter_map(|w| {
                    let id: u64 = w.content_id.parse().ok()?;
                    Some(json!({ "id": id, "kind": w.kind, "name": w.title, "poster": poster_url(&host, Some(&w.poster)) }))
                })
                .collect();
            json!(rows)
        }
        ("GET", "calendar") => {
            let rows: Vec<Value> = api
                .calendar()?
                .into_iter()
                .map(|u| json!({ "kind": u.kind, "id": u.id, "name": u.name, "poster": poster_url(&host, Some(&u.poster)), "season": u.season, "episode": u.episode, "title": u.title, "airs_at": u.airs_at }))
                .collect();
            json!(rows)
        }
        ("POST", "watchlist") => {
            let w: WatchIn = serde_json::from_slice(body)?;
            api.add_watchlist(&w.kind, w.id, &w.name, w.poster.as_deref())?;
            json!({ "ok": true })
        }
        ("POST", "watchlist/remove") => {
            let w: RemoveIn = serde_json::from_slice(body)?;
            api.remove_watchlist(&w.kind, w.id)?;
            json!({ "ok": true })
        }
        ("GET", "recommendations") => {
            let rows: Vec<TitleOut> = api
                .recommendations()
                .unwrap_or_default()
                .into_iter()
                .take(20)
                .map(|t| title_out(&host, t))
                .collect();
            json!(rows)
        }
        ("GET", "trailer") => {
            let kind = parse_kind(&q("kind"))?;
            json!({ "url": api.trailer(kind, q("id").parse().unwrap_or(0)) })
        }
        ("POST", "progress") => {
            let p: Progress = serde_json::from_slice(body)?;
            api.save_progress(&ProgressUpdate {
                kind: &p.kind,
                content_id: p.id.to_string(),
                season: p.season,
                episode: p.episode,
                position: p.position,
                duration: p.duration,
                title: &p.title,
            })?;
            json!({ "ok": true })
        }
        ("POST", "login") | ("POST", "register") => {
            let c: Login = serde_json::from_slice(body)?;
            let token = if path == "login" {
                api.login(&c.username, &c.password)?
            } else {
                api.register(&c.username, &c.password)?
            };
            set_token(Some(token.clone()));
            json!({ "token": token, "username": c.username })
        }
        ("POST", "logout") => {
            set_token(None);
            json!({ "ok": true })
        }
        ("GET", "me") => json!({ "username": api.me().unwrap_or(None) }),
        ("GET", "activity") => {
            let a = relay::activity();
            json!({
                "inflight": a.inflight,
                "served": a.served,
                "fetching": a.fetching.map(|(what, seconds)| json!({ "what": what, "seconds": seconds })),
                "sources": probes().lock().unwrap().clone(),
            })
        }
        ("POST", "history/remove") => {
            let p: RemoveIn = serde_json::from_slice(body)?;
            api.remove_history(&p.kind, p.id)?;
            json!({ "ok": true })
        }
        ("POST", "settings") => {
            let s: Settings = serde_json::from_slice(body)?;
            if let Some(h) = s.max_height {
                relay::set_max_height(h);
            }
            if let Some(l) = s.subtitle_lang.filter(|l| !l.trim().is_empty()) {
                if state().read().unwrap().audio_lang.is_none() {
                    relay::set_audio_lang(&l);
                }
                state().write().unwrap().subtitle_lang = l;
            }
            if let Some(a) = s.audio_lang {
                let a = a.trim().to_string();
                if a.is_empty() {
                    let fallback = state().read().unwrap().subtitle_lang.clone();
                    relay::set_audio_lang(&fallback);
                    state().write().unwrap().audio_lang = None;
                } else {
                    relay::set_audio_lang(&a);
                    state().write().unwrap().audio_lang = Some(a);
                }
            }
            if let Some(h) = s.lan_host {
                relay::set_lan_host(Some(h));
            }
            json!({ "ok": true })
        }
        ("POST", "log") => {
            let batch: LogBatch = serde_json::from_slice(body)?;
            let mut lines = pending().lock().unwrap();
            for line in batch.lines {
                lines.push(json!({ "t": line.t, "tag": line.tag, "msg": line.msg, "device": batch.device, "build": batch.build }));
            }
            json!({ "ok": true })
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn parse_kind(kind: &str) -> Result<Kind> {
    Ok(match kind {
        "movie" => Kind::Movie,
        "tv" => Kind::Tv,
        "anime" => Kind::Anime,
        other => bail!("unknown kind {other}"),
    })
}

#[derive(Serialize)]
struct TitleOut {
    id: u64,
    kind: &'static str,
    name: String,
    year: String,
    rating: f64,
    overview: Option<String>,
    poster: Option<String>,
    backdrop: Option<String>,
    genres: Vec<String>,
    seasons: Vec<SeasonOut>,
}

#[derive(Serialize)]
struct SeasonOut {
    id: u64,
    season: u64,
    title: String,
    episodes: u64,
    year: String,
    poster: Option<String>,
}

#[derive(Serialize)]
struct DetailsOut {
    #[serde(flatten)]
    title: TitleOut,
    logo: Option<String>,
    runtime: Option<u32>,
    tagline: String,
    cast: Vec<String>,
    season_count: u32,
    collection: Option<CollectionOut>,
}

#[derive(Serialize)]
struct CollectionOut {
    name: String,
    parts: Vec<TitleOut>,
}

#[derive(Serialize)]
struct ContinueOut {
    id: u64,
    kind: &'static str,
    show: String,
    poster: Option<String>,
    season: u32,
    episode: u32,
    position: f64,
    duration: f64,
    resume_at: Option<f64>,
}

#[derive(Serialize)]
struct EpisodeOut {
    season: u32,
    number: f64,
    name: String,
    overview: String,
    still: Option<String>,
    runtime: Option<u32>,
    sources: Vec<Source>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Source {
    provider: String,
    id: String,
}

#[derive(Serialize)]
struct StreamOut {
    source: String,
    url: String,
    hls: bool,
    subs: Vec<SubOut>,
    intro: Option<(f64, f64)>,
    outro: Option<(f64, f64)>,
    thumbs: Option<String>,
}

#[derive(Serialize)]
struct SubOut {
    lang: String,
    url: String,
}

#[derive(Deserialize)]
struct PlayIn {
    kind: String,
    id: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    season: u32,
    #[serde(default)]
    episode: f64,
    #[serde(default)]
    dub: bool,
    #[serde(default)]
    sources: Vec<Source>,
}

#[derive(Deserialize)]
struct Progress {
    kind: String,
    id: u64,
    #[serde(default)]
    season: u32,
    #[serde(default)]
    episode: u32,
    position: f64,
    duration: f64,
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RemoveIn {
    kind: String,
    id: u64,
}

#[derive(Deserialize)]
struct DownloadIn {
    #[serde(flatten)]
    play: PlayIn,
    key: String,
    #[serde(default)]
    meta: Value,
    max_height: Option<u32>,
}

#[derive(Deserialize)]
struct KeyIn {
    key: String,
    error: Option<String>,
}

#[derive(Deserialize)]
struct WatchIn {
    kind: String,
    id: u64,
    #[serde(default)]
    name: String,
    poster: Option<String>,
}

#[derive(Serialize)]
struct HistoryOut {
    id: u64,
    kind: String,
    show: String,
    poster: Option<String>,
    season: u32,
    episode: u32,
    position: f64,
    duration: f64,
    updated_at: u64,
}

#[derive(Deserialize)]
struct Settings {
    max_height: Option<u32>,
    subtitle_lang: Option<String>,
    audio_lang: Option<String>,
    lan_host: Option<String>,
}

#[derive(Deserialize)]
struct LogBatch {
    #[serde(default)]
    device: String,
    #[serde(default)]
    build: String,
    #[serde(default)]
    lines: Vec<LogLine>,
}

#[derive(Deserialize)]
struct LogLine {
    #[serde(default)]
    t: u64,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    msg: String,
}

pub fn poster_url(host: &str, path: Option<&str>) -> Option<String> {
    image_url(host, path, "w500")
}

fn image_url(host: &str, path: Option<&str>, size: &str) -> Option<String> {
    let path = path.filter(|p| !p.is_empty())?;
    Some(if path.starts_with("/proxy") {
        format!("{host}{path}")
    } else if path.starts_with("http") {
        format!("{host}/proxy/image?url={}", urlencoding::encode(path))
    } else {
        format!(
            "{host}/proxy/image?url={}",
            urlencoding::encode(&format!("https://image.tmdb.org/t/p/{size}{path}"))
        )
    })
}

fn title_out(host: &str, t: Title) -> TitleOut {
    TitleOut {
        id: t.id,
        kind: t.kind.slug(),
        name: t.name,
        year: t.year,
        rating: t.rating,
        overview: t.overview,
        poster: poster_url(host, t.poster_path.as_deref()),
        backdrop: image_url(host, t.backdrop_path.as_deref(), "original"),
        genres: t.genres,
        seasons: t
            .seasons
            .into_iter()
            .map(|s| SeasonOut {
                id: s.id,
                season: s.season,
                title: s.title,
                episodes: s.episodes,
                year: s.year,
                poster: poster_url(host, s.cover.as_deref()),
            })
            .collect(),
    }
}

fn details_out(host: &str, d: Details) -> DetailsOut {
    DetailsOut {
        logo: image_url(host, d.logo.as_deref(), "w500"),
        title: title_out(host, d.title),
        runtime: d.runtime,
        tagline: d.tagline,
        cast: d.cast,
        season_count: d.seasons,
        collection: d.collection.map(|c| CollectionOut {
            name: c.name,
            parts: c.parts.into_iter().map(|t| title_out(host, t)).collect(),
        }),
    }
}

fn continue_out(host: &str, r: ContinueRow) -> ContinueOut {
    ContinueOut {
        id: r.id,
        kind: r.kind.slug(),
        show: r.show.clone(),
        poster: poster_url(host, r.poster.as_deref()),
        season: r.season,
        episode: r.episode,
        position: r.position,
        duration: r.duration,
        resume_at: r.resume_at(),
    }
}

fn home(api: &Api, host: &str) -> Value {
    let (resume, today, week, top_rated, popular_tv, anime) = thread::scope(|s| {
        let a = s.spawn(|| api.continue_rows().unwrap_or_default());
        let b = s.spawn(|| api.list("trending/all/day").unwrap_or_default());
        let c = s.spawn(|| api.trending().unwrap_or_default());
        let d = s.spawn(|| api.list("movie/top_rated").unwrap_or_default());
        let e = s.spawn(|| api.list("tv/popular").unwrap_or_default());
        let f = s.spawn(|| api.anime_trending().unwrap_or_default());
        (
            a.join().unwrap_or_default(),
            b.join().unwrap_or_default(),
            c.join().unwrap_or_default(),
            d.join().unwrap_or_default(),
            e.join().unwrap_or_default(),
            f.join().unwrap_or_default(),
        )
    });
    let hero = today
        .first()
        .and_then(|t| api.details(t.kind, t.id).ok())
        .map(|d| details_out(host, d));
    let out = |list: Vec<Title>, limit: usize| -> Vec<TitleOut> {
        list.into_iter()
            .take(limit)
            .map(|t| title_out(host, t))
            .collect()
    };
    json!({
        "hero": hero,
        "continue": resume.into_iter().map(|r| continue_out(host, r)).collect::<Vec<_>>(),
        "top": out(today, 10),
        "trending": out(week, 20),
        "top_rated": out(top_rated, 20),
        "popular_tv": out(popular_tv, 20),
        "anime": out(anime, 20),
    })
}

fn search(api: &Api, host: &str, query: &str) -> Value {
    let query = query.trim();
    if query.is_empty() {
        return json!([]);
    }
    let (media, anime) = thread::scope(|s| {
        let a = s.spawn(|| api.search(query).unwrap_or_default());
        let b = s.spawn(|| api.anime_search_merged(query));
        (a.join().unwrap_or_default(), b.join().unwrap_or_default())
    });
    let mut rows: Vec<TitleOut> = anime
        .into_iter()
        .take(6)
        .map(|t| title_out(host, t))
        .collect();
    rows.extend(media.into_iter().take(14).map(|t| title_out(host, t)));
    json!(rows)
}

fn title(api: &Api, host: &str, kind: &str, id: u64, dub: bool) -> Result<Value> {
    match kind {
        "tv" => {
            let seasons = api.seasons(id)?;
            let lists: Vec<_> = thread::scope(|s| {
                let jobs: Vec<_> = seasons
                    .iter()
                    .map(|season| {
                        s.spawn(move || api.episodes(id, season.number).unwrap_or_default())
                    })
                    .collect();
                jobs.into_iter()
                    .map(|j| j.join().unwrap_or_default())
                    .collect()
            });
            let mut eps = Vec::new();
            for (season, list) in seasons.iter().zip(lists) {
                for e in list {
                    eps.push(EpisodeOut {
                        season: season.number,
                        number: e.number as f64,
                        name: e.name,
                        overview: e.overview,
                        still: image_url(host, e.still.as_deref(), "w300"),
                        runtime: e.runtime,
                        sources: Vec::new(),
                    });
                }
            }
            Ok(json!({ "episodes": eps, "dub": false }))
        }
        "anime" => {
            let mut dub = dub;
            let mut eps = anime_episodes(api, host, id, dub)?;
            if eps.is_empty() {
                dub = !dub;
                eps = anime_episodes(api, host, id, dub)?;
            }
            Ok(json!({ "episodes": eps, "dub": dub }))
        }
        _ => bail!("unknown kind {kind}"),
    }
}

fn anime_episodes(api: &Api, host: &str, id: u64, dub: bool) -> Result<Vec<EpisodeOut>> {
    let (providers, meta, runtime) = thread::scope(|s| {
        let a = s.spawn(|| api.anime_providers(id, dub));
        let b = s.spawn(|| api.anime_episode_meta(id));
        let c = s.spawn(|| api.details(Kind::Anime, id).ok().and_then(|d| d.runtime));
        (
            a.join().unwrap_or_else(|_| Ok(Vec::new())),
            b.join().unwrap_or_default(),
            c.join().ok().flatten(),
        )
    });
    let providers = providers?;
    Ok(api::merge_episodes(&providers)
        .into_iter()
        .map(|e| {
            let m = meta.iter().find(|m| m.num == e.number);
            EpisodeOut {
                season: 0,
                number: e.number,
                name: m
                    .and_then(|m| m.title.clone())
                    .or(e.title.clone())
                    .unwrap_or_default(),
                overview: m.and_then(|m| m.desc.clone()).unwrap_or_default(),
                still: poster_url(host, m.and_then(|m| m.still.as_deref())),
                runtime,
                sources: api::episode_sources(&providers, e.number)
                    .into_iter()
                    .map(|(provider, id)| Source { provider, id })
                    .collect(),
            }
        })
        .collect())
}

fn play(api: &Api, lang: &str, p: PlayIn) -> Result<Value> {
    let started = Instant::now();
    let (kind, id, season, episode) = (p.kind.clone(), p.id, p.season, p.episode);
    let streams = resolve_streams(api, lang, p)?;
    let names: Vec<&str> = streams.iter().map(|s| s.source.as_str()).collect();
    log(
        "resolve",
        format!(
            "{} {} s{} e{} → [{}] in {:.1}s",
            kind,
            id,
            season,
            episode,
            names.join(", "),
            started.elapsed().as_secs_f32()
        ),
    );
    let out: Vec<StreamOut> = streams.into_iter().map(|s| stream_out(s, lang)).collect();
    Ok(json!({ "streams": out }))
}

fn resolve_streams(api: &Api, lang: &str, p: PlayIn) -> Result<Vec<Stream>> {
    relay::reset_activity();
    let streams: Vec<Stream> = match p.kind.as_str() {
        "movie" | "tv" => {
            let movie = p.kind == "movie";
            let path = if movie {
                p.id.to_string()
            } else {
                format!("{}/{}/{}", p.id, p.season, p.episode as u32)
            };
            let names: Vec<String> = Api::sources_for(movie)
                .into_iter()
                .map(str::to_string)
                .collect();
            let client = api.clone();
            let title = p.title.clone();
            let target = path.clone();
            let found = resolve_all(names, move |src| {
                client
                    .extract(src, &target, &title, None)
                    .filter(api::playable)
            });
            with_subs(api, lang, &path, found)
        }
        "anime" => {
            let sources = if p.sources.is_empty() {
                api.anime_providers(p.id, p.dub)
                    .map(|providers| api::episode_sources(&providers, p.episode))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(provider, id)| Source { provider, id })
                    .collect()
            } else {
                p.sources
            };
            let names: Vec<String> = sources.iter().map(|s| s.provider.clone()).collect();
            let ids: HashMap<String, String> =
                sources.into_iter().map(|s| (s.provider, s.id)).collect();
            let client = api.clone();
            let title = p.title.clone();
            resolve_all(names, move |provider| {
                ids.get(provider)
                    .and_then(|id| client.anime_stream(provider, id, &title))
            })
        }
        other => bail!("unknown kind {other}"),
    };
    Ok(streams)
}

fn audio_tracks(link: &str) -> Result<Value> {
    let query: HashMap<String, String> = link
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((k.to_string(), urlencoding::decode(v).ok()?.into_owned()))
        })
        .collect();
    let url = query.get("u").cloned().unwrap_or_default();
    if url.is_empty() {
        bail!("missing stream");
    }
    let text = crate::net::text(&url, query.get("r").map(String::as_str))?;
    if !crate::hls::is_master(&text) {
        return Ok(json!([]));
    }
    let mut seen = Vec::new();
    let mut rows = Vec::new();
    for variant in crate::hls::variants(&text, &url) {
        let Some(group) = variant.audio.as_deref() else {
            continue;
        };
        for track in crate::hls::audio_tracks(&text, &url, group) {
            let tag = if track.lang.is_empty() {
                track.name.clone()
            } else {
                track.lang.clone()
            };
            if tag.is_empty() || seen.contains(&tag) {
                continue;
            }
            seen.push(tag.clone());
            rows.push(json!({ "lang": tag, "name": track.name, "default": track.default }));
        }
    }
    Ok(json!(rows))
}

fn history(api: &Api, host: &str) -> Value {
    let entries = api.history().unwrap_or_default();
    let mut keys: Vec<(String, u64)> = Vec::new();
    for e in &entries {
        if let Ok(id) = e.content_id.parse::<u64>() {
            if !keys.iter().any(|k| k.0 == e.kind && k.1 == id) {
                keys.push((e.kind.clone(), id));
            }
        }
    }
    keys.truncate(60);
    let found: Vec<Option<(String, Option<String>)>> = thread::scope(|s| {
        let jobs: Vec<_> = keys
            .iter()
            .map(|(kind, id)| {
                s.spawn(move || {
                    if let Some(hit) = names().lock().unwrap().get(&(kind.clone(), *id)) {
                        return Some(hit.clone());
                    }
                    let info = if kind == "anime" {
                        api.anime_info(*id)
                    } else {
                        api.title_info(kind, *id)
                    }?;
                    names()
                        .lock()
                        .unwrap()
                        .insert((kind.clone(), *id), info.clone());
                    Some(info)
                })
            })
            .collect();
        jobs.into_iter().map(|j| j.join().ok().flatten()).collect()
    });
    let rows: Vec<HistoryOut> = entries
        .into_iter()
        .filter_map(|e| {
            let id = e.content_id.parse::<u64>().ok()?;
            let at = keys.iter().position(|k| k.0 == e.kind && k.1 == id)?;
            let (show, poster) = found[at].clone().unwrap_or_else(|| {
                (
                    e.title
                        .clone()
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| "untitled".into()),
                    e.poster.clone(),
                )
            });
            Some(HistoryOut {
                id,
                kind: e.kind,
                show,
                poster: poster_url(host, poster.as_deref()),
                season: e.season.max(0) as u32,
                episode: e.episode.max(0) as u32,
                position: e.position,
                duration: e.duration,
                updated_at: e.updated_at,
            })
        })
        .collect();
    json!(rows)
}

type NameCache = Mutex<HashMap<(String, u64), (String, Option<String>)>>;

fn names() -> &'static NameCache {
    static NAMES: OnceLock<NameCache> = OnceLock::new();
    NAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_subs(api: &Api, lang: &str, path: &str, mut streams: Vec<Stream>) -> Vec<Stream> {
    let subs = api::pick_subs(api.subtitles(path), lang);
    for s in &mut streams {
        s.subs = subs.clone();
    }
    streams
}

fn stream_out(s: Stream, lang: &str) -> StreamOut {
    let referer = s.referer.as_deref();
    let url = relay::media(&s.url, referer, s.hls);
    let mut subs = s.subs.clone();
    subs.sort_by_key(|x| !api::lang_matches(&x.lang, lang));
    StreamOut {
        source: s.source.clone(),
        url,
        hls: s.hls,
        subs: subs
            .into_iter()
            .take(8)
            .map(|x| SubOut {
                lang: x.lang,
                url: relay::subtitle(&x.url, x.referer.as_deref()),
            })
            .collect(),
        intro: s.intro,
        outro: s.outro,
        thumbs: s.thumbs.as_deref().map(|t| relay::subtitle(t, referer)),
    }
}
