//! Downloads widget: the newest finished files in the Downloads folder (or a chosen one).
//! Watches the folder with Windows change notifications (no polling); a file counts as
//! finished once its size has not changed for 2 s. Partial/temp/hidden files are ignored.
use crate::platform::files;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

/// A file counts as finished once its size has stayed the same this long.
pub const SETTLE_MS: i64 = 2_000;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DlItem { pub path: String, pub name: String, pub size: u64, /** arrived (ms) */ pub at: i64 }

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DlState { pub folder: String, pub items: Vec<DlItem>, pub total_size: u64, pub error: Option<String> }

/// Partial downloads, temp files and system files never show.
pub fn is_ignored(name: &str) -> bool {
    let n = name.to_lowercase();
    n == "desktop.ini" || n == "thumbs.db" || n.starts_with("~$") || n.starts_with(".~")
        || [".crdownload", ".part", ".partial", ".tmp", ".download", ".opdownload"].iter().any(|e| n.ends_with(e))
}

/// Remembers each file's size and since when it has had it.
#[derive(Default)]
pub struct Settle { seen: HashMap<PathBuf, (u64, i64)> }
impl Settle {
    /// Record the current size; true once it has not changed for `SETTLE_MS`.
    pub fn finished(&mut self, path: &Path, size: u64, now: i64) -> bool {
        let e = self.seen.entry(path.to_path_buf()).or_insert((size, now));
        if e.0 != size { *e = (size, now); }
        now - e.1 >= SETTLE_MS
    }
    pub fn forget_missing(&mut self, present: &std::collections::HashSet<PathBuf>) { self.seen.retain(|p, _| present.contains(p)); }
}

fn ms(t: SystemTime) -> i64 { t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) }

/// Every file under `dir` (one level, or a few with `deep`), not hidden or ignored:
/// (path, name, size, arrived ms). Arrived = the later of created / modified.
fn scan(dir: &Path, deep: bool) -> Vec<(PathBuf, String, u64, i64)> {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN: u32 = 0x2;
    const SYSTEM: u32 = 0x4;
    let mut out = vec![];
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.file_attributes() & (HIDDEN | SYSTEM) != 0 { continue; }
            let name = e.file_name().to_string_lossy().to_string();
            if m.is_dir() { if deep && depth < 3 { stack.push((e.path(), depth + 1)); } continue; }
            if is_ignored(&name) { continue; }
            let at = ms(m.modified().unwrap_or(UNIX_EPOCH)).max(ms(m.created().unwrap_or(UNIX_EPOCH)));
            out.push((e.path(), name, m.len(), at));
        }
    }
    out
}

static STATE: Mutex<Option<DlState>> = Mutex::new(None);

fn settings(app: &AppHandle) -> (bool, PathBuf, bool, bool) {
    let cfg = app.state::<crate::AppState>().config.lock().unwrap().clone();
    let s = cfg.widget_settings("downloads");
    let folder = s["folder"].as_str().filter(|f| !f.trim().is_empty()).map(PathBuf::from)
        .or_else(|| files().downloads_dir().map(PathBuf::from)).unwrap_or_default();
    (cfg.uses("downloads"), folder, s["subfolders"] == true, s["toast"] == true)
}

