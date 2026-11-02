//! SQLite storage for to-dos, notes, alerts, timers, screen time, caches.
//! Migrations only move forward; an existing file is never recreated.
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

/// The one connection, plus its path so it can be reopened.
pub struct Db { conn: Mutex<Connection>, path: PathBuf }

/// Set when a query said "database disk image is malformed" although the file was fine at
/// start: the connection's own view went bad (seen after two GlassBars overlapped). The next
/// `lock()` opens a fresh connection, which reads the file correctly again.
static REOPEN: AtomicBool = AtomicBool::new(false);

pub fn is_corrupt(e: &rusqlite::Error) -> bool {
    matches!(e.sqlite_error_code(), Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase))
}

/// Call with any error from this database: a corrupt one makes the next `lock()` reopen.
pub fn note(e: &rusqlite::Error) {
    static LOGGED: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    if !is_corrupt(e) { return; }
    REOPEN.store(true, Ordering::Relaxed);
    let now = now_ms();
    if now - LOGGED.swap(now, Ordering::Relaxed) > 60_000 { crate::applog::log(format!("db: {e}: reopening the connection")); }
}

impl Db {
    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        // A panic while locked (an unwrap on a bad query) must not break every later caller.
        let mut g = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if REOPEN.swap(false, Ordering::Relaxed) {
            match connect(&self.path) {
                Ok(c) => *g = c,
                Err(e) => crate::applog::log(format!("db: reopen failed: {e}")),
            }
        }
        g
    }

    /// A write the user is waiting on: on "malformed", reopen and try once more.
    pub fn write<T>(&self, f: impl Fn(&Connection) -> rusqlite::Result<T>) -> Result<T, String> {
        let r = f(&self.lock());
        let r = match r { Err(e) if is_corrupt(&e) => { note(&e); f(&self.lock()) } r => r };
        r.map_err(|e| { note(&e); e.to_string() })
    }
}

/// Schema steps; PRAGMA user_version = how many have run.
pub const MIGRATIONS: &[&str] = &[r"
CREATE TABLE todos(id INTEGER PRIMARY KEY, text TEXT NOT NULL, done INT NOT NULL DEFAULT 0, due_at INT, created_at INT NOT NULL);
CREATE TABLE notes(id INTEGER PRIMARY KEY, text TEXT NOT NULL DEFAULT '', color TEXT NOT NULL DEFAULT 'yellow',
  pinned INT NOT NULL DEFAULT 0, images TEXT NOT NULL DEFAULT '[]', updated_at INT NOT NULL, deleted_at INT);
CREATE TABLE alerts(id INTEGER PRIMARY KEY, source TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL,
  at INT NOT NULL, missed INT NOT NULL DEFAULT 0, read INT NOT NULL DEFAULT 0);
CREATE TABLE timers(id INTEGER PRIMARY KEY, label TEXT NOT NULL, duration_ms INT NOT NULL, ends_at INT, remaining_ms INT);
CREATE TABLE screen_time(day TEXT NOT NULL, exe TEXT NOT NULL, seconds INT NOT NULL, PRIMARY KEY(day, exe));
CREATE TABLE kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE fired(key TEXT PRIMARY KEY, at INT NOT NULL);
", r"
CREATE TABLE ocr_history(id INTEGER PRIMARY KEY, at INT NOT NULL, text TEXT NOT NULL, mode TEXT NOT NULL, engine TEXT NOT NULL);
", r"
CREATE TABLE screenshots(id INTEGER PRIMARY KEY, at INT NOT NULL, path TEXT NOT NULL, w INT NOT NULL, h INT NOT NULL, kind TEXT NOT NULL, thumb TEXT NOT NULL);
", r"
CREATE TABLE clipboard(id INTEGER PRIMARY KEY, at INT NOT NULL, kind TEXT NOT NULL, text TEXT, path TEXT, thumb TEXT, w INT, h INT, hash TEXT NOT NULL);
", r"
ALTER TABLE alerts ADD COLUMN data TEXT;
", r"
ALTER TABLE clipboard ADD COLUMN pinned INT NOT NULL DEFAULT 0;
", r"
CREATE TABLE water(at INT NOT NULL, ml INT NOT NULL);
CREATE INDEX water_at ON water(at);
"];

pub fn migrate(c: &Connection) -> rusqlite::Result<()> {
    let v: usize = c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(v) {
        c.execute_batch(&format!("BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;", i + 1))?;
    }
    Ok(())
}

fn connect(path: &Path) -> rusqlite::Result<Connection> {
    let c = Connection::open(path)?;
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    c.pragma_update(None, "journal_mode", "WAL")?;
    Ok(c)
}

/// What `quick_check` found wrong, or None when the file is fine.
fn problem(c: &Connection) -> Option<String> {
    match c.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)) {
        Ok(s) if s == "ok" => None,
        Ok(s) => Some(s),
        Err(e) => Some(e.to_string()),
    }
}

