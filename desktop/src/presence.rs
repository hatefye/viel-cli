use discord_rich_presence::activity::{Activity, ActivityType, Assets, Button, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::Deserialize;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "1518877871567933610";

#[derive(Deserialize, Clone, PartialEq)]
pub struct Playing {
    pub kind: String,
    pub id: u64,
    pub show: String,
    #[serde(default)]
    pub episode_title: String,
    #[serde(default)]
    pub season: u32,
    #[serde(default)]
    pub episode: f64,
    #[serde(default)]
    pub position: f64,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub poster: String,
    #[serde(default)]
    pub host: String,
}

fn sender() -> &'static Mutex<Option<Sender<Option<Playing>>>> {
    static TX: OnceLock<Mutex<Option<Sender<Option<Playing>>>>> = OnceLock::new();
    TX.get_or_init(|| Mutex::new(None))
}

pub fn update(playing: Option<Playing>) {
    let mut slot = sender().lock().unwrap();
    if slot.is_none() {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || worker(rx));
        *slot = Some(tx);
    }
    if let Some(tx) = slot.as_ref() {
        let _ = tx.send(playing);
    }
}

fn worker(rx: mpsc::Receiver<Option<Playing>>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut want: Option<Playing> = None;
    let mut shown: Option<Playing> = None;
    let mut retry = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(next) => want = next,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if want == shown && want.is_some() {
            continue;
        }
        match &want {
            Some(playing) => {
                if client.is_none() {
                    if retry > Instant::now() {
                        continue;
                    }
                    client = connect();
                    if client.is_none() {
                        retry = Instant::now() + Duration::from_secs(30);
                        continue;
                    }
                }
                let ok = client.as_mut().map(|c| push(c, playing)).unwrap_or(false);
                if ok {
                    shown = want.clone();
                } else {
                    client = None;
                    shown = None;
                    retry = Instant::now() + Duration::from_secs(15);
                }
            }
            None => {
                if shown.is_some() {
                    if let Some(c) = client.as_mut() {
                        if c.clear_activity().is_err() {
                            client = None;
                        }
                    }
                    shown = None;
                }
            }
        }
    }
}

fn connect() -> Option<DiscordIpcClient> {
    let mut client = DiscordIpcClient::new(CLIENT_ID).ok()?;
    client.connect().ok()?;
    Some(client)
}

fn push(client: &mut DiscordIpcClient, p: &Playing) -> bool {
    let tag = match p.kind.as_str() {
        "tv" => format!("Season {} \u{b7} Episode {}", p.season, p.episode),
        "anime" => format!("Episode {}", p.episode),
        _ => "Movie".to_string(),
    };
    let state = if p.paused {
        format!("\u{23f8} Paused \u{b7} {tag}")
    } else if p.kind != "movie" && !p.episode_title.is_empty() && p.episode_title != p.show {
        format!("{tag} \u{b7} {}", p.episode_title)
    } else {
        tag
    };
    let host = if p.host.is_empty() { "https://viel.tv" } else { p.host.as_str() };
    let link = match p.kind.as_str() {
        "tv" => format!("{host}/tv/{}/{}/{}", p.id, p.season, p.episode),
        "anime" => format!("{host}/anime/{}/{}", p.id, p.episode),
        _ => format!("{host}/movie/{}", p.id),
    };
    let show = clamp(&p.show);
    let state = clamp(&state);
    let mut act = Activity::new()
        .activity_type(ActivityType::Watching)
        .details(&show)
        .state(&state);
    if !p.paused {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        act = act.timestamps(Timestamps::new().start((now - p.position as i64).max(0)));
    }
    if p.poster.starts_with("https://") {
        act = act.assets(Assets::new().large_image(&p.poster).large_text(&show));
    }
    act = act.buttons(vec![
        Button::new("Watch on viel", &link),
        Button::new("Get viel", "https://viel.tv/desktop"),
    ]);
    client.set_activity(act).is_ok()
}

fn clamp(text: &str) -> String {
    let mut end = text.len().min(120);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}
