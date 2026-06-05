use crate::api::Api;
use std::io::{stdout, Write};
use std::process::{Command, Stdio};

pub struct Report {
    pub player: bool,
    pub host: bool,
}

pub fn check(api: &Api, player: &str) -> Report {
    bar(0, 2);
    let player_ok = has_player(player);
    bar(1, 2);
    let host = api.reachable();
    bar(2, 2);
    println!();
    println!(
        "  {} {}      {} server",
        mark(player_ok),
        player,
        if host { "reachable" } else { "unreachable" },
    );
    println!();
    Report { player: player_ok, host }
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

fn has_player(player: &str) -> bool {
    Command::new(player)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn mark(ok: bool) -> &'static str {
    if ok {
        "\x1b[32m✓\x1b[0m"
    } else {
        "\x1b[31m✗\x1b[0m"
    }
}

fn bar(done: usize, total: usize) {
    let width = 24;
    let filled = width * done / total;
    print!(
        "\r checking dependencies [{}{}]",
        "█".repeat(filled),
        "░".repeat(width - filled),
    );
    let _ = stdout().flush();
}
