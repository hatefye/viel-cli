use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use tauri::{AppHandle, Emitter};
use viel::local;

use crate::downloads;

fn safe_name(raw: &str) -> String {
    let name: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() || " .-_()".contains(c) { c } else { ' ' })
        .collect();
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() { "clip".into() } else { name.chars().take(90).collect() }
}

fn copy_to(mut from: impl Read, path: &Path, mut each: impl FnMut(u64)) -> Result<u64> {
    let partial = path.with_extension("part");
    let mut file = fs::File::create(&partial)?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut total = 0u64;
    loop {
        let n = from.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        total += n as u64;
        each(total);
    }
    file.flush()?;
    drop(file);
    fs::rename(&partial, path)?;
    Ok(total)
}

fn save_clip(path: &str, name: &str) -> Result<PathBuf> {
    if !path.starts_with("/api/download/") {
        bail!("not a clip request");
    }
    let dir = downloads::root_dir().ok_or_else(|| anyhow!("no downloads folder"))?.join("clips");
    fs::create_dir_all(&dir)?;
    let resp = local::client().open(path)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body: Value = resp.json().unwrap_or_default();
        match body.get("error").and_then(Value::as_str) {
            Some(message) => bail!("{message}"),
            None => bail!("the server answered {status}"),
        }
    }
    let base = safe_name(name);
    let mut target = dir.join(format!("{base}.mp4"));
    let mut n = 2;
    while target.exists() {
        target = dir.join(format!("{base} ({n}).mp4"));
        n += 1;
    }
    let bytes = copy_to(resp, &target, |_| {})?;
    if bytes < 2048 {
        let _ = fs::remove_file(&target);
        bail!("the clip came back empty");
    }
    Ok(target)
}

pub fn clip(handle: AppHandle, path: String, name: String) {
    thread::spawn(move || {
        let payload = match save_clip(&path, &name) {
            Ok(file) => json!({ "ok": true, "file": file.to_string_lossy(), "name": file.file_name().map(|f| f.to_string_lossy().to_string()) }),
            Err(err) => json!({ "ok": false, "error": err.to_string() }),
        };
        let _ = handle.emit("clip", payload);
    });
}

fn version_parts(v: &str) -> Vec<u32> {
    v.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect()
}

pub fn newer(a: &str, b: &str) -> bool {
    let (x, y) = (version_parts(a), version_parts(b));
    for i in 0..3 {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p > q;
        }
    }
    false
}

fn update_dir(data: &Path) -> PathBuf {
    data.join("update")
}

fn installer(data: &Path, version: &str) -> PathBuf {
    let clean: String = version.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    update_dir(data).join(format!("viel-setup-{clean}.exe"))
}

fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn fetch_update(data: &Path, host: &str, url: &str, version: &str, sha256: &str, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf> {
    let trusted = url.starts_with("https://viel.tv/") || (host.starts_with("https://") && url.starts_with(&format!("{host}/")));
    if !trusted {
        bail!("update link is not on the viel server");
    }
    let want = sha256.trim().to_lowercase();
    if want.len() != 64 {
        bail!("the update has no checksum");
    }
    let target = installer(data, version);
    if target.exists() && digest(&target)? == want {
        return Ok(target);
    }
    let dir = update_dir(data);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let resp = viel::net::send_with(url, None, None, 2)?;
    if !resp.status().is_success() {
        bail!("download failed ({})", resp.status());
    }
    let total = resp.content_length().unwrap_or(0);
    copy_to(resp, &target, |done| progress(done, total))?;
    if digest(&target)? != want {
        let _ = fs::remove_file(&target);
        bail!("the update file did not match its checksum");
    }
    Ok(target)
}

pub fn update(handle: AppHandle, data: PathBuf, host: String, url: String, version: String, sha256: String) {
    thread::spawn(move || {
        let mut last = 0u64;
        let emitter = handle.clone();
        let result = fetch_update(&data, &host, &url, &version, &sha256, |done, total| {
            if done - last > 4 * 1024 * 1024 {
                last = done;
                let _ = emitter.emit("update", json!({ "state": "downloading", "done": done, "total": total }));
            }
        });
        let payload = match result {
            Ok(_) => json!({ "state": "ready", "version": version }),
            Err(err) => json!({ "state": "failed", "error": err.to_string() }),
        };
        let _ = handle.emit("update", payload);
    });
}

pub fn pending(data: &Path, current: &str) -> Option<(String, PathBuf)> {
    let mut best: Option<(String, PathBuf)> = None;
    for entry in fs::read_dir(update_dir(data)).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(version) = name.strip_prefix("viel-setup-").and_then(|n| n.strip_suffix(".exe")) else { continue };
        if !newer(version, current) {
            let _ = fs::remove_file(entry.path());
            continue;
        }
        if best.as_ref().map(|(v, _)| newer(version, v)).unwrap_or(true) {
            best = Some((version.to_string(), entry.path()));
        }
    }
    best
}

pub fn install(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let script = format!("Start-Process -FilePath '{}' -ArgumentList '/S' -Verb RunAs", path.to_string_lossy().replace('\'', "''"));
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(0x0800_0000)
            .spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        bail!("updates install on Windows only")
    }
}
