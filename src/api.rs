use crate::{hls, net};
use anyhow::{anyhow, bail, Result};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{COOKIE, SET_COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::thread;
use std::time::Duration;

const SOURCES: [&str; 5] = ["cno", "vixsrc", "mapple", "vidbolt", "vidrock"];
const ANIME_SUB: [&str; 4] = ["kototv", "animeparadise", "animekai", "luffy"];
const ANIME_DUB: [&str; 3] = ["kototv", "animekai", "luffy"];

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
    pub backdrop_path: Option<String>,
    pub genres: Vec<String>,
}

impl Title {
    pub fn bare(id: u64, kind: Kind, name: String, poster_path: Option<String>) -> Self {
        Title {
            id,
            kind,
            name,
            year: String::new(),
            rating: 0.0,
            overview: None,
            poster_path,
            seasons: Vec::new(),
            backdrop_path: None,
            genres: Vec::new(),
        }
    }
}

pub fn genre_name(id: u32) -> Option<&'static str> {
    Some(match id {
        28 => "Action",
        12 => "Adventure",
        16 => "Animation",
        35 => "Comedy",
        80 => "Crime",
        99 => "Documentary",
        18 => "Drama",
        10751 => "Family",
        14 => "Fantasy",
        36 => "History",
        27 => "Horror",
        10402 => "Music",
        9648 => "Mystery",
        10749 => "Romance",
        878 => "Science Fiction",
        10770 => "TV Movie",
        53 => "Thriller",
        10752 => "War",
        37 => "Western",
        10759 => "Action & Adventure",
        10762 => "Kids",
        10763 => "News",
        10764 => "Reality",
        10765 => "Sci-Fi & Fantasy",
        10766 => "Soap",
        10767 => "Talk",
        10768 => "War & Politics",
        _ => return None,
    })
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
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
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
            year: self.year.clone(),
            overview: Some(plural(self.episodes, "episode")),
            ..Title::bare(
                self.id,
                Kind::Anime,
                self.title.clone(),
                self.cover.or(poster),
            )
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
            write!(
                f,
                "{}{} · {} · {:.1}",
                self.name, year, self.kind, self.rating
            )
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
        write!(
            f,
            "S{} · {} ({} eps)",
            self.number, self.name, self.episodes
        )
    }
}

pub struct Episode {
    pub number: u32,
    pub name: String,
    pub overview: String,
    pub still: Option<String>,
    pub runtime: Option<u32>,
}

pub struct Details {
    pub title: Title,
    pub logo: Option<String>,
    pub runtime: Option<u32>,
    pub tagline: String,
    pub cast: Vec<String>,
    pub seasons: u32,
    pub collection: Option<Collection>,
}

