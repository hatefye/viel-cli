use crate::ui;

use std::process::{Command, Stdio};

pub fn install_tool(tool: &str) -> bool {
    let Some(plan) = plan(tool) else {
        println!("no supported package manager was found. install {tool} manually.");
        if tool == "mpv" && ui::confirm("open the mpv install page?").unwrap_or(false) {
            open_install_page();
        }
        return false;
    };
    println!("{tool} is needed and was not found.");
    if !ui::confirm(&format!("install it now with `{}`?", plan.pretty)).unwrap_or(false) {
        return false;
    }
    println!();
    let ran = Command::new(&plan.program)
        .args(&plan.args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ran {
        println!("\ninstall did not complete.");
        return false;
    }
    let found = if tool == "ffmpeg" {
        resolve_ffmpeg()
    } else {
        resolve_player(tool)
    };
    match found {
        Some(_) => println!("\n{tool} installed."),
        None => println!("\n{tool} installed, but it is not on PATH yet. restart your terminal and run viel again."),
    }
    true
}

pub fn prompt_player_path() -> Option<String> {
    println!("couldn't find mpv automatically.");
    loop {
        let entered = match ui::text("paste the full path to mpv (blank to skip)") {
            Ok(Some(s)) => s,
            _ => return None,
        };
        let raw = entered.trim().trim_matches('"');
        if raw.is_empty() {
            return None;
        }
        let candidate = normalize_player_path(raw);
        if has(&candidate) {
            return Some(candidate);
        }
        println!("that didn't run as mpv — check the path and try again.");
    }
}

fn normalize_player_path(input: &str) -> String {
    let path = std::path::Path::new(input);
    if path.is_dir() {
        let exe = if cfg!(windows) { "mpv.exe" } else { "mpv" };
        return path.join(exe).display().to_string();
    }
    input.to_string()
}

pub fn open_install_page() {
    let url = "https://mpv.io/installation/";
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).spawn();
}

struct Plan {
    program: String,
    args: Vec<String>,
    pretty: String,
}

fn plan(tool: &str) -> Option<Plan> {
    #[cfg(target_os = "macos")]
    {
        if has("brew") {
            return Some(build("brew", &["install", tool], false, None));
        }
    }
    #[cfg(target_os = "windows")]
    {
        let cmd = format!(
            "if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {{ Invoke-RestMethod get.scoop.sh | Invoke-Expression }}; scoop install git; scoop bucket add extras; scoop install {tool}"
        );
        return Some(Plan {
            program: "powershell".into(),
            args: vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-Command".into(),
                cmd,
            ],
            pretty: format!("scoop install {tool} (sets up scoop if needed)"),
        });
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if has("apt-get") {
            return Some(build("apt-get", &["install", "-y", tool], true, None));
        }
        if has("dnf") {
            return Some(build("dnf", &["install", "-y", tool], true, None));
        }
        if has("pacman") {
            return Some(build("pacman", &["-S", "--noconfirm", tool], true, None));
        }
        if has("zypper") {
            return Some(build("zypper", &["install", "-y", tool], true, None));
        }
        if has("apk") {
            return Some(build("apk", &["add", tool], true, None));
        }
    }
    #[allow(unreachable_code)]
    {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn build(program: &str, args: &[&str], sudo: bool, pretty: Option<&str>) -> Plan {
    let pretty = pretty.map(str::to_string).unwrap_or_else(|| {
        format!(
            "{}{} {}",
            if sudo { "sudo " } else { "" },
            program,
            args.join(" ")
        )
    });
    if sudo {
        let mut full = vec![program.to_string()];
        full.extend(args.iter().map(|a| a.to_string()));
        Plan {
            program: "sudo".into(),
            args: full,
            pretty,
        }
    } else {
        Plan {
            program: program.into(),
            args: args.iter().map(|a| a.to_string()).collect(),
            pretty,
        }
    }
}

pub fn resolve_ffmpeg() -> Option<String> {
    if has("ffmpeg") {
        return Some("ffmpeg".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        for candidate in ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"] {
            if has(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(home) = std::env::var("USERPROFILE") {
            for c in [
                format!("{home}\\scoop\\shims\\ffmpeg.exe"),
                format!("{home}\\scoop\\apps\\ffmpeg\\current\\bin\\ffmpeg.exe"),
            ] {
                if has(&c) {
                    return Some(c);
                }
            }
        }
    }
    None
}

pub fn resolve_player(player: &str) -> Option<String> {
    if has(player) {
        return Some(player.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        for candidate in ["/opt/homebrew/bin/mpv", "/usr/local/bin/mpv"] {
            if has(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;
        let mut roots: Vec<(PathBuf, bool)> = Vec::new();
        if let Ok(pf) = std::env::var("ProgramFiles") {
            roots.push((PathBuf::from(pf), true));
        }
        if let Ok(pfx) = std::env::var("ProgramFiles(x86)") {
            roots.push((PathBuf::from(pfx), true));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            roots.push((
                PathBuf::from(&local).join("Microsoft\\WinGet\\Packages"),
                true,
            ));
            roots.push((
                PathBuf::from(&local).join("Microsoft\\WinGet\\Links"),
                false,
            ));
            roots.push((PathBuf::from(&local).join("Programs"), true));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            roots.push((
                PathBuf::from(&home).join("scoop\\apps\\mpv\\current"),
                false,
            ));
            roots.push((PathBuf::from(&home).join("scoop\\shims"), false));
        }
        roots.push((PathBuf::from("C:\\ProgramData\\chocolatey\\bin"), false));
        for (root, filter_mpv) in roots {
            if let Some(found) = search_root(&root, filter_mpv) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn search_root(root: &std::path::Path, filter_mpv: bool) -> Option<String> {
    let direct = root.join("mpv.exe");
    if direct.is_file() {
        let path = direct.display().to_string();
        if has(&path) {
            return Some(path);
        }
    }
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if filter_mpv {
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !name.contains("mpv") {
                continue;
            }
        }
        if let Some(found) = find_exe(&dir, 2) {
            if has(&found) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn find_exe(dir: &std::path::Path, depth: u8) -> Option<String> {
    let direct = dir.join("mpv.exe");
    if direct.is_file() {
        return Some(direct.display().to_string());
    }
    if depth == 0 {
        return None;
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_exe(&path, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

fn has(bin: &str) -> bool {
    let flag = if bin.ends_with("ffmpeg") || bin.ends_with("ffmpeg.exe") {
        "-version"
    } else {
        "--version"
    };
    Command::new(bin)
        .arg(flag)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
