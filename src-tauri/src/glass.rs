//! Liquid Glass: a picture of the screen behind a glass window (a dock, the flyout), so the
//! page can show what is really there through the glass and bend it at the rim. Windows
//! only blurs behind a window; it never hands the page those pixels.
//! The pictured window is left out of the capture for that moment (WDA_EXCLUDEFROMCAPTURE,
//! Windows 10 2004+). On screen it never goes away; a screen recorder may miss it for a frame.
use crate::platform::{screen, Rect};
use crate::AppState;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};

/// The latest picture per window label: where it was taken, a checksum, and the bytes the
/// page gets (see `packet`).
struct Pic { sum: u64, at: Instant, data: Vec<u8> }
static PICS: Mutex<Option<HashMap<String, Pic>>> = Mutex::new(None);
/// Something on screen probably changed (the dock slid in): picture the docks on the next tick.
static NUDGE: AtomicBool = AtomicBool::new(false);

pub fn on(app: &AppHandle) -> bool { app.state::<AppState>().config.lock().unwrap().theme.preset == "liquid" }
fn is_glass(label: &str) -> bool { label == "bar" || label.starts_with("bar-") || label == "popup" }
fn overlaps(a: Rect, b: Rect) -> bool { a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h }
pub fn nudge() { NUDGE.store(true, Ordering::Relaxed); }

/// Picture `area` (physical px) for window `label`, leaving out every visible glass window
/// over it. True when the picture differs from the last one.
pub fn take(app: &AppHandle, label: &str, area: Rect) -> bool {
    let hide: Vec<HWND> = app.webview_windows().into_iter()
        .filter(|(l, w)| is_glass(l) && w.is_visible().unwrap_or(false) && crate::magnify::rect_of(w).is_some_and(|r| overlaps(r, area)))
        .filter_map(|(_, w)| w.hwnd().ok()).map(|h| HWND(h.0)).collect();
    let shot = unsafe {
        for h in &hide { let _ = SetWindowDisplayAffinity(*h, WDA_EXCLUDEFROMCAPTURE); }
        // The exclusion takes effect with the next frame Windows composes.
        if !hide.is_empty() { let _ = DwmFlush(); let _ = DwmFlush(); }
        let s = screen().capture_rect(area.x, area.y, area.w, area.h);
        for h in &hide { let _ = SetWindowDisplayAffinity(*h, WDA_NONE); }
        s
    };
    let Some(s) = shot else { return false };
    let sum = checksum(&s.bgra);
    let mut g = PICS.lock().unwrap();
    let pics = g.get_or_insert_with(HashMap::new);
    if pics.get(label).is_some_and(|p| p.sum == sum) {
        if let Some(p) = pics.get_mut(label) { p.at = Instant::now(); }
        return false;
    }
    pics.insert(label.into(), Pic { sum, at: Instant::now(), data: packet(Rect { x: s.x, y: s.y, w: s.w, h: s.h }, &s.bgra) });
    true
}

/// What a glass window shows: its own rectangle, or for the flyout the full-height column
/// it sits in (it grows and moves along it as its content changes).
fn area(app: &AppHandle, label: &str, w: &WebviewWindow) -> Option<Rect> {
    let r = crate::magnify::rect_of(w)?;
    if label != "popup" { return Some(r); }
    let mon = w.current_monitor().ok().flatten().map(|m| crate::bar::monitor_rect(&m))
        .or_else(|| app.get_webview_window("bar").and_then(|b| b.current_monitor().ok().flatten()).map(|m| crate::bar::monitor_rect(&m)))?;
    Some(Rect { x: r.x, y: mon.y, w: r.w, h: mon.h })
}

/// Picture the flyout's column now (it was just placed) and tell its page.
pub fn refresh_popup(app: &AppHandle) {
    if !on(app) { return; }
    let Some(w) = app.get_webview_window("popup") else { return };
    if let Some(a) = area(app, "popup", &w) { if take(app, "popup", a) { let _ = app.emit_to("popup", "glass:behind", ()); } }
}

/// The page asks for its picture: the stored one, or a new one when `fresh` (or none yet).
/// Bytes: x, y, w, h (i32) and mean luminance (f32, 0..1), little-endian, then a BMP.
#[tauri::command]
pub async fn glass_behind(app: AppHandle, window: WebviewWindow, fresh: bool) -> tauri::ipc::Response {
    let label = window.label().to_string();
    let have = PICS.lock().unwrap().as_ref().is_some_and(|p| p.contains_key(&label));
    if (fresh || !have) && on(&app) {
        if let Some(a) = area(&app, &label, &window) { take(&app, &label, a); }
    }
    let data = PICS.lock().unwrap().as_ref().and_then(|p| p.get(&label)).map(|p| p.data.clone()).unwrap_or_default();
    tauri::ipc::Response::new(data)
}