pub struct Collection {
    pub name: String,
    pub parts: Vec<Title>,
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
    pub outro: Option<(f64, f64)>,
    pub thumbs: Option<String>,
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
            Kind::Tv => write!(
                f,
                "{} · S{}E{} · {} left",
                self.show, self.season, self.episode, left
            ),
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
        let resp: SearchResp = self
            .client
            .get(url)
            .query(&[("query", query)])
            .send()?
            .json()?;
        Ok(resp
            .results
            .into_iter()
            .filter_map(|i| i.into_title(None))
            .collect())
    }

    pub fn trending(&self) -> Result<Vec<Title>> {
        self.list("trending/all/week")
    }

    pub fn list(&self, path: &str) -> Result<Vec<Title>> {
        let kind = if path.starts_with("tv/") || path.starts_with("discover/tv") {
            Some(Kind::Tv)
        } else if path.starts_with("movie/") || path.starts_with("discover/movie") {
            Some(Kind::Movie)
        } else {
            None
        };
        let url = format!("{}/proxy/tmdb/{}", self.host, path);
        let resp: SearchResp = self.client.get(url).send()?.json()?;
        Ok(resp
            .results
            .into_iter()
            .filter_map(|i| i.into_title(kind))
            .collect())
    }

    pub fn details(&self, kind: Kind, id: u64) -> Result<Details> {
        if kind == Kind::Anime {
            return self.anime_details(id);
        }
        let slug = kind.slug();
        let (detail, images, credits) = thread::scope(|s| {
            let a = s.spawn(|| -> Result<DetailResp> {
                Ok(self
                    .client
                    .get(format!("{}/proxy/tmdb/{slug}/{id}", self.host))
                    .send()?
                    .json()?)
            });
            let b = s.spawn(|| {
                self.client
                    .get(format!("{}/proxy/tmdb/{slug}/{id}/images", self.host))
                    .query(&[("include_image_language", "en,null")])
                    .send()
                    .ok()?
                    .json::<ImagesResp>()
                    .ok()
            });
            let c = s.spawn(|| {
                self.client
                    .get(format!("{}/proxy/tmdb/{slug}/{id}/credits", self.host))
                    .send()
                    .ok()?
                    .json::<CreditsResp>()
                    .ok()
            });
            (
                a.join()
                    .unwrap_or_else(|_| Err(anyhow!("details lookup failed"))),
                b.join().ok().flatten(),
                c.join().ok().flatten(),
            )
        });
        let detail = detail?;
        let logo = images.and_then(|i| {
            let png = i
                .logos
                .iter()
                .find(|l| l.file_path.ends_with(".png"))
                .map(|l| l.file_path.clone());
            png.or_else(|| i.logos.into_iter().next().map(|l| l.file_path))
        });
        let date = detail
            .release_date
            .or(detail.first_air_date)
            .unwrap_or_default();
        let runtime = detail
            .runtime
            .or_else(|| detail.episode_run_time.and_then(|r| r.into_iter().next()));
        let collection = detail
            .belongs_to_collection
            .filter(|_| kind == Kind::Movie)
            .and_then(|c| self.collection(c.id));
        Ok(Details {
            title: Title {
                id,
                kind,
                name: detail.title.or(detail.name).unwrap_or_default(),
                year: date.split('-').next().unwrap_or_default().to_string(),
                rating: detail.vote_average.unwrap_or(0.0),
                overview: detail.overview.filter(|s| !s.is_empty()),
                poster_path: detail.poster_path.filter(|s| !s.is_empty()),
                seasons: Vec::new(),
                backdrop_path: detail.backdrop_path.filter(|s| !s.is_empty()),
                genres: detail.genres.into_iter().map(|g| g.name).collect(),
            },
            logo,
            runtime,
            tagline: detail.tagline.unwrap_or_default(),
            cast: credits
                .map(|c| c.cast.into_iter().take(6).map(|m| m.name).collect())
                .unwrap_or_default(),
            seasons: detail.number_of_seasons.unwrap_or(0),
            collection,
        })
    }

    fn collection(&self, id: u64) -> Option<Collection> {
        let resp: CollectionResp = self
            .client
            .get(format!("{}/proxy/tmdb/collection/{id}", self.host))
            .timeout(Duration::from_secs(10))
            .send()
            .ok()?
            .json()
            .ok()?;
        let mut parts: Vec<(String, Title)> = resp
            .parts
            .into_iter()
            .filter(|p| p.title.as_deref().map(|t| !t.is_empty()).unwrap_or(false))
            .map(|p| {
                let date = p.release_date.unwrap_or_default();
                let title = Title {
                    year: date.split('-').next().unwrap_or_default().to_string(),
                    rating: p.vote_average.unwrap_or(0.0),
                    overview: p.overview.filter(|s| !s.is_empty()),
                    ..Title::bare(p.id, Kind::Movie, p.title.unwrap_or_default(), p.poster_path)
                };
                (if date.is_empty() { "9999".to_string() } else { date }, title)
            })
            .collect();
        parts.sort_by(|a, b| a.0.cmp(&b.0));
        (parts.len() > 1).then(|| Collection {
            name: resp.name,
            parts: parts.into_iter().map(|(_, t)| t).collect(),
        })
    }

    fn anime_art(&self, id: u64) -> Option<(Option<String>, Option<String>)> {
        let mapped: TmdbMatch = self
            .client
            .get(format!("{}/miruro/tmdb-episodes/{id}", self.host))
            .send()
            .ok()?
            .json()
            .ok()?;
        let images: ImagesResp = self
            .client
            .get(format!("{}/proxy/tmdb/tv/{}/images", self.host, mapped.tmdb_id?))
            .query(&[("include_image_language", "en,null")])
            .send()
            .ok()?
            .json()
            .ok()?;
        let logo = images
            .logos
            .iter()
            .find(|l| l.file_path.ends_with(".png"))
            .or_else(|| images.logos.first())
            .map(|l| l.file_path.clone());
        let backdrop = images.backdrops.first().map(|b| b.file_path.clone());
        Some((logo, backdrop))
    }

    fn anime_details(&self, id: u64) -> Result<Details> {
        let url = format!("{}/miruro/info/{}", self.host, id);
        let (info, art, chain) = thread::scope(|s| {
            let a = s.spawn(|| -> Result<AnimeInfoFull> { Ok(self.client.get(url).send()?.json()?) });
            let b = s.spawn(|| self.anime_art(id));
            let c = s.spawn(|| self.anime_chain(id));
            (
                a.join().unwrap_or_else(|_| Err(anyhow!("anime lookup failed"))),
                b.join().ok().flatten(),
                c.join().unwrap_or_default(),
            )
        });
        let info = info?;
        let (logo, backdrop) = art.unwrap_or((None, None));
        let name = info
            .title
            .as_ref()
            .and_then(|t| t.english.clone().or_else(|| t.romaji.clone()))
            .unwrap_or_default();
        let overview = info
            .description
            .map(|d| strip_html(&d))
            .filter(|d| !d.is_empty());
        let studios = info
            .studios
            .map(|s| {
                s.nodes
                    .into_iter()
                    .filter(|n| n.is_animation_studio)
                    .map(|n| n.name)
                    .take(3)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Details {
            title: Title {
                year: info.season_year.map(|y| y.to_string()).unwrap_or_default(),
                rating: info.average_score.map(|s| s as f64 / 10.0).unwrap_or(0.0),
                overview,
                backdrop_path: backdrop.or(info.banner_image.filter(|s| !s.is_empty())),
                genres: info.genres.unwrap_or_default(),
                seasons: if chain.len() > 1 { chain } else { Vec::new() },
                ..Title::bare(
                    id,
                    Kind::Anime,
                    name,
                    info.cover_image.and_then(|c| c.extra_large.or(c.large)),
                )
            },
            logo,
            runtime: info.duration,
            tagline: String::new(),
            cast: studios,
            seasons: 0,
            collection: None,
        })
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
                runtime: e.runtime,
            })
            .collect())
    }

    pub fn resolve(&self, path: &str, title: &str, start: Option<f64>, movie: bool) -> Vec<Stream> {
        let found: Vec<Option<Stream>> = std::thread::scope(|scope| {
            let jobs: Vec<_> = SOURCES
                .iter()
                .filter(|src| !(movie && **src == "vidbolt"))
                .map(|src| {
                    scope.spawn(move || self.extract(src, path, title, start).filter(playable))
                })
                .collect();
            jobs.into_iter().map(|j| j.join().ok().flatten()).collect()
        });
        found.into_iter().flatten().collect()
    }

    pub fn extract(
        &self,
        src: &str,
        path: &str,
        title: &str,
        start: Option<f64>,
    ) -> Option<Stream> {
        let url = format!("{}/proxy/extractor/{}/{}", self.host, src, path);
        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_secs(25))
            .send()
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: Extractor = resp.json().ok()?;
        let referer = body
            .headers
            .and_then(|h| h.get("Referer").or_else(|| h.get("referer")).cloned());
        let (raw, hls) = match (body.m3u8, body.mp4) {
            (Some(m), _) => (m, true),
            (None, Some(m)) => (m, false),
            _ => return None,
        };
        let url = if raw.starts_with('/') {
            format!("{}{}", self.host, raw)
        } else {
            raw
        };
        Some(Stream {
            source: src.to_string(),
            url,
            referer,
            hls,
            title: title.to_string(),
            start,
            subs: Vec::new(),
            intro: None,
            outro: None,
            thumbs: None,
        })
    }

    pub fn poster(&self, poster_path: &str) -> Option<Vec<u8>> {
        let resp = if poster_path.starts_with("/proxy") {
            self.client
                .get(format!("{}{}", self.host, poster_path))
                .send()
                .ok()?
        } else if poster_path.starts_with("http") {
            self.client
                .get(format!("{}/proxy/image", self.host))
                .query(&[("url", poster_path)])
                .send()
                .ok()?
        } else {
            let src = format!("https://image.tmdb.org/t/p/w500{poster_path}");
            self.client
                .get(format!("{}/proxy/image", self.host))
                .query(&[("url", &src)])
                .send()
                .ok()?
        };
        if !resp.status().is_success() {
            return None;
        }
        resp.bytes().ok().map(|b| b.to_vec())
    }

    pub fn anime_search(&self, query: &str) -> Result<Vec<Title>> {
        let url = format!("{}/miruro/search", self.host);
        let resp: AnimeSearchResp = self
            .client
            .get(url)
            .query(&[("query", query)])
            .send()?
            .json()?;
        Ok(resp
            .results
            .into_iter()
            .map(AnimeItem::into_title)
            .collect())
    }

    pub fn anime_trending(&self) -> Result<Vec<Title>> {
        self.anime_trending_page(1)
    }

    pub fn anime_trending_page(&self, page: u32) -> Result<Vec<Title>> {
        let url = format!("{}/miruro/trending", self.host);
        let resp: AnimeSearchResp = self
            .client
            .get(url)
            .query(&[("page", page.to_string())])
            .send()?
            .json()?;
        Ok(resp
            .results
            .into_iter()
            .map(AnimeItem::into_title)
            .collect())
    }

    pub fn log_event(&self, body: &serde_json::Value) {
        let url = format!("{}/api/playerlog", self.host);
        let _ = self
            .authed(self.client.post(url))
            .timeout(Duration::from_secs(10))
            .json(body)
            .send();
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
            let jobs: Vec<_> = found
                .iter()
                .map(|t| s.spawn(move || self.anime_chain(t.id)))
                .collect();
            jobs.into_iter()
                .map(|j| j.join().unwrap_or_default())
                .collect()
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
                year: first.year.clone(),
                overview: Some(format!(
                    "{} · {}",
                    plural(chain.len() as u64, "season"),
                    plural(total, "episode")
                )),
                seasons: chain.clone(),
                ..Title::bare(
                    root,
                    Kind::Anime,
                    first.title.clone(),
                    first.cover.clone().or(title.poster_path),
                )
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
                let eps = if dub {
                    raw.episodes.dub
                } else {
                    raw.episodes.sub
                };
                AnimeProvider {
                    name,
                    episodes: eps
                        .unwrap_or_default()
                        .into_iter()
                        .map(AnimeEpRaw::into_ep)
                        .collect(),
                }
            })
            .filter(|p| !p.episodes.is_empty() && order.contains(&p.name.as_str()))
            .collect();
        providers.sort_by_key(|p| order.iter().position(|o| *o == p.name));
        Ok(providers)
    }

    pub fn anime_stream(&self, provider: &str, episode_id: &str, title: &str) -> Option<Stream> {
        let url = format!("{}/miruro/{}", self.host, episode_id);
        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: AnimeSourcesResp = resp.json().ok()?;
        let chosen = body.streams.into_iter().find(|s| !s.url.is_empty())?;
        let hls = chosen.kind.eq_ignore_ascii_case("hls")
            || chosen.url.contains(".m3u8")
            || chosen.kind.is_empty();
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
        let intro = body
            .intro
            .filter(|i| i.end > i.start)
            .map(|i| (i.start, i.end));
        let outro = body
            .outro
            .filter(|i| i.end > i.start)
            .map(|i| (i.start, i.end));
        let thumbs = body.thumbnails.filter(|t| t.starts_with("http"));
        Some(Stream {
            source: provider.to_string(),
            url: chosen.url,
            referer: chosen.referer,
            hls,
            title: title.to_string(),
            start: None,
            subs,
            intro,
            outro,
            thumbs,
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
        let Some(body) = self
            .client
            .get(url)
            .send()
            .ok()
            .and_then(|r| r.json::<SubsResp>().ok())
        else {
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
        let resp = self
            .client
            .post(url)
            .json(&Credentials { username, password })
            .send()?;
        if !resp.status().is_success() {
            bail!("sign up failed (username may be taken, or the password is too short)");
        }
        token_from(&resp)
    }

    pub fn login(&self, username: &str, password: &str) -> Result<String> {
        self.ensure_secure()?;
        let url = format!("{}/api/auth/login", self.host);
        let resp = self
            .client
            .post(url)
            .json(&Credentials { username, password })
            .send()?;
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
            let (show, poster) = if kind == Kind::Anime {
                self.anime_info(id)
            } else {
                self.title_info(&entry.kind, id)
            }
            .unwrap_or_else(|| {
                (
                    entry
                        .title
                        .clone()
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| "untitled".into()),
                    None,
                )
            });
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
        let resp = self.authed(self.client.post(url)).json(update).send()?;
        if !resp.status().is_success() {
            bail!("progress not saved ({})", resp.status());
        }
        Ok(())
    }

    pub fn watchlist(&self) -> Result<Vec<WatchEntry>> {
        let url = format!("{}/api/me/watchlist", self.host);
        let resp = self.authed(self.client.get(url)).send()?;
        if !resp.status().is_success() {
            bail!("not signed in");
        }
        Ok(resp.json::<WatchResp>()?.items)
    }

    pub fn skips(&self, id: u64, season: u32, episode: u32) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
        let fetched: Option<SkipResp> = self
            .client
            .get(format!("{}/api/skip/{id}/{season}/{episode}", self.host))
            .timeout(Duration::from_secs(8))
            .send()
            .ok()
            .and_then(|r| r.json().ok());
        let span = |s: Option<SkipSpan>| s.filter(|x| x.end_sec > x.start_sec + 1.0).map(|x| (x.start_sec, x.end_sec));
        match fetched {
            Some(resp) => (span(resp.intro.or(resp.recap)), span(resp.outro)),
            None => (None, None),
        }
    }

    pub fn calendar(&self) -> Result<Vec<Upcoming>> {
        let entries = self.watchlist()?;
        let mut rows: Vec<Upcoming> = thread::scope(|s| {
            let jobs: Vec<_> = entries
                .iter()
                .map(|w| s.spawn(move || self.upcoming(w)))
                .collect();
            jobs.into_iter()
                .filter_map(|j| j.join().ok())
                .flatten()
                .collect()
        });
        rows.sort_by_key(|r| r.airs_at);
        Ok(rows)
    }

    fn upcoming(&self, w: &WatchEntry) -> Vec<Upcoming> {
        let Ok(id) = w.content_id.parse::<u64>() else { return Vec::new() };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let week = 7 * 86_400;
        let fetch = |url: String| {
            self.client
                .get(url)
                .timeout(Duration::from_secs(12))
                .send()
                .ok()
        };
        let mut out = Vec::new();
        match w.kind.as_str() {
            "anime" => {
                let Some(info) = fetch(format!("{}/miruro/info/{}", self.host, id))
                    .and_then(|r| r.json::<AnimeAiring>().ok())
                else {
                    return out;
                };
                if let Some(next) = info.next {
                    if next.airing_at + week > now {
                        out.push(Upcoming {
                            kind: "anime".into(),
                            id,
                            name: w.title.clone(),
                            poster: w.poster.clone(),
                            season: 0,
                            episode: next.episode as u32,
                            title: String::new(),
                            airs_at: next.airing_at,
                        });
                    }
                }
            }
            "tv" => {
                let Some(info) = fetch(format!("{}/proxy/tmdb/tv/{}", self.host, id))
                    .and_then(|r| r.json::<TvAiring>().ok())
                else {
                    return out;
                };
                for ep in [info.last_episode_to_air, info.next_episode_to_air].into_iter().flatten() {
                    let Some(at) = ep.air_date.as_deref().and_then(civil_to_unix) else { continue };
                    if at + week < now || at > now + 60 * 86_400 {
                        continue;
                    }
                    out.push(Upcoming {
                        kind: "tv".into(),
                        id,
                        name: w.title.clone(),
                        poster: w.poster.clone(),
                        season: ep.season_number,
                        episode: ep.episode_number,
                        title: ep.name.clone(),
                        airs_at: at,
                    });
                }
            }
            _ => {}
        }
        out
    }

    pub fn add_watchlist(
        &self,
        kind: &str,
        id: u64,
        title: &str,
        poster: Option<&str>,
    ) -> Result<()> {
        let url = format!("{}/api/me/watchlist", self.host);
        let body = serde_json::json!({ "type": kind, "content_id": id.to_string(), "title": title, "poster": poster.unwrap_or("") });
        let resp = self.authed(self.client.post(url)).json(&body).send()?;
        if !resp.status().is_success() {
            bail!("could not save ({})", resp.status());
        }
        Ok(())
    }

    pub fn remove_watchlist(&self, kind: &str, id: u64) -> Result<()> {
        let url = format!("{}/api/me/watchlist/{}/{}", self.host, kind, id);
        let resp = self.authed(self.client.delete(url)).send()?;
        if !resp.status().is_success() {
            bail!("could not remove ({})", resp.status());
        }
        Ok(())
    }

    pub fn recommendations(&self) -> Result<Vec<Title>> {
        let url = format!("{}/api/me/recommendations", self.host);
        let resp = self.authed(self.client.get(url)).send()?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        Ok(resp
            .json::<RecResp>()?
            .items
            .into_iter()
            .map(|r| {
                let kind = if r.kind == "tv" {
                    Kind::Tv
                } else {
                    Kind::Movie
                };
                let mut t = Title::bare(r.id, kind, r.title, r.poster_path);
                t.year = r
                    .release_date
                    .or(r.first_air_date)
                    .unwrap_or_default()
                    .chars()
                    .take(4)
                    .collect();
                t.rating = r.vote_average.unwrap_or(0.0);
                t.overview = r.overview;
                t.backdrop_path = r.backdrop_path;
                t
            })
            .collect())
    }

    pub fn trailer(&self, kind: Kind, id: u64) -> Option<String> {
        if kind == Kind::Anime {
            let url = format!("{}/miruro/info/{}", self.host, id);
            let info: AnimeTrailerResp = self.client.get(url).send().ok()?.json().ok()?;
            let t = info.trailer?;
            let key = t.id.trim();
            return (!key.is_empty() && t.site.eq_ignore_ascii_case("youtube"))
                .then(|| format!("https://www.youtube.com/watch?v={key}"));
        }
        let url = format!("{}/proxy/tmdb/{}/{}/videos", self.host, kind.slug(), id);
        let resp: VideosResp = self.client.get(url).send().ok()?.json().ok()?;
        let youtube: Vec<&VideoRaw> = resp
            .results
            .iter()
            .filter(|v| v.site.eq_ignore_ascii_case("youtube") && !v.key.is_empty())
            .collect();
        let pick = youtube
            .iter()
            .find(|v| v.kind == "Trailer")
            .or_else(|| youtube.iter().find(|v| v.kind == "Teaser"))
            .or_else(|| youtube.first())?;
        Some(format!("https://www.youtube.com/watch?v={}", pick.key))
    }

    pub fn remove_history(&self, kind: &str, id: u64) -> Result<()> {
        let url = format!("{}/api/me/history/{}/{}", self.host, kind, id);
        let resp = self.authed(self.client.delete(url)).send()?;
        if !resp.status().is_success() {
            bail!("could not remove ({})", resp.status());
        }
        Ok(())
    }

    pub fn sources_for(movie: bool) -> Vec<&'static str> {
        SOURCES
            .iter()
            .copied()
            .filter(|s| !(movie && *s == "vidbolt"))
            .collect()
    }

    pub fn title_info(&self, kind: &str, id: u64) -> Option<(String, Option<String>)> {
        let slug = if kind == "tv" { "tv" } else { "movie" };
        let url = format!("{}/proxy/tmdb/{}/{}", self.host, slug, id);
        let resp: NameResp = self.client.get(url).send().ok()?.json().ok()?;
        Some((
            resp.name.or(resp.title)?,
            resp.poster_path.filter(|p| !p.is_empty()),
        ))
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
        bail!(
            "refusing to send credentials over an insecure connection: {}\nuse an https host",
            self.host
        )
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
    pub poster: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

pub fn playable(stream: &Stream) -> bool {
    let referer = stream.referer.as_deref();
    if !stream.hls {
        return net::send_with(&stream.url, referer, Some("bytes=0-4095"), 2)
            .and_then(|r| Ok(r.bytes()?.to_vec()))
            .map(|b| net::looks_like_video(&b))
            .unwrap_or(false);
    }
    let check = || -> Result<bool> {
        let mut url = stream.url.clone();
        let mut text = net::send_with(&url, referer, None, 2)?.text()?;
        if hls::is_master(&text) {
            let variant = hls::best(hls::variants(&text, &url), 1080)
                .ok_or_else(|| anyhow!("empty master"))?;
            url = variant.url;
            text = net::send_with(&url, referer, None, 2)?.text()?;
        }
        let segment = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .ok_or_else(|| anyhow!("empty playlist"))?;
        let bytes =
            net::send_with(&net::join(&url, segment), referer, Some("bytes=0-4095"), 2)?.bytes()?;
        Ok(net::looks_like_video(&bytes))
    };
    check().unwrap_or(false)
}

pub fn pick_subs(all: Vec<Subtitle>, pref: &str) -> Vec<Subtitle> {
    let pref = pref.to_lowercase();
    let mut subs: Vec<_> = all
        .into_iter()
        .filter(|s| lang_matches(&s.lang, &pref) || lang_matches(&s.lang, "en"))
        .collect();
    subs.sort_by_key(|s| !lang_matches(&s.lang, &pref));
    subs.truncate(3);
    subs
}

pub fn lang_matches(lang: &str, pref: &str) -> bool {
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
    backdrop_path: Option<String>,
    #[serde(default)]
    genre_ids: Vec<u32>,
}

impl SearchItem {
    fn into_title(self, fallback: Option<Kind>) -> Option<Title> {
        let kind = match self.media_type.as_deref() {
            Some("movie") => Kind::Movie,
            Some("tv") => Kind::Tv,
            Some(_) => return None,
            None => fallback?,
        };
        let name = self.title.or(self.name)?;
        let date = self
            .release_date
            .or(self.first_air_date)
            .unwrap_or_default();
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
            backdrop_path: self.backdrop_path.filter(|s| !s.is_empty()),
            genres: self
                .genre_ids
                .iter()
                .filter_map(|g| genre_name(*g))
                .map(str::to_string)
                .collect(),
        })
    }
}

