use anyhow::{anyhow, bail, Result};
use reqwest::blocking::{Client, RequestBuilder};
use crate::{hls, net};
use reqwest::header::{COOKIE, SET_COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

const SOURCES: [&str; 5] = ["cno", "mapple", "vixsrc", "vidbolt", "vidrock"];
const ANIME_SUB: [&str; 4] = ["animeparadise", "animekai", "kototv", "luffy"];
const ANIME_DUB: [&str; 3] = ["animekai", "kototv", "luffy"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Movie,
    Tv,
    Anime,
}

impl Kind {
    pub fn slug(self) -> &'static str {
        match self {
            Kind::Movie => "movie",
            Kind::Tv => "tv",
            Kind::Anime => "anime",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.slug())
    }
}

#[derive(Clone)]
pub struct Title {
    pub id: u64,
    pub kind: Kind,
    pub name: String,
    pub year: String,
    pub rating: f64,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub seasons: Vec<AnimeSeason>,
}

#[derive(Clone, Deserialize)]
pub struct AnimeSeason {
    #[serde(deserialize_with = "num")]
    pub id: u64,
    #[serde(default, deserialize_with = "num")]
    pub season: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default, deserialize_with = "num")]
    pub episodes: u64,
    #[serde(default, deserialize_with = "text")]
    pub year: String,
    pub cover: Option<String>,
}

fn text<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        N(u64),
        S(String),
        Nil(()),
    }
    Ok(match Raw::deserialize(d)? {
        Raw::N(n) => n.to_string(),
        Raw::S(s) => s,
        Raw::Nil(()) => String::new(),
    })
}

pub fn plural(n: u64, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

fn num<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        N(u64),
        S(String),
    }
    Ok(match Raw::deserialize(d)? {
        Raw::N(n) => n,
        Raw::S(s) => s.trim().parse().unwrap_or(0),
    })
}

impl AnimeSeason {
    pub fn into_title(self, poster: Option<String>) -> Title {
        Title {
            id: self.id,
            kind: Kind::Anime,
            name: self.title.clone(),
            year: self.year.clone(),
            rating: 0.0,
            overview: Some(plural(self.episodes, "episode")),
            poster_path: self.cover.or(poster),
            seasons: Vec::new(),
        }
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let year = if self.year.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.year)
        };
        if self.rating > 0.0 {
            write!(f, "{}{} · {} · {:.1}", self.name, year, self.kind, self.rating)
        } else {
            write!(f, "{}{} · {}", self.name, year, self.kind)
        }
    }
}

pub struct Season {
    pub number: u32,
    pub name: String,
    pub episodes: u32,
}

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "S{} · {} ({} eps)", self.number, self.name, self.episodes)
    }
}

pub struct Episode {
    pub number: u32,
    pub name: String,
    pub overview: String,
    pub still: Option<String>,
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "E{} · {}", self.number, self.name)
    }
}

#[derive(Clone)]
pub struct Stream {
    pub source: String,
    pub url: String,
    pub referer: Option<String>,
    pub hls: bool,
    pub title: String,
    pub start: Option<f64>,
    pub subs: Vec<Subtitle>,
    pub intro: Option<(f64, f64)>,
}

#[derive(Clone)]
pub struct Subtitle {
    pub url: String,
    pub referer: Option<String>,
    pub lang: String,
}

#[derive(Clone)]
pub struct ContinueRow {
    pub id: u64,
    pub kind: Kind,
    pub show: String,
    pub poster: Option<String>,
    pub season: u32,
    pub episode: u32,
    pub position: f64,
    pub duration: f64,
}

impl ContinueRow {
    pub fn resume_at(&self) -> Option<f64> {
        (self.position > 5.0 && self.position < self.duration - 30.0).then_some(self.position)
    }
}

