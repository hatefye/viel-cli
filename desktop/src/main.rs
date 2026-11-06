#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod downloads;
mod mpv;
mod native;
mod presence;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, ShortcutState};
use viel::{local, relay};

#[derive(Serialize, Deserialize, Default, Clone)]
struct Store {
    token: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    tray: Option<bool>,
}

struct App {
    dir: PathBuf,
    store: Mutex<Store>,
    backdrop: Mutex<&'static str>,
    player: mpv::Slot,
}

impl App {
    fn host(&self) -> String {
        self.store
            .lock()
            .unwrap()
            .host
            .clone()
            .filter(|h| !h.trim().is_empty())
            .unwrap_or_else(|| "https://viel.tv".into())
    }

    fn save(&self) {
        let store = self.store.lock().unwrap().clone();
        if let Ok(bytes) = serde_json::to_vec_pretty(&store) {
            let _ = fs::write(self.dir.join("state.json"), bytes);
        }
    }
}

fn load_store(dir: &PathBuf) -> Store {
    fs::read(dir.join("state.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[tauri::command]
fn boot(app: State<App>) -> Value {
    let store = app.store.lock().unwrap().clone();
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "backdrop": *app.backdrop.lock().unwrap(),
        "token": store.token.is_some(),
        "username": store.username,
        "host": app.host(),
        "downloads": downloads::root_dir(),
        "mpv": mpv::available().map(|_| Value::Null).unwrap_or_else(|e| json!(e.to_string())),
        "port": relay::port(),
    })
}

#[tauri::command]
async fn api(
    app: State<'_, App>,
    method: String,
    path: String,
    query: Option<HashMap<String, String>>,
    body: Option<Value>,
) -> Result<Value, String> {
    let query = query.unwrap_or_default();
    let bytes = body.map(|b| serde_json::to_vec(&b).unwrap_or_default()).unwrap_or_default();
    let (m, p) = (method.clone(), path.clone());
    let (status, _, out) = tauri::async_runtime::spawn_blocking(move || local::handle(&m, &p, &query, &bytes))
        .await
        .map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_slice(&out).unwrap_or(Value::Null);
    if !status.starts_with("200") {
        return Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("request failed")
            .to_string());
    }
    if method == "POST" {
        match path.as_str() {
            "login" | "register" => {
                let mut store = app.store.lock().unwrap();
                store.token = value.get("token").and_then(Value::as_str).map(str::to_string);
                store.username = value.get("username").and_then(Value::as_str).map(str::to_string);
                drop(store);
                app.save();
            }
            "logout" => {
                let mut store = app.store.lock().unwrap();
                store.token = None;
                store.username = None;
                drop(store);
                app.save();
            }
            _ => {}
        }
    }
    Ok(value)
}

#[tauri::command]
fn set_tray(app: State<App>, on: bool) {
    app.store.lock().unwrap().tray = Some(on);
    app.save();
}

#[tauri::command]
fn presence(playing: Option<presence::Playing>) {
    presence::update(playing);
}

