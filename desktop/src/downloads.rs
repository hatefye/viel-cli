use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use tauri::{AppHandle, Emitter};
use viel::{local, net};

#[derive(Deserialize)]
struct Plan {
    key: String,
    source: String,
    referer: Option<String>,
    total: usize,
    parts: Vec<Part>,
}

#[derive(Deserialize, Clone)]
struct Part {
    url: String,
    file: String,
    ts: bool,
}

#[derive(Deserialize)]
struct Finish {
    done: bool,
    files: Option<Vec<String>>,
}

struct Job {
    cancel: Arc<AtomicBool>,
    done: Arc<AtomicUsize>,
    total: usize,
    state: Mutex<String>,
}

fn jobs() -> &'static Mutex<HashMap<String, Arc<Job>>> {
    static JOBS: OnceLock<Mutex<HashMap<String, Arc<Job>>>> = OnceLock::new();
    JOBS.get_or_init(Default::default)
}

fn root() -> &'static Mutex<Option<PathBuf>> {
    static ROOT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    ROOT.get_or_init(Default::default)
}

pub fn set_root(path: &Path) {
    *root().lock().unwrap() = Some(path.to_path_buf());
    viel::offline::set_dir(&path.to_string_lossy());
}

pub fn root_dir() -> Option<PathBuf> {
    root().lock().unwrap().clone()
}

pub fn active() -> Vec<Value> {
    jobs()
        .lock()
        .unwrap()
        .iter()
        .map(|(key, job)| {
            json!({
                "key": key,
                "done": job.done.load(Ordering::Relaxed),
                "total": job.total,
                "state": job.state.lock().unwrap().clone(),
            })
        })
        .collect()
}

pub fn cancel(key: &str) {
    if let Some(job) = jobs().lock().unwrap().get(key) {
        job.cancel.store(true, Ordering::Relaxed);
    }
}

fn call(method: &str, path: &str, body: Value) -> Result<Value> {
    let (status, _, bytes) = local::handle(method, path, &HashMap::new(), &serde_json::to_vec(&body)?);
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if !status.starts_with("200") {
        bail!(
            "{}",
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("download failed")
        );
    }
    Ok(value)
}

fn publish(app: &AppHandle, key: &str, job: &Job, state: &str) {
    *job.state.lock().unwrap() = state.to_string();
    let _ = app.emit(
        "download",
        json!({
            "key": key,
            "done": job.done.load(Ordering::Relaxed),
            "total": job.total,
            "state": state,
        }),
    );
}

fn fetch_parts(app: &AppHandle, key: &str, job: &Arc<Job>, folder: &Path, referer: Option<&str>, parts: Vec<Part>) {
    let queue = Arc::new(Mutex::new(parts));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let queue = queue.clone();
            let job = job.clone();
            let folder = folder.to_path_buf();
            let referer = referer.map(str::to_string);
            let app = app.clone();
            let key = key.to_string();
            thread::spawn(move || loop {
                if job.cancel.load(Ordering::Relaxed) {
                    return;
                }
                let Some(part) = queue.lock().unwrap().pop() else {
                    return;
                };
                let target = folder.join(&part.file);
                if target.is_file() {
                    job.done.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                let ok = net::send(&part.url, referer.as_deref(), None)
                    .and_then(|r| r.bytes().map_err(Into::into))
                    .ok()
                    .and_then(|bytes| {
                        let data = if part.ts { net::unmask_ts(&bytes).to_vec() } else { bytes.to_vec() };
                        let tmp = folder.join(format!("{}.part", part.file));
                        fs::write(&tmp, data).ok()?;
                        fs::rename(&tmp, &target).ok()
                    })
                    .is_some();
                if ok {
                    let n = job.done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n % 3 == 0 || n == job.total {
                        publish(&app, &key, &job, "downloading");
                    }
                }
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
}

pub fn start(app: AppHandle, body: Value) -> Result<()> {
    let key = body
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing key"))?
        .to_string();
    if jobs().lock().unwrap().contains_key(&key) {
        bail!("already downloading");
    }
    let dir = root_dir().ok_or_else(|| anyhow!("downloads folder not set"))?;
    let job = Arc::new(Job {
        cancel: Arc::new(AtomicBool::new(false)),
        done: Arc::new(AtomicUsize::new(0)),
        total: 0,
        state: Mutex::new("resolving".into()),
    });
    jobs().lock().unwrap().insert(key.clone(), job.clone());
    publish(&app, &key, &job, "resolving");
    thread::spawn(move || {
        let outcome = run(&app, &key, job.clone(), &dir, body);
        jobs().lock().unwrap().remove(&key);
        match outcome {
            Ok(true) => publish(&app, &key, &job, "done"),
            Ok(false) => publish(&app, &key, &job, "cancelled"),
            Err(e) => {
                local::log("download", format!("{key}: {e}"));
                let _ = call("POST", "downloads/fail", json!({ "key": key, "error": e.to_string() }));
                publish(&app, &key, &job, &format!("failed: {e}"));
            }
        }
    });
    Ok(())
}

fn run(app: &AppHandle, key: &str, job: Arc<Job>, dir: &Path, body: Value) -> Result<bool> {
    let plan: Plan = serde_json::from_value(call("POST", "downloads/plan", body)?)?;
    let folder = dir.join(&plan.key);
    fs::create_dir_all(&folder)?;
    let job = Arc::new(Job {
        cancel: job.cancel.clone(),
        done: job.done.clone(),
        total: plan.total,
        state: Mutex::new("downloading".into()),
    });
    jobs().lock().unwrap().insert(key.to_string(), job.clone());
    local::log("download", format!("{key}: {} parts via {}", plan.total, plan.source));
    publish(app, key, &job, "downloading");
    let mut parts = plan.parts.clone();
    for attempt in 0..4 {
        fetch_parts(app, key, &job, &folder, plan.referer.as_deref(), parts.clone());
        if job.cancel.load(Ordering::Relaxed) {
            let _ = call("POST", "downloads/remove", json!({ "key": key }));
            return Ok(false);
        }
        let reply: Finish = serde_json::from_value(call("POST", "downloads/finish", json!({ "key": key }))?)?;
        if reply.done {
            return Ok(true);
        }
        let missing = reply.files.unwrap_or_default();
        if attempt == 3 {
            bail!("{} parts could not be fetched", missing.len());
        }
        parts = plan.parts.iter().filter(|p| missing.contains(&p.file)).cloned().collect();
        job.done.store(plan.total - parts.len(), Ordering::Relaxed);
        thread::sleep(std::time::Duration::from_secs(2));
    }
    bail!("gave up")
}