/// Watch thread: idle unless the widget is on a bar.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
        use windows::Win32::Storage::FileSystem::*;
        use windows::Win32::System::Threading::WaitForSingleObject;
        let mut settle = Settle::default();
        let mut known: Option<std::collections::HashSet<PathBuf>> = None; // finished files already announced
        let mut watching: Option<(PathBuf, bool, HANDLE)> = None;
        loop {
            let (on, folder, deep, toast) = settings(&app);
            if !on || folder.as_os_str().is_empty() {
                if let Some((_, _, h)) = watching.take() { unsafe { let _ = FindCloseChangeNotification(h); } }
                known = None;
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
            // (Re)arm the watcher when the folder or the subfolder choice changes.
            if watching.as_ref().map(|(f, d, _)| (f, *d)) != Some((&folder, deep)) {
                if let Some((_, _, h)) = watching.take() { unsafe { let _ = FindCloseChangeNotification(h); } }
                let w: Vec<u16> = folder.to_string_lossy().encode_utf16().chain(Some(0)).collect();
                let h = unsafe { FindFirstChangeNotificationW(PCWSTR(w.as_ptr()), deep,
                    FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME | FILE_NOTIFY_CHANGE_SIZE | FILE_NOTIFY_CHANGE_LAST_WRITE) };
                match h {
                    Ok(h) => watching = Some((folder.clone(), deep, h)),
                    Err(e) => {
                        *STATE.lock().unwrap() = Some(DlState { folder: folder.to_string_lossy().into(), error: Some(format!("Can't watch this folder: {}", e.message())), ..Default::default() });
                        let _ = app.emit("downloads:changed", ());
                        std::thread::sleep(Duration::from_secs(10));
                        continue;
                    }
                }
                known = None;
            }
            // Scan, keep only finished files, newest first.
            let now = ms(SystemTime::now());
            let all = scan(&folder, deep);
            let present: std::collections::HashSet<PathBuf> = all.iter().map(|f| f.0.clone()).collect();
            settle.forget_missing(&present);
            let mut pending = false;
            let mut items: Vec<DlItem> = all.iter().filter_map(|(p, name, size, at)| {
                if settle.finished(p, *size, now) { Some(DlItem { path: p.to_string_lossy().into(), name: name.clone(), size: *size, at: *at }) }
                else { pending = true; None }
            }).collect();
            items.sort_by(|a, b| b.at.cmp(&a.at));
            let total_size = all.iter().map(|f| f.2).sum();
            items.truncate(200);
            // New finished files (not on the first scan): pulse the pill, optional alert.
            let finished: std::collections::HashSet<PathBuf> = items.iter().map(|i| PathBuf::from(&i.path)).collect();
            if let Some(k) = &known {
                for i in items.iter().filter(|i| !k.contains(&PathBuf::from(&i.path))) {
                    let _ = app.emit("downloads:new", i);
                    if toast { crate::alerts::notify(&app, "downloads", "Download finished", &i.name, false); }
                }
            }
            known = Some(finished);
            let changed = {
                let mut st = STATE.lock().unwrap();
                let next = DlState { folder: folder.to_string_lossy().into(), items, total_size, error: None };
                let changed = st.as_ref().map(|s| (&s.items, s.total_size, &s.folder)) != Some((&next.items, next.total_size, &next.folder));
                *st = Some(next);
                changed
            };
            if changed { let _ = app.emit("downloads:changed", ()); }
            // Sleep until the folder changes (or briefly, while a file is still settling;
            // or 5 s, only to notice settings changes).
            let h = watching.as_ref().unwrap().2;
            let r = unsafe { WaitForSingleObject(h, if pending { 1000 } else { 5000 }) };
            if r == WAIT_OBJECT_0 {
                unsafe { let _ = FindNextChangeNotification(h); }
                std::thread::sleep(Duration::from_millis(300)); // let a burst of changes land
            } else if !pending {
                // Timed out with nothing pending: skip the rescan unless settings changed.
                loop {
                    let (on2, f2, d2, _) = settings(&app);
                    if !on2 || f2 != folder || d2 != deep { break; }
                    let r = unsafe { WaitForSingleObject(h, 5000) };
                    if r == WAIT_OBJECT_0 { unsafe { let _ = FindNextChangeNotification(h); } std::thread::sleep(Duration::from_millis(300)); break; }
                }
            }
        }
    });
}

fn rescan_soon(app: &AppHandle) { let _ = app.emit("downloads:changed", ()); }

#[tauri::command]
pub fn downloads_state() -> DlState {
    STATE.lock().unwrap().clone().unwrap_or_default()
}

fn exists(path: &str) -> Result<(), String> {
    if Path::new(path).exists() { Ok(()) } else { Err("File no longer exists".into()) }
}

#[tauri::command]
pub fn downloads_open(path: String) -> Result<(), String> { exists(&path)?; crate::platform::shell().launch(&path, "") }
#[tauri::command]
pub fn downloads_reveal(path: String) -> Result<(), String> { exists(&path)?; files().reveal(&path) }
#[tauri::command]
pub fn downloads_open_folder(app: AppHandle) -> Result<(), String> {
    let (_, folder, _, _) = settings(&app);
    crate::platform::shell().launch(&folder.to_string_lossy(), "")
}
#[tauri::command]
pub fn downloads_open_with(path: String) -> Result<(), String> { exists(&path)?; files().open_with(&path) }

/// Rename in place. Only a plain file name: no folders, no characters Windows forbids.
#[tauri::command]
pub fn downloads_rename(app: AppHandle, path: String, name: String) -> Result<String, String> {
    exists(&path)?;
    let name = name.trim();
    if name.is_empty() || name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) || name == "." || name == ".." {
        return Err("Not a valid file name".into());
    }
    let to = Path::new(&path).with_file_name(name);
    if to.exists() { return Err("A file with that name already exists".into()); }
    std::fs::rename(&path, &to).map_err(|e| e.to_string())?;
    rescan_soon(&app);
    Ok(to.to_string_lossy().into())
}