#[derive(Deserialize)]
struct DetailResp {
    title: Option<String>,
    name: Option<String>,
    overview: Option<String>,
    tagline: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f64>,
    runtime: Option<u32>,
    episode_run_time: Option<Vec<u32>>,
    number_of_seasons: Option<u32>,
    #[serde(default)]
    genres: Vec<GenreItem>,
    belongs_to_collection: Option<CollectionStub>,
}

#[derive(Deserialize)]
struct CollectionStub {
    id: u64,
}

#[derive(Deserialize)]
struct CollectionResp {
    name: String,
    #[serde(default)]
    parts: Vec<CollectionPart>,
}

#[derive(Deserialize)]
struct CollectionPart {
    id: u64,
    title: Option<String>,
    release_date: Option<String>,
    poster_path: Option<String>,
    vote_average: Option<f64>,
    overview: Option<String>,
}

#[derive(Deserialize)]
struct GenreItem {
    name: String,
}

#[derive(Deserialize)]
struct ImagesResp {
    #[serde(default)]
    logos: Vec<ImageItem>,
    #[serde(default)]
    backdrops: Vec<ImageItem>,
}

#[derive(Deserialize)]
struct TmdbMatch {
    #[serde(rename = "tmdbId")]
    tmdb_id: Option<u64>,
}