impl fmt::Display for ContinueRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let left = remaining(self.duration - self.position);
        match self.kind {
            Kind::Tv => write!(f, "{} · S{}E{} · {} left", self.show, self.season, self.episode, left),
            Kind::Anime => write!(f, "{} · E{} · {} left", self.show, self.episode, left),
            Kind::Movie => write!(f, "{} · {} left", self.show, left),
        }
    }
}

#[derive(Clone)]
pub struct Api {
    client: Client,
    host: String,
    token: Option<String>,
}

impl Api {
    pub fn new(host: &str, token: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("http client");
        Self {
            client,
            host: host.trim_end_matches('/').to_string(),
            token,
        }
    }

    pub fn reachable(&self) -> bool {
        let url = format!("{}/api/announcement", self.host);
        self.client
            .get(url)
            .timeout(Duration::from_secs(8))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub fn search(&self, query: &str) -> Result<Vec<Title>> {
        let url = format!("{}/proxy/tmdb/search/multi", self.host);
        let resp: SearchResp = self.client.get(url).query(&[("query", query)]).send()?.json()?;
        Ok(resp.results.into_iter().filter_map(SearchItem::into_title).collect())
    }

    pub fn trending(&self) -> Result<Vec<Title>> {
        let url = format!("{}/proxy/tmdb/trending/all/week", self.host);
        let resp: SearchResp = self.client.get(url).send()?.json()?;
        Ok(resp.results.into_iter().filter_map(SearchItem::into_title).collect())
    }

    pub fn seasons(&self, id: u64) -> Result<Vec<Season>> {
        let url = format!("{}/proxy/tmdb/tv/{}", self.host, id);
        let detail: TvDetail = self.client.get(url).send()?.json()?;
        Ok(detail
            .seasons
            .into_iter()
            .filter(|s| s.season_number > 0)
            .map(|s| Season {
                number: s.season_number,
                name: s.name,
                episodes: s.episode_count,
            })
            .collect())
    }

    pub fn episodes(&self, id: u64, season: u32) -> Result<Vec<Episode>> {
        let url = format!("{}/proxy/tmdb/tv/{}/season/{}", self.host, id, season);
        let detail: SeasonDetail = self.client.get(url).send()?.json()?;
        Ok(detail
            .episodes
            .into_iter()
            .map(|e| Episode {
                number: e.episode_number,
                name: e.name,
                overview: e.overview.unwrap_or_default(),
                still: e.still_path.filter(|s| !s.is_empty()),
            })
            .collect())
    }

    pub fn resolve(&self, path: &str, title: &str, start: Option<f64>, movie: bool) -> Vec<Stream> {
        let found: Vec<Option<Stream>> = std::thread::scope(|scope| {
            let jobs: Vec<_> = SOURCES
                .iter()
                .filter(|src| !(movie && **src == "vidbolt"))
                .map(|src| scope.spawn(move || self.extract(src, path, title, start).filter(playable)))
                .collect();
            jobs.into_iter().map(|j| j.join().ok().flatten()).collect()
        });
        found.into_iter().flatten().collect()
    }

    fn extract(&self, src: &str, path: &str, title: &str, start: Option<f64>) -> Option<Stream> {
        let url = format!("{}/proxy/extractor/{}/{}", self.host, src, path);
        let resp = self.client.get(url).timeout(Duration::from_secs(25)).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: Extractor = resp.json().ok()?;
        let referer = body.headers.and_then(|h| h.get("Referer").or_else(|| h.get("referer")).cloned());
        let (raw, hls) = match (body.m3u8, body.mp4) {
            (Some(m), _) => (m, true),
            (None, Some(m)) => (m, false),
            _ => return None,
        };
        let url = if raw.starts_with('/') { format!("{}{}", self.host, raw) } else { raw };
        Some(Stream {
            source: src.to_string(),
            url,
            referer,
            hls,
            title: title.to_string(),
            start,
            subs: Vec::new(),
            intro: None,
        })
    }

    pub fn poster(&self, poster_path: &str) -> Option<Vec<u8>> {
        let resp = if poster_path.starts_with("/proxy") {
            // already a proxied path (anilist covers come back like /proxy/image?url=...)
            self.client.get(format!("{}{}", self.host, poster_path)).send().ok()?
        } else if poster_path.starts_with("http") {
            self.client.get(format!("{}/proxy/image", self.host)).query(&[("url", poster_path)]).send().ok()?
        } else {
            let src = format!("https://image.tmdb.org/t/p/w500{poster_path}");
            self.client.get(format!("{}/proxy/image", self.host)).query(&[("url", &src)]).send().ok()?
        };
        if !resp.status().is_success() {
            return None;
        }
        resp.bytes().ok().map(|b| b.to_vec())
    }

    pub fn anime_search(&self, query: &str) -> Result<Vec<Title>> {
        let url = format!("{}/miruro/search", self.host);
        let resp: AnimeSearchResp = self.client.get(url).query(&[("query", query)]).send()?.json()?;
        Ok(resp.results.into_iter().map(AnimeItem::into_title).collect())
    }

    pub fn anime_chain(&self, anilist_id: u64) -> Vec<AnimeSeason> {
        let url = format!("{}/miruro/seasons/{}", self.host, anilist_id);
        self.client
            .get(url)
            .timeout(Duration::from_secs(15))
            .send()
            .ok()
            .and_then(|r| r.json::<ChainResp>().ok())
            .map(|r| r.seasons)
            .unwrap_or_default()
    }

    pub fn anime_search_merged(&self, query: &str) -> Vec<Title> {
        let found = self.anime_search(query).unwrap_or_default();
        let chains: Vec<Vec<AnimeSeason>> = std::thread::scope(|s| {
            let jobs: Vec<_> = found.iter().map(|t| s.spawn(move || self.anime_chain(t.id))).collect();
            jobs.into_iter().map(|j| j.join().unwrap_or_default()).collect()
        });
        let mut out: Vec<Title> = Vec::new();
        let mut seen = HashSet::new();
        for (title, chain) in found.into_iter().zip(chains) {
            let root = chain.first().map(|c| c.id).unwrap_or(title.id);
            if !seen.insert(root) {
                continue;
            }
            if chain.len() < 2 {
                out.push(title);
                continue;
            }
            let first = &chain[0];
            let total: u64 = chain.iter().map(|c| c.episodes).sum();
            out.push(Title {
                id: root,
                kind: Kind::Anime,
                name: first.title.clone(),
                year: first.year.clone(),
                rating: 0.0,
                overview: Some(format!("{} · {}", plural(chain.len() as u64, "season"), plural(total, "episode"))),
                poster_path: first.cover.clone().or(title.poster_path),
                seasons: chain,
            });
        }
        out
    }

    pub fn anime_providers(&self, anilist_id: u64, dub: bool) -> Result<Vec<AnimeProvider>> {
        let url = format!("{}/miruro/episodes/{}", self.host, anilist_id);
        let resp: EpisodesResp = self.client.get(url).send()?.json()?;
        let order: &[&str] = if dub { &ANIME_DUB } else { &ANIME_SUB };
        let mut providers: Vec<AnimeProvider> = resp
            .providers
            .into_iter()
            .map(|(name, raw)| {
                let eps = if dub { raw.episodes.dub } else { raw.episodes.sub };
                AnimeProvider { name, episodes: eps.unwrap_or_default().into_iter().map(AnimeEpRaw::into_ep).collect() }
            })
            .filter(|p| !p.episodes.is_empty() && order.contains(&p.name.as_str()))
            .collect();
        providers.sort_by_key(|p| order.iter().position(|o| *o == p.name));
        Ok(providers)
    }

    pub fn anime_stream(&self, provider: &str, episode_id: &str, title: &str) -> Option<Stream> {
        let url = format!("{}/miruro/{}", self.host, episode_id);
        let resp = self.client.get(url).timeout(Duration::from_secs(30)).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: AnimeSourcesResp = resp.json().ok()?;
        let chosen = body.streams.into_iter().find(|s| !s.url.is_empty())?;
        let hls = chosen.kind.eq_ignore_ascii_case("hls") || chosen.url.contains(".m3u8") || chosen.kind.is_empty();
        let subs = body
            .tracks
            .into_iter()
            .filter(|t| !t.file.is_empty() && !t.file.contains("thumbnails"))
            .map(|t| Subtitle {
                url: t.file,
                referer: t.r#ref.or_else(|| chosen.referer.clone()),
                lang: t.label.or(t.lang).unwrap_or_default(),
            })
            .collect();
        let intro = body.intro.filter(|i| i.end > i.start).map(|i| (i.start, i.end));
        Some(Stream {
            source: provider.to_string(),
            url: chosen.url,
            referer: chosen.referer,
            hls,
            title: title.to_string(),
            start: None,
            subs,
            intro,
        })
        .filter(playable)
    }

    pub fn anime_name(&self, anilist_id: u64) -> Option<String> {
        self.anime_info(anilist_id).map(|(n, _)| n)
    }

    pub fn anime_info(&self, anilist_id: u64) -> Option<(String, Option<String>)> {
        let url = format!("{}/miruro/info/{}", self.host, anilist_id);
        let info: AnimeInfo = self.client.get(url).send().ok()?.json().ok()?;
        let name = info.title.and_then(|t| t.english.or(t.romaji))?;
        let cover = info.cover_image.and_then(|c| c.extra_large.or(c.large));
        Some((name, cover))
    }

    pub fn anime_episode_meta(&self, anilist_id: u64) -> Vec<EpisodeMeta> {
        let url = format!("{}/miruro/tmdb-episodes/{}", self.host, anilist_id);
        self.client
            .get(url)
            .send()
            .ok()
            .and_then(|r| r.json::<EpisodeMetaResp>().ok())
            .map(|r| r.episodes)
            .unwrap_or_default()
    }

    pub fn subtitles(&self, path: &str) -> Vec<Subtitle> {
        let url = format!("{}/proxy/vidcore-subs/{}", self.host, path);
        let Some(body) = self.client.get(url).send().ok().and_then(|r| r.json::<SubsResp>().ok()) else {
            return Vec::new();
        };
        body.subtitles
            .into_iter()
            .filter(|s| !s.file.is_empty())
            .map(|s| Subtitle {
                url: s.file,
                referer: s.r#ref.filter(|r| !r.is_empty()),
                lang: s.lang.or(s.label).unwrap_or_default(),
            })
            .collect()
    }

    pub fn register(&self, username: &str, password: &str) -> Result<String> {
        self.ensure_secure()?;
        let url = format!("{}/api/auth/register", self.host);
        let resp = self.client.post(url).json(&Credentials { username, password }).send()?;
        if !resp.status().is_success() {
            bail!("sign up failed (username may be taken, or the password is too short)");
        }
        token_from(&resp)
    }

    pub fn login(&self, username: &str, password: &str) -> Result<String> {
        self.ensure_secure()?;
        let url = format!("{}/api/auth/login", self.host);
        let resp = self.client.post(url).json(&Credentials { username, password }).send()?;
        if !resp.status().is_success() {
            bail!("login failed (check username and password)");
        }
        token_from(&resp)
    }

    pub fn me(&self) -> Result<Option<String>> {
        let url = format!("{}/api/auth/me", self.host);
        let resp: MeResp = self.authed(self.client.get(url)).send()?.json()?;
        Ok(resp.user.map(|u| u.username))
    }

    pub fn history(&self) -> Result<Vec<HistoryEntry>> {
        let url = format!("{}/api/me/history", self.host);
        let resp = self.authed(self.client.get(url)).send()?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        Ok(resp.json::<HistoryResp>()?.items)
    }

    pub fn continue_rows(&self) -> Result<Vec<ContinueRow>> {
        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        for entry in self.history()? {
            if !seen.insert((entry.kind.clone(), entry.content_id.clone())) {
                continue;
            }
            let id: u64 = match entry.content_id.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let kind = match entry.kind.as_str() {
                "tv" => Kind::Tv,
                "anime" => Kind::Anime,
                _ => Kind::Movie,
            };
            let (show, poster) = if kind == Kind::Anime { self.anime_info(id) } else { self.title_info(&entry.kind, id) }
                .unwrap_or_else(|| (entry.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| "untitled".into()), None));
            rows.push(ContinueRow {
                id,
                kind,
                show,
                poster,
                season: entry.season as u32,
                episode: entry.episode as u32,
                position: entry.position,
                duration: entry.duration,
            });
            if rows.len() >= 8 {
                break;
            }
        }
        Ok(rows)
    }

