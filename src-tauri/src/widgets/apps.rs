use super::blocking;
use crate::platform::{shell, AppWindow, InstalledApp, Res};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
pub struct PinTarget { pub path: String, pub args: String }

/// A .lnk becomes its real program (so it matches running windows); anything else as is.
fn pin_target(path: String) -> Result<PinTarget, String> {
    if !std::path::Path::new(&path).exists() { return Err(format!("{path} doesn't exist")); }
    if path.to_lowercase().ends_with(".lnk") {
        if let Some((t, args)) = shell().resolve_shortcut(&path) { return Ok(PinTarget { path: t, args }); }
    }
    Ok(PinTarget { path, args: String::new() })
}

/// Check a typed/pasted path before pinning.
#[tauri::command]
pub async fn app_pin_target(path: String) -> Result<PinTarget, String> { blocking(move || pin_target(path)).await? }

/// File picker for a program or shortcut. None = cancelled.
#[tauri::command]
pub async fn app_browse(app: AppHandle) -> Result<Option<PinTarget>, String> {
    blocking(move || {
        let Some(f) = app.dialog().file().set_title("Pin a program").add_filter("Programs and shortcuts", &["exe", "lnk"]).blocking_pick_file() else { return Ok(None) };
        let p = f.into_path().map_err(|e| e.to_string())?;
        pin_target(p.to_string_lossy().into_owned()).map(Some)
    }).await?
}

#[tauri::command]
pub async fn apps_installed() -> Vec<InstalledApp> { blocking(installed).await.unwrap_or_default() }

/// Listing the apps takes ~2 s (asking Windows for its Store apps), too slow for a search box:
/// answer from the last list and refresh it in the background when it's over a minute old.
pub fn installed() -> Vec<InstalledApp> {
    use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};
    use std::time::{Duration, Instant};
    static LAST: Mutex<Option<(Instant, Vec<InstalledApp>)>> = Mutex::new(None);
    static BUSY: AtomicBool = AtomicBool::new(false);
    let scan = || { let v = shell().installed_apps(); *LAST.lock().unwrap() = Some((Instant::now(), v.clone())); v };
    let last = LAST.lock().unwrap().clone();
    match last {
        None => scan(),
        Some((at, v)) => {
            if at.elapsed() > Duration::from_secs(60) && !BUSY.swap(true, Ordering::Relaxed) {
                std::thread::spawn(move || { scan(); BUSY.store(false, Ordering::Relaxed); });
            }
            v
        }
    }
}

/// Which of these paths still exist (pinned apps that were uninstalled show as missing).
/// A Store app whose versioned folder vanished after an update still counts while installed.
#[tauri::command]
pub async fn paths_exist(paths: Vec<String>) -> Vec<bool> {
    blocking(move || paths.iter().map(|p| std::path::Path::new(p).exists() || shell().aumid_for_path(p).is_some()).collect()).await.unwrap_or_default()
}

#[tauri::command]
pub async fn apps_list() -> Vec<AppWindow> { blocking(move || shell().list_windows()).await.unwrap_or_default() }
/// Real app icon: the Store package's own icon for packaged apps (also when only a
/// WindowsApps path is known), else the program file's icon, else the window's.
#[tauri::command]
pub async fn app_icon(exe: String, window: Option<isize>, aumid: Option<String>) -> Option<String> {
    blocking(move || {
        let aumid = aumid.or_else(|| shell().aumid_for_path(&exe));
        // An installed web app runs as the browser's exe: never fall back to the browser's
        // icon before the app's own (its Start-menu entry, then its window icon).
        let web = aumid.as_deref().is_some_and(crate::platform::is_web_app_id);
        let by_id = aumid.and_then(|a| shell().aumid_icon(&a));
        if web { return by_id.or_else(|| window.and_then(|w| shell().window_icon(w))).or_else(|| shell().exe_icon(&exe)); }
        by_id
            .or_else(|| shell().exe_icon(&exe))
            .or_else(|| window.and_then(|w| shell().window_icon(w)))
    }).await.ok().flatten()
}
const VK_LWIN: u16 = 0x5B;

/// Start button: the same as pressing the Windows key.
#[tauri::command]
pub fn start_menu() { shell().press_keys(&[VK_LWIN]) }

/// Search button: Win+S.
#[tauri::command]
pub fn windows_search() { shell().press_keys(&[VK_LWIN, b'S' as u16]) }

#[tauri::command]
pub async fn app_activate(id: isize) { let _ = blocking(move || shell().activate(id)).await; }
#[tauri::command]
pub async fn app_close(id: isize) { let _ = blocking(move || shell().close(id)).await; }
/// Store apps start through their app identity (Windows refuses their .exe in WindowsApps);
/// everything else through the shell, like double-clicking it.
#[tauri::command]
pub async fn app_launch(path: String, args: String, aumid: Option<String>) -> Res<()> {
    blocking(move || match aumid.filter(|a| !a.is_empty()).or_else(|| shell().aumid_for_path(&path)) {
        Some(a) => shell().launch_aumid(&a),
        None => shell().launch(&path, &args),
    }).await?
}
/// "Run as administrator": Windows asks first (UAC). Programs and shortcuts only.
#[tauri::command]
pub async fn app_launch_admin(path: String, args: String) -> Res<()> {
    blocking(move || shell().launch_elevated(&path, &args, true)).await?
}
/// The UAC shield for the menu item above (PNG bytes).
#[tauri::command]
pub fn shield_icon() -> Option<Vec<u8>> { shell().shield_icon() }
