//! Minimize / restore effects. Windows can't bend another app's window, so GlassBar draws
//! live DWM thumbnails of it (one per strip for Genie/Suck) on a transparent, click-through
//! overlay and moves them every frame. Checked on Windows 11 26200: thumbnails draw on a
//! WS_EX_NOREDIRECTIONBITMAP overlay; clicks pass through it; at EVENT_SYSTEM_MINIMIZESTART
//! the window is already iconic, so its old place comes from GetWindowPlacement; and
//! DWMWA_TRANSITIONS_FORCEDISABLED is accepted on other processes' windows.
use super::super::Rect;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::{w, BOOL};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

fn h(id: isize) -> HWND { HWND(id as *mut _) }

// ---- Window geometry ----

/// Invisible resize borders (left, top, right, bottom) per window, learned while it was on
/// screen. Thumbnails include them, drawn black: they are cropped off.
static INSETS: Mutex<Option<HashMap<isize, (i32, i32, i32, i32)>>> = Mutex::new(None);

fn learn_insets(w: HWND) {
    unsafe {
        if IsIconic(w).as_bool() { return; }
        let (mut r, mut f) = (RECT::default(), RECT::default());
        if GetWindowRect(w, &mut r).is_err() { return; }
        if DwmGetWindowAttribute(w, DWMWA_EXTENDED_FRAME_BOUNDS, &mut f as *mut _ as *mut _, 16).is_err() { return; }
        let i = ((f.left - r.left).max(0), (f.top - r.top).max(0), (r.right - f.right).max(0), (r.bottom - f.bottom).max(0));
        INSETS.lock().unwrap().get_or_insert_with(HashMap::new).insert(w.0 as isize, i);
    }
}
fn insets(id: isize) -> (i32, i32, i32, i32) { INSETS.lock().unwrap().as_ref().and_then(|m| m.get(&id).copied()).unwrap_or_default() }

/// Where a (minimized) window's visible frame is when it's not minimized, in screen px.
pub fn visible_rect(id: isize) -> Option<Rect> {
    unsafe {
        let w = h(id);
        let mut p = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        GetWindowPlacement(w, &mut p).ok()?;
        let n = p.rcNormalPosition;
        let mon = MonitorFromRect(&n, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() { return None; }
        let (work, m) = (mi.rcWork, mi.rcMonitor);
        if p.flags.0 & WPF_RESTORETOMAXIMIZED.0 != 0 {
            return Some(Rect { x: work.left, y: work.top, w: work.right - work.left, h: work.bottom - work.top });
        }
        // The normal position is in work-area coordinates, except for tool windows.
        let tool = GetWindowLongPtrW(w, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0;
        let (dx, dy) = if tool { (0, 0) } else { (work.left - m.left, work.top - m.top) };
        let (l, t, r, b) = insets(id);
        let r = Rect { x: n.left + dx + l, y: n.top + dy + t, w: n.right - n.left - l - r, h: n.bottom - n.top - t - b };
        (r.w > 0 && r.h > 0).then_some(r)
    }
}

// ---- Windows' own animation ----

static SUPPRESS: AtomicBool = AtomicBool::new(false);
static SUPPRESSED: Mutex<Option<HashSet<isize>>> = Mutex::new(None);

fn set_transitions(w: HWND, off: bool) {
    let v: BOOL = off.into();
    unsafe { let _ = DwmSetWindowAttribute(w, DWMWA_TRANSITIONS_FORCEDISABLED, &v as *const _ as *const _, 4); }
}

/// Called for every window the window list reports.
pub fn on_listed(w: HWND) {
    learn_insets(w);
    if SUPPRESS.load(Ordering::Relaxed) && SUPPRESSED.lock().unwrap().get_or_insert_with(HashSet::new).insert(w.0 as isize) {
        set_transitions(w, true);
    }
}

pub fn suppress(on: bool) {
    SUPPRESS.store(on, Ordering::Relaxed);
    if on {
        for w in super::windows_list::list() { on_listed(h(w.id)); }
    } else {
        for id in SUPPRESSED.lock().unwrap().take().unwrap_or_default() { set_transitions(h(id), false); }
    }
}

// ---- Minimize notifications ----

static MIN_TX: OnceLock<Sender<isize>> = OnceLock::new();

/// From the window hook: a window started minimizing.
pub fn minimizing(w: HWND) {
    if let Some(tx) = MIN_TX.get() { let _ = tx.send(w.0 as isize); }
}

pub fn watch(f: Box<dyn Fn(isize, Rect) + Send>) {
    let (tx, rx) = channel::<isize>();
    if MIN_TX.set(tx).is_err() { return; }
    std::thread::spawn(move || {
        while let Ok(id) = rx.recv() {
            if !super::windows_list::is_taskbar_window(h(id)) { continue; }
            if let Some(r) = visible_rect(id) { f(id, r); }
        }
    });
}

// ---- The overlay ----

static OVERLAY: OnceLock<isize> = OnceLock::new();
/// Effects running now; the overlay shows with the first and hides after the last. Shown and
/// hidden under this lock, so one effect ending can't hide another that just started.
static ACTIVE: Mutex<usize> = Mutex::new(0);

unsafe extern "system" fn wndproc(w: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT { unsafe { DefWindowProcW(w, m, wp, lp) } }

fn overlay() -> Option<HWND> {
    let id = *OVERLAY.get_or_init(|| {
        let (tx, rx) = channel();
        std::thread::spawn(move || unsafe {
            let Ok(inst) = GetModuleHandleW(None) else { let _ = tx.send(0); return };
            let class = w!("GlassBarEffects");
            RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: inst.into(), lpszClassName: class, ..Default::default() });
            let w = CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class, w!("GlassBar effects"), WS_POPUP, 0, 0, 1, 1, None, None, Some(inst.into()), None);
            let Ok(w) = w else { let _ = tx.send(0); return };
            let _ = SetLayeredWindowAttributes(w, COLORREF(0), 255, LWA_ALPHA);
            let _ = tx.send(w.0 as isize);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() { DispatchMessageW(&msg); }
        });
        rx.recv_timeout(Duration::from_secs(2)).unwrap_or(0)
    });
    (id != 0).then(|| h(id))
}