    pub fn save_progress(&self, update: &ProgressUpdate) -> Result<()> {
        let url = format!("{}/api/me/history", self.host);
        self.authed(self.client.post(url)).json(update).send()?;
        Ok(())
    }

    fn title_info(&self, kind: &str, id: u64) -> Option<(String, Option<String>)> {
        let slug = if kind == "tv" { "tv" } else { "movie" };
        let url = format!("{}/proxy/tmdb/{}/{}", self.host, slug, id);
        let resp: NameResp = self.client.get(url).send().ok()?.json().ok()?;
        Some((resp.name.or(resp.title)?, resp.poster_path.filter(|p| !p.is_empty())))
    }

    fn authed(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.token {
            Some(token) => builder.header(COOKIE, format!("viel_sess={token}")),
            None => builder,
        }
    }

    fn ensure_secure(&self) -> Result<()> {
        if self.host.starts_with("https://")
            || self.host.contains("localhost")
            || self.host.contains("127.0.0.1")
        {
            return Ok(());
        }
        bail!("refusing to send credentials over an insecure connection: {}\nuse an https host", self.host)
    }
}

fn token_from(resp: &reqwest::blocking::Response) -> Result<String> {
    for value in resp.headers().get_all(SET_COOKIE) {
        if let Ok(text) = value.to_str() {
            if let Some(rest) = text.strip_prefix("viel_sess=") {
                let token = rest.split(';').next().unwrap_or_default().to_string();
                if !token.is_empty() {
                    return Ok(token);
                }
            }
        }
    }
    bail!("authenticated but no session cookie was returned")
}