#[derive(Deserialize)]
struct ImageItem {
    file_path: String,
}

#[derive(Deserialize)]
struct CreditsResp {
    #[serde(default)]
    cast: Vec<CastItem>,
}

#[derive(Deserialize)]
struct CastItem {
    name: String,
}

#[derive(Deserialize)]
struct AnimeInfoFull {
    title: Option<AnimeTitleField>,
    description: Option<String>,
    #[serde(rename = "coverImage")]
    cover_image: Option<AnimeCover>,
    #[serde(rename = "bannerImage")]
    banner_image: Option<String>,
    #[serde(rename = "seasonYear", default, deserialize_with = "opt_num")]
    season_year: Option<u64>,
    #[serde(default, deserialize_with = "opt_num")]
    duration: Option<u32>,
    #[serde(rename = "averageScore", default, deserialize_with = "opt_num")]
    average_score: Option<u32>,
    genres: Option<Vec<String>>,
    studios: Option<Studios>,
}

#[derive(Deserialize)]
struct Studios {
    #[serde(default)]
    nodes: Vec<Studio>,
}

#[derive(Deserialize)]
struct Studio {
    name: String,
    #[serde(rename = "isAnimationStudio", default)]
    is_animation_studio: bool,
}

fn opt_num<'de, D: serde::Deserializer<'de>, T: TryFrom<u64>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        N(u64),
        S(String),
        Nil(()),
    }
    let n = match Raw::deserialize(d)? {
        Raw::N(n) => Some(n),
        Raw::S(s) => s.trim().parse().ok(),
        Raw::Nil(()) => None,
    };
    Ok(n.and_then(|n| T::try_from(n).ok()))
}

