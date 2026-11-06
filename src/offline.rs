use crate::api::Stream;
use crate::fetch;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone)]
struct Plan {
    meta: Value,
    main: String,
    source: String,
    files: Vec<String>,
    subs: Vec<PlanSub>,
    lang: String,
    intro: Option<(f64, f64)>,
    outro: Option<(f64, f64)>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct PlanSub {
    url: String,
    referer: Option<String>,
    lang: String,
}

fn dir() -> &'static Mutex<Option<PathBuf>> {
    static DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    DIR.get_or_init(|| Mutex::new(None))
}

pub fn set_dir(path: &str) {
    let root = PathBuf::from(path);
    fs::create_dir_all(&root).ok();
    *dir().lock().unwrap() = Some(root);
}

fn root() -> Result<PathBuf> {
    dir()
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| anyhow!("downloads are not set up"))
}

fn safe(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn folder(key: &str) -> Result<PathBuf> {
    if !safe(key) {
        bail!("bad download key");
    }
    Ok(root()?.join(key))
}

pub fn path(key: &str, file: &str) -> Option<PathBuf> {
    if file.contains("..") || file.contains('/') || file.is_empty() {
        return None;
    }
    let path = folder(key).ok()?.join(file);
    path.is_file().then_some(path)
}

fn read_plan(key: &str) -> Option<Plan> {
    serde_json::from_slice(&fs::read(folder(key).ok()?.join("plan.json")).ok()?).ok()
}

fn write_plan(key: &str, plan: &Plan) -> Result<()> {
    fs::write(folder(key)?.join("plan.json"), serde_json::to_vec(plan)?)?;
    Ok(())
}

pub fn plan(
    key: &str,
    meta: Value,
    streams: &[Stream],
    max_height: u32,
    lang: &str,
) -> Result<Value> {
    let folder = folder(key)?;
    if folder.join("meta.json").exists() {
        bail!("already downloaded");
    }
    if streams.is_empty() {
        bail!("no source has this one right now");
    }
    let mut last = None;
    for stream in streams {
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder)?;
        let prepared = if stream.hls {
            fetch::prepare(stream, &folder, max_height, lang)
        } else {
            Ok((
                "video.mp4".to_string(),
                vec![fetch::Task {
                    url: stream.url.clone(),
                    file: folder.join("video.mp4"),
                    ts: false,
                }],
            ))
        };
        match prepared {
            Ok((main, tasks)) => {
                let files: Vec<String> = tasks
                    .iter()
                    .map(|t| {
                        t.file
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    })
                    .collect();
                let plan = Plan {
                    meta,
                    main,
                    source: stream.source.clone(),
                    files: files.clone(),
                    subs: stream
                        .subs
                        .iter()
                        .map(|s| PlanSub {
                            url: s.url.clone(),
                            referer: s.referer.clone(),
                            lang: s.lang.clone(),
                        })
                        .collect(),
                    lang: lang.to_string(),
                    intro: stream.intro,
                    outro: stream.outro,
                    error: None,
                };
                write_plan(key, &plan)?;
                let parts: Vec<Value> = tasks
                    .iter()
                    .zip(&files)
                    .map(|(t, name)| json!({ "url": t.url, "file": name, "ts": t.ts }))
                    .collect();
                return Ok(json!({
                    "key": key,
                    "source": stream.source,
                    "referer": stream.referer,
                    "total": parts.len(),
                    "parts": parts,
                }));
            }
            Err(err) => last = Some(format!("{}: {err}", stream.source)),
        }
    }
    let _ = fs::remove_dir_all(&folder);
    bail!(
        "{}",
        last.unwrap_or_else(|| "no source has this one".into())
    )
}

pub fn missing(key: &str) -> Result<Vec<String>> {
    let plan = read_plan(key).ok_or_else(|| anyhow!("no download in progress"))?;
    let folder = folder(key)?;
    Ok(plan
        .files
        .into_iter()
        .filter(|f| !folder.join(f).is_file())
        .collect())
}

