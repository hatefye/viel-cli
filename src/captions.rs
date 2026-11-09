use crate::{hls, net};
use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

type Decryptor = cbc::Decryptor<aes::Aes128>;

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn text() -> &'static Mutex<String> {
    static TEXT: OnceLock<Mutex<String>> = OnceLock::new();
    TEXT.get_or_init(|| Mutex::new(String::from("WEBVTT\n")))
}

pub fn set(vtt: &str) {
    *text().lock().unwrap() = vtt.to_string();
}

fn offsets() -> &'static Mutex<HashMap<String, Option<u64>>> {
    static MAP: OnceLock<Mutex<HashMap<String, Option<u64>>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

pub fn inject(master: &str, playlist: &str) -> String {
    let media = format!(
        "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"viel\",NAME=\"Subtitles\",LANGUAGE=\"und\",DEFAULT=NO,AUTOSELECT=NO,FORCED=NO,URI=\"{playlist}\""
    );
    let mut out = Vec::new();
    let mut placed = false;
    for line in master.lines() {
        if line.starts_with("#EXT-X-STREAM-INF") {
            if !placed {
                out.push(media.clone());
                placed = true;
            }
            out.push(format!("{line},SUBTITLES=\"viel\""));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

pub fn first_variant(master: &str, base: &str) -> Option<String> {
    hls::variants(master, base).into_iter().next().map(|v| v.url)
}

pub fn playlist(variant: &str, referer: Option<&str>, vtt_link: &str) -> Result<String> {
    let text = net::send_with(variant, referer, None, 2)?.text()?;
    let total: f64 = text
        .lines()
        .filter_map(|l| l.strip_prefix("#EXTINF:"))
        .filter_map(|l| l.split(',').next()?.trim().parse::<f64>().ok())
        .sum();
    let length = if total > 0.0 { total.ceil() } else { 7200.0 };
    Ok(format!(
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:{length}\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXTINF:{length:.3},\n{vtt_link}\n#EXT-X-ENDLIST\n"
    ))
}

pub fn vtt(variant: &str, referer: Option<&str>) -> String {
    let cached = offsets().lock().unwrap().get(variant).copied();
    let start = match cached {
        Some(v) => v,
        None => {
            let v = first_pts(variant, referer).ok().flatten();
            offsets().lock().unwrap().insert(variant.to_string(), v);
            v
        }
    };
    let body = text().lock().unwrap().clone();
    let cues = body
        .trim_start_matches('\u{feff}')
        .strip_prefix("WEBVTT")
        .unwrap_or(&body)
        .trim_start_matches(|c| c != '\n')
        .trim_start_matches('\n')
        .to_string();
    match start {
        Some(pts) => format!("WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:{pts},LOCAL:00:00:00.000\n\n{cues}"),
        None => format!("WEBVTT\n\n{cues}"),
    }
}

fn first_pts(variant: &str, referer: Option<&str>) -> Result<Option<u64>> {
    let text = net::send_with(variant, referer, None, 2)?.text()?;
    if text.lines().any(|l| l.starts_with("#EXT-X-MAP")) {
        return Ok(None);
    }
    let mut key: Option<(String, Option<String>)> = None;
    let mut sequence = 0u64;
    let mut segment = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            sequence = v.trim().parse().unwrap_or(0);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-KEY:") {
            key = if attrs.contains("METHOD=AES-128") {
                Some((attr(attrs, "URI").unwrap_or_default(), attr(attrs, "IV")))
            } else {
                None
            };
        } else if !line.is_empty() && !line.starts_with('#') {
            segment = Some(net::join(variant, line.trim()));
            break;
        }
    }
    let segment = segment.ok_or_else(|| anyhow!("empty playlist"))?;
    let mut bytes = net::send_with(&segment, referer, Some("bytes=0-65535"), 2)?
        .bytes()?
        .to_vec();
    if let Some((uri, iv)) = key {
        let raw = net::send_with(&net::join(variant, &uri), referer, None, 2)?.bytes()?;
        let key: [u8; 16] = raw.get(..16).and_then(|k| k.try_into().ok()).ok_or_else(|| anyhow!("short key"))?;
        let iv = iv.and_then(|h| parse_iv(&h)).unwrap_or_else(|| {
            let mut iv = [0u8; 16];
            iv[8..].copy_from_slice(&sequence.to_be_bytes());
            iv
        });
        bytes.truncate(bytes.len() / 16 * 16);
        Decryptor::new(&key.into(), &iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut bytes)
            .map_err(|_| anyhow!("decrypt"))?;
    }
    Ok(scan(&bytes))
}

fn attr(attrs: &str, name: &str) -> Option<String> {
    let at = attrs.find(&format!("{name}="))? + name.len() + 1;
    let rest = &attrs[at..];
    let value = if let Some(quoted) = rest.strip_prefix('"') {
        quoted.split('"').next()?
    } else {
        rest.split(',').next()?
    };
    Some(value.to_string())
}

fn parse_iv(hex: &str) -> Option<[u8; 16]> {
    let digits = hex.trim_start_matches("0x").trim_start_matches("0X");
    let value = u128::from_str_radix(digits, 16).ok()?;
    Some(value.to_be_bytes())
}

fn scan(data: &[u8]) -> Option<u64> {
    let sync = (0..data.len().min(188 * 4)).find(|&i| {
        data.get(i) == Some(&0x47) && data.get(i + 188) == Some(&0x47) && data.get(i + 376) == Some(&0x47)
    })?;
    let mut best: Option<u64> = None;
    for packet in data[sync..].chunks_exact(188) {
        if packet[0] != 0x47 || packet[1] & 0x40 == 0 {
            continue;
        }
        let mut at = 4;
        if packet[3] & 0x20 != 0 {
            at += 1 + packet[4] as usize;
        }
        let pes = packet.get(at..)?;
        if pes.len() < 14 || pes[..3] != [0, 0, 1] || pes[7] & 0x80 == 0 {
            continue;
        }
        let p = &pes[9..14];
        let pts = ((p[0] as u64 >> 1) & 0x07) << 30
            | (p[1] as u64) << 22
            | ((p[2] as u64) >> 1) << 15
            | (p[3] as u64) << 7
            | (p[4] as u64) >> 1;
        best = Some(best.map_or(pts, |b| b.min(pts)));
    }
    best
}