#[derive(Serialize)]
struct Credentials<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
pub struct ProgressUpdate<'a> {
    #[serde(rename = "type")]
    pub kind: &'a str,
    pub content_id: String,
    pub season: u32,
    pub episode: u32,
    pub position: f64,
    pub duration: f64,
    pub title: &'a str,
}

#[derive(Deserialize)]
pub struct HistoryEntry {
    #[serde(rename = "type")]
    pub kind: String,
    pub content_id: String,
    #[serde(default)]
    pub season: i64,
    #[serde(default)]
    pub episode: i64,
    pub position: f64,
    pub duration: f64,
    pub title: Option<String>,
}

fn playable(stream: &Stream) -> bool {
    let referer = stream.referer.as_deref();
    if !stream.hls {
        return net::send(&stream.url, referer, Some("bytes=0-4095"))
            .and_then(|r| Ok(r.bytes()?.to_vec()))
            .map(|b| net::looks_like_video(&b))
            .unwrap_or(false);
    }
    let check = || -> Result<bool> {
        let mut url = stream.url.clone();
        let mut text = net::text(&url, referer)?;
        if hls::is_master(&text) {
            let variant = hls::best(hls::variants(&text, &url), 1080).ok_or_else(|| anyhow!("empty master"))?;
            url = variant.url;
            text = net::text(&url, referer)?;
        }
        let segment = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .ok_or_else(|| anyhow!("empty playlist"))?;
        let bytes = net::send(&net::join(&url, segment), referer, Some("bytes=0-4095"))?.bytes()?;
        Ok(net::looks_like_video(&bytes))
    };
    check().unwrap_or(false)
}

