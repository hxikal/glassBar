//! Dock magnification: the enlarged App Launcher icons are drawn in their own small window
//! over the launcher. The dock's native glass covers its whole window, so icons growing past
//! the dock can't be drawn in the dock itself. This window is transparent, never focusable
//! and lets every click through to the dock below.
use crate::platform::{shell, Glass, Rect};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const LABEL: &str = "magnify";

/// One enlarged icon, in the magnifier's own CSS px.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MagIcon { pub src: String, pub x: f64, pub y: f64, pub size: f64 }

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window(LABEL) { return Some(w); }
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("GlassBar magnifier")
        .decorations(false).transparent(true).shadow(false)
        .always_on_top(true).skip_taskbar(true).resizable(false)
        .focused(false).focusable(false).visible(false)
        .build().ok()?;
    let _ = w.set_ignore_cursor_events(true);
    shell().style_window(&w, &Glass { native: false, dark: true, material: String::new(), radius: 0 });
    shell().no_animation(&w);
    Some(w)
}

// Async on purpose: building a window inside a synchronous command (which runs on the main
// thread) deadlocks on Windows.

/// Create the magnifier ahead of the first hover (its page takes a moment to load).
#[tauri::command]
pub async fn magnify_prepare(app: AppHandle) { let _ = window(&app); }

/// Is the pointer off every one of `rects`?
pub fn pointer_away(rects: &[Rect], x: i32, y: i32) -> bool { !rects.iter().any(|&r| crate::autohide::contains(r, x, y, 2)) }

static WATCH: AtomicU64 = AtomicU64::new(0);

pub(crate) fn rect_of(w: &WebviewWindow) -> Option<Rect> {
    let (p, s) = (w.inner_position().ok()?, w.inner_size().ok()?);
    Some(Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 })
}

/// The page may miss the pointer leaving (a native context menu swallows it, the page
/// reloads): once the pointer is off the dock and the magnifier for 300 ms, put both back.
fn watchdog(app: AppHandle, dock: WebviewWindow) {
    let gen = WATCH.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let mut away = 0;
        while WATCH.load(Ordering::SeqCst) == gen {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let rects: Vec<Rect> = [Some(dock.clone()), app.get_webview_window(LABEL)].iter().flatten().filter_map(rect_of).collect();
            let Some((x, y)) = shell().cursor_pos() else { continue };
            away = if pointer_away(&rects, x, y) { away + 1 } else { 0 };
            if away >= 3 {
                let _ = dock.emit_to(dock.label(), "magnify:reset", ());
                magnify_hide(app.clone());
                crate::bar::bar_grow(app.clone(), dock.clone(), 0.0);
                return;
            }
        }
    });
}

/// Show the magnifier over `x, y, w, h`, given in the calling dock's CSS px.
#[tauri::command]
pub async fn magnify_show(app: AppHandle, window: WebviewWindow, x: f64, y: f64, w: f64, h: f64) {
    watchdog(app.clone(), window.clone());
    let (Some(m), Ok(pos), Ok(s)) = (self::window(&app), window.inner_position(), window.scale_factor()) else { return };
    let _ = m.set_position(PhysicalPosition::new(pos.x + (x * s).round() as i32, pos.y + (y * s).round() as i32));
    let _ = m.set_size(PhysicalSize::new((w * s).round().max(1.0) as u32, (h * s).round().max(1.0) as u32));
    let _ = m.show();
    shell().raise_top(&m);
}

#[tauri::command]
pub fn magnify_frame(app: AppHandle, icons: Vec<MagIcon>) { let _ = app.emit_to(LABEL, "magnify:frame", icons); }

#[tauri::command]
pub fn magnify_hide(app: AppHandle) {
    WATCH.fetch_add(1, Ordering::SeqCst); // stop the watchdog
    let _ = app.emit_to(LABEL, "magnify:frame", Vec::<MagIcon>::new());
    if let Some(m) = app.get_webview_window(LABEL) { let _ = m.hide(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Rect;

    #[test]
    fn the_pointer_is_away_only_when_off_every_rect() {
        let rects = [Rect { x: 0, y: 1000, w: 1000, h: 60 }, Rect { x: 0, y: 900, w: 1000, h: 160 }];
        assert!(!pointer_away(&rects, 500, 1030));
        assert!(!pointer_away(&rects, 500, 950));
        assert!(pointer_away(&rects, 500, 500));
    }
}