/// The file itself is damaged: move it (and its -wal/-shm) aside as `.corrupt-<ms>` and copy
/// every row that can still be read into a fresh file. Rows on damaged pages are lost.
fn salvage(path: &Path, why: &str) -> rusqlite::Result<Connection> {
    let bad = PathBuf::from(format!("{}.corrupt-{}", path.display(), now_ms()));
    // Still open elsewhere (another GlassBar): copying it now would only make an empty copy.
    if let Err(e) = std::fs::rename(path, &bad) {
        crate::applog::log(format!("db: check said \"{why}\" but the file could not be set aside ({e}); using it as it is"));
        return connect(path);
    }
    for ext in ["-wal", "-shm"] {
        let _ = std::fs::rename(format!("{}{ext}", path.display()), format!("{}{ext}", bad.display()));
    }
    let c = connect(path)?;
    migrate(&c)?;
    c.execute("ATTACH ?1 AS old", [bad.to_string_lossy()])?;
    let tables: Vec<String> = c.prepare("SELECT name FROM main.sqlite_master WHERE type = 'table'")?
        .query_map([], |r| r.get(0))?.filter_map(Result::ok).collect();
    for t in tables {
        // `WHERE 1` makes SQLite insert row by row. Without it, it copies the old index pages
        // as they are, broken ones included, and every start found the same damage again.
        // A broken index also let an app's day in twice: add those up rather than drop one.
        let merge = if t == "screen_time" { " ON CONFLICT(day, exe) DO UPDATE SET seconds = seconds + excluded.seconds" } else { "" };
        let or = if merge.is_empty() { "OR IGNORE " } else { "" };
        let _ = c.execute(&format!("INSERT {or}INTO main.\"{t}\" SELECT * FROM old.\"{t}\" WHERE 1{merge}"), []);
    }
    c.execute("DETACH old", [])?;
    crate::applog::log(format!("db: the file was damaged ({why}); kept a copy as {} and moved what could be read into a new one", bad.display()));
    Ok(c)
}

/// At most once a day, copy the healthy database to `<name>.bak`, keeping the one before as
/// `.bak.old`: a bad day can always be undone from yesterday's copy.
fn backup(c: &Connection, path: &Path) {
    let bak = PathBuf::from(format!("{}.bak", path.display()));
    let day_old = std::fs::metadata(&bak).and_then(|m| m.modified()).map(|t| t.elapsed().unwrap_or_default().as_secs() > 86_400).unwrap_or(true);
    if !day_old { return; }
    // Written under a name of its own first: VACUUM INTO refuses a file that exists, and two
    // GlassBars starting together used to trip over each other's `.bak`.
    let tmp = format!("{}.tmp-{}", bak.display(), std::process::id());
    if let Err(e) = c.execute("VACUUM INTO ?1", [&tmp]) { crate::applog::log(format!("db: backup failed: {e}")); return; }
    let _ = std::fs::rename(&bak, format!("{}.old", bak.display()));
    if let Err(e) = std::fs::rename(&tmp, &bak) { crate::applog::log(format!("db: backup failed: {e}")); }
}

pub fn open(path: &Path) -> rusqlite::Result<Db> {
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let c = connect(path)?;
    let c = match problem(&c) { None => c, Some(why) => { drop(c); salvage(path, &why)? } };
    migrate(&c)?;
    backup(&c, path);
    Ok(Db { conn: Mutex::new(c), path: path.to_path_buf() })
}

pub fn kv_get(c: &Connection, key: &str) -> Option<String> {
    c.query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0)).optional().map_err(|e| note(&e)).ok().flatten()
}