pub fn strip_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&amp;", "&")
        .trim()
        .to_string()
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
    runtime: Option<u32>,
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
struct WatchResp {
    items: Vec<WatchEntry>,
}

#[derive(Deserialize)]
struct SkipResp {
    intro: Option<SkipSpan>,
    recap: Option<SkipSpan>,
    outro: Option<SkipSpan>,
}

#[derive(Deserialize)]
struct SkipSpan {
    #[serde(default)]
    start_sec: f64,
    #[serde(default)]
    end_sec: f64,
}

pub struct Upcoming {
    pub kind: String,
    pub id: u64,
    pub name: String,
    pub poster: String,
    pub season: u32,
    pub episode: u32,
    pub title: String,
    pub airs_at: u64,
}

#[derive(Deserialize)]
struct AnimeAiring {
    #[serde(rename = "nextAiringEpisode")]
    next: Option<AiringNext>,
}

#[derive(Deserialize)]
struct AiringNext {
    #[serde(rename = "airingAt", deserialize_with = "num")]
    airing_at: u64,
    #[serde(deserialize_with = "num")]
    episode: u64,
}

#[derive(Deserialize)]
struct TvAiring {
    next_episode_to_air: Option<TvEpisodeStub>,
    last_episode_to_air: Option<TvEpisodeStub>,
}