pub fn finish(key: &str) -> Result<Value> {
    let plan = read_plan(key).ok_or_else(|| anyhow!("no download in progress"))?;
    let folder = folder(key)?;
    let missing: Vec<&String> = plan
        .files
        .iter()
        .filter(|f| !folder.join(f).is_file())
        .collect();
    if !missing.is_empty() {
        return Ok(json!({ "done": false, "missing": missing.len(), "files": missing }));
    }
    let stream = Stream {
        source: plan.source.clone(),
        url: String::new(),
        referer: None,
        hls: true,
        title: String::new(),
        start: None,
        subs: plan
            .subs
            .iter()
            .map(|s| crate::api::Subtitle {
                url: s.url.clone(),
                referer: s.referer.clone(),
                lang: s.lang.clone(),
            })
            .collect(),
        intro: None,
        outro: None,
        thumbs: None,
    };
    let sub = fetch::save_subtitle(&stream, &plan.lang, &folder);
    let main = match crate::mux::remux(&folder, &plan.main) {
        Ok(main) => {
            crate::mux::cleanup(&folder, &main);
            main
        }
        Err(err) => {
            let _ = fs::remove_file(folder.join("video.mp4.part"));
            crate::local::log("download", format!("{key} kept as playlist: {err}"));
            plan.main.clone()
        }
    };
    let bytes = folder_size(&folder);
    let mut meta = plan.meta.clone();
    if let Value::Object(map) = &mut meta {
        map.insert("main".into(), json!(main));
        map.insert("source".into(), json!(plan.source));
        map.insert("bytes".into(), json!(bytes));
        map.insert("intro".into(), json!(plan.intro));
        map.insert("outro".into(), json!(plan.outro));
        map.insert("saved_at".into(), json!(now()));
        if let Some(sub) = sub {
            map.insert(
                "sub".into(),
                json!(sub.file_name().map(|n| n.to_string_lossy().to_string())),
            );
            map.insert("sub_lang".into(), json!(plan.lang));
        }
    }
    fs::write(folder.join("meta.json"), serde_json::to_vec(&meta)?)?;
    let _ = fs::remove_file(folder.join("plan.json"));
    crate::local::log(
        "download",
        format!(
            "{key} saved via {}, {:.0} MB",
            plan.source,
            bytes as f64 / 1_048_576.0
        ),
    );
    Ok(json!({ "done": true, "bytes": bytes }))
}

pub fn fail(key: &str, error: &str) -> Result<()> {
    let mut plan = read_plan(key).ok_or_else(|| anyhow!("no download in progress"))?;
    plan.error = Some(error.to_string());
    write_plan(key, &plan)?;
    crate::local::log("download", format!("{key} failed: {error}"));
    Ok(())
}

pub fn remove(key: &str) -> Result<()> {
    let _ = fs::remove_dir_all(folder(key)?);
    Ok(())
}

pub fn list() -> Vec<Value> {
    let Ok(root) = root() else { return Vec::new() };
    let Ok(read) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut rows: Vec<(u64, Value)> = read
        .flatten()
        .filter_map(|item| {
            let key = item.file_name().to_string_lossy().to_string();
            let folder = item.path();
            if let Ok(raw) = fs::read(folder.join("meta.json")) {
                let meta: Value = serde_json::from_slice(&raw).ok()?;
                let at = meta.get("saved_at").and_then(Value::as_u64).unwrap_or(0);
                return Some((
                    at,
                    json!({
                        "key": key,
                        "meta": meta,
                        "state": "done",
                        "source": meta.get("source").cloned().unwrap_or(Value::Null),
                        "done": 0,
                        "total": 0,
                        "bytes": meta.get("bytes").cloned().unwrap_or(json!(0)),
                        "error": Value::Null,
                    }),
                ));
            }
            let plan = read_plan(&key)?;
            let done = plan
                .files
                .iter()
                .filter(|f| folder.join(f).is_file())
                .count();
            Some((
                u64::MAX,
                json!({
                    "key": key,
                    "meta": plan.meta,
                    "state": if plan.error.is_some() { "failed" } else { "fetching" },
                    "source": plan.source,
                    "done": done,
                    "total": plan.files.len(),
                    "bytes": folder_size(&folder),
                    "error": plan.error,
                }),
            ))
        })
        .collect();
    rows.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    rows.into_iter().map(|(_, v)| v).collect()
}

pub fn playback(key: &str) -> Option<Value> {
    let folder = folder(key).ok()?;
    let meta: Value = serde_json::from_slice(&fs::read(folder.join("meta.json")).ok()?).ok()?;
    let main = meta.get("main")?.as_str()?.to_string();
    let port = crate::relay::port();
    let sub = meta
        .get("sub")
        .and_then(Value::as_str)
        .map(|name| json!([{ "lang": meta.get("sub_lang").cloned().unwrap_or(json!("en")), "url": format!("http://127.0.0.1:{port}/d/{key}/{name}") }]))
        .unwrap_or(json!([]));
    Some(json!({
        "streams": [{
            "source": "offline",
            "url": format!("http://127.0.0.1:{port}/d/{key}/{main}"),
            "hls": main.ends_with(".m3u8"),
            "subs": sub,
            "intro": meta.get("intro").cloned().unwrap_or(Value::Null),
            "outro": meta.get("outro").cloned().unwrap_or(Value::Null),
            "thumbs": Value::Null,
        }]
    }))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn folder_size(folder: &std::path::Path) -> u64 {
    fs::read_dir(folder)
        .map(|read| {
            read.flatten()
                .filter_map(|f| f.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}
