//! Settings › Storage: what GlassBar keeps in %APPDATA%\GlassBar, split by owner (widget,
//! app data, leftovers), and cleanup that says what it removes before it removes it.
//! Deletes only inside the data folder. Before touching the database it copies glassbar.db*
//! to a dated `backup-storage-*` folder, and runs VACUUM afterwards so the file shrinks.
use crate::db::{now_ms, Db};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

const DAY: i64 = 86_400_000;
/// Cleanups that delete database rows (and so back it up first and VACUUM after).
const DB_KINDS: &[&str] = &["clipboard", "screenshots", "ocr_history", "alerts", "screen_time", "water", "kv"];

#[derive(Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Group { Widget, App, Leftover, Other }

pub type Owner = (Group, &'static str);

const DATABASE: Owner = (Group::App, "Database");
const OTHER: Owner = (Group::Other, "Other");

/// Who owns a database table. Every table in db::MIGRATIONS has an owner (tested).
pub fn owner_of_table(t: &str) -> Owner {
    match t {
        "clipboard" => (Group::Widget, "Clipboard"),
        "screenshots" => (Group::Widget, "Screenshots"),
        "notes" => (Group::Widget, "Notes"),
        "todos" => (Group::Widget, "To-do"),
        "timers" => (Group::Widget, "Timer"),
        "screen_time" => (Group::Widget, "Screen time"),
        "ocr_history" => (Group::Widget, "Text capture (OCR)"),
        "water" => (Group::Widget, "Water"),
        "alerts" => (Group::Widget, "Notifications"),
        // Which reminders already went off: bookkeeping for alerts, not anyone's data.
        "fired" => DATABASE,
        // Split row by row between widgets: see owner_of_kv.
        "kv" => DATABASE,
        t if t.starts_with("sqlite_") => DATABASE,
        _ => OTHER,
    }
}

/// Who owns a `kv` row, by its key's prefix (the part before the first ':').
pub fn owner_of_kv(key: &str) -> Owner {
    match key.split(':').next().unwrap_or("") {
        "lyrics" | "lyrics2" => (Group::Widget, "Lyrics"),
        "ipu" => (Group::Widget, "Air quality"),
        "fx" => (Group::Widget, "Currency"),
        "gmail" => (Group::Widget, "Gmail"),
        "solat" => (Group::Widget, "Prayer times"),
        "weather" => (Group::Widget, "Weather"),
        "water" => (Group::Widget, "Water"),
        "pomodoro" | "pomo" | "stopwatch" => (Group::Widget, "Focus timer"),
        "tasbih" => (Group::Widget, "Tasbih"),
        "location" | "place" => (Group::App, "Location"),
        "whatsnew" => DATABASE,
        "boost" => (Group::App, "Performance"),
        _ => OTHER,
    }
}

/// `kv` prefixes that only hold downloaded data the widget fetches again: "Clear cache".
const KV_CACHES: &[(&str, &str)] = &[("Lyrics", "lyrics2"), ("Air quality", "ipu"), ("Currency", "fx")];

/// Who owns a file or folder at the top of the data folder.
pub fn owner_of_entry(name: &str, is_dir: bool) -> Owner {
    let n = name.to_ascii_lowercase();
    if is_dir {
        return match n.as_str() {
            "clipboard" => (Group::Widget, "Clipboard"),
            "screenshots" => (Group::Widget, "Screenshots"),
            "notes" => (Group::Widget, "Notes"),
            "lyrics" => (Group::Widget, "Lyrics"),
            "aiusage-web" => (Group::Leftover, "aiusage-web"),
            n if n.starts_with("backup-") || n.starts_with("pre-shot-backup-") || n.starts_with("incident-")
                || n.starts_with("promo-backup") => (Group::Leftover, "Backups"),
            _ => OTHER,
        };
    }
    match n.as_str() {
        "glassbar.db" | "glassbar.db-wal" | "glassbar.db-shm" => DATABASE,
        "glassbar.db.bak" | "glassbar.db.bak.old" => (Group::App, "Daily backup"),
        "glassbar.log" | "glassbar.log.old" | "panic.log" | "config.json" | "taskbar-original.txt" => (Group::App, "Log and settings"),
        n if n.starts_with("glassbar.db.bak.tmp-") => (Group::App, "Daily backup"),
        n if n.starts_with("glassbar.db.corrupt-") || n.starts_with("glassbar.db.before-restore") => (Group::Leftover, "Old database copies"),
        n if n.starts_with("config.json.bak") => (Group::Leftover, "Old settings copies"),
        _ => OTHER,
    }
}

/// Leftover groups with many dated entries: "older than 7 days" and "all but the newest".
const DATED: &[&str] = &["Backups", "Old database copies", "Old settings copies"];

/// Indexes of the entries to delete: those modified before `cutoff`, never the newest one.
pub fn deletable(modified: &[i64], cutoff: i64) -> Vec<usize> {
    let newest = (0..modified.len()).max_by_key(|&i| modified[i]);
    (0..modified.len()).filter(|&i| Some(i) != newest && modified[i] < cutoff).collect()
}

/// `p` is inside `root` (not `root` itself), after resolving `..` and links.
pub fn inside(root: &Path, p: &Path) -> bool {
    match (root.canonicalize(), p.canonicalize()) {
        (Ok(r), Ok(c)) => c != r && c.starts_with(&r),
        _ => false,
    }
}

/// Bytes in a file or folder, as Explorer adds them up. Links are not followed.
pub fn size_of(p: &Path) -> u64 {
    let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
    if m.is_symlink() { return 0; }
    if !m.is_dir() { return m.len(); }
    std::fs::read_dir(p).map(|d| d.filter_map(Result::ok).map(|e| size_of(&e.path())).sum()).unwrap_or(0)
}

/// Delete a file or folder inside `root` and return the bytes it held. A link is removed,
/// never what it points to.
fn remove(root: &Path, p: &Path) -> u64 {
    if !inside(root, p) { crate::applog::log(format!("storage: refused to delete {} (outside {})", p.display(), root.display())); return 0; }
    let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
    let bytes = size_of(p);
    let r = if m.is_symlink() { std::fs::remove_file(p).or_else(|_| std::fs::remove_dir(p)) }
            else if m.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
    match r { Ok(()) => bytes, Err(e) => { crate::applog::log(format!("storage: could not delete {}: {e}", p.display())); 0 } }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item { pub name: String, pub bytes: u64, pub modified: i64, pub dir: bool }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub id: String, pub label: String,
    /// What exactly goes, for the confirmation.
    pub detail: String,
    pub bytes: u64, pub destructive: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub group: Group, pub name: String,
    /// Files plus its share of the database.
    pub bytes: u64, pub db_bytes: u64,
    /// "20 items, 3 pinned" and the like.
    pub note: String,
    pub items: Vec<Item>, pub actions: Vec<Action>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub dir: String, pub total: u64, pub rows: Vec<Row>,
    /// WebView2's own folder (outside the data folder): its size, and how much is cache.
    pub webview: Option<(String, u64, u64)>,
}

/// Bytes each table and index takes inside the database (dbstat), by table.
fn table_bytes(c: &Connection) -> Vec<(String, u64)> {
    c.prepare("SELECT coalesce(m.tbl_name, s.name), sum(s.pgsize) FROM dbstat s LEFT JOIN sqlite_master m ON m.name = s.name GROUP BY 1")
        .and_then(|mut q| q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))).map(|r| r.filter_map(Result::ok).collect()))
        .unwrap_or_default()
}