#[derive(Deserialize)]
struct TvEpisodeStub {
    air_date: Option<String>,
    #[serde(default)]
    episode_number: u32,
    #[serde(default)]
    season_number: u32,
    #[serde(default)]
    name: String,
}

fn civil_to_unix(date: &str) -> Option<u64> {
    let mut parts = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + 20 * 3_600).ok()
}

#[derive(Deserialize)]
pub struct WatchEntry {
    #[serde(rename = "type")]
    pub kind: String,
    pub content_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub poster: String,
}

#[derive(Deserialize)]
struct RecResp {
    #[serde(default)]
    items: Vec<RecRaw>,
}

#[derive(Deserialize)]
struct RecRaw {
    #[serde(rename = "type")]
    kind: String,
    id: u64,
    #[serde(default)]
    title: String,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    overview: Option<String>,
}

#[derive(Deserialize)]
struct VideosResp {
    #[serde(default)]
    results: Vec<VideoRaw>,
}

#[derive(Deserialize)]
struct VideoRaw {
    #[serde(default)]
    site: String,
    #[serde(default)]
    key: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Deserialize)]
struct AnimeTrailerResp {
    trailer: Option<AnimeTrailer>,
}

#[derive(Deserialize)]
struct AnimeTrailer {
    #[serde(default)]
    id: String,
    #[serde(default)]
    site: String,
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
                None => merged.push(AnimeEpisode {
                    id: ep.id.clone(),
                    number: ep.number,
                    title: ep.title.clone(),
                }),
            }
        }
    }
    merged.sort_by(|a, b| a.number.total_cmp(&b.number));
    merged
}