fn remaining(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[derive(Deserialize)]
struct SearchResp {
    results: Vec<SearchItem>,
}

#[derive(Deserialize)]
struct SearchItem {
    id: u64,
    media_type: Option<String>,
    title: Option<String>,
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f64>,
    overview: Option<String>,
    poster_path: Option<String>,
}

impl SearchItem {
    fn into_title(self) -> Option<Title> {
        let kind = match self.media_type.as_deref() {
            Some("movie") => Kind::Movie,
            Some("tv") => Kind::Tv,
            _ => return None,
        };
        let name = self.title.or(self.name)?;
        let date = self.release_date.or(self.first_air_date).unwrap_or_default();
        let year = date.split('-').next().unwrap_or_default().to_string();
        Some(Title {
            id: self.id,
            kind,
            name,
            year,
            rating: self.vote_average.unwrap_or(0.0),
            overview: self.overview.filter(|s| !s.is_empty()),
            poster_path: self.poster_path.filter(|s| !s.is_empty()),
            seasons: Vec::new(),
        })
    }
}

#[derive(Deserialize)]
struct TvDetail {
    seasons: Vec<SeasonItem>,
}

#[derive(Deserialize)]
struct SeasonItem {
    season_number: u32,
    name: String,
    episode_count: u32,
}

#[derive(Deserialize)]
struct SeasonDetail {
    episodes: Vec<EpisodeItem>,
}

#[derive(Deserialize)]
struct EpisodeItem {
    episode_number: u32,
    name: String,
    overview: Option<String>,
    still_path: Option<String>,
}

#[derive(Deserialize)]
struct Extractor {
    m3u8: Option<String>,
    mp4: Option<String>,
    headers: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct HistoryResp {
    items: Vec<HistoryEntry>,
}

#[derive(Deserialize)]
struct MeResp {
    user: Option<MeUser>,
}

#[derive(Deserialize)]
struct MeUser {
    username: String,
}

#[derive(Deserialize)]
struct NameResp {
    name: Option<String>,
    title: Option<String>,
    poster_path: Option<String>,
}

#[derive(Deserialize)]
struct EpisodeMetaResp {
    #[serde(default)]
    episodes: Vec<EpisodeMeta>,
}

#[derive(Deserialize, Clone)]
pub struct EpisodeMeta {
    #[serde(default)]
    pub num: f64,
    pub title: Option<String>,
    pub desc: Option<String>,
    pub still: Option<String>,
}

pub struct AnimeEpisode {
    pub id: String,
    pub number: f64,
    pub title: Option<String>,
}

impl fmt::Display for AnimeEpisode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = if self.number.fract() == 0.0 {
            format!("{}", self.number as i64)
        } else {
            format!("{}", self.number)
        };
        match &self.title {
            Some(t) => write!(f, "E{n} · {t}"),
            None => write!(f, "E{n}"),
        }
    }
}

