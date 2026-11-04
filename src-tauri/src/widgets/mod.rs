//! Backend command modules, one per widget. Each only calls `crate::platform` traits.
pub mod ai_usage;
pub mod apps;
pub mod awake;
pub mod clipboard;
pub mod countdown;
pub mod currency;
pub mod devstack;
pub mod downloads;
pub mod focus;
pub mod gmail;
pub mod holidays;
pub mod ipu;
pub mod jumplist;
pub mod location;
pub mod lyrics;
pub mod mirror;
pub mod media;
pub mod mic;
pub mod notes;
pub mod ocr;
pub mod ocr_text;
pub mod quick;
pub mod screenshots;
pub mod screentime;
pub mod shelf;
pub mod water;
pub mod solat;
pub mod sound;
pub mod spectrum;
pub mod stitch;
pub mod sysmon;
pub mod tasbih;
pub mod todo;
pub mod trash;
pub mod weather;

use tauri::{AppHandle, Emitter};

/// Live-update watchers: OS change -> Tauri event the widgets subscribe to.
pub fn start_watchers(app: &AppHandle) {
    let p = crate::platform::shell();
    let a = app.clone();
    p.watch_windows(Box::new(move || { let _ = a.emit("apps:changed", crate::platform::shell().list_windows()); }));
    crate::effects::init(app);
    mic::start(app);
    p.watch_minimize(Box::new(crate::effects::on_minimize));
    let a = app.clone();
    let b = app.clone();
    // Changes from outside GlassBar also go to the dock as a volume / brightness alert (the
    // page checks Settings > Dock alerts; dockAlert.ts).
    // Each such change also catches Windows' own pop-up as it appears (windows/osd.rs).
    crate::platform::audio().watch(Box::new(move |s| { let _ = a.emit("audio:changed", s); }), Box::new(move |v, muted| {
        crate::platform::shell().windows_osd_nudge();
        let _ = b.emit_to("bar", "dock:alert", serde_json::json!({ "kind": "volume", "percent": (v * 100.0).round(), "muted": muted }));
    }));
    let a = app.clone();
    crate::platform::display().watch_brightness(Box::new(move |p| {
        crate::platform::shell().windows_osd_nudge();
        let _ = a.emit_to("bar", "dock:alert", serde_json::json!({ "kind": "brightness", "percent": p }));
    }));
    let a = app.clone();
    crate::platform::media().watch(Box::new(move |m| { let _ = a.emit("media:changed", m); }));
}

/// True the first time `key` is asked for since GlassBar started: that read skips the
/// cache, so a fresh start always shows current data (weather, air quality, rates).
pub fn first_since_start(key: &str) -> bool {
    static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let mut seen = SEEN.lock().unwrap();
    if seen.iter().any(|k| k == key) { return false; }
    seen.push(key.to_string());
    true
}

/// Run a blocking OS call (COM, WinRT `.join()`, DDC/CI, display changes) on the
/// blocking pool, so slow calls can't starve the async workers other commands use.
pub async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod start_tests {
    #[test]
    fn first_read_after_start_skips_the_cache_once_per_key() {
        assert!(super::first_since_start("test:a"));
        assert!(!super::first_since_start("test:a"));
        assert!(super::first_since_start("test:b"));
    }
}
