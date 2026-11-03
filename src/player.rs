use crate::api::Stream;
use crate::relay;
use anyhow::{Context, Result};
use std::process::Command;

pub fn play(player: &str, stream: &Stream, progress: impl FnMut(f64, f64)) -> Result<()> {
    #[cfg(unix)]
    let result = tracked(player, stream, progress);
    #[cfg(not(unix))]
    let result = {
        let _ = progress;
        blocking(player, stream)
    };
    result
}

fn base(player: &str, stream: &Stream) -> Command {
    let mut cmd = Command::new(player);
    let referer = stream.referer.as_deref();
    cmd.arg(if stream.hls { relay::playlist(&stream.url, referer) } else { relay::video(&stream.url, referer) });
    cmd.arg(format!("--force-media-title={}", stream.title));
    cmd.arg("--force-window=yes");
    cmd.arg("--hls-bitrate=max");
    for sub in &stream.subs {
        cmd.arg(format!("--sub-file={}", relay::subtitle(&sub.url, sub.referer.as_deref())));
    }
    if !stream.subs.is_empty() {
        cmd.arg("--sub-visibility=yes");
    }
    if let Some(start) = stream.start.filter(|s| *s > 0.0) {
        cmd.arg(format!("--start={}", start as u64));
    }
    cmd
}

#[cfg(not(unix))]
fn blocking(player: &str, stream: &Stream) -> Result<()> {
    use std::process::Stdio;
    let output = base(player, stream)
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not launch {player}; install it and ensure it is on PATH"))?
        .wait_with_output()
        .with_context(|| format!("{player} failed to run"))?;
    if !output.status.success() {
        let detail = tail_lines(&String::from_utf8_lossy(&output.stderr), 20);
        if detail.is_empty() {
            anyhow::bail!("{player} exited with an error (no output)");
        }
        anyhow::bail!("{player} error:\n{detail}");
    }
    Ok(())
}

#[cfg(not(unix))]
fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].iter().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n")
}

#[cfg(unix)]
fn tracked(player: &str, stream: &Stream, mut progress: impl FnMut(f64, f64)) -> Result<()> {
    use std::thread::sleep;
    use std::time::Duration;

    let socket = std::env::temp_dir().join(format!("viel-mpv-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket);

    let mut cmd = base(player, stream);
    cmd.arg(format!("--input-ipc-server={}", socket.display()));
    let mut child = cmd
        .spawn()
        .with_context(|| format!("could not launch {player}; install it and ensure it is on PATH"))?;

    let (mut last_pos, mut last_dur) = (0.0, 0.0);
    let mut elapsed = 0u64;
    let mut skipped = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        sleep(Duration::from_millis(500));
        elapsed += 500;
        if let (Some((from, to)), false) = (stream.intro, skipped) {
            if let Some(pos) = ask(&socket, "time-pos") {
                if pos >= from && pos < to - 1.0 {
                    command(&socket, &format!("[\"set_property\",\"time-pos\",{to}]"));
                    command(&socket, "[\"show-text\",\"skipped intro\",1500]");
                    skipped = true;
                } else if pos >= to {
                    skipped = true;
                }
            }
        }
        if elapsed >= 10_000 {
            elapsed = 0;
            if let (Some(pos), Some(dur)) = (ask(&socket, "time-pos"), ask(&socket, "duration")) {
                last_pos = pos;
                last_dur = dur;
                progress(pos, dur);
            }
        }
    };

    let _ = std::fs::remove_file(&socket);
    if last_dur > 0.0 {
        progress(last_pos, last_dur);
    } else if !status.success() {
        anyhow::bail!("{player} exited before playback started");
    }
    Ok(())
}

#[cfg(unix)]
fn ask(socket: &std::path::Path, property: &str) -> Option<f64> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let mut conn = UnixStream::connect(socket).ok()?;
    conn.set_read_timeout(Some(Duration::from_millis(1500))).ok()?;
    writeln!(conn, "{{\"command\":[\"get_property\",\"{property}\"]}}").ok()?;
    let reader = BufReader::new(conn);
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(reply) = serde_json::from_str::<Reply>(&line) {
            if let Some(value) = reply.data {
                return Some(value);
            }
        }
    }
    None
}

#[cfg(unix)]
fn command(socket: &std::path::Path, args: &str) {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    if let Ok(mut conn) = UnixStream::connect(socket) {
        let _ = writeln!(conn, "{{\"command\":{args}}}");
    }
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct Reply {
    data: Option<f64>,
}
