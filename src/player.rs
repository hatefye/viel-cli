use crate::api::Stream;
use crate::relay;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub struct Watched {
    pub position: f64,
    pub duration: f64,
}

impl Watched {
    pub fn finished(&self) -> bool {
        self.duration > 0.0
            && (self.duration - self.position < 180.0 || self.position / self.duration > 0.92)
    }
}

pub fn play(player: &str, stream: &Stream, mut progress: impl FnMut(f64, f64)) -> Result<Watched> {
    let socket = socket_path();
    let _ = std::fs::remove_file(&socket);

    let mut cmd = Command::new(player);
    let referer = stream.referer.as_deref();
    cmd.arg(if stream.hls {
        relay::playlist(&stream.url, referer)
    } else {
        relay::video(&stream.url, referer)
    });
    cmd.arg(format!("--force-media-title={}", stream.title));
    cmd.arg("--force-window=immediate");
    cmd.arg("--hls-bitrate=max");
    cmd.arg(format!("--input-ipc-server={}", socket.display()));
    for sub in &stream.subs {
        cmd.arg(format!(
            "--sub-file={}",
            relay::subtitle(&sub.url, sub.referer.as_deref())
        ));
    }
    if !stream.subs.is_empty() {
        cmd.arg("--sub-visibility=yes");
    }
    if let Some(start) = stream.start.filter(|s| *s > 0.0) {
        cmd.arg(format!("--start={}", start as u64));
    }
    let mut child = cmd.spawn().with_context(|| {
        format!("could not launch {player}; install it and ensure it is on PATH")
    })?;

    let mut last = Watched {
        position: 0.0,
        duration: 0.0,
    };
    let mut elapsed = 0u64;
    let mut skipped = stream.intro.is_none();
    let launched = Instant::now();
    let mut started = false;
    relay::reset_activity();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if !started {
            if ask(&socket, "time-pos").is_some_and(|p| p > 0.0) {
                started = true;
                print!(
                    "\r\x1b[2K  ▶ started after {:.0}s\n",
                    launched.elapsed().as_secs_f32()
                );
            } else {
                let a = relay::activity();
                let what = a
                    .fetching
                    .map(|(w, t)| format!("{w} · {t:.0}s"))
                    .unwrap_or_else(|| "waiting for mpv".into());
                print!(
                    "\r\x1b[2K  loading {:.0}s · {} parts · {what}",
                    launched.elapsed().as_secs_f32(),
                    a.served
                );
            }
            let _ = std::io::stdout().flush();
        }
        sleep(Duration::from_millis(500));
        elapsed += 500;
        if !skipped {
            if let (Some((from, to)), Some(pos)) = (stream.intro, ask(&socket, "time-pos")) {
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
                last = Watched {
                    position: pos,
                    duration: dur,
                };
                progress(pos, dur);
            }
        }
    };

    let _ = std::fs::remove_file(&socket);
    if last.duration > 0.0 {
        progress(last.position, last.duration);
    } else if !status.success() {
        anyhow::bail!("{player} exited before playback started");
    }
    Ok(last)
}

#[cfg(unix)]
fn socket_path() -> PathBuf {
    std::env::temp_dir().join(format!("viel-mpv-{}.sock", std::process::id()))
}

#[cfg(windows)]
fn socket_path() -> PathBuf {
    PathBuf::from(format!("\\\\.\\pipe\\viel-mpv-{}", std::process::id()))
}

#[cfg(unix)]
fn connect(socket: &Path) -> Option<Box<dyn ReadWrite>> {
    let conn = std::os::unix::net::UnixStream::connect(socket).ok()?;
    conn.set_read_timeout(Some(Duration::from_millis(1500)))
        .ok()?;
    Some(Box::new(conn))
}

#[cfg(windows)]
fn connect(socket: &Path) -> Option<Box<dyn ReadWrite>> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(socket)
        .ok()?;
    Some(Box::new(file))
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn ask(socket: &Path, property: &str) -> Option<f64> {
    let mut conn = connect(socket)?;
    writeln!(
        conn,
        "{{\"command\":[\"get_property\",\"{property}\"],\"request_id\":7}}"
    )
    .ok()?;
    let reader = BufReader::new(conn);
    for line in reader.lines().map_while(Result::ok).take(50) {
        if let Ok(reply) = serde_json::from_str::<Reply>(&line) {
            if reply.request_id == Some(7) {
                return reply.data;
            }
        }
    }
    None
}

fn command(socket: &Path, args: &str) {
    if let Some(mut conn) = connect(socket) {
        let _ = writeln!(conn, "{{\"command\":{args}}}");
    }
}

#[derive(serde::Deserialize)]
struct Reply {
    data: Option<f64>,
    request_id: Option<u64>,
}