/// Keep the docks' pictures current while Liquid Glass is on. With reserved space windows
/// stop at the dock, so only the desktop is behind it: a slow check is enough there.
/// ponytail: polling; hook window move/foreground events if 2 s feels laggy.
pub fn watch(app: &AppHandle) {
    let app = app.clone();
    std::thread::Builder::new().name("glass".into()).spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        if !on(&app) || crate::bar::fullscreen() { continue; }
        let b = app.state::<AppState>().config.lock().unwrap().profile().bar.clone();
        let every = Duration::from_secs(if !b.auto_hide && b.floating.is_none() && b.reserve_space { 30 } else { 2 });
        let nudged = NUDGE.swap(false, Ordering::Relaxed);
        for (label, w) in app.webview_windows() {
            if label == "popup" || !is_glass(&label) || !w.is_visible().unwrap_or(false) { continue; }
            let due = PICS.lock().unwrap().as_ref().and_then(|p| p.get(&label)).is_none_or(|p| p.at.elapsed() >= every);
            if !(due || nudged) { continue; }
            if let Some(a) = area(&app, &label, &w) { if take(&app, &label, a) { let _ = app.emit_to(label.as_str(), "glass:behind", ()); } }
        }
    }).ok();
}

/// Cheap change check: every 61st pixel.
fn checksum(bgra: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for p in bgra.chunks_exact(4).step_by(61) { for &b in &p[..3] { h = (h ^ b as u64).wrapping_mul(0x100000001b3); } }
    h ^ bgra.len() as u64
}

fn packet(r: Rect, bgra: &[u8]) -> Vec<u8> {
    let mut lum = 0.0f64;
    let mut n = 0;
    for p in bgra.chunks_exact(4).step_by(97) { lum += (0.0722 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.2126 * p[2] as f64) / 255.0; n += 1; }
    let lum = if n > 0 { (lum / n as f64) as f32 } else { 0.0 };
    let mut out = Vec::with_capacity(20 + 54 + bgra.len());
    for v in [r.x, r.y, r.w, r.h] { out.extend(v.to_le_bytes()); }
    out.extend(lum.to_le_bytes());
    out.extend(bmp(r.w, r.h, bgra));
    out
}

/// A top-down 32-bit BMP around the captured pixels as they are (no re-encoding).
fn bmp(w: i32, h: i32, bgra: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(54 + bgra.len());
    b.extend(b"BM");
    b.extend((54 + bgra.len() as u32).to_le_bytes());
    b.extend([0u8; 4]);
    b.extend(54u32.to_le_bytes());
    b.extend(40u32.to_le_bytes());
    b.extend(w.to_le_bytes());
    b.extend((-h).to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(32u16.to_le_bytes());
    b.extend(0u32.to_le_bytes()); // BI_RGB
    b.extend((bgra.len() as u32).to_le_bytes());
    b.extend([0u8; 16]);
    b.extend_from_slice(bgra);
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_carries_rect_luminance_and_a_valid_bmp() {
        let px = [255u8, 255, 255, 255].repeat(6); // 3 x 2 white
        let p = packet(Rect { x: -5, y: 10, w: 3, h: 2 }, &px);
        assert_eq!(i32::from_le_bytes(p[0..4].try_into().unwrap()), -5);
        assert_eq!(i32::from_le_bytes(p[12..16].try_into().unwrap()), 2);
        assert!((f32::from_le_bytes(p[16..20].try_into().unwrap()) - 1.0).abs() < 1e-3);
        let b = &p[20..];
        assert_eq!(&b[..2], b"BM");
        assert_eq!(u32::from_le_bytes(b[2..6].try_into().unwrap()) as usize, b.len());
        assert_eq!(i32::from_le_bytes(b[22..26].try_into().unwrap()), -2); // top-down
        assert_eq!(b.len(), 54 + 24);
    }
    #[test]
    fn checksum_sees_a_change() {
        let a = vec![10u8; 4 * 200];
        let mut b = a.clone();
        b[0] = 11;
        assert_ne!(checksum(&a), checksum(&b));
    }
}
