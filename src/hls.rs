use crate::net;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Playlist,
    Segment(&'static str),
    Key,
    Init,
}

pub fn is_master(text: &str) -> bool {
    text.contains("#EXT-X-STREAM-INF")
}

fn segment_ext(url: &str, fmp4: bool) -> &'static str {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "ts" => "ts",
        "m4s" | "mp4" | "m4v" | "m4a" | "cmfv" | "cmfa" => "m4s",
        "aac" => "aac",
        "vtt" | "webvtt" => "vtt",
        _ if fmp4 => "m4s",
        _ => "ts",
    }
}

fn uri_attr(line: &str) -> Option<(usize, usize)> {
    let start = line.find("URI=\"")? + 5;
    let len = line[start..].find('"')?;
    Some((start, start + len))
}

pub fn rewrite(text: &str, base: &str, mut map: impl FnMut(Part, String) -> String) -> String {
    let fmp4 = text.contains("#EXT-X-MAP");
    let mut next_is_variant = false;
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#EXT-X-I-FRAME-STREAM-INF") {
            continue;
        }
        if line.starts_with('#') {
            next_is_variant |= line.starts_with("#EXT-X-STREAM-INF");
            let part = if line.starts_with("#EXT-X-KEY") || line.starts_with("#EXT-X-SESSION-KEY") {
                Part::Key
            } else if line.starts_with("#EXT-X-MAP") {
                Part::Init
            } else if line.starts_with("#EXT-X-MEDIA") {
                Part::Playlist
            } else {
                out.push(line.to_string());
                continue;
            };
            match uri_attr(line) {
                Some((a, b)) if !line.contains("METHOD=NONE") => {
                    let mapped = map(part, net::join(base, &line[a..b]));
                    if mapped.is_empty() {
                        continue;
                    }
                    out.push(format!("{}{}{}", &line[..a], mapped, &line[b..]));
                }
                _ => out.push(line.to_string()),
            }
            continue;
        }
        let abs = net::join(base, line);
        let part = if next_is_variant || abs.split('?').next().unwrap_or("").ends_with(".m3u8") {
            Part::Playlist
        } else {
            Part::Segment(segment_ext(&abs, fmp4))
        };
        next_is_variant = false;
        let mapped = map(part, abs);
        if !mapped.is_empty() {
            out.push(mapped);
        }
    }
    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}

pub struct Variant {
    pub url: String,
    pub height: u32,
    pub bandwidth: u64,
    pub inf: String,
    pub audio: Option<String>,
}

pub struct Rendition {
    pub line: String,
    pub group: String,
    pub name: String,
    pub lang: String,
    pub default: bool,
    pub url: String,
}

fn attr(line: &str, key: &str) -> Option<String> {
    let at = line.find(&format!("{key}="))? + key.len() + 1;
    let rest = &line[at..];
    if let Some(quoted) = rest.strip_prefix('"') {
        quoted.split('"').next().map(str::to_string)
    } else {
        rest.split(',').next().map(str::to_string)
    }
}

pub fn variants(text: &str, base: &str) -> Vec<Variant> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("#EXT-X-STREAM-INF"))
        .filter_map(|(i, l)| {
            let uri = lines[i + 1..]
                .iter()
                .find(|n| !n.is_empty() && !n.starts_with('#'))?;
            let height = attr(l, "RESOLUTION")
                .and_then(|r| r.split('x').nth(1).and_then(|h| h.parse().ok()))
                .unwrap_or(0);
            Some(Variant {
                url: net::join(base, uri),
                height,
                bandwidth: attr(l, "BANDWIDTH")
                    .and_then(|b| b.parse().ok())
                    .unwrap_or(0),
                inf: l.to_string(),
                audio: attr(l, "AUDIO"),
            })
        })
        .collect()
}

pub fn best(mut list: Vec<Variant>, max_height: u32) -> Option<Variant> {
    list.sort_by_key(|v| {
        (
            v.height <= max_height || v.height == 0,
            v.height,
            v.bandwidth,
        )
    });
    list.pop()
}

pub fn audio_tracks(text: &str, base: &str, group: &str) -> Vec<Rendition> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("#EXT-X-MEDIA") && l.contains("TYPE=AUDIO"))
        .filter(|l| attr(l, "GROUP-ID").as_deref() == Some(group))
        .filter_map(|l| {
            Some(Rendition {
                line: l.to_string(),
                group: group.to_string(),
                name: attr(l, "NAME").unwrap_or_default(),
                lang: attr(l, "LANGUAGE").unwrap_or_default().to_lowercase(),
                default: attr(l, "DEFAULT").as_deref() == Some("YES"),
                url: net::join(base, &attr(l, "URI")?),
            })
        })
        .collect()
}

pub fn pick_audio(tracks: Vec<Rendition>, lang: &str) -> Option<Rendition> {
    let want = lang.to_lowercase();
    let mut tracks = tracks;
    let idx = tracks
        .iter()
        .position(|t| {
            !want.is_empty()
                && (t.lang.starts_with(&want) || t.name.to_lowercase().starts_with(&want))
        })
        .or_else(|| {
            tracks.iter().position(|t| {
                t.lang.starts_with("en") || t.name.to_lowercase().starts_with("english")
            })
        })
        .or_else(|| tracks.iter().position(|t| t.default))
        .or(if tracks.is_empty() { None } else { Some(0) })?;
    Some(tracks.swap_remove(idx))
}

pub fn trim_master(text: &str, base: &str, max_height: u32, lang: &str) -> Option<String> {
    let variant = best(variants(text, base), max_height)?;
    let mut out = vec!["#EXTM3U".to_string(), "#EXT-X-VERSION:3".to_string()];
    let mut inf = variant.inf.clone();
    if let Some(group) = &variant.audio {
        if let Some(track) = pick_audio(audio_tracks(text, base, group), lang) {
            let line = track.line.replace(
                &format!("URI=\"{}\"", raw_uri(&track.line)),
                &format!("URI=\"{}\"", track.url),
            );
            out.push(line);
        }
    }
    inf = inf
        .split(',')
        .filter(|kv| !kv.starts_with("SUBTITLES=") && !kv.starts_with("CLOSED-CAPTIONS="))
        .map(|kv| match kv.split_once('=') {
            Some((
                key @ ("BANDWIDTH" | "AVERAGE-BANDWIDTH" | "#EXT-X-STREAM-INF:BANDWIDTH"),
                value,
            )) => value
                .parse::<u64>()
                .map(|b| format!("{key}={}", b * 3))
                .unwrap_or_else(|_| kv.to_string()),
            _ => kv.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",");
    out.push(inf);
    out.push(variant.url);
    out.push(String::new());
    Some(out.join("\n"))
}

pub fn raw_uri(line: &str) -> &str {
    line.split("URI=\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .unwrap_or("")
}