pub struct AnimeProvider {
    pub name: String,
    pub episodes: Vec<AnimeEpisode>,
}

pub fn merge_episodes(providers: &[AnimeProvider]) -> Vec<AnimeEpisode> {
    let mut merged: Vec<AnimeEpisode> = Vec::new();
    for provider in providers {
        for ep in &provider.episodes {
            match merged.iter_mut().find(|m| m.number == ep.number) {
                Some(m) if m.title.is_none() => m.title = ep.title.clone(),
                Some(_) => {}
                None => merged.push(AnimeEpisode { id: ep.id.clone(), number: ep.number, title: ep.title.clone() }),
            }
        }
    }
    merged.sort_by(|a, b| a.number.total_cmp(&b.number));
    merged
}

pub fn episode_sources(providers: &[AnimeProvider], number: f64) -> Vec<(String, String)> {
    providers
        .iter()
        .filter_map(|p| p.episodes.iter().find(|e| e.number == number).map(|e| (p.name.clone(), e.id.clone())))
        .collect()
}

#[derive(Deserialize)]
struct ChainResp {
    #[serde(default)]
    seasons: Vec<AnimeSeason>,
}

#[derive(Deserialize)]
struct AnimeSearchResp {
    results: Vec<AnimeItem>,
}

#[derive(Deserialize)]
struct AnimeItem {
    id: u64,
    title: Option<AnimeTitleField>,
    #[serde(rename = "seasonYear")]
    season_year: Option<u32>,
    episodes: Option<u32>,
    cover_image: Option<AnimeCover>,
    #[serde(rename = "coverImage")]
    cover_image_alt: Option<AnimeCover>,
}