fn reveal(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn set_host(app: State<App>, host: String) {
    let host = host.trim().trim_end_matches('/').to_string();
    let token = {
        let mut store = app.store.lock().unwrap();
        store.host = (!host.is_empty()).then_some(host.clone());
        store.token.clone()
    };
    app.save();
    local::configure(&app.host(), token);
}

#[tauri::command]
fn player_init(app: State<App>, window: WebviewWindow, handle: AppHandle) -> Result<(), String> {
    let mut slot = app.player.0.lock().unwrap();
    if slot.is_some() {
        return Ok(());
    }
    let wid = native::window_id(&window);
    let emitter = handle.clone();
    let player = mpv::Player::new(wid, &[], move |payload| {
        let _ = emitter.emit("mpv", payload);
    })
    .map_err(|e| e.to_string())?;
    *slot = Some(player);
    native::settle_child(&window, false);
    Ok(())
}

#[tauri::command]
fn player_cmd(app: State<App>, args: Vec<String>) -> Result<(), String> {
    let slot = app.player.0.lock().unwrap();
    let player = slot.as_ref().ok_or("player not ready")?;
    player.command(&args).map_err(|e| e.to_string())
}

#[tauri::command]
fn player_set(app: State<App>, name: String, value: String) -> Result<(), String> {
    let slot = app.player.0.lock().unwrap();
    let player = slot.as_ref().ok_or("player not ready")?;
    player.set(&name, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn player_get(app: State<App>, name: String) -> Option<String> {
    let slot = app.player.0.lock().unwrap();
    slot.as_ref().and_then(|p| p.get(&name))
}

#[tauri::command]
fn player_show(window: WebviewWindow, visible: bool) {
    native::settle_child(&window, visible);
}

#[tauri::command]
fn download_start(handle: AppHandle, body: Value) -> Result<(), String> {
    downloads::start(handle, body).map_err(|e| e.to_string())
}

#[tauri::command]
fn download_cancel(key: String) {
    downloads::cancel(&key);
}

#[tauri::command]
fn downloads_active() -> Vec<Value> {
    downloads::active()
}

#[tauri::command]
fn open_external(url: String) {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return;
    }
    native::open(&url);
}

#[tauri::command]
fn open_downloads() {
    if let Some(dir) = downloads::root_dir() {
        let _ = fs::create_dir_all(&dir);
        native::open(&dir.to_string_lossy());
    }
}

#[tauri::command]
fn log(tag: String, msg: String) {
    local::log(&tag, msg);
}

#[tauri::command]
fn win(window: WebviewWindow, action: String, on: Option<bool>) -> Result<Value, String> {
    let on = on.unwrap_or(true);
    let result = match action.as_str() {
        "minimize" => window.minimize().map(|_| Value::Null),
        "maximize" => {
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize().map(|_| Value::Null)
            } else {
                window.maximize().map(|_| Value::Null)
            }
        }
        "close" => window.close().map(|_| Value::Null),
        "fullscreen" => window.set_fullscreen(on).map(|_| Value::Null),
        "ontop" => window.set_always_on_top(on).map(|_| Value::Null),
        "drag" => window.start_dragging().map(|_| Value::Null),
        "mini" => mini(&window, on).map(|_| Value::Null),
        "state" => Ok(json!({
            "maximized": window.is_maximized().unwrap_or(false),
            "fullscreen": window.is_fullscreen().unwrap_or(false),
        })),
        _ => Ok(Value::Null),
    };
    result.map_err(|e| e.to_string())
}

fn mini(window: &WebviewWindow, on: bool) -> tauri::Result<()> {
    static SAVED: Mutex<Option<(tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>, bool)>> = Mutex::new(None);
    if on {
        let was = (window.outer_position()?, window.outer_size()?, window.is_maximized()?);
        *SAVED.lock().unwrap() = Some(was);
        if was.2 {
            window.unmaximize()?;
        }
        let scale = window.scale_factor().unwrap_or(1.0);
        let size = tauri::LogicalSize::new(520.0, 300.0);
        window.set_size(size)?;
        if let Some(monitor) = window.current_monitor()? {
            let area = monitor.size();
            let origin = monitor.position();
            let w = (520.0 * scale) as i32;
            let h = (300.0 * scale) as i32;
            let margin = (24.0 * scale) as i32;
            window.set_position(tauri::PhysicalPosition::new(
                origin.x + area.width as i32 - w - margin,
                origin.y + area.height as i32 - h - margin - (48.0 * scale) as i32,
            ))?;
        }
        window.set_always_on_top(true)?;
    } else {
        window.set_always_on_top(false)?;
        if let Some((pos, size, maxed)) = SAVED.lock().unwrap().take() {
            window.set_size(size)?;
            window.set_position(pos)?;
            if maxed {
                window.maximize()?;
            }
        }
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("viel"));
            let _ = fs::create_dir_all(&dir);
            let store = load_store(&dir);
            let host = store
                .host
                .clone()
                .filter(|h| !h.trim().is_empty())
                .unwrap_or_else(|| "https://viel.tv".into());
            local::configure(&host, store.token.clone());
            downloads::set_root(&dir.join("downloads"));
            relay::port();
            let window = app.get_webview_window("main").expect("main window");
            let backdrop = native::backdrop(&window);
            app.manage(App {
                dir,
                store: Mutex::new(store),
                backdrop: Mutex::new(backdrop),
                player: mpv::Slot::default(),
            });
            let _ = window.show();
            let open = MenuItem::with_id(app, "open", "Open viel", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("viel")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => reveal(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        reveal(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            let keys = ["MediaPlayPause", "MediaTrackNext", "MediaTrackPrevious", "MediaStop"];
            for key in keys {
                let _ = app.global_shortcut().register(key);
            }
            Ok(())
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let key = match shortcut.key {
                        Code::MediaPlayPause => "playpause",
                        Code::MediaTrackNext => "next",
                        Code::MediaTrackPrevious => "prev",
                        Code::MediaStop => "stop",
                        _ => return,
                    };
                    let _ = app.emit("media", key);
                })
                .build(),
        )
        .on_window_event(|window, event| match event {
            WindowEvent::Resized(_) => {
                if let Some(w) = window.get_webview_window("main") {
                    native::fit_child(&w);
                }
            }
            WindowEvent::CloseRequested { api, .. } => {
                let keep = window
                    .try_state::<App>()
                    .map(|a| a.store.lock().unwrap().tray.unwrap_or(true))
                    .unwrap_or(true);
                if keep {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = window.emit("tray", json!({ "hidden": true }));
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            boot,
            api,
            set_host,
            player_init,
            player_cmd,
            player_set,
            player_get,
            player_show,
            download_start,
            download_cancel,
            downloads_active,
            open_external,
            open_downloads,
            log,
            win,
            set_tray,
            presence
        ])
        .run(tauri::generate_context!())
        .expect("viel failed to start");
}
