use anyhow::{anyhow, bail, Result};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{COOKIE, SET_COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

impl HistoryEntry {
    pub fn name(&self) -> &str {
        self.title.as_deref().unwrap_or("untitled")
    }
}

impl fmt::Display for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let left = remaining(self.duration - self.position);
        if self.kind == "tv" {
            write!(f, "{} · S{}E{} · {} left", self.name(), self.season, self.episode, left)
        } else {
            write!(f, "{} · {} left", self.name(), left)
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

    pub fn search(&self, kind: Kind, query: &str) -> Result<Vec<Title>> {
        let url = format!("{}/proxy/tmdb/search/multi", self.host);
        let resp: SearchResp = self.client.get(url).query(&[("query", query)]).send()?.json()?;
        Ok(resp
            .results
            .into_iter()
            .filter_map(SearchItem::into_title)
            .filter(|t| t.kind == kind)
            .collect())
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
            if let Some(stream_url) = body.m3u8.or(body.mp4) {
                let referer = body
                    .headers
                    .and_then(|h| h.get("Referer").or_else(|| h.get("referer")).cloned());
                return Ok(Stream {
                    url: stream_url,
                    referer,
                    title: title.to_string(),
                    start,
                });
            }
        }
        Err(anyhow!("no playable source found"))
    }

    pub fn login(&self, username: &str, password: &str) -> Result<String> {
        let url = format!("{}/api/auth/login", self.host);
        let resp = self.client.post(url).json(&Credentials { username, password }).send()?;
        if !resp.status().is_success() {
            bail!("login failed (check username and password)");
        }
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
        bail!("login succeeded but no session cookie was returned")
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

    pub fn save_progress(&self, update: &ProgressUpdate) -> Result<()> {
        let url = format!("{}/api/me/history", self.host);
        self.authed(self.client.post(url)).json(update).send()?;
        Ok(())
    }

    fn authed(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.token {
            Some(token) => builder.header(COOKIE, format!("viel_sess={token}")),
            None => builder,
        }
    }
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
