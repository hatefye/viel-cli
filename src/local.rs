use crate::api::{self, Api, ContinueRow, ProgressUpdate, Stream, Title};
use crate::relay;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::thread;

struct State {
    api: Api,
    host: String,
    subtitle_lang: String,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(State { api: Api::new("https://viel.tv", None), host: "https://viel.tv".into(), subtitle_lang: "en".into() })
    })
}

pub fn configure(host: &str, token: Option<String>) {
    let mut s = state().write().unwrap();
    s.host = host.trim_end_matches('/').to_string();
    s.api = Api::new(&s.host, token);
}

pub fn set_token(token: Option<String>) {
    let mut s = state().write().unwrap();
    s.api = Api::new(&s.host, token);
}

type Reply = (&'static str, &'static str, Vec<u8>);

pub fn handle(method: &str, path: &str, query: &HashMap<String, String>, body: &[u8]) -> Reply {
    match route(method, path, query, body) {
        Ok(Some(v)) => ("200 OK", "application/json", v.to_string().into_bytes()),
        Ok(None) => ("404 Not Found", "application/json", br#"{"error":"not found"}"#.to_vec()),
        Err(e) => ("500 Internal Server Error", "application/json", json!({ "error": e.to_string() }).to_string().into_bytes()),
    }
}

fn route(method: &str, path: &str, query: &HashMap<String, String>, body: &[u8]) -> Result<Option<Value>> {
    let (api, host, lang) = {
        let s = state().read().unwrap();
        (s.api.clone(), s.host.clone(), s.subtitle_lang.clone())
    };
    let q = |k: &str| query.get(k).cloned().unwrap_or_default();
    let value = match (method, path) {
        ("GET", "home") => home(&api, &host),
        ("GET", "search") => search(&api, &host, &q("q")),
        ("GET", "title") => title(&api, &host, &q("kind"), q("id").parse().unwrap_or(0), q("dub") == "1")?,
        ("POST", "play") => play(&api, &lang, serde_json::from_slice(body)?)?,
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
            let token = if path == "login" { api.login(&c.username, &c.password)? } else { api.register(&c.username, &c.password)? };
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
            })
        }
        ("POST", "settings") => {
            let s: Settings = serde_json::from_slice(body)?;
            if let Some(h) = s.max_height {
                relay::set_max_height(h);
            }
            if let Some(l) = s.subtitle_lang.filter(|l| !l.trim().is_empty()) {
                relay::set_audio_lang(&l);
                state().write().unwrap().subtitle_lang = l;
            }
            json!({ "ok": true })
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
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
struct Settings {
    max_height: Option<u32>,
    subtitle_lang: Option<String>,
}

pub fn poster_url(host: &str, path: Option<&str>) -> Option<String> {
    let path = path.filter(|p| !p.is_empty())?;
    Some(if path.starts_with("/proxy") {
        format!("{host}{path}")
    } else if path.starts_with("http") {
        format!("{host}/proxy/image?url={}", urlencoding::encode(path))
    } else {
        format!("{host}/proxy/image?url={}", urlencoding::encode(&format!("https://image.tmdb.org/t/p/w500{path}")))
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
    let (resume, trending) = thread::scope(|s| {
        let a = s.spawn(|| api.continue_rows().unwrap_or_default());
        let b = s.spawn(|| api.trending().unwrap_or_default());
        (a.join().unwrap_or_default(), b.join().unwrap_or_default())
    });
    json!({
        "continue": resume.into_iter().map(|r| continue_out(host, r)).collect::<Vec<_>>(),
        "trending": trending.into_iter().map(|t| title_out(host, t)).collect::<Vec<_>>(),
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
    let mut rows: Vec<TitleOut> = anime.into_iter().take(6).map(|t| title_out(host, t)).collect();
    rows.extend(media.into_iter().take(14).map(|t| title_out(host, t)));
    json!(rows)
}

fn title(api: &Api, host: &str, kind: &str, id: u64, dub: bool) -> Result<Value> {
    match kind {
        "tv" => {
            let seasons = api.seasons(id)?;
            let lists: Vec<_> = thread::scope(|s| {
                let jobs: Vec<_> = seasons.iter().map(|season| s.spawn(move || api.episodes(id, season.number).unwrap_or_default())).collect();
                jobs.into_iter().map(|j| j.join().unwrap_or_default()).collect()
            });
            let mut eps = Vec::new();
            for (season, list) in seasons.iter().zip(lists) {
                for e in list {
                    eps.push(EpisodeOut {
                        season: season.number,
                        number: e.number as f64,
                        name: e.name,
                        overview: e.overview,
                        still: poster_url(host, e.still.as_deref()),
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
    let providers = api.anime_providers(id, dub)?;
    let meta = api.anime_episode_meta(id);
    Ok(api::merge_episodes(&providers)
        .into_iter()
        .map(|e| {
            let m = meta.iter().find(|m| m.num == e.number);
            EpisodeOut {
                season: 0,
                number: e.number,
                name: m.and_then(|m| m.title.clone()).or(e.title.clone()).unwrap_or_default(),
                overview: m.and_then(|m| m.desc.clone()).unwrap_or_default(),
                still: poster_url(host, m.and_then(|m| m.still.as_deref())),
                sources: api::episode_sources(&providers, e.number).into_iter().map(|(provider, id)| Source { provider, id }).collect(),
            }
        })
        .collect())
}

fn play(api: &Api, lang: &str, p: PlayIn) -> Result<Value> {
    let streams: Vec<Stream> = match p.kind.as_str() {
        "movie" => {
            let id = p.id.to_string();
            with_subs(api, lang, &id, api.resolve(&id, &p.title, None, true))
        }
        "tv" => {
            let path = format!("{}/{}/{}", p.id, p.season, p.episode as u32);
            with_subs(api, lang, &path, api.resolve(&path, &p.title, None, false))
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
            let title = &p.title;
            let found: Vec<Option<Stream>> = thread::scope(|s| {
                let jobs: Vec<_> = sources.iter().map(|src| s.spawn(move || api.anime_stream(&src.provider, &src.id, title))).collect();
                jobs.into_iter().map(|j| j.join().ok().flatten()).collect()
            });
            found.into_iter().flatten().collect()
        }
        other => bail!("unknown kind {other}"),
    };
    let out: Vec<StreamOut> = streams.into_iter().map(|s| stream_out(s, lang)).collect();
    Ok(json!({ "streams": out }))
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
    let url = if s.hls { relay::playlist(&s.url, referer) } else { relay::video(&s.url, referer) };
    let mut subs = s.subs.clone();
    subs.sort_by_key(|x| !api::lang_matches(&x.lang, lang));
    StreamOut {
        source: s.source.clone(),
        url,
        hls: s.hls,
        subs: subs.into_iter().take(8).map(|x| SubOut { lang: x.lang, url: relay::subtitle(&x.url, x.referer.as_deref()) }).collect(),
        intro: s.intro,
    }
}