pub fn episode_sources(providers: &[AnimeProvider], number: f64) -> Vec<(String, String)> {
    providers
        .iter()
        .filter_map(|p| {
            p.episodes
                .iter()
                .find(|e| e.number == number)
                .map(|e| (p.name.clone(), e.id.clone()))
        })
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
    #[serde(deserialize_with = "num")]
    id: u64,
    title: Option<AnimeTitleField>,
    #[serde(rename = "seasonYear", default, deserialize_with = "opt_num")]
    season_year: Option<u32>,
    #[serde(default, deserialize_with = "opt_num")]
    episodes: Option<u32>,
    cover_image: Option<AnimeCover>,
    #[serde(rename = "coverImage")]
    cover_image_alt: Option<AnimeCover>,
    #[serde(rename = "bannerImage")]
    banner_image: Option<String>,
    genres: Option<Vec<String>>,
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
            year: self.season_year.map(|y| y.to_string()).unwrap_or_default(),
            overview: self.episodes.map(|e| format!("{e} episodes")),
            backdrop_path: self.banner_image.filter(|s| !s.is_empty()),
            genres: self.genres.unwrap_or_default(),
            ..Title::bare(self.id, Kind::Anime, name, poster)
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
        AnimeEpisode {
            id: self.id,
            number: self.number,
            title: self.title,
        }
    }
}

#[derive(Deserialize)]
struct AnimeSourcesResp {
    #[serde(default)]
    streams: Vec<AnimeStreamRaw>,
    #[serde(default)]
    tracks: Vec<TrackRaw>,
    intro: Option<Span>,
    outro: Option<Span>,
    thumbnails: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_to_unix("1970-01-01"), Some(72_000));
        assert_eq!(civil_to_unix("2026-08-09"), Some(1_786_305_600));
        assert_eq!(civil_to_unix("nope"), None);
    }

    #[test]
    #[ignore]
    fn upcoming_rows() {
        let api = Api::new("https://viel.tv", None);
        let entry = WatchEntry { kind: "tv".into(), content_id: "94997".into(), title: "House of the Dragon".into(), poster: String::new() };
        for row in api.upcoming(&entry) {
            println!("{} S{} E{} {} at {}", row.name, row.season, row.episode, row.title, row.airs_at);
        }
        let entry = WatchEntry { kind: "anime".into(), content_id: "21".into(), title: "One Piece".into(), poster: String::new() };
        for row in api.upcoming(&entry) {
            println!("{} E{} at {}", row.name, row.episode, row.airs_at);
        }
    }
}