/// kv's bytes split by owner, in proportion to the size of each owner's keys and values.
fn kv_split(c: &Connection, kv_bytes: u64) -> Vec<(Owner, u64)> {
    let rows: Vec<(String, u64)> = c.prepare("SELECT key, length(key) + length(value) FROM kv")
        .and_then(|mut q| q.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64))).map(|r| r.filter_map(Result::ok).collect()))
        .unwrap_or_default();
    let sum: u64 = rows.iter().map(|r| r.1).sum::<u64>().max(1);
    let mut by: BTreeMap<Owner, u64> = BTreeMap::new();
    for (k, n) in rows { *by.entry(owner_of_kv(&k)).or_default() += n; }
    by.into_iter().map(|(o, n)| (o, kv_bytes * n / sum)).collect()
}

fn count(c: &Connection, sql: &str) -> i64 { c.query_row(sql, [], |r| r.get(0)).unwrap_or(0) }

/// Sum of the files the rows name, counting only those in `root` (the only ones a cleanup deletes).
fn files_bytes(c: &Connection, sql: &str, root: &Path) -> u64 {
    c.prepare(sql).and_then(|mut q| q.query_map([], |r| r.get::<_, String>(0))
        .map(|r| r.filter_map(Result::ok).map(|p| Path::new(&p).to_path_buf()).filter(|p| inside(root, p)).map(|p| size_of(&p)).sum()))
        .unwrap_or(0)
}

