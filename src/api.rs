use anyhow::{anyhow, bail, Result};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{COOKIE, RANGE, SET_COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

const SOURCES: [&str; 4] = ["vidrock", "vidzee", "videasy", "anyembed"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Movie,
    Tv,
}

impl Kind {
    pub fn slug(self) -> &'static str {
        match self {
            Kind::Movie => "movie",
            Kind::Tv => "tv",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.slug())
    }
}

pub struct Title {
    pub id: u64,
    pub kind: Kind,
    pub name: String,
    pub year: String,
    pub rating: f64,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
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
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "E{} · {}", self.number, self.name)
    }
}

pub struct Stream {
    pub url: String,
    pub referer: Option<String>,
    pub title: String,
    pub start: Option<f64>,
}

pub struct ContinueRow {
    pub id: u64,
    pub kind: Kind,
    pub show: String,
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
            Kind::Movie => write!(f, "{} · {} left", self.show, left),
        }
    }
}

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
            })
            .collect())
    }

    pub fn resolve(&self, path: &str, title: &str, start: Option<f64>) -> Result<Stream> {
        for src in SOURCES {
            let url = format!("{}/proxy/extractor/{}/{}", self.host, src, path);
            let resp = match self.client.get(&url).send() {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };
            let body: Extractor = match resp.json() {
                Ok(b) => b,
                Err(_) => continue,
            };
            let referer = body
                .headers
                .and_then(|h| h.get("Referer").or_else(|| h.get("referer")).cloned());
            let (raw, endpoint) = if let Some(m) = body.m3u8 {
                (m, "manifest")
            } else if let Some(m) = body.mp4 {
                (m, "segment")
            } else {
                continue;
            };
            let mut proxied = format!("{}/proxy/{}?url={}", self.host, endpoint, urlencoding::encode(&raw));
            if let Some(referer) = &referer {
                proxied.push_str("&ref=");
                proxied.push_str(&urlencoding::encode(referer));
            }
            if endpoint == "segment" || self.playable(&proxied) {
                return Ok(Stream {
                    url: proxied,
                    referer: None,
                    title: title.to_string(),
                    start,
                });
            }
        }
        Err(anyhow!("no playable source found"))
    }

    pub fn poster(&self, poster_path: &str) -> Option<Vec<u8>> {
        let src = format!("https://image.tmdb.org/t/p/w342{poster_path}");
        let url = format!("{}/proxy/image", self.host);
        let resp = self.client.get(url).query(&[("url", &src)]).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.bytes().ok().map(|b| b.to_vec())
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

    fn playable(&self, master_url: &str) -> bool {
        let master = match self.client.get(master_url).send().and_then(|r| r.text()) {
            Ok(t) => t,
            Err(_) => return false,
        };
        if !master.contains("#EXTM3U") {
            return false;
        }
        let child = match first_uri(&master) {
            Some(c) => self.absolute(&c),
            None => return false,
        };
        if child.contains("/proxy/manifest") {
            let media = match self.client.get(&child).send().and_then(|r| r.text()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            match first_uri(&media) {
                Some(s) => self.segment_ok(&self.absolute(&s)),
                None => false,
            }
        } else {
            self.segment_ok(&child)
        }
    }

    fn segment_ok(&self, url: &str) -> bool {
        let resp = match self.client.get(url).header(RANGE, "bytes=0-1024").send() {
            Ok(r) => r,
            Err(_) => return false,
        };
        if !resp.status().is_success() {
            return false;
        }
        match resp.bytes() {
            Ok(bytes) => looks_like_video(&bytes),
            Err(_) => false,
        }
    }

    fn absolute(&self, line: &str) -> String {
        let line = line.trim();
        if line.starts_with("http") {
            line.to_string()
        } else if line.starts_with('/') {
            format!("{}{}", self.host, line)
        } else {
            format!("{}/{}", self.host, line)
        }
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
            let kind = if entry.kind == "tv" { Kind::Tv } else { Kind::Movie };
            let show = self
                .title_name(&entry.kind, id)
                .or_else(|| entry.title.clone())
                .unwrap_or_else(|| "untitled".into());
            rows.push(ContinueRow {
                id,
                kind,
                show,
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

    fn title_name(&self, kind: &str, id: u64) -> Option<String> {
        let slug = if kind == "tv" { "tv" } else { "movie" };
        let url = format!("{}/proxy/tmdb/{}/{}", self.host, slug, id);
        let resp: NameResp = self.client.get(url).send().ok()?.json().ok()?;
        resp.name.or(resp.title)
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

fn first_uri(playlist: &str) -> Option<String> {
    playlist
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
}

fn looks_like_video(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    if bytes.starts_with(b"\x89PNG")
        || bytes.starts_with(b"GIF8")
        || bytes.starts_with(&[0xFF, 0xD8])
        || bytes.first() == Some(&b'<')
    {
        return false;
    }
    true
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
}