pub fn kv_set(c: &Connection, key: &str, value: &str) {
    if let Err(e) = c.execute("INSERT INTO kv(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value]) {
        note(&e);
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("glassbar-db-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("glassbar.db")
    }

    #[test]
    fn migrate_is_idempotent_and_keeps_rows() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c.execute("INSERT INTO todos(text, created_at) VALUES ('a', 1)", []).unwrap();
        migrate(&c).unwrap();
        let n: i64 = c.query_row("SELECT count(*) FROM todos", [], |r| r.get(0)).unwrap();
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!((n, v), (1, MIGRATIONS.len() as i64));
    }

    #[test]
    fn kv_round_trip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        assert_eq!(kv_get(&c, "x"), None);
        kv_set(&c, "x", "1");
        kv_set(&c, "x", "2");
        assert_eq!(kv_get(&c, "x").as_deref(), Some("2"));
    }

    #[test]
    fn open_does_not_truncate_existing_file() {
        let p = tmp("reopen");
        {
            let db = open(&p).unwrap();
            db.lock().execute("INSERT INTO todos(text, created_at) VALUES ('keep', 1)", []).unwrap();
        }
        let db = open(&p).unwrap();
        let t: String = db.lock().query_row("SELECT text FROM todos", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "keep");
    }

    #[test]
    fn a_damaged_file_is_set_aside_and_readable_rows_are_kept() {
        let p = tmp("salvage");
        {
            let db = open(&p).unwrap();
            let c = db.lock();
            c.execute("INSERT INTO todos(text, created_at) VALUES ('keep', 1)", []).unwrap();
            // Enough clipboard rows to fill pages past the to-do table.
            for i in 0..400 { c.execute("INSERT INTO clipboard(at, kind, text, hash) VALUES (?1, 'text', ?2, 'h')", params![i, "x".repeat(500)]).unwrap(); }
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        }
        // Scribble over the last pages (clipboard data), as a torn write would.
        let mut bytes = std::fs::read(&p).unwrap();
        let n = bytes.len();
        for b in &mut bytes[n - 3 * 4096..] { *b = 0xA5; }
        std::fs::write(&p, &bytes).unwrap();

        let db = open(&p).unwrap();
        let t: String = db.lock().query_row("SELECT text FROM todos", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "keep");
        assert_eq!(problem(&db.lock()), None);
        let aside = std::fs::read_dir(p.parent().unwrap()).unwrap().filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(aside, "the damaged file is kept");
    }

    #[test]
    fn a_broken_index_is_rebuilt_once_not_salvaged_on_every_start() {
        let p = tmp("index");
        { open(&p).unwrap().lock().execute("INSERT INTO water(at, ml) VALUES (1, 100)", []).unwrap(); }
        // Make water_at miss a row (what screen_time's index had): hide the index, add a row, put it back.
        {
            let c = Connection::open(&p).unwrap();
            let (root, sql): (i64, String) = c.query_row("SELECT rootpage, sql FROM sqlite_master WHERE name = 'water_at'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            c.execute_batch("PRAGMA writable_schema = ON; DELETE FROM sqlite_master WHERE name = 'water_at';").unwrap();
            drop(c);
            let c = Connection::open(&p).unwrap();
            c.execute("INSERT INTO water(at, ml) VALUES (2, 200)", []).unwrap();
            c.execute_batch("PRAGMA writable_schema = ON;").unwrap();
            c.execute("INSERT INTO sqlite_master VALUES ('index', 'water_at', 'water', ?1, ?2)", params![root, sql]).unwrap();
        }
        assert!(problem(&Connection::open(&p).unwrap()).is_some(), "the test really broke the index");
        let corrupt = || std::fs::read_dir(p.parent().unwrap()).unwrap().filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(|c: char| c.is_ascii_digit())).count();
        drop(open(&p).unwrap());
        let db = open(&p).unwrap();
        assert_eq!(problem(&db.lock()), None);
        let n: i64 = db.lock().query_row("SELECT count(*) FROM water", [], |r| r.get(0)).unwrap();
        assert_eq!((n, corrupt()), (2, 1), "rows kept, set aside once only");
    }

    #[test]
    fn open_keeps_a_daily_backup_with_the_rows() {
        let p = tmp("backup");
        { open(&p).unwrap().lock().execute("INSERT INTO todos(text, created_at) VALUES ('keep', 1)", []).unwrap(); }
        // The first open made a backup (no rows yet). Make it two days old: the next open replaces it.
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 86_400);
        std::fs::File::options().write(true).open(format!("{}.bak", p.display())).unwrap().set_modified(old).unwrap();
        drop(open(&p).unwrap());
        let b = Connection::open(format!("{}.bak", p.display())).unwrap();
        let t: String = b.query_row("SELECT text FROM todos", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "keep");
        assert!(std::path::Path::new(&format!("{}.bak.old", p.display())).exists(), "the previous backup is kept");
    }

    #[test]
    fn write_reopens_and_retries_once_on_malformed() {
        let db = open(&tmp("retry")).unwrap();
        let tries = std::cell::Cell::new(0);
        let r = db.write(|c| {
            tries.set(tries.get() + 1);
            if tries.get() == 1 { return Err(rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT), None)); }
            c.execute("INSERT INTO todos(text, created_at) VALUES ('ok', 1)", [])
        });
        assert_eq!((r, tries.get()), (Ok(1), 2));
        assert!(!REOPEN.load(Ordering::Relaxed), "the retry's lock() reopened");
    }
}