/// Files in a widget's folder that no row uses: said, never deleted. A sweep for them once
/// deleted every image after the database came up empty.
fn strays(r: &Row, c: &Connection, sql: &str, root: &Path) -> String {
    let n = (r.bytes - r.db_bytes).saturating_sub(files_bytes(c, sql, root));
    if n < 64 * 1024 { String::new() } else { format!(" · {} in older files no item uses (left alone)", mb(n)) }
}

/// What a widget's rows plus files would free; a rough per-row estimate is fine for the preview.
fn share(db: u64, part: i64, all: i64) -> u64 { if all <= 0 { 0 } else { db * part.max(0) as u64 / all as u64 } }

fn mb(b: u64) -> String { human(b) }

pub fn human(b: u64) -> String {
    let b = b as f64;
    if b >= 1_073_741_824.0 { format!("{:.1} GB", b / 1_073_741_824.0) }
    else if b >= 1_048_576.0 { format!("{:.1} MB", b / 1_048_576.0) }
    else if b >= 1024.0 { format!("{:.0} KB", b / 1024.0) }
    else { format!("{b} bytes") }
}

fn plural(n: i64, one: &str) -> String { if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") } }

fn modified_ms(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn webview_dir(app: &AppHandle) -> Option<PathBuf> { app.path().app_local_data_dir().ok().map(|d| d.join("EBWebView")) }

const WEBVIEW_CACHES: &[&str] = &["Default/Cache", "Default/Code Cache", "Default/GPUCache", "GrShaderCache", "ShaderCache"];

pub fn scan(app: &AppHandle, dir: &Path) -> Report {
    let mut rows: BTreeMap<Owner, Row> = BTreeMap::new();
    fn row(rows: &mut BTreeMap<Owner, Row>, o: Owner) -> &mut Row {
        rows.entry(o).or_insert_with(|| Row { group: o.0, name: o.1.into(), bytes: 0, db_bytes: 0, note: String::new(), items: vec![], actions: vec![] })
    }
    // Files on disk.
    let mut total = 0;
    for e in std::fs::read_dir(dir).map(|d| d.filter_map(Result::ok).collect::<Vec<_>>()).unwrap_or_default() {
        let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
        let name = e.file_name().to_string_lossy().into_owned();
        let bytes = size_of(&e.path());
        total += bytes;
        let r = row(&mut rows, owner_of_entry(&name, m.is_dir()));
        r.bytes += bytes;
        r.items.push(Item { name, bytes, modified: modified_ms(&m), dir: m.is_dir() });
    }
    // The database, split by owner. Its files are already counted under Database: move each
    // owner's share over so the total still matches Explorer.
    let db = app.state::<Db>();
    let c = db.lock();
    let mut moved = 0;
    for (t, b) in table_bytes(&c) {
        let parts = if t == "kv" { kv_split(&c, b) } else { vec![(owner_of_table(&t), b)] };
        for (o, b) in parts {
            if o == DATABASE { continue; }
            let r = row(&mut rows, o);
            r.db_bytes += b;
            r.bytes += b;
            moved += b;
        }
    }
    let d = row(&mut rows, DATABASE);
    let moved = moved.min(d.bytes);
    d.bytes -= moved;
    // Notes and actions, widget by widget. Each preview counts exactly what its action removes.
    let page: u64 = c.query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0)).unwrap_or(4096) as u64;
    let free = c.query_row("PRAGMA freelist_count", [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64 * page;
    let wal = std::fs::metadata(dir.join("glassbar.db-wal")).map(|m| m.len()).unwrap_or(0);
    d.note = format!("glassbar.db and its write-ahead log; {} unused inside", mb(free));
    if free + wal > 0 {
        d.actions.push(Action { id: "compact".into(), label: "Compact".into(), destructive: false, bytes: free + wal,
            detail: "Rewrites the database without its unused space and folds the write-ahead log back in. No data is removed.".into() });
    }
    {
        let all = count(&c, "SELECT count(*) FROM clipboard");
        let (pinned, unp) = (count(&c, "SELECT count(*) FROM clipboard WHERE pinned = 1"), count(&c, "SELECT count(*) FROM clipboard WHERE pinned = 0"));
        let r = row(&mut rows, (Group::Widget, "Clipboard"));
        r.note = format!("{}, {pinned} pinned{}", plural(all, "item"), strays(r, &c, "SELECT path FROM clipboard WHERE path IS NOT NULL", &dir.join("clipboard")));
        let files = files_bytes(&c, "SELECT path FROM clipboard WHERE pinned = 0 AND path IS NOT NULL", &dir.join("clipboard"));
        if unp > 0 {
            let b = files + share(r.db_bytes, unp, all);
            r.actions.push(Action { id: "clipboard".into(), label: "Clear unpinned history".into(), destructive: true, bytes: b,
                detail: format!("Removes {} from the clipboard history and their image files ({}). Pinned items stay.", plural(unp, "unpinned item"), mb(b)) });
        }
    }
    {
        let n = count(&c, "SELECT count(*) FROM screenshots");
        let r = row(&mut rows, (Group::Widget, "Screenshots"));
        r.note = format!("{}{}", plural(n, "screenshot"), strays(r, &c, "SELECT path FROM screenshots", &dir.join("screenshots")));
        if n > 0 {
            let b = files_bytes(&c, "SELECT path FROM screenshots", &dir.join("screenshots")) + r.db_bytes;
            r.actions.push(Action { id: "screenshots".into(), label: "Delete all screenshots".into(), destructive: true, bytes: b,
                detail: format!("Deletes all {} GlassBar saved ({}). Copies you saved elsewhere are not touched.", plural(n, "screenshot"), mb(b)) });
        }
    }
    for (name, table, what) in [("Text capture (OCR)", "ocr_history", "captured text"), ("Notifications", "alerts", "notification")] {
        let n = count(&c, &format!("SELECT count(*) FROM {table}"));
        let r = row(&mut rows, (Group::Widget, name));
        r.note = plural(n, if table == "alerts" { "notification" } else { "capture" });
        if n > 0 {
            r.actions.push(Action { id: table.into(), label: "Clear history".into(), destructive: true, bytes: r.db_bytes,
                detail: format!("Removes all {} of {what} history ({}).", n, mb(r.db_bytes)) });
        }
    }
    {
        let cutoff = (chrono::Local::now().date_naive() - chrono::Duration::days(7)).to_string();
        let (all, old) = (count(&c, "SELECT count(*) FROM screen_time"), c.query_row("SELECT count(*) FROM screen_time WHERE day < ?1", [&cutoff], |r| r.get::<_, i64>(0)).unwrap_or(0));
        let r = row(&mut rows, (Group::Widget, "Screen time"));
        r.note = format!("{} of app use; kept for 30 days", plural(count(&c, "SELECT count(DISTINCT day) FROM screen_time"), "day"));
        if old > 0 {
            let b = share(r.db_bytes, old, all);
            r.actions.push(Action { id: "screen_time".into(), label: "Clear older than 7 days".into(), destructive: true, bytes: b,
                detail: format!("Removes {} of app usage from before {cutoff} ({}).", plural(old, "app-day"), mb(b)) });
        }
    }
    {
        let cutoff = now_ms() - 30 * DAY;
        let (all, old) = (count(&c, "SELECT count(*) FROM water"), c.query_row("SELECT count(*) FROM water WHERE at < ?1", [cutoff], |r| r.get::<_, i64>(0)).unwrap_or(0));
        let r = row(&mut rows, (Group::Widget, "Water"));
        r.note = plural(all, "drink");
        if old > 0 {
            let b = share(r.db_bytes, old, all);
            r.actions.push(Action { id: "water".into(), label: "Clear older than 30 days".into(), destructive: true, bytes: b,
                detail: format!("Removes {} logged more than 30 days ago ({}).", plural(old, "drink"), mb(b)) });
        }
    }
    for (name, prefix) in KV_CACHES {
        let n = c.query_row("SELECT count(*) FROM kv WHERE key LIKE ?1", [format!("{prefix}%")], |r| r.get::<_, i64>(0)).unwrap_or(0);
        let r = row(&mut rows, (Group::Widget, name));
        if *name == "Lyrics" { r.note = format!("{} cached", plural(n, "song")); }
        if n > 0 {
            let b = r.db_bytes;
            r.actions.push(Action { id: format!("kv:{prefix}"), label: "Clear cache".into(), destructive: false, bytes: b,
                detail: format!("Removes {} of downloaded data ({}). It is fetched again when needed.", plural(n, "cached entry"), mb(b)) });
        }
    }
    {
        let r = row(&mut rows, (Group::Widget, "Notes"));
        r.note = format!("{}; kept", plural(count(&c, "SELECT count(*) FROM notes WHERE deleted_at IS NULL"), "note"));
        let r = row(&mut rows, (Group::Widget, "To-do"));
        r.note = format!("{}; kept", plural(count(&c, "SELECT count(*) FROM todos"), "task"));
    }
    drop(c);
    // Every database cleanup ends with VACUUM and a checkpoint: that frees the unused pages
    // and the write-ahead log too, so say so in its size.
    let extra = free + wal;
    for a in rows.values_mut().flat_map(|r| r.actions.iter_mut()).filter(|a| DB_KINDS.contains(&a.id.split(':').next().unwrap_or(""))) {
        a.bytes += extra;
        if extra >= 64 * 1024 { a.detail += &format!(" Then compacts the database, which frees about {} more.", mb(extra)); }
    }
    // Leftovers: one folder each, or dated groups.
    let cutoff = now_ms() - 7 * DAY;
    for r in rows.values_mut().filter(|r| r.group == Group::Leftover) {
        r.items.sort_by_key(|i| std::cmp::Reverse(i.modified));
        if DATED.contains(&r.name.as_str()) {
            let what = if r.name == "Backups" { "folder" } else { "file" };
            let m: Vec<i64> = r.items.iter().map(|i| i.modified).collect();
            for (id, label, idx) in [("old", "Delete older than 7 days", deletable(&m, cutoff)), ("allbut", "Delete all but the newest", deletable(&m, i64::MAX))] {
                if idx.is_empty() { continue; }
                let b: u64 = idx.iter().map(|&i| r.items[i].bytes).sum();
                r.actions.push(Action { id: format!("{id}:{}", r.name), label: label.into(), destructive: true, bytes: b,
                    detail: format!("Deletes {} ({}). The newest one, {}, is kept.", plural(idx.len() as i64, what), mb(b), r.items[0].name) });
            }
            if r.actions.len() == 2 && r.actions[0].bytes == r.actions[1].bytes { r.actions.pop(); }
        } else {
            let b = r.bytes;
            r.actions.push(Action { id: format!("entry:{}", r.name), label: "Delete folder".into(), destructive: true, bytes: b,
                detail: format!("Deletes the folder {} ({}). No current widget uses it.", r.name, mb(b)) });
        }
        r.note = match r.name.as_str() {
            "aiusage-web" => "From the removed AI Usage widget".into(),
            "Backups" => format!("{} made by updates and tests", plural(r.items.len() as i64, "folder")),
            _ => plural(r.items.len() as i64, "file"),
        };
    }
    let mut rows: Vec<Row> = rows.into_values().collect();
    rows.sort_by(|a, b| a.group.cmp(&b.group).then(b.bytes.cmp(&a.bytes)));
    let webview = webview_dir(app).filter(|d| d.is_dir()).map(|d| {
        let cache = WEBVIEW_CACHES.iter().map(|s| size_of(&d.join(s))).sum();
        (d.to_string_lossy().into_owned(), size_of(&d), cache)
    });
    Report { dir: dir.to_string_lossy().into_owned(), total, rows, webview }
}

#[tauri::command]
pub async fn storage_scan(app: AppHandle) -> Report { scan(&app, &crate::data_dir()) }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Cleaned { pub freed: u64, pub backup: Option<String>, pub report: Report }

/// Copy glassbar.db* into a new `backup-storage-<time>` folder. The caller holds the lock, so
/// after the checkpoint the files are complete and nothing writes to them during the copy.
fn backup_db(c: &Connection, dir: &Path) -> Result<PathBuf, String> {
    c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e| e.to_string())?;
    let to = dir.join(format!("backup-storage-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
    for f in ["glassbar.db", "glassbar.db-wal", "glassbar.db-shm"] {
        let p = dir.join(f);
        if p.exists() { std::fs::copy(&p, to.join(f)).map_err(|e| format!("backup failed: {e}"))?; }
    }
    Ok(to)
}

/// Runs a database cleanup: backup, the delete, then VACUUM. `files` lists the rows' own
/// files, deleted only when they sit in `files_dir` (the widget's folder).
fn db_clean(c: &Connection, dir: &Path, files_sql: Option<&str>, files_dir: &str, delete: &str, p: &[&dyn rusqlite::ToSql]) -> Result<PathBuf, String> {
    let bak = backup_db(c, dir)?;
    let files: Vec<String> = files_sql.map(|q| c.prepare(q).and_then(|mut s| s.query_map([], |r| r.get(0)).map(|r| r.filter_map(Result::ok).collect())).unwrap_or_default()).unwrap_or_default();
    c.execute(delete, p).map_err(|e| { crate::db::note(&e); e.to_string() })?;
    let root = dir.join(files_dir);
    for f in files { remove(&root, Path::new(&f)); }
    c.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e| e.to_string())?;
    Ok(bak)
}

/// Clears the WebView2 disk cache through WebView2 itself (it owns those files).
async fn clear_webview_cache(app: &AppHandle) -> Result<(), String> {
    use webview2_com::ClearBrowsingDataCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Profile2, ICoreWebView2_13, COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE};
    use windows::core::Interface;
    let win = app.get_webview_window("settings").or_else(|| app.webview_windows().into_values().next()).ok_or("no window")?;
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    win.with_webview(move |wv| unsafe {
        let r = (|| -> windows::core::Result<()> {
            let p: ICoreWebView2Profile2 = wv.controller().CoreWebView2()?.cast::<ICoreWebView2_13>()?.Profile()?.cast()?;
            let tx2 = tx.clone();
            p.ClearBrowsingData(COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, &ClearBrowsingDataCompletedHandler::create(Box::new(move |r| {
                let _ = tx2.send(r.map_err(|e| e.to_string())); Ok(())
            })))
        })();
        if let Err(e) = r { let _ = tx.send(Err(format!("this WebView2 can't clear its cache ({e})"))); }
    }).map_err(|e| e.to_string())?;
    // ponytail: blocks one async worker until WebView2 answers; fine for a button press.
    rx.recv_timeout(std::time::Duration::from_secs(60)).map_err(|_| "WebView2 did not answer".to_string())?
}

#[tauri::command]
pub async fn storage_clean(app: AppHandle, id: String) -> Result<Cleaned, String> {
    let dir = crate::data_dir();
    let before = size_of(&dir);
    let web_before = webview_dir(&app).map(|d| size_of(&d)).unwrap_or(0);
    let mut backup = None;
    let (kind, arg) = id.split_once(':').unwrap_or((id.as_str(), ""));
    match kind {
        "webview" => clear_webview_cache(&app).await?,
        "entry" | "old" | "allbut" => {
            // Recompute from disk rather than trust the page: only what scan() offers is deleted.
            let rep = scan(&app, &dir);
            let r = rep.rows.iter().find(|r| r.group == Group::Leftover && r.name == arg).ok_or("nothing to delete")?;
            let pick: Vec<&Item> = match kind {
                "entry" => r.items.iter().collect(),
                _ => {
                    let m: Vec<i64> = r.items.iter().map(|i| i.modified).collect();
                    deletable(&m, if kind == "old" { now_ms() - 7 * DAY } else { i64::MAX }).into_iter().map(|i| &r.items[i]).collect()
                }
            };
            for i in pick {
                let n = remove(&dir, &dir.join(&i.name));
                crate::applog::log(format!("storage: deleted {} ({})", i.name, human(n)));
            }
        }
        _ => {
            let db = app.state::<Db>();
            let c = db.lock();
            let week = (chrono::Local::now().date_naive() - chrono::Duration::days(7)).to_string();
            let month = now_ms() - 30 * DAY;
            let like = format!("{arg}%");
            let b = match kind {
                "compact" => { let b = backup_db(&c, &dir)?; c.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e| e.to_string())?; b }
                "clipboard" => db_clean(&c, &dir, Some("SELECT path FROM clipboard WHERE pinned = 0 AND path IS NOT NULL"), "clipboard", "DELETE FROM clipboard WHERE pinned = 0", &[])?,
                "screenshots" => db_clean(&c, &dir, Some("SELECT path FROM screenshots"), "screenshots", "DELETE FROM screenshots", &[])?,
                "ocr_history" => db_clean(&c, &dir, None, "", "DELETE FROM ocr_history", &[])?,
                "alerts" => db_clean(&c, &dir, None, "", "DELETE FROM alerts", &[])?,
                "screen_time" => db_clean(&c, &dir, None, "", "DELETE FROM screen_time WHERE day < ?1", &[&week])?,
                "water" => db_clean(&c, &dir, None, "", "DELETE FROM water WHERE at < ?1", &[&month])?,
                "kv" if KV_CACHES.iter().any(|(_, p)| *p == arg) => db_clean(&c, &dir, None, "", "DELETE FROM kv WHERE key LIKE ?1", &[&like])?,
                _ => return Err(format!("unknown cleanup {id}")),
            };
            backup = Some(b.to_string_lossy().into_owned());
            drop(c);
            for ev in ["clip:changed", "storage:cleaned"] { let _ = app.emit(ev, ()); }
        }
    }
    let after = size_of(&dir);
    let web_after = webview_dir(&app).map(|d| size_of(&d)).unwrap_or(0);
    let bak = backup.as_ref().map(|b| size_of(Path::new(b))).unwrap_or(0);
    let freed = (before + web_before + bak).saturating_sub(after + web_after);
    crate::applog::log(format!("storage: {id} freed {}{}", human(freed), backup.as_ref().map(|b| format!(" (database backed up to {b})")).unwrap_or_default()));
    Ok(Cleaned { freed, backup, report: scan(&app, &dir) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn every_table_has_an_owner() {
        let c = Connection::open_in_memory().unwrap();
        crate::db::migrate(&c).unwrap();
        let tables: Vec<String> = c.prepare("SELECT name FROM sqlite_master WHERE type = 'table'").unwrap()
            .query_map([], |r| r.get(0)).unwrap().filter_map(Result::ok).collect();
        assert!(tables.len() >= 11);
        for t in tables { assert_ne!(owner_of_table(&t), OTHER, "{t} has no owner"); }
        assert_eq!(owner_of_table("clipboard"), (Group::Widget, "Clipboard"));
        assert_eq!(owner_of_table("fired"), DATABASE);
        assert_eq!(owner_of_table("something_new"), OTHER);
    }

    #[test]
    fn kv_keys_go_to_their_widget() {
        for (k, o) in [("lyrics2:adele|hello", "Lyrics"), ("ipu", "Air quality"), ("ipu:band", "Air quality"), ("fx:MYR", "Currency"),
                       ("solat:week:SGR01", "Prayer times"), ("solat:active", "Prayer times"), ("weather", "Weather"),
                       ("water:goal:123", "Water"), ("water:snooze", "Water"), ("pomodoro", "Focus timer"), ("pomo:history", "Focus timer"),
                       ("stopwatch", "Focus timer"), ("tasbih", "Tasbih"), ("gmail:inbox:me@gmail.com", "Gmail")] {
            assert_eq!(owner_of_kv(k), (Group::Widget, o), "{k}");
        }
        assert_eq!(owner_of_kv("location").1, "Location");
        assert_eq!(owner_of_kv("whatsnew:seen"), DATABASE);
        assert_eq!(owner_of_kv("mystery"), OTHER);
        // Every "Clear cache" prefix belongs to the row that shows it.
        for (name, p) in KV_CACHES { assert_eq!(owner_of_kv(&format!("{p}:x")).1, *name); }
    }

    #[test]
    fn folders_and_files_go_to_their_owner() {
        let d = |n| owner_of_entry(n, true);
        let f = |n| owner_of_entry(n, false);
        assert_eq!(d("clipboard"), (Group::Widget, "Clipboard"));
        assert_eq!(d("screenshots"), (Group::Widget, "Screenshots"));
        assert_eq!(d("notes"), (Group::Widget, "Notes"));
        assert_eq!(d("lyrics"), (Group::Widget, "Lyrics"));
        assert_eq!(d("aiusage-web"), (Group::Leftover, "aiusage-web"));
        for b in ["backup-perf-swap-20261102-150123", "pre-shot-backup-20261102-101518", "incident-20261102-1502", "promo-backup-20261102", "backup-storage-20261102-221400"] {
            assert_eq!(d(b), (Group::Leftover, "Backups"), "{b}");
        }
        assert_eq!(d("my-stuff"), OTHER, "unknown folders are never offered for deletion");
        for n in ["glassbar.db", "glassbar.db-wal", "glassbar.db-shm"] { assert_eq!(f(n), DATABASE); }
        assert_eq!(f("glassbar.db.bak"), (Group::App, "Daily backup"));
        assert_eq!(f("glassbar.db.bak.old"), (Group::App, "Daily backup"));
        for n in ["config.json", "glassbar.log", "taskbar-original.txt"] { assert_eq!(f(n), (Group::App, "Log and settings"), "{n}"); }
        assert_eq!(f("glassbar.db.corrupt-1793346385650"), (Group::Leftover, "Old database copies"));
        assert_eq!(f("glassbar.db.corrupt-1793346608249-shm"), (Group::Leftover, "Old database copies"));
        assert_eq!(f("glassbar.db.before-restore-20261029-131829"), (Group::Leftover, "Old database copies"));
        assert_eq!(f("config.json.bak-before-equalizer"), (Group::Leftover, "Old settings copies"));
        assert_eq!(f("notes.txt"), OTHER);
    }

    #[test]
    fn the_newest_is_never_deletable() {
        let now = 100 * DAY;
        let m = [now - 10 * DAY, now - 9 * DAY, now - DAY];
        assert_eq!(deletable(&m, now - 7 * DAY), vec![0, 1]);
        assert_eq!(deletable(&m, i64::MAX), vec![0, 1]);
        // All old: still keeps the newest.
        assert_eq!(deletable(&[1, 3, 2], i64::MAX), vec![0, 2]);
        assert_eq!(deletable(&[5], i64::MAX), Vec::<usize>::new());
        assert_eq!(deletable(&[], i64::MAX), Vec::<usize>::new());
    }

    #[test]
    fn remove_stays_inside_the_root() {
        let base = std::env::temp_dir().join(format!("glassbar-storage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (root, outside) = (base.join("GlassBar"), base.join("Shelf"));
        std::fs::create_dir_all(root.join("clipboard")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("mine.txt"), "keep").unwrap();
        std::fs::write(root.join("clipboard/a.png"), "12345").unwrap();
        assert_eq!(remove(&root, &outside.join("mine.txt")), 0);
        assert_eq!(remove(&root, &root.join("clipboard/../../Shelf/mine.txt")), 0);
        assert_eq!(remove(&root, &root), 0, "never the data folder itself");
        assert!(outside.join("mine.txt").exists());
        assert_eq!(remove(&root, &root.join("clipboard/a.png")), 5);
        assert!(!root.join("clipboard/a.png").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn dbstat_is_available() {
        let c = Connection::open_in_memory().unwrap();
        crate::db::migrate(&c).unwrap();
        c.execute("INSERT INTO kv(key, value) VALUES ('lyrics2:a', ?1), ('fx:MYR', 'x')", [&"y".repeat(5000)]).unwrap();
        let t = table_bytes(&c);
        assert!(t.iter().any(|(n, b)| n == "kv" && *b > 0), "{t:?}");
        let kv = t.iter().find(|r| r.0 == "kv").unwrap().1;
        let split = kv_split(&c, kv);
        let lyr = split.iter().find(|(o, _)| o.1 == "Lyrics").unwrap().1;
        assert!(lyr > kv * 9 / 10, "lyrics holds nearly all of kv: {split:?}");
    }

    #[test]
    fn backup_then_vacuum_frees_space_and_keeps_a_copy() {
        let base = std::env::temp_dir().join(format!("glassbar-storage-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let db = crate::db::open(&base.join("glassbar.db")).unwrap();
        let c = db.lock();
        for i in 0..300 { c.execute("INSERT INTO kv(key, value) VALUES (?1, ?2)", params![format!("lyrics2:{i}"), "z".repeat(2000)]).unwrap(); }
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        let before = size_of(&base.join("glassbar.db"));
        let bak = db_clean(&c, &base, None, "", "DELETE FROM kv WHERE key LIKE ?1", &[&"lyrics2%"]).unwrap();
        let after = size_of(&base.join("glassbar.db")) + size_of(&base.join("glassbar.db-wal"));
        assert!(after < before / 4, "{before} -> {after}");
        let n: i64 = Connection::open(bak.join("glassbar.db")).unwrap().query_row("SELECT count(*) FROM kv", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 300, "the backup has the rows");
        drop(c);
        let _ = std::fs::remove_dir_all(&base);
    }
}