#[derive(Deserialize)]
struct AnimeTitleField {
    english: Option<String>,
    romaji: Option<String>,
}

#[derive(Deserialize)]
struct AnimeCover {
    large: Option<String>,
    #[serde(rename = "extraLarge")]
    extra_large: Option<String>,
}

impl AnimeItem {
    fn into_title(self) -> Title {
        let title = self.title.as_ref();
        let name = title
            .and_then(|t| t.english.clone().or_else(|| t.romaji.clone()))
            .unwrap_or_else(|| "Untitled".into());
        let cover = self.cover_image.or(self.cover_image_alt);
        let poster = cover.and_then(|c| c.extra_large.or(c.large));
        Title {
            id: self.id,
            kind: Kind::Anime,
            name,
            year: self.season_year.map(|y| y.to_string()).unwrap_or_default(),
            rating: 0.0,
            overview: self.episodes.map(|e| format!("{e} episodes")),
            poster_path: poster,
            seasons: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
struct EpisodesResp {
    #[serde(default)]
    providers: HashMap<String, ProviderRaw>,
}

#[derive(Deserialize)]
struct ProviderRaw {
    #[serde(default)]
    episodes: CategoriesRaw,
}

#[derive(Deserialize, Default)]
struct CategoriesRaw {
    sub: Option<Vec<AnimeEpRaw>>,
    dub: Option<Vec<AnimeEpRaw>>,
}

#[derive(Deserialize)]
struct AnimeEpRaw {
    id: String,
    #[serde(default)]
    number: f64,
    title: Option<String>,
}

impl AnimeEpRaw {
    fn into_ep(self) -> AnimeEpisode {
        AnimeEpisode { id: self.id, number: self.number, title: self.title }
    }
}

#[derive(Deserialize)]
struct AnimeSourcesResp {
    #[serde(default)]
    streams: Vec<AnimeStreamRaw>,
    #[serde(default)]
    tracks: Vec<TrackRaw>,
    intro: Option<Span>,
}

#[derive(Deserialize)]
struct TrackRaw {
    #[serde(default)]
    file: String,
    label: Option<String>,
    lang: Option<String>,
    r#ref: Option<String>,
}

#[derive(Deserialize)]
struct Span {
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
}

#[derive(Deserialize)]
struct AnimeInfo {
    title: Option<AnimeTitleField>,
    #[serde(rename = "coverImage")]
    cover_image: Option<AnimeCover>,
}

#[derive(Deserialize)]
struct AnimeStreamRaw {
    url: String,
    #[serde(rename = "type", default)]
    kind: String,
    referer: Option<String>,
}

#[derive(Deserialize)]
struct SubsResp {
    #[serde(default)]
    subtitles: Vec<SubRaw>,
}

#[derive(Deserialize)]
struct SubRaw {
    file: String,
    lang: Option<String>,
    label: Option<String>,
    r#ref: Option<String>,
}
