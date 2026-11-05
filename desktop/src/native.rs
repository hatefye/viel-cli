use tauri::WebviewWindow;

#[cfg(windows)]
mod win32 {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetClientRect, MoveWindow, SetWindowPos, ShowWindow, HWND_BOTTOM,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNA,
    };

    unsafe extern "system" fn visit(child: HWND, out: LPARAM) -> BOOL {
        let mut name = [0u16; 32];
        let len = GetClassNameW(child, name.as_mut_ptr(), name.len() as i32);
        let class = String::from_utf16_lossy(&name[..len.max(0) as usize]);
        if class == "mpv" {
            *(out as *mut HWND) = child;
            return 0;
        }
        1
    }

    pub fn mpv_child(parent: isize) -> Option<isize> {
        let mut found: HWND = std::ptr::null_mut();
        unsafe { EnumChildWindows(parent as HWND, Some(visit), &mut found as *mut HWND as LPARAM) };
        (!found.is_null()).then(|| found as isize)
    }

    pub fn fit(parent: isize, child: isize) {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe {
            if GetClientRect(parent as HWND, &mut rect) != 0 {
                MoveWindow(child as HWND, 0, 0, rect.right - rect.left, rect.bottom - rect.top, 1);
            }
        }
    }

    pub fn sink(child: isize) {
        unsafe {
            SetWindowPos(child as HWND, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    pub fn show(child: isize, visible: bool) {
        unsafe {
            ShowWindow(child as HWND, if visible { SW_SHOWNA } else { SW_HIDE });
        }
    }
}

pub fn window_id(window: &WebviewWindow) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as isize),
        RawWindowHandle::Xlib(h) => Some(h.window as isize),
        RawWindowHandle::Xcb(h) => Some(h.window.get() as isize),
        _ => None,
    }
}

#[cfg(windows)]
pub fn settle_child(window: &WebviewWindow, visible: bool) {
    let Some(parent) = window_id(window) else { return };
    let _ = window.run_on_main_thread(move || {
        for _ in 0..20 {
            if let Some(child) = win32::mpv_child(parent) {
                win32::sink(child);
                win32::fit(parent, child);
                win32::show(child, visible);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    });
}

#[cfg(not(windows))]
pub fn settle_child(_window: &WebviewWindow, _visible: bool) {}

#[cfg(windows)]
pub fn fit_child(window: &WebviewWindow) {
    if let Some(parent) = window_id(window) {
        if let Some(child) = win32::mpv_child(parent) {
            win32::fit(parent, child);
        }
    }
}

#[cfg(not(windows))]
pub fn fit_child(_window: &WebviewWindow) {}

pub fn backdrop(window: &WebviewWindow) -> &'static str {
    #[cfg(windows)]
    {
        if window_vibrancy::apply_mica(window, Some(true)).is_ok() {
            return "mica";
        }
        if window_vibrancy::apply_acrylic(window, Some((12, 13, 18, 160))).is_ok() {
            return "acrylic";
        }
    }
    let _ = window;
    "none"
}

pub fn open(target: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(target).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(target).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}
