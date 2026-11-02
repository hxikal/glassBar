//! Boost: gives back memory that idle apps hold but aren't using, without closing anything.
//! It empties the working sets of idle background processes (the pages move to standby or
//! the compression store and come back on demand), trims GlassBar's own processes and caches,
//! and on request lowers runaway background CPU hogs to Below normal until GlassBar restarts.
//! It never terminates, suspends or force-closes a process. Runs only when asked.
//! "Close apps I'm not using" is separate: a list the user ticks, closed with WM_CLOSE.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TokenSessionId, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::ProcessStatus::{K32EmptyWorkingSet, K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{
    GetPriorityClass, GetProcessTimes, OpenProcess, OpenProcessToken, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS,
    NORMAL_PRIORITY_CLASS, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

/// Used within this long = "in use": Boost leaves it alone and it is not offered for closing.
pub const RECENT_MS: i64 = 10 * 60_000;
/// A process using this much of the whole CPU is working, not idle: trimming it would only
/// make it fault its pages back in.
const BUSY_PCT: f32 = 1.0;
/// A background process above this is a "CPU hog" (the optional priority step).
const HOG_PCT: f32 = 10.0;

/// Windows' own processes. Most are also services or protected; listed so a quirk of one
/// PC (an unprotected csrss, a readable token) can't let one through.
const SYSTEM: &[&str] = &["system", "registry", "smss", "csrss", "wininit", "winlogon", "services", "lsass", "lsaiso", "dwm",
    "fontdrvhost", "memory compression", "secure system", "svchost", "audiodg", "explorer", "sihost", "ctfmon", "taskhostw",
    "shellexperiencehost", "startmenuexperiencehost", "searchhost", "textinputhost", "lockapp", "logonui", "userinit", "conhost",
    "dllhost", "wudfhost", "spoolsv", "msedgewebview2"];

/// Anti-virus and anti-cheat, by part of the exe name. Touching these can trip their
/// self-protection or a game's cheat detection.
const SECURITY: &[&str] = &["msmpeng", "mpdefender", "nissrv", "securityhealth", "smartscreen", "antimalware", "defender",
    "anticheat", "easyanticheat", "battleye", "beservice", "bedaisy", "vgc", "vgtray", "vanguard", "faceit", "eac", "punkbuster",
    "avast", "avg", "avp", "kaspersky", "bdagent", "bdservice", "bitdefender", "ekrn", "egui", "eset", "norton", "mcafee",
    "mcshield", "sophos", "falcon", "sentinel", "malwarebytes", "mbam", "webroot", "trend", "avira", "comodo", "zonealarm", "cylance"];

/// Why a process is left alone. The order of the checks is the order here.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Skip { System, Service, Protected, Security, Elevated, NeverTouch, Foreground, Fullscreen, Audio, Recent, Busy }

/// What Boost knows about one process.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub pid: u32,
    /// Exe name without ".exe", lowercase ("chrome").
    pub stem: String,
    /// None: its token couldn't be read (a protected or other user's process).
    pub session: Option<u32>,
    pub elevated: Option<bool>,
    /// Windows let us open it to trim (PROCESS_SET_QUOTA).
    pub can_trim: bool,
    /// % of the whole CPU over the measuring second.
    pub cpu: f32,
}

/// What's going on on the PC, by exe stem: the app in front, full-screen apps, apps with
/// sound or a media session, apps used in the last 10 minutes, and the "never touch" list.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub me_elevated: bool,
    pub foreground: HashSet<String>,
    pub fullscreen: HashSet<String>,
    pub audio: HashSet<String>,
    pub recent: HashSet<String>,
    pub never: HashSet<String>,
}

/// "Spotify.exe", "C:\x\Spotify.exe", " spotify " -> "spotify".
pub fn stem(name: &str) -> String {
    let n = name.trim().rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
    n.strip_suffix(".exe").unwrap_or(&n).to_string()
}