/// Move to the Recycle Bin (never a permanent delete).
#[tauri::command]
pub async fn downloads_recycle(app: AppHandle, path: String) -> Result<(), String> {
    exists(&path)?;
    crate::widgets::blocking(move || files().recycle(&path)).await??;
    recycle_stale();
    let _ = app.emit("recycle:changed", ());
    Ok(())
}

/// Drag the file out of the panel into another app (runs on the UI thread).
#[tauri::command]
pub fn downloads_drag(app: AppHandle, window: tauri::WebviewWindow, path: String) -> Result<(), String> {
    exists(&path)?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    app.run_on_main_thread(move || { let _ = files().drag_out(hwnd, &path); }).map_err(|e| e.to_string())
}

static ICONS: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

/// Boost: drop the icon cache; icons are read again when shown.
pub fn free_icons() { *ICONS.lock().unwrap() = None; }

/// The real Windows icon for a file: per type, but per file for apps/shortcuts/icons.
#[tauri::command]
pub async fn file_icon(path: String) -> Option<String> {
    let ext = Path::new(&path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let key = if ["exe", "lnk", "ico", "url", "appref-ms"].contains(&ext.as_str()) { path.to_lowercase() } else { format!(".{ext}") };
    if let Some(hit) = ICONS.lock().unwrap().get_or_insert_with(HashMap::new).get(&key) { return hit.clone(); }
    let icon = crate::widgets::blocking(move || files().file_icon(&path)).await.ok().flatten();
    ICONS.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, icon.clone());
    icon
}

// ---- Recycle Bin (System Monitor) ----

/// Size and count of the Recycle Bin, per drive too. An error carries Windows' reason
/// (e.g. a policy), never a guessed number.
#[tauri::command]
pub async fn recycle_info() -> Result<crate::platform::RecycleInfo, String> {
    crate::widgets::blocking(recycle_cached).await?
}

/// Windows counts every item (seconds for a big bin): share one answer for 50 s and never
/// run two queries at once (the panel card, the pill and Downloads all ask).
static BIN: Mutex<Option<(std::time::Instant, crate::platform::RecycleInfo)>> = Mutex::new(None);
static BIN_QUERY: Mutex<()> = Mutex::new(());
fn recycle_cached() -> Result<crate::platform::RecycleInfo, String> {
    let fresh = || BIN.lock().unwrap().as_ref().filter(|(t, _)| t.elapsed() < Duration::from_secs(50)).map(|(_, i)| i.clone());
    if let Some(i) = fresh() { return Ok(i); }
    let _one = BIN_QUERY.lock().unwrap();
    if let Some(i) = fresh() { return Ok(i); } // another caller just finished
    let i = files().recycle_info()?;
    *BIN.lock().unwrap() = Some((std::time::Instant::now(), i.clone()));
    Ok(i)
}
pub fn recycle_stale() { *BIN.lock().unwrap() = None; }

/// Empty the bin (all drives, or one). The panel has already asked the user.
#[tauri::command]
pub async fn recycle_empty(app: AppHandle, root: Option<String>) -> Result<(), String> {
    crate::widgets::blocking(move || files().recycle_empty(root.as_deref())).await??;
    recycle_stale();
    let _ = app.emit("recycle:changed", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_and_temp_files_are_ignored() {
        for n in ["video.mp4.crdownload", "big.iso.part", "x.TMP", "file.download", "desktop.ini", "Desktop.INI", "~$report.docx", "a.opdownload"] {
            assert!(is_ignored(n), "{n} should be ignored");
        }
        for n in ["report.pdf", "setup.exe", "photo.part1.rar", "partner.txt", "download.zip"] {
            assert!(!is_ignored(n), "{n} should show");
        }
    }

    #[test]
    fn finished_once_size_stops_changing_for_2s() {
        let mut s = Settle::default();
        let p = Path::new("C:/d/a.zip");
        assert!(!s.finished(p, 100, 0), "just appeared");
        assert!(!s.finished(p, 500, 1_500), "still growing");
        assert!(!s.finished(p, 500, 3_000), "same size for 1.5 s only");
        assert!(s.finished(p, 500, 3_500), "same size for 2 s");
        assert!(!s.finished(p, 900, 4_000), "grew again: not finished");
    }
}
