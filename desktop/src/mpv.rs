use anyhow::{anyhow, bail, Result};
use libloading::{Library, Symbol};
use serde_json::{json, Value};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

type Handle = *mut c_void;

#[repr(C)]
struct Event {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct EventProperty {
    name: *const c_char,
    format: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct EventEndFile {
    reason: c_int,
    error: c_int,
    playlist_entry_id: i64,
    playlist_insert_id: i64,
    playlist_insert_num_entries: c_int,
}

#[repr(C)]
struct EventLogMessage {
    prefix: *const c_char,
    level: *const c_char,
    text: *const c_char,
    log_level: c_int,
}

const FORMAT_STRING: c_int = 1;
const FORMAT_FLAG: c_int = 3;
const FORMAT_INT64: c_int = 4;
const FORMAT_DOUBLE: c_int = 5;

const EVENT_SHUTDOWN: c_int = 1;
const EVENT_LOG_MESSAGE: c_int = 2;
const EVENT_START_FILE: c_int = 6;
const EVENT_END_FILE: c_int = 7;
const EVENT_FILE_LOADED: c_int = 8;
const EVENT_SEEK: c_int = 20;
const EVENT_PLAYBACK_RESTART: c_int = 21;
const EVENT_PROPERTY_CHANGE: c_int = 22;

struct Api {
    _lib: Library,
    create: unsafe extern "C" fn() -> Handle,
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    observe_property: unsafe extern "C" fn(Handle, u64, *const c_char, c_int) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, f64) -> *mut Event,
    request_log_messages: unsafe extern "C" fn(Handle, *const c_char) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    free: unsafe extern "C" fn(*mut c_void),
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

fn library_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["libmpv-2.dll", "mpv-2.dll", "mpv-1.dll"] {
                out.push(dir.join(name));
            }
            out.push(dir.join("lib").join("libmpv-2.dll"));
        }
    }
    for name in ["libmpv-2.dll", "libmpv.so.2", "libmpv.so", "libmpv.2.dylib", "libmpv.dylib"] {
        out.push(PathBuf::from(name));
    }
    out
}

impl Api {
    fn load() -> Result<Self> {
        let mut last = None;
        for path in library_candidates() {
            match unsafe { Library::new(&path) } {
                Ok(lib) => return unsafe { Self::bind(lib) },
                Err(e) => last = Some(format!("{}: {e}", path.display())),
            }
        }
        bail!("libmpv not found ({})", last.unwrap_or_default())
    }

    unsafe fn bind(lib: Library) -> Result<Self> {
        macro_rules! sym {
            ($name:literal) => {{
                let s: Symbol<_> = lib.get(concat!($name, "\0").as_bytes())?;
                *s
            }};
        }
        Ok(Api {
            create: sym!("mpv_create"),
            set_option_string: sym!("mpv_set_option_string"),
            initialize: sym!("mpv_initialize"),
            command: sym!("mpv_command"),
            set_property_string: sym!("mpv_set_property_string"),
            get_property_string: sym!("mpv_get_property_string"),
            observe_property: sym!("mpv_observe_property"),
            wait_event: sym!("mpv_wait_event"),
            request_log_messages: sym!("mpv_request_log_messages"),
            terminate_destroy: sym!("mpv_terminate_destroy"),
            free: sym!("mpv_free"),
            error_string: sym!("mpv_error_string"),
            _lib: lib,
        })
    }

    fn describe(&self, code: c_int) -> String {
        unsafe { CStr::from_ptr((self.error_string)(code)) }
            .to_string_lossy()
            .into_owned()
    }
}

fn api() -> Result<&'static Api> {
    static API: OnceLock<Result<Api, String>> = OnceLock::new();
    API.get_or_init(|| Api::load().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| anyhow!("{e}"))
}

