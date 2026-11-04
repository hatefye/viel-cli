use std::ffi::{c_char, CStr};

fn text(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    (!s.trim().is_empty()).then_some(s)
}

#[no_mangle]
pub extern "C" fn viel_start(
    host: *const c_char,
    token: *const c_char,
    downloads: *const c_char,
) -> u16 {
    viel::local::configure(
        &text(host).unwrap_or_else(|| "https://viel.tv".into()),
        text(token),
    );
    if let Some(dir) = text(downloads) {
        viel::offline::set_dir(&dir);
    }
    viel::relay::port()
}

#[no_mangle]
pub extern "C" fn viel_set_token(token: *const c_char) {
    viel::local::set_token(text(token));
}
