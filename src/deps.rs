use crate::api::Api;
use crate::ui;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::execute;
use crossterm::terminal::{size, Clear, ClearType};
use std::io::{stdout, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const PENDING: u8 = 0;
const OK: u8 = 1;
const FAIL: u8 = 2;

pub struct Report {
    pub player: bool,
    pub host: bool,
}

pub fn check(api: &Api, player: &str) -> Report {
    let (cols, rows) = size().unwrap_or((80, 24));
    let mid = rows / 2;
    let _ = execute!(stdout(), Clear(ClearType::All), Hide);
    put(cols, mid.saturating_sub(2), 4, "\x1b[1m\x1b[38;2;220;38;38mviel\x1b[0m");
    put(cols, mid.saturating_sub(1), 21, "\x1b[2mchecking dependencies\x1b[0m");

    let player_state = AtomicU8::new(PENDING);
    let host_state = AtomicU8::new(PENDING);

    thread::scope(|scope| {
        scope.spawn(|| {
            let ok = has(player);
            player_state.store(if ok { OK } else { FAIL }, Ordering::SeqCst);
        });
        scope.spawn(|| {
            let ok = api.reachable();
            host_state.store(if ok { OK } else { FAIL }, Ordering::SeqCst);
        });

        let mut frame = 0;
        loop {
            let p = player_state.load(Ordering::SeqCst);
            let h = host_state.load(Ordering::SeqCst);
            draw(cols, mid + 1, player, p, frame);
            draw(cols, mid + 2, "server", h, frame);
            if p != PENDING && h != PENDING {
                break;
            }
            thread::sleep(Duration::from_millis(80));
            frame += 1;
        }
    });

    thread::sleep(Duration::from_millis(600));
    let _ = execute!(stdout(), Show, MoveTo(0, rows.saturating_sub(1)));
    println!();

    Report {
        player: player_state.load(Ordering::SeqCst) == OK,
        host: host_state.load(Ordering::SeqCst) == OK,
    }
}

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
    if has(tool) {
        println!("\n{tool} installed.");
    } else {
        println!("\n{tool} installed. restart your terminal so it lands on PATH, then run viel again.");
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
        format!("{}{} {}", if sudo { "sudo " } else { "" }, program, args.join(" "))
    });
    if sudo {
        let mut full = vec![program.to_string()];
        full.extend(args.iter().map(|a| a.to_string()));
        Plan { program: "sudo".into(), args: full, pretty }
    } else {
        Plan { program: program.into(), args: args.iter().map(|a| a.to_string()).collect(), pretty }
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
            roots.push((PathBuf::from(&local).join("Microsoft\\WinGet\\Packages"), true));
            roots.push((PathBuf::from(&local).join("Microsoft\\WinGet\\Links"), false));
            roots.push((PathBuf::from(&local).join("Programs"), true));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            roots.push((PathBuf::from(&home).join("scoop\\apps\\mpv\\current"), false));
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
            let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
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
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn draw(cols: u16, row: u16, label: &str, state: u8, frame: usize) {
    let width = 2 + label.chars().count();
    let colored = match state {
        OK => format!("\x1b[32m✓\x1b[0m {label}"),
        FAIL => format!("\x1b[31m✗\x1b[0m {label}"),
        _ => format!("{} {label}", FRAMES[frame % FRAMES.len()]),
    };
    put(cols, row, width, &colored);
}

fn put(cols: u16, row: u16, width: usize, text: &str) {
    let col = cols.saturating_sub(width as u16) / 2;
    let _ = execute!(stdout(), MoveTo(col, row));
    print!("{text}");
    let _ = stdout().flush();
}