/// Why Boost must not touch this process, or None when it may be trimmed.
pub fn skip(p: &Facts, s: &Scene) -> Option<Skip> {
    if p.pid <= 4 || SYSTEM.contains(&p.stem.as_str()) { return Some(Skip::System); }
    match p.session { None => return Some(Skip::Protected), Some(0) => return Some(Skip::Service), _ => {} }
    if SECURITY.iter().any(|x| p.stem.contains(x)) { return Some(Skip::Security); }
    if p.elevated != Some(false) && !s.me_elevated { return Some(Skip::Elevated); }
    if !p.can_trim { return Some(Skip::Protected); }
    if s.never.contains(&p.stem) { return Some(Skip::NeverTouch); }
    if s.foreground.contains(&p.stem) { return Some(Skip::Foreground); }
    if s.fullscreen.contains(&p.stem) { return Some(Skip::Fullscreen); }
    if s.audio.contains(&p.stem) { return Some(Skip::Audio); }
    if s.recent.contains(&p.stem) { return Some(Skip::Recent); }
    if p.cpu >= BUSY_PCT { return Some(Skip::Busy); }
    None
}

/// A background CPU hog that may be lowered: busy, and nothing else protects it.
pub fn hog(p: &Facts, s: &Scene) -> bool { skip(p, s) == Some(Skip::Busy) && p.cpu >= HOG_PCT }

/// Media session ids ("Spotify.exe", "Chrome", "MSEdge", "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify")
/// to a stem to match processes against.
pub fn media_stem(id: &str) -> String {
    let base = id.rsplit('!').next().unwrap_or(id);
    stem(base)
}

// ---- Who was used recently: the perf sampler notes the foreground app every few seconds. ----

static SEEN: Mutex<Option<(i64, HashMap<String, i64>)>> = Mutex::new(None);

/// Called by the perf sampler (every 1–5 s): remember when each app was last in front.
pub fn note_foreground() {
    let Some(exe) = crate::platform::shell().foreground_exe() else { return };
    let now = crate::db::now_ms();
    let mut g = SEEN.lock().unwrap();
    g.get_or_insert_with(|| (now, HashMap::new())).1.insert(stem(&exe), now);
}

/// Stems used within RECENT_MS. Until GlassBar has watched for that long it can't know, so
/// every app with a window counts as used.
fn recent(now: i64, with_windows: &HashSet<String>) -> HashSet<String> {
    let g = SEEN.lock().unwrap();
    let Some((since, seen)) = g.as_ref() else { return with_windows.clone() };
    let mut out: HashSet<String> = seen.iter().filter(|(_, &t)| now - t < RECENT_MS).map(|(k, _)| k.clone()).collect();
    if now - since < RECENT_MS { out.extend(with_windows.iter().cloned()); }
    out
}

// ---- Win32 ----

struct Handle(HANDLE);
impl Drop for Handle { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } }

fn open(pid: u32, rights: PROCESS_ACCESS_RIGHTS) -> Option<Handle> {
    unsafe { OpenProcess(rights, false, pid).ok().map(Handle) }
}

/// (session id, elevated) from the process token; None when it can't be read.
fn token_facts(pid: u32) -> (Option<u32>, Option<bool>) {
    let Some(h) = open(pid, PROCESS_QUERY_LIMITED_INFORMATION) else { return (None, None) };
    unsafe {
        let mut tok = HANDLE::default();
        if OpenProcessToken(h.0, TOKEN_QUERY, &mut tok).is_err() { return (None, None); }
        let tok = Handle(tok);
        let mut len = 0u32;
        let mut session = 0u32;
        let s = GetTokenInformation(tok.0, TokenSessionId, Some(&mut session as *mut _ as *mut _), 4, &mut len).is_ok();
        let mut el = TOKEN_ELEVATION::default();
        let e = GetTokenInformation(tok.0, TokenElevation, Some(&mut el as *mut _ as *mut _), size_of::<TOKEN_ELEVATION>() as u32, &mut len).is_ok();
        (s.then_some(session), e.then_some(el.TokenIsElevated != 0))
    }
}

fn working_set(h: &Handle) -> u64 {
    let mut m = PROCESS_MEMORY_COUNTERS::default();
    unsafe { if K32GetProcessMemoryInfo(h.0, &mut m, size_of::<PROCESS_MEMORY_COUNTERS>() as u32).as_bool() { m.WorkingSetSize as u64 } else { 0 } }
}

/// Empty a process's working set; returns the bytes its working set shrank by.
fn trim(pid: u32) -> u64 {
    let Some(h) = open(pid, PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION) else { return 0 };
    let before = working_set(&h);
    if !unsafe { K32EmptyWorkingSet(h.0) }.as_bool() { return 0; }
    before.saturating_sub(working_set(&h))
}

