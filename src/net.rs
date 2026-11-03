use anyhow::{bail, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{RANGE, REFERER};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::thread::sleep;
use std::time::Duration;

pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

fn build(h2: bool) -> Client {
    let builder = Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(40))
        .connect_timeout(Duration::from_secs(12))
        .pool_max_idle_per_host(16);
    if h2 { builder } else { builder.http1_only() }
        .build()
        .expect("http client")
}

pub fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| build(false))
}

fn h2_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| build(true))
}

fn h2_hosts() -> &'static Mutex<HashSet<String>> {
    static HOSTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    HOSTS.get_or_init(Default::default)
}

pub fn send(url: &str, referer: Option<&str>, range: Option<&str>) -> Result<Response> {
    let mut last = None;
    let mut wait = Duration::from_millis(500);
    let mut h2 = h2_hosts().lock().unwrap().contains(host(url));
    for _ in 0..7 {
        let mut req = if h2 { h2_client() } else { client() }.get(url);
        if let Some(r) = referer.filter(|r| !r.is_empty()) {
            req = req.header(REFERER, r);
        }
        if let Some(r) = range {
            req = req.header(RANGE, r);
        }
        match req.send() {
            Ok(resp) if resp.status().is_success() => return Ok(resp),
            Ok(resp) if matches!(resp.status().as_u16(), 429 | 503) => {
                let hint = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(Duration::from_secs);
                last = Some(anyhow::anyhow!("{} from {}", resp.status(), host(url)));
                sleep(hint.unwrap_or(wait).min(Duration::from_secs(15)));
                wait = (wait * 2).min(Duration::from_secs(8));
                continue;
            }
            Ok(resp) if resp.status().as_u16() == 403 && !h2 => {
                h2 = true;
                h2_hosts().lock().unwrap().insert(host(url).to_string());
                continue;
            }
            Ok(resp) if resp.status().is_client_error() => bail!("{} from {}", resp.status(), host(url)),
            Ok(resp) => last = Some(anyhow::anyhow!("{} from {}", resp.status(), host(url))),
            Err(err) => last = Some(err.into()),
        }
        sleep(wait);
        wait = (wait * 2).min(Duration::from_secs(8));
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("request failed")))
}

pub fn bytes(url: &str, referer: Option<&str>) -> Result<Vec<u8>> {
    Ok(send(url, referer, None)?.bytes()?.to_vec())
}

pub fn text(url: &str, referer: Option<&str>) -> Result<String> {
    Ok(send(url, referer, None)?.text()?)
}

pub fn host(url: &str) -> &str {
    url.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or(url)
}

pub fn join(base: &str, rel: &str) -> String {
    reqwest::Url::parse(base)
        .and_then(|b| b.join(rel.trim()))
        .map(|u| u.to_string())
        .unwrap_or_else(|_| rel.trim().to_string())
}

pub fn unmask_ts(buf: &[u8]) -> &[u8] {
    if buf.len() > 376 && buf[0] == 0x47 && buf[188] == 0x47 {
        return buf;
    }
    let limit = buf.len().saturating_sub(376).min(8192);
    (0..limit)
        .find(|&i| buf[i] == 0x47 && buf[i + 188] == 0x47 && buf[i + 376] == 0x47)
        .map(|i| &buf[i..])
        .unwrap_or(buf)
}

pub fn looks_like_video(bytes: &[u8]) -> bool {
    if bytes.len() < 8 {
        return false;
    }
    let ts = unmask_ts(bytes);
    if ts.first() == Some(&0x47) {
        return true;
    }
    if &bytes[4..8] == b"ftyp" || &bytes[4..8] == b"styp" || &bytes[4..8] == b"moof" || &bytes[4..8] == b"sidx" {
        return true;
    }
    !(bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"GIF8") || bytes.starts_with(&[0xFF, 0xD8]) || bytes[0] == b'<' || bytes[0] == b'{')
}