struct Inner {
    handle: Handle,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

pub struct Player {
    inner: Arc<Inner>,
    alive: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

const OBSERVED: &[(&str, c_int)] = &[
    ("pause", FORMAT_FLAG),
    ("time-pos", FORMAT_DOUBLE),
    ("duration", FORMAT_DOUBLE),
    ("paused-for-cache", FORMAT_FLAG),
    ("seeking", FORMAT_FLAG),
    ("eof-reached", FORMAT_FLAG),
    ("core-idle", FORMAT_FLAG),
    ("idle-active", FORMAT_FLAG),
    ("demuxer-cache-duration", FORMAT_DOUBLE),
    ("cache-speed", FORMAT_INT64),
    ("speed", FORMAT_DOUBLE),
    ("volume", FORMAT_DOUBLE),
    ("mute", FORMAT_FLAG),
    ("track-list", FORMAT_STRING),
    ("video-params/w", FORMAT_INT64),
    ("video-params/h", FORMAT_INT64),
    ("video-codec", FORMAT_STRING),
    ("audio-codec-name", FORMAT_STRING),
    ("hwdec-current", FORMAT_STRING),
    ("video-bitrate", FORMAT_INT64),
    ("audio-bitrate", FORMAT_INT64),
    ("frame-drop-count", FORMAT_INT64),
    ("estimated-vf-fps", FORMAT_DOUBLE),
    ("container-fps", FORMAT_DOUBLE),
    ("sid", FORMAT_STRING),
    ("aid", FORMAT_STRING),
    ("sub-visibility", FORMAT_FLAG),
];

impl Player {
    pub fn new(wid: Option<isize>, options: &[(&str, &str)], mut emit: impl FnMut(Value) + Send + 'static) -> Result<Self> {
        let api = api()?;
        let handle = unsafe { (api.create)() };
        if handle.is_null() {
            bail!("mpv_create failed");
        }
        let mut base: Vec<(String, String)> = vec![
            ("config", "no"),
            ("terminal", "no"),
            ("osc", "no"),
            ("osd-level", "0"),
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("input-cursor", "no"),
            ("cursor-autohide", "no"),
            ("ytdl", "no"),
            ("idle", "yes"),
            ("keep-open", "yes"),
            ("force-window", "yes"),
            ("vo", "gpu-next,gpu"),
            ("hwdec", "auto-safe"),
            ("gpu-context", "d3d11,auto"),
            ("background", "color"),
            ("background-color", "#000000"),
            ("sub-auto", "no"),
            ("audio-file-auto", "no"),
            ("demuxer-max-bytes", "150MiB"),
            ("demuxer-readahead-secs", "60"),
            ("cache", "yes"),
            ("hls-bitrate", "max"),
            ("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) viel"),
            ("sub-font-size", "48"),
            ("sub-border-size", "2.4"),
            ("sub-margin-y", "60"),
            ("video-sync", "audio"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        if let Some(w) = wid {
            base.push(("wid".into(), w.to_string()));
        }
        for (k, v) in options {
            base.retain(|(name, _)| name != k);
            base.push((k.to_string(), v.to_string()));
        }
        for (k, v) in &base {
            let key = CString::new(k.as_str())?;
            let value = CString::new(v.as_str())?;
            unsafe { (api.set_option_string)(handle, key.as_ptr(), value.as_ptr()) };
        }
        let code = unsafe { (api.initialize)(handle) };
        if code < 0 {
            unsafe { (api.terminate_destroy)(handle) };
            bail!("mpv_initialize: {}", api.describe(code));
        }
        for (i, (name, format)) in OBSERVED.iter().enumerate() {
            let key = CString::new(*name)?;
            unsafe { (api.observe_property)(handle, i as u64, key.as_ptr(), *format) };
        }
        let level = CString::new("warn")?;
        unsafe { (api.request_log_messages)(handle, level.as_ptr()) };
        let inner = Arc::new(Inner { handle });
        let alive = Arc::new(AtomicBool::new(true));
        let worker = inner.clone();
        let flag = alive.clone();
        let thread = thread::Builder::new()
            .name("mpv-events".into())
            .spawn(move || {
                let mut last_time = Instant::now() - Duration::from_secs(1);
                while flag.load(Ordering::Relaxed) {
                    let ev = unsafe { (api.wait_event)(worker.handle, 0.5) };
                    if ev.is_null() {
                        continue;
                    }
                    let ev = unsafe { &*ev };
                    match ev.event_id {
                        0 => {}
                        EVENT_SHUTDOWN => {
                            emit(json!({ "event": "shutdown" }));
                            break;
                        }
                        EVENT_PROPERTY_CHANGE => {
                            let prop = unsafe { &*(ev.data as *const EventProperty) };
                            let name = unsafe { CStr::from_ptr(prop.name) }.to_string_lossy().into_owned();
                            if name == "time-pos" && last_time.elapsed() < Duration::from_millis(200) {
                                continue;
                            }
                            if name == "time-pos" {
                                last_time = Instant::now();
                            }
                            let value = match prop.format {
                                FORMAT_FLAG => json!(unsafe { *(prop.data as *const c_int) } != 0),
                                FORMAT_DOUBLE => json!(unsafe { *(prop.data as *const f64) }),
                                FORMAT_INT64 => json!(unsafe { *(prop.data as *const i64) }),
                                FORMAT_STRING => {
                                    let ptr = unsafe { *(prop.data as *const *const c_char) };
                                    if ptr.is_null() {
                                        Value::Null
                                    } else {
                                        let text = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
                                        if name == "track-list" {
                                            serde_json::from_str(&text).unwrap_or(Value::Null)
                                        } else {
                                            json!(text)
                                        }
                                    }
                                }
                                _ => Value::Null,
                            };
                            emit(json!({ "event": "prop", "name": name, "value": value }));
                        }
                        EVENT_START_FILE => emit(json!({ "event": "start" })),
                        EVENT_FILE_LOADED => emit(json!({ "event": "loaded" })),
                        EVENT_SEEK => emit(json!({ "event": "seek" })),
                        EVENT_PLAYBACK_RESTART => emit(json!({ "event": "restart" })),
                        EVENT_END_FILE => {
                            let end = unsafe { &*(ev.data as *const EventEndFile) };
                            let reason = match end.reason {
                                0 => "eof",
                                2 => "stop",
                                3 => "quit",
                                4 => "error",
                                5 => "redirect",
                                _ => "unknown",
                            };
                            let error = (end.reason == 4).then(|| api.describe(end.error));
                            emit(json!({ "event": "end", "reason": reason, "error": error }));
                        }
                        EVENT_LOG_MESSAGE => {
                            let msg = unsafe { &*(ev.data as *const EventLogMessage) };
                            let text = unsafe { CStr::from_ptr(msg.text) }.to_string_lossy().trim().to_string();
                            let prefix = unsafe { CStr::from_ptr(msg.prefix) }.to_string_lossy().into_owned();
                            let level = unsafe { CStr::from_ptr(msg.level) }.to_string_lossy().into_owned();
                            if !text.is_empty() {
                                emit(json!({ "event": "log", "prefix": prefix, "level": level, "text": text }));
                            }
                        }
                        _ => {}
                    }
                }
            })?;
        Ok(Player { inner, alive, thread: Some(thread) })
    }

    pub fn command(&self, args: &[String]) -> Result<()> {
        let api = api()?;
        let strings: Vec<CString> = args.iter().map(|a| CString::new(a.as_str())).collect::<Result<_, _>>()?;
        let mut ptrs: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        let code = unsafe { (api.command)(self.inner.handle, ptrs.as_mut_ptr()) };
        if code < 0 {
            bail!("{}: {}", args.join(" "), api.describe(code));
        }
        Ok(())
    }

    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        let api = api()?;
        let key = CString::new(name)?;
        let val = CString::new(value)?;
        let code = unsafe { (api.set_property_string)(self.inner.handle, key.as_ptr(), val.as_ptr()) };
        if code < 0 {
            bail!("set {name}: {}", api.describe(code));
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let api = api().ok()?;
        let key = CString::new(name).ok()?;
        let ptr = unsafe { (api.get_property_string)(self.inner.handle, key.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        let text = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
        unsafe { (api.free)(ptr as *mut c_void) };
        Some(text)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        if let Ok(api) = api() {
            let _ = self.command(&["quit".to_string()]);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            unsafe { (api.terminate_destroy)(self.inner.handle) };
        }
    }
}

pub struct Slot(pub Mutex<Option<Player>>);

impl Default for Slot {
    fn default() -> Self {
        Slot(Mutex::new(None))
    }
}

pub fn available() -> Result<()> {
    api().map(|_| ())
}
