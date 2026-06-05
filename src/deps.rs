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
            let ok = has_player(player);
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

pub fn install_mpv(player: &str) -> bool {
    let Some(plan) = plan() else {
        println!("no supported package manager was found.");
        if ui::confirm("open the mpv install page?").unwrap_or(false) {
            open_install_page();
        }
        return false;
    };
    println!("mpv is needed to play video and was not found.");
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
        println!("\ninstall did not complete. see https://mpv.io/installation/");
        return false;
    }
    if has_player(player) {
        println!("\nmpv installed.");
    } else {
        println!("\nmpv installed. restart your terminal so it lands on PATH, then run viel again.");
    }
    true
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

fn plan() -> Option<Plan> {
    #[cfg(target_os = "macos")]
    {
        if which("brew") {
            return Some(build("brew", &["install", "mpv"], false));
        }
    }
    #[cfg(target_os = "windows")]
    {
        if which("winget") {
            return Some(build(
                "winget",
                &[
                    "install",
                    "--id",
                    "shinchiro.mpv",
                    "--exact",
                    "--source",
                    "winget",
                    "--silent",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ],
                false,
            ));
        }
        if which("scoop") {
            return Some(build("scoop", &["install", "mpv"], false));
        }
        if which("choco") {
            return Some(build("choco", &["install", "mpv", "-y"], false));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if which("apt-get") {
            return Some(build("apt-get", &["install", "-y", "mpv"], true));
        }
        if which("dnf") {
            return Some(build("dnf", &["install", "-y", "mpv"], true));
        }
        if which("pacman") {
            return Some(build("pacman", &["-S", "--noconfirm", "mpv"], true));
        }
        if which("zypper") {
            return Some(build("zypper", &["install", "-y", "mpv"], true));
        }
        if which("apk") {
            return Some(build("apk", &["add", "mpv"], true));
        }
    }
    None
}

fn build(program: &str, args: &[&str], sudo: bool) -> Plan {
    let pretty = format!("{}{} {}", if sudo { "sudo " } else { "" }, program, args.join(" "));
    if sudo {
        let mut full = vec![program.to_string()];
        full.extend(args.iter().map(|a| a.to_string()));
        Plan { program: "sudo".into(), args: full, pretty }
    } else {
        Plan { program: program.into(), args: args.iter().map(|a| a.to_string()).collect(), pretty }
    }
}

fn which(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn has_player(player: &str) -> bool {
    Command::new(player)
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