/// (RAM in use, total RAM), bytes: what Task Manager's Memory graph shows.
pub fn ram_in_use() -> (u64, u64) {
    let mut m = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { if GlobalMemoryStatusEx(&mut m).is_err() { return (0, 0); } }
    (m.ullTotalPhys - m.ullAvailPhys, m.ullTotalPhys)
}

fn created(h: &Handle) -> u64 {
    let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { let _ = GetProcessTimes(h.0, &mut c, &mut x, &mut k, &mut u); }
    ((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64
}

/// A visible window covers its whole monitor (a full-screen game or video).
fn covers_monitor(id: isize) -> bool {
    unsafe {
        let w = HWND(id as *mut _);
        let mut r = RECT::default();
        if GetWindowRect(w, &mut r).is_err() { return false; }
        let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(w, MONITOR_DEFAULTTONEAREST), &mut mi).as_bool() { return false; }
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

fn scene(app: &AppHandle) -> (Scene, Vec<crate::platform::AppWindow>) {
    let shell = crate::platform::shell();
    let windows = shell.list_windows();
    let mut s = Scene { me_elevated: token_facts(std::process::id()).1 == Some(true), ..Default::default() };
    let with_windows: HashSet<String> = windows.iter().map(|w| stem(&w.exe)).collect();
    for w in &windows {
        if w.active { s.foreground.insert(stem(&w.exe)); }
        if !w.minimized && covers_monitor(w.id) { s.fullscreen.insert(stem(&w.exe)); }
    }
    if let Some(e) = shell.foreground_exe() { s.foreground.insert(stem(&e)); }
    for a in crate::platform::audio().sessions().unwrap_or_default() {
        if a.pid != 0 { s.audio.insert(stem(&a.exe)); }
    }
    if let Some(id) = crate::platform::media().source_id() { s.audio.insert(media_stem(&id)); }
    s.recent = recent(crate::db::now_ms(), &with_windows);
    s.never = app.state::<crate::AppState>().config.lock().unwrap().performance.never_touch.iter().map(|n| stem(n)).filter(|n| !n.is_empty()).collect();
    s.foreground.remove("glassbar");
    (s, windows)
}

/// Every process with what Boost needs to judge it; CPU measured over one second.
fn facts() -> (Vec<Facts>, sysinfo::System) {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    let mut sys = sysinfo::System::new();
    let kind = ProcessRefreshKind::nothing().with_cpu();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    std::thread::sleep(Duration::from_millis(1000));
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    let cores = sys.cpus().len().max(1) as f32;
    let cores = if cores <= 1.0 { std::thread::available_parallelism().map(|n| n.get() as f32).unwrap_or(1.0) } else { cores };
    let out = sys.processes().iter().map(|(pid, p)| {
        let pid = pid.as_u32();
        let (session, elevated) = token_facts(pid);
        Facts { pid, stem: stem(&p.name().to_string_lossy()), session, elevated,
                can_trim: open(pid, PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION).is_some(), cpu: p.cpu_usage() / cores }
    }).collect();
    (out, sys)
}

fn ours(sys: &sysinfo::System) -> Vec<u32> {
    let procs: Vec<(u32, u32, bool)> = sys.processes().iter()
        .filter_map(|(pid, p)| Some((pid.as_u32(), p.parent()?.as_u32(), p.name().eq_ignore_ascii_case("msedgewebview2.exe")))).collect();
    super::usage::ours(std::process::id(), &procs)
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Trimmed { pub name: String, pub bytes: u64 }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BoostReport {
    pub at: i64,
    /// RAM in use (bytes) right before, and two seconds after.
    pub before: u64, pub after: u64, pub total: u64,
    /// Processes trimmed, and how much their working sets shrank in all.
    pub trimmed: usize, pub trimmed_bytes: u64,
    /// The five that gave back the most.
    pub top: Vec<Trimmed>,
    /// GlassBar's own processes (it and its WebView2 helpers).
    pub own_bytes: u64,
    /// Apps lowered to Below normal.
    pub lowered: Vec<String>,
    /// How many processes each rule left alone.
    pub skipped: Vec<(Skip, usize)>,
}

const LOWERED_KEY: &str = "boost:lowered";

/// One process Boost lowered: enough to find exactly that one again (pid + creation time).
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Lowered { pid: u32, created: u64, name: String }

fn lower(f: &Facts) -> Option<Lowered> {
    let h = open(f.pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION)?;
    unsafe {
        if GetPriorityClass(h.0) != NORMAL_PRIORITY_CLASS.0 { return None; } // already changed by someone
        SetPriorityClass(h.0, BELOW_NORMAL_PRIORITY_CLASS).ok()?;
    }
    Some(Lowered { pid: f.pid, created: created(&h), name: f.stem.clone() })
}

/// Put back what an earlier Boost lowered: at start-up and when GlassBar quits.
pub fn restore(app: &AppHandle) {
    let db = app.state::<crate::db::Db>();
    let c = db.lock();
    let Some(list) = crate::db::kv_get(&c, LOWERED_KEY).and_then(|s| serde_json::from_str::<Vec<Lowered>>(&s).ok()) else { return };
    for l in &list {
        let Some(h) = open(l.pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION) else { continue };
        // Same pid but a newer process: not ours to change.
        if created(&h) != l.created { continue; }
        unsafe {
            if GetPriorityClass(h.0) == BELOW_NORMAL_PRIORITY_CLASS.0 && SetPriorityClass(h.0, NORMAL_PRIORITY_CLASS).is_ok() {
                crate::applog::log(format!("boost: {} (pid {}) back to Normal priority", l.name, l.pid));
            }
        }
    }
    let _ = c.execute("DELETE FROM kv WHERE key = ?1", [LOWERED_KEY]);
}

pub fn run(app: &AppHandle, lower_hogs: bool) -> BoostReport {
    let (before, total) = ram_in_use();
    let (s, _) = scene(app);
    let (all, sys) = facts();
    let own: HashSet<u32> = ours(&sys).into_iter().collect();
    let mut skipped: HashMap<Skip, usize> = HashMap::new();
    let mut trimmed: Vec<Trimmed> = Vec::new();
    let mut lowered: Vec<Lowered> = Vec::new();
    for f in all.iter().filter(|f| !own.contains(&f.pid)) {
        if lower_hogs && hog(f, &s) { if let Some(l) = lower(f) { lowered.push(l); } }
        match skip(f, &s) {
            Some(k) => *skipped.entry(k).or_default() += 1,
            None => { let b = trim(f.pid); if b > 0 { trimmed.push(Trimmed { name: f.stem.clone(), bytes: b }); } }
        }
    }
    // GlassBar itself: drop icon caches, then trim it and its WebView2 helpers.
    crate::platform::shell().free_caches();
    crate::widgets::downloads::free_icons();
    let _ = app.emit("boost:free", ());
    let own_bytes: u64 = own.iter().map(|&p| trim(p)).sum();
    if !lowered.is_empty() {
        let db = app.state::<crate::db::Db>();
        let c = db.lock();
        let mut list: Vec<Lowered> = crate::db::kv_get(&c, LOWERED_KEY).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        list.extend(lowered.iter().cloned());
        crate::db::kv_set(&c, LOWERED_KEY, &serde_json::to_string(&list).unwrap_or_default());
    }
    // Let Windows move the trimmed pages off "in use" before measuring again.
    std::thread::sleep(Duration::from_millis(2000));
    let (after, _) = ram_in_use();
    // Same app, several processes (browsers): one line each.
    let mut by: HashMap<String, u64> = HashMap::new();
    for t in &trimmed { *by.entry(t.name.clone()).or_default() += t.bytes; }
    let mut top: Vec<Trimmed> = by.into_iter().map(|(name, bytes)| Trimmed { name, bytes }).collect();
    top.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    top.truncate(5);
    let mut skipped: Vec<(Skip, usize)> = skipped.into_iter().collect();
    skipped.sort_by(|a, b| b.1.cmp(&a.1));
    let r = BoostReport { at: crate::db::now_ms(), before, after, total, trimmed: trimmed.len(), trimmed_bytes: trimmed.iter().map(|t| t.bytes).sum(),
        top, own_bytes, lowered: lowered.iter().map(|l| l.name.clone()).collect(), skipped };
    let h = crate::storage::human;
    crate::applog::log(format!("boost: RAM in use {} -> {}; trimmed {} processes ({}), GlassBar {}; top: {}; lowered: [{}]; left alone: {:?}",
        h(r.before), h(r.after), r.trimmed, h(r.trimmed_bytes), h(r.own_bytes),
        r.top.iter().map(|t| format!("{} {}", t.name, h(t.bytes))).collect::<Vec<_>>().join(", "), r.lowered.join(", "), r.skipped));
    r
}

#[tauri::command]
pub async fn boost_run(app: AppHandle, lower_hogs: bool) -> Result<BoostReport, String> {
    tauri::async_runtime::spawn_blocking(move || run(&app, lower_hogs)).await.map_err(|e| e.to_string())
}

// ---- "Close apps I'm not using": a list to pick from, never part of Boost. ----

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdleApp { pub exe: String, pub name: String, pub ram: u64, pub windows: usize }

/// Heavy apps with windows that the user hasn't touched in 10 minutes, heaviest first.
/// Same skip rules as Boost, minus "busy" (a busy app can still be one you're not using).
pub fn idle_apps(app: &AppHandle) -> Vec<IdleApp> {
    let (s, windows) = scene(app);
    let (all, _) = facts();
    let mut by: HashMap<String, IdleApp> = HashMap::new();
    for w in &windows {
        let st = stem(&w.exe);
        let e = by.entry(st.clone()).or_insert_with(|| IdleApp { exe: w.exe.clone(), name: String::new(), ram: 0, windows: 0 });
        e.windows += 1;
        if e.name.is_empty() { e.name = st; }
    }
    for f in &all {
        let Some(a) = by.get_mut(&f.stem) else { continue };
        if !matches!(skip(f, &s), None | Some(Skip::Busy)) { a.ram = u64::MAX; continue; } // ruled out
        if a.ram != u64::MAX { a.ram += crate::platform::gpu().private_ws(f.pid).unwrap_or(0); }
    }
    let mut out: Vec<IdleApp> = by.into_values().filter(|a| a.ram != u64::MAX && a.ram >= 50 << 20 && a.name != "glassbar").collect();
    out.sort_by(|a, b| b.ram.cmp(&a.ram));
    out.truncate(10);
    out
}

#[tauri::command]
pub async fn boost_idle_apps(app: AppHandle) -> Result<Vec<IdleApp>, String> {
    tauri::async_runtime::spawn_blocking(move || idle_apps(&app)).await.map_err(|e| e.to_string())
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Closed { pub closed: Vec<String>, pub still_open: Vec<String> }

/// Ask the picked apps to close (WM_CLOSE: they may ask to save). An app that stays open is
/// reported and left alone: never forced.
pub fn close(exes: &[String]) -> Closed {
    let shell = crate::platform::shell();
    let want: HashSet<String> = exes.iter().map(|e| e.to_lowercase()).collect();
    let mut asked: Vec<String> = Vec::new();
    for w in shell.list_windows().iter().filter(|w| want.contains(&w.exe.to_lowercase())) {
        shell.close(w.id);
        if !asked.iter().any(|a| a.eq_ignore_ascii_case(&w.exe)) { asked.push(w.exe.clone()); }
    }
    // Only apps that had a window to ask count: one not found was neither asked nor closed.
    if !asked.is_empty() { std::thread::sleep(Duration::from_secs(5)); }
    let open: HashSet<String> = shell.list_windows().iter().map(|w| w.exe.to_lowercase()).collect();
    let (still, closed): (Vec<String>, Vec<String>) = asked.into_iter().partition(|e| open.contains(&e.to_lowercase()));
    crate::applog::log(format!("boost: asked to close {:?}; closed {:?}; still open (left alone) {:?}",
        exes.iter().map(|e| stem(e)).collect::<Vec<_>>(), closed.iter().map(|e| stem(e)).collect::<Vec<_>>(), still.iter().map(|e| stem(e)).collect::<Vec<_>>()));
    Closed { closed, still_open: still }
}

#[tauri::command]
pub async fn boost_close(exes: Vec<String>) -> Result<Closed, String> {
    tauri::async_runtime::spawn_blocking(move || close(&exes)).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(stem: &str) -> Facts { Facts { pid: 1000, stem: stem.into(), session: Some(1), elevated: Some(false), can_trim: true, cpu: 0.0 } }
    fn set(a: &[&str]) -> HashSet<String> { a.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn an_idle_background_app_is_trimmed() {
        assert_eq!(skip(&p("discord"), &Scene::default()), None);
    }

    #[test]
    fn windows_itself_is_never_touched() {
        let s = Scene::default();
        for n in ["csrss", "dwm", "explorer", "lsass", "svchost", "audiodg", "msedgewebview2"] { assert_eq!(skip(&p(n), &s), Some(Skip::System), "{n}"); }
        assert_eq!(skip(&Facts { pid: 4, ..p("whatever") }, &s), Some(Skip::System));
        assert_eq!(skip(&Facts { session: Some(0), ..p("someservice") }, &s), Some(Skip::Service));
        assert_eq!(skip(&Facts { session: None, ..p("unreadable") }, &s), Some(Skip::Protected));
        assert_eq!(skip(&Facts { can_trim: false, ..p("ppl") }, &s), Some(Skip::Protected));
    }

    #[test]
    fn security_and_anti_cheat_are_left_alone() {
        for n in ["msmpeng", "easyanticheat_eos", "beservice", "vgc", "avastui", "mbamtray", "securityhealthsystray"] {
            assert_eq!(skip(&p(n), &Scene::default()), Some(Skip::Security), "{n}");
        }
    }

    #[test]
    fn elevated_apps_only_when_glassbar_is_elevated_too() {
        let admin = Facts { elevated: Some(true), ..p("regedit") };
        assert_eq!(skip(&admin, &Scene::default()), Some(Skip::Elevated));
        assert_eq!(skip(&Facts { elevated: None, ..p("odd") }, &Scene::default()), Some(Skip::Elevated));
        assert_eq!(skip(&admin, &Scene { me_elevated: true, ..Default::default() }), None);
    }

    #[test]
    fn apps_in_use_are_left_alone() {
        let s = Scene { foreground: set(&["code"]), fullscreen: set(&["eldenring"]), audio: set(&["spotify"]),
                        recent: set(&["notepad"]), never: set(&["obs64"]), ..Default::default() };
        assert_eq!(skip(&p("code"), &s), Some(Skip::Foreground));
        assert_eq!(skip(&p("eldenring"), &s), Some(Skip::Fullscreen));
        assert_eq!(skip(&p("spotify"), &s), Some(Skip::Audio));
        assert_eq!(skip(&p("notepad"), &s), Some(Skip::Recent));
        assert_eq!(skip(&p("obs64"), &s), Some(Skip::NeverTouch));
        assert_eq!(skip(&Facts { cpu: 3.0, ..p("compiler") }, &s), Some(Skip::Busy));
    }

    #[test]
    fn only_busy_unprotected_background_apps_are_hogs() {
        let s = Scene { audio: set(&["spotify"]), ..Default::default() };
        assert!(hog(&Facts { cpu: 40.0, ..p("miner") }, &s));
        assert!(!hog(&Facts { cpu: 5.0, ..p("miner") }, &s), "busy but not a hog");
        assert!(!hog(&Facts { cpu: 40.0, ..p("spotify") }, &s), "playing audio");
        assert!(!hog(&Facts { cpu: 40.0, ..p("svchost") }, &s), "Windows");
    }

    #[test]
    fn names_match_however_they_are_written() {
        assert_eq!(stem("C:\\Program Files\\Spotify\\Spotify.exe"), "spotify");
        assert_eq!(stem(" Notepad.EXE "), "notepad");
        assert_eq!(stem("obs64"), "obs64");
        assert_eq!(media_stem("Spotify.exe"), "spotify");
        assert_eq!(media_stem("MSEdge"), "msedge");
        assert_eq!(media_stem("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "spotify");
    }

    #[test]
    fn until_ten_minutes_are_watched_every_app_with_a_window_counts_as_used() {
        let w = set(&["word"]);
        *SEEN.lock().unwrap() = None;
        assert_eq!(recent(1_000, &w), w);
        *SEEN.lock().unwrap() = Some((0, [("code".to_string(), 0)].into()));
        assert_eq!(recent(60_000, &w), set(&["code", "word"]));
        assert_eq!(recent(RECENT_MS + 5_000, &w), set(&[]), "code was last used over 10 minutes ago");
        *SEEN.lock().unwrap() = None;
    }
}