fn overlay_area() -> Rect {
    let screen = unsafe { Rect { x: GetSystemMetrics(SM_XVIRTUALSCREEN), y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                                 w: GetSystemMetrics(SM_CXVIRTUALSCREEN), h: GetSystemMetrics(SM_CYVIRTUALSCREEN) } };
    crate::effects::overlay_rect(screen)
}

/// Run one effect on its own thread (see `Shell::effect_play`).
pub fn play(src: isize, run: u64, frames: Box<dyn Fn(f64) -> Vec<(Rect, Rect)> + Send>, ms: u32, then: Box<dyn FnOnce() + Send>) {
    std::thread::spawn(move || {
        let current = || crate::effects::RUNS.lock().unwrap().as_ref().is_some_and(|r| r.current(src, run));
        if let Some(ov) = overlay() {
            let vs = overlay_area();
            {
                let mut n = ACTIVE.lock().unwrap();
                if *n == 0 { unsafe { let _ = SetWindowPos(ov, Some(HWND_TOPMOST), vs.x, vs.y, vs.w, vs.h, SWP_NOACTIVATE | SWP_SHOWWINDOW); } }
                *n += 1;
            }
            let thumbs = animate(ov, src, &frames, ms, &current, vs);
            if let Some(r) = crate::effects::RUNS.lock().unwrap().as_mut() { r.end(src, run); }
            // `then` (restore: un-minimize) runs while the last frame still shows, and the frame
            // goes only once the window is drawn: hiding first left a blank frame between them.
            let (tx, rx) = channel();
            std::thread::spawn(move || { then(); let _ = tx.send(()); });
            let _ = rx.recv_timeout(Duration::from_millis(300)); // a hung app: don't wait on it
            unsafe { let _ = DwmFlush(); } // the restored window gets drawn
            for t in thumbs { unsafe { let _ = DwmUnregisterThumbnail(t); } }
            let mut n = ACTIVE.lock().unwrap();
            *n -= 1;
            if *n == 0 { unsafe { let _ = ShowWindow(ov, SW_HIDE); } }
            return;
        }
        if let Some(r) = crate::effects::RUNS.lock().unwrap().as_mut() { r.end(src, run); }
        then();
    });
}

/// Plays the frames; returns the thumbnails, still showing the last one.
fn animate(ov: HWND, src: isize, frames: &dyn Fn(f64) -> Vec<(Rect, Rect)>, ms: u32, current: &dyn Fn() -> bool, vs: Rect) -> Vec<isize> {
    let first = frames(0.0);
    let mut thumbs = Vec::new();
    if first.is_empty() { return thumbs; }
    for _ in 0..first.len() {
        match unsafe { DwmRegisterThumbnail(ov, h(src)) } { Ok(t) => thumbs.push(t), Err(_) => break }
    }
    if thumbs.len() == first.len() {
        let slices: Vec<Rect> = first.iter().map(|p| p.0).collect();
        let size = unsafe { DwmQueryThumbnailSourceSize(thumbs[0]) }.map(|s| (s.cx, s.cy)).unwrap_or((0, 0));
        // Slices are the same in every frame (only destinations move).
        let sources = crate::effects::source_rects(&slices, size, insets(src));
        let start = Instant::now();
        let total = ms.max(1) as f64;
        loop {
            let k = (start.elapsed().as_millis() as f64 / total).min(1.0);
            if !current() { break; }
            let mut ok = true;
            for ((th, s), (_, d)) in thumbs.iter().zip(&sources).zip(frames(k)) {
                let p = DWM_THUMBNAIL_PROPERTIES {
                    dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_RECTSOURCE | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
                    rcDestination: RECT { left: d.x - vs.x, top: d.y - vs.y, right: d.x - vs.x + d.w, bottom: d.y - vs.y + d.h },
                    rcSource: RECT { left: s.x, top: s.y, right: s.x + s.w, bottom: s.y + s.h },
                    opacity: 255, fVisible: (d.w > 0 && d.h > 0).into(), fSourceClientAreaOnly: false.into(), ..Default::default()
                };
                if unsafe { DwmUpdateThumbnailProperties(*th, &p) }.is_err() { ok = false; break; } // window gone
            }
            if !ok || k >= 1.0 { break; }
            if unsafe { DwmFlush() }.is_err() { std::thread::sleep(Duration::from_millis(8)); }
        }
    }
    thumbs
}
