//! Settings › Performance: what GlassBar itself costs, and how light it runs.
//! A sampler measures GlassBar's processes every 5 s (1 s while the page is open), keeps an hour
//! of samples, and decides the effective mode: the manual choice, or Auto's pick.
pub mod auto;
pub mod boost;
pub mod suspend;
pub mod usage;

use crate::config::{PerfMode, Performance};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const HISTORY_MS: i64 = 3_600_000;
/// The page renews this lease every 2 s; once it lapses (page or window closed) sampling slows.
const LEASE_MS: i64 = 5_000;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PerfSample {
    pub at: i64, pub ram_mb: f32, pub cpu: f32, pub gpu: Option<f32>, pub net_bytes: u64,
    /// Process by process; in the history only the newest sample keeps it.
    pub procs: Vec<usage::ProcUsage>,
}

/// The mode in force. `reason` (Auto's why, in words) only while Auto is on.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PerfState { pub mode: PerfMode, pub auto: bool, pub reason: Option<String> }

static HIST: Mutex<VecDeque<PerfSample>> = Mutex::new(VecDeque::new());
static STATE: Mutex<PerfState> = Mutex::new(PerfState { mode: PerfMode::Normal, auto: false, reason: None });
static WATCHED: AtomicI64 = AtomicI64::new(0);
/// Auto's state and latest pick, while Auto is on. Cleared when it's turned off, so turning it
/// back on starts fresh from Normal.
static AUTO: Mutex<Option<(auto::Auto, (PerfMode, String))>> = Mutex::new(None);

pub fn period_ms(now_ms: i64, watched_at_ms: i64) -> u64 { if now_ms - watched_at_ms < LEASE_MS { 1000 } else { 5000 } }

pub fn trim(h: &mut VecDeque<PerfSample>, now_ms: i64) {
    while h.front().is_some_and(|s| s.at < now_ms - HISTORY_MS) { h.pop_front(); }
}

/// Add a sample; the one before it drops its process list (an hour of those is dead weight).
pub fn push(h: &mut VecDeque<PerfSample>, s: PerfSample) {
    if let Some(prev) = h.back_mut() { prev.procs.clear(); }
    h.push_back(s);
}

/// The mode Rust-side work should follow (sysmon period, freeing hidden windows).
pub fn effective() -> PerfMode { STATE.lock().unwrap().mode }

fn decide(p: &Performance, pick: Option<(PerfMode, String)>) -> PerfState {
    if !p.auto { return PerfState { mode: p.mode, auto: false, reason: None }; }
    let (mode, reason) = pick.unwrap_or((PerfMode::Normal, "your PC has room".into()));
    PerfState { mode, auto: true, reason: Some(reason) }
}

/// Recompute from config now (a manual pick applies at once) and tell every window if it changed.
pub fn refresh(app: &AppHandle) {
    let p = app.state::<crate::AppState>().config.lock().unwrap().performance.clone();
    let pick = {
        let mut a = AUTO.lock().unwrap();
        if !p.auto { *a = None; }
        a.as_ref().map(|(_, pick)| pick.clone())
    };
    let next = decide(&p, pick);
    let changed = {
        let mut cur = STATE.lock().unwrap();
        let changed = *cur != next;
        *cur = next;
        changed
    };
    if changed { let _ = app.emit("perf:changed", ()); }
}

pub fn start(app: AppHandle) {
    refresh(&app);
    boost::restore(&app); // a Boost before a crash or power cut may have left apps lowered
    std::thread::spawn(move || {
        let mut meter = usage::Meter::new();
        let mut last = Instant::now();
        loop {
            boost::note_foreground();
            let snap = meter.sample();
            let now = crate::db::now_ms();
            let s = PerfSample { at: now, ram_mb: snap.ram_mb, cpu: snap.cpu, gpu: snap.gpu,
                                 net_bytes: crate::net::bytes_received(), procs: snap.procs };
            { let mut h = HIST.lock().unwrap(); push(&mut h, s.clone()); trim(&mut h, now); }
            let _ = app.emit_to("settings", "perf:sample", &s);
            let dt = last.elapsed().as_millis().min(u32::MAX as u128) as u32;
            last = Instant::now();
            if app.state::<crate::AppState>().config.lock().unwrap().performance.auto {
                let load = auto::Load { mem_pct: snap.sys_mem_pct, cpu_pct: snap.sys_cpu_pct,
                                        saver: crate::platform::power().saver_on() == Some(true) };
                let mut a = AUTO.lock().unwrap();
                let (state, pick) = a.get_or_insert_with(|| (auto::Auto::default(), (PerfMode::Normal, "your PC has room".into())));
                *pick = state.step(&load, dt);
            }
            refresh(&app);
            std::thread::sleep(Duration::from_millis(period_ms(crate::db::now_ms(), WATCHED.load(Ordering::Relaxed))));
        }
    });
}

#[tauri::command]
pub fn perf_history() -> Vec<PerfSample> { HIST.lock().unwrap().iter().cloned().collect() }

#[tauri::command]
pub fn perf_state() -> PerfState { STATE.lock().unwrap().clone() }

/// The Performance page is open: sample every second for the next few seconds.
#[tauri::command]
pub fn perf_watch() { WATCHED.store(crate::db::now_ms(), Ordering::Relaxed); }

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test] fn fast_only_while_the_page_renews_its_lease() {
        assert_eq!(period_ms(10_000, 6_000), 1000);
        assert_eq!(period_ms(10_000, 5_000), 5000);  // lease expired: window closed without unmounting
        assert_eq!(period_ms(10_000, 0), 5000);      // never opened
    }

    #[test] fn history_keeps_one_hour() {
        let s = |at| PerfSample { at, ram_mb: 0.0, cpu: 0.0, gpu: None, net_bytes: 0, procs: vec![] };
        let mut h: VecDeque<_> = [s(0), s(1_000), s(3_600_500)].into();
        trim(&mut h, 3_601_000);
        assert_eq!(h.iter().map(|x| x.at).collect::<Vec<_>>(), vec![1_000, 3_600_500]);
    }

    #[test] fn only_the_newest_sample_keeps_its_processes() {
        let p = usage::ProcUsage { pid: 1, name: "GlassBar".into(), ram_mb: 1.0, cpu: 0.0, gpu: None };
        let s = |at| PerfSample { at, ram_mb: 1.0, cpu: 0.0, gpu: None, net_bytes: 0, procs: vec![p.clone()] };
        let mut h = VecDeque::new();
        push(&mut h, s(1));
        push(&mut h, s(2));
        assert_eq!(h.iter().map(|x| x.procs.len()).collect::<Vec<_>>(), vec![0, 1]);
    }
}
