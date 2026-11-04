//! Minimize / restore effects: pure geometry, no Windows calls (those are in
//! platform/windows/effects.rs). Each frame is a list of (source slice, destination) pairs:
//! the slice is in the source window's own pixels, the destination in screen pixels.

use crate::config::Edge;
use crate::platform::{shell, Rect};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::WebviewWindow;

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Effect { #[default] Genie, Scale, Suck, None }

pub fn duration_ms(e: Effect) -> u32 {
    match e { Effect::Genie => 450, Effect::Suck => 300, Effect::Scale => 250, Effect::None => 0 }
}

/// A rect turned so the dock is always "below": u runs along the dock, v grows toward it.
#[derive(Clone, Copy)]
struct Canon { u0: f64, u1: f64, v0: f64, v1: f64 }

fn canon(r: Rect, edge: Edge) -> Canon {
    let (x0, x1, y0, y1) = (r.x as f64, (r.x + r.w) as f64, r.y as f64, (r.y + r.h) as f64);
    match edge {
        Edge::Bottom => Canon { u0: x0, u1: x1, v0: y0, v1: y1 },
        Edge::Top => Canon { u0: x0, u1: x1, v0: -y1, v1: -y0 },
        Edge::Right => Canon { u0: y0, u1: y1, v0: x0, v1: x1 },
        Edge::Left => Canon { u0: y0, u1: y1, v0: -x1, v1: -x0 },
    }
}

/// Back to a screen rect. Edges are rounded (not sizes), so neighbouring strips still tile.
fn uncanon(c: Canon, edge: Edge) -> Rect {
    let r = |a: f64, b: f64| { let (a, b) = (a.round() as i32, b.round() as i32); (a.min(b), (a - b).abs()) };
    let ((x, w), (y, h)) = match edge {
        Edge::Bottom => (r(c.u0, c.u1), r(c.v0, c.v1)),
        Edge::Top => (r(c.u0, c.u1), r(-c.v1, -c.v0)),
        Edge::Right => (r(c.v0, c.v1), r(c.u0, c.u1)),
        Edge::Left => (r(-c.v1, -c.v0), r(c.u0, c.u1)),
    };
    Rect { x, y, w, h }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }

/// One frame of effect `e` at time `t` (0..1) flying `from` (the window) into `to` (the
/// dock target). Restore plays the same frames with `t` running from 1 to 0.
pub fn frame(e: Effect, from: Rect, to: Rect, edge: Edge, t: f64, strips: usize) -> Vec<(Rect, Rect)> {
    let t = t.clamp(0.0, 1.0);
    let whole = Rect { x: 0, y: 0, w: from.w, h: from.h };
    if matches!(e, Effect::Scale | Effect::None) || strips < 2 {
        let k = t * t; // ease in
        let d = Rect { x: lerp(from.x as f64, to.x as f64, k).round() as i32, y: lerp(from.y as f64, to.y as f64, k).round() as i32,
                       w: lerp(from.w as f64, to.w as f64, k).round() as i32, h: lerp(from.h as f64, to.h as f64, k).round() as i32 };
        return vec![(whole, d)];
    }
    let (f, g, src) = (canon(from, edge), canon(to, edge), canon(whole, edge));
    let suck = e == Effect::Suck;
    let k = if suck { t * t } else { t * t * (3.0 - 2.0 * t) };
    let a = (2.0 * k).min(1.0); // phase 1: the sides bend in toward the target
    let c = (2.0 * k - 1.0).max(0.0); // phase 2: the column slides into the target
    // Suck first pulls toward the target's corner nearest the window, then into the target.
    let near = if (f.u0 + f.u1) / 2.0 < (g.u0 + g.u1) / 2.0 { g.u0 } else { g.u1 };
    let (bu0, bu1) = if suck { (near, near) } else { (g.u0, g.u1) };
    let (cv0, cv1) = (lerp(f.v0, g.v0, c), lerp(f.v1, g.v1, c));
    let n = strips as f64;
    (0..strips).map(|i| {
        let (s0, s1) = (i as f64 / n, (i + 1) as f64 / n);
        let p = 1.0 - (i as f64 + 0.5) / n; // 0 = the side nearest the dock
        let curve = if suck { 1.0 - p } else { (1.0 - p) * (1.0 - p) };
        let bend = a * curve;
        let u0 = lerp(lerp(f.u0, bu0, bend), g.u0, c);
        let u1 = lerp(lerp(f.u1, bu1, bend), g.u1, c);
        let dest = Canon { u0, u1, v0: lerp(cv0, cv1, s0), v1: lerp(cv0, cv1, s1) };
        let slice = Canon { u0: src.u0, u1: src.u1, v0: lerp(src.v0, src.v1, s0), v1: lerp(src.v0, src.v1, s1) };
        (uncanon(slice, edge), uncanon(dest, edge))
    }).collect()
}

/// Where each slice lies in the DWM thumbnail's source. Slices cover the visible window
/// (their union); the source usually also has the window's invisible borders `insets`
/// (left, top, right, bottom), unless its size already matches the visible window.
pub fn source_rects(slices: &[Rect], size: (i32, i32), insets: (i32, i32, i32, i32)) -> Vec<Rect> {
    let w = slices.iter().map(|s| s.x + s.w).max().unwrap_or(1).max(1);
    let h = slices.iter().map(|s| s.y + s.h).max().unwrap_or(1).max(1);
    let (l, t, r, b) = if (size.0 - w).abs() <= 4 && (size.1 - h).abs() <= 4 { (0, 0, 0, 0) } else { insets };
    let (sx, sy) = ((size.0 - l - r).max(1) as f64 / w as f64, (size.1 - t - b).max(1) as f64 / h as f64);
    slices.iter().map(|s| {
        let (x0, y0) = (l + (s.x as f64 * sx).round() as i32, t + (s.y as f64 * sy).round() as i32);
        let (x1, y1) = (l + ((s.x + s.w) as f64 * sx).round() as i32, t + ((s.y + s.h) as f64 * sy).round() as i32);
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }).collect()
}

// ---- Where windows fly to (sent by the App Launcher) ----

/// A rect in a dock window's CSS px, as the page measures it.
#[derive(Deserialize, Clone, Copy, Debug)]
pub struct RectCss { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }

/// What the first App Launcher reports: the effect to play and where each window goes (its
/// icon; windows it doesn't list fly to the dock's middle).
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EffectTargets { pub effect: Effect, pub per_window: HashMap<isize, RectCss>, pub dock: RectCss }

/// The same in screen pixels.
#[derive(Clone, Debug)]
pub struct Targets { pub effect: Effect, pub edge: Edge, pub per_window: HashMap<isize, Rect>, pub dock: Rect }

pub fn to_screen(r: RectCss, origin: (i32, i32), scale: f64) -> Rect {
    let p = |v: f64| (v * scale).round() as i32;
    Rect { x: origin.0 + p(r.x), y: origin.1 + p(r.y), w: p(r.w), h: p(r.h) }
}

/// The window's icon, else the dock's middle.
pub fn pick_target(t: &Targets, hwnd: isize) -> Rect {
    if let Some(r) = t.per_window.get(&hwnd) { return *r; }
    let d = t.dock;
    let s = d.w.min(d.h);
    Rect { x: d.x + (d.w - s) / 2, y: d.y + (d.h - s) / 2, w: s, h: s }
}

/// One running effect per window: starting another cancels the first.
#[derive(Default)]
pub struct Registry { next: u64, runs: HashMap<isize, u64> }
impl Registry {
    pub fn start(&mut self, hwnd: isize) -> u64 { self.next += 1; self.runs.insert(hwnd, self.next); self.next }
    pub fn current(&self, hwnd: isize, run: u64) -> bool { self.runs.get(&hwnd) == Some(&run) }
    pub fn end(&mut self, hwnd: isize, run: u64) { if self.current(hwnd, run) { self.runs.remove(&hwnd); } }
}

/// Per dock window (with "All displays" each dock's first launcher reports its own). No App
/// Launcher on any dock: no effect, Windows animates as usual.
static TARGETS: Mutex<Option<HashMap<String, Targets>>> = Mutex::new(None);
pub static RUNS: Mutex<Option<Registry>> = Mutex::new(None);

/// How many strips Genie/Suck cut a window into: one per ~4 px across it, so the bent edge
/// stays smooth on big windows (a fixed 40 left 25 px steps on a full-height one).
/// ponytail: each strip is a DWM thumbnail updated every frame; 160 kept 60 fps here.
pub fn strips_for(window: Rect, edge: Edge) -> usize {
    let across = if matches!(edge, Edge::Left | Edge::Right) { window.w } else { window.h };
    (across / 4).clamp(40, 160) as usize
}

/// The overlay's area: the whole virtual screen, so effects running at the same time share
/// one origin, inset by a pixel because DWM draws no thumbnails on a click-through window
/// that covers a full monitor (checked on 26200).
pub fn overlay_rect(screen: Rect) -> Rect { Rect { x: screen.x + 1, y: screen.y + 1, w: screen.w - 2, h: screen.h - 2 } }

fn effect_now() -> Effect { TARGETS.lock().unwrap().as_ref().and_then(|m| m.values().next()).map_or(Effect::None, |t| t.effect) }

/// A dock closed: windows no longer fly to it.
pub fn forget(label: &str) { if let Some(m) = TARGETS.lock().unwrap().as_mut() { m.remove(label); } }

#[tauri::command]
pub fn effects_targets(app: tauri::AppHandle, window: WebviewWindow, t: EffectTargets) {
    use tauri::Manager;
    if !crate::docks::is_dock(window.label()) { return; } // Settings' dock preview isn't a dock
    let edge = app.state::<crate::AppState>().config.lock().unwrap().profile().bar.edge;
    let (Ok(pos), Ok(s)) = (window.inner_position(), window.scale_factor()) else { return };
    let o = (pos.x, pos.y);
    let was = effect_now();
    TARGETS.lock().unwrap().get_or_insert_with(HashMap::new).insert(window.label().to_string(), Targets {
        effect: t.effect, edge,
        per_window: t.per_window.into_iter().map(|(k, r)| (k, to_screen(r, o, s))).collect(),
        dock: to_screen(t.dock, o, s),
    });
    if (was == Effect::None) != (t.effect == Effect::None) { shell().suppress_native_animations(t.effect != Effect::None); }
}

/// Play effect `e` for window `hwnd` between its rect and its dock target. `then` runs at the
/// end, while the last frame still shows (restore puts the window back under it).
fn play(hwnd: isize, window: Rect, reverse: bool, then: Box<dyn FnOnce() + Send>) {
    // The dock nearest the window (its own monitor's, with "All displays").
    let t = {
        let map = TARGETS.lock().unwrap();
        let docks: Vec<(String, Rect)> = map.iter().flatten().map(|(l, t)| (l.clone(), t.dock)).collect();
        crate::docks::nearest_dock(&docks, window).and_then(|l| map.as_ref()?.get(l).cloned())
    };
    let Some(t) = t else { return then() };
    if t.effect == Effect::None || window.w <= 0 || window.h <= 0 { return then(); }
    let to = pick_target(&t, hwnd);
    let (edge, e, strips) = (t.edge, t.effect, strips_for(window, t.edge));
    let run = RUNS.lock().unwrap().get_or_insert_with(Registry::default).start(hwnd);
    shell().effect_play(hwnd, run, Box::new(move |k: f64| frame(e, window, to, edge, if reverse { 1.0 - k } else { k }, strips)),
                        duration_ms(e), then);
}

/// A window began to minimize (from the window hook).
pub fn on_minimize(hwnd: isize, rect: Rect) { play(hwnd, rect, false, Box::new(|| {})); }

/// The dock restores a minimized window: fly it out of its target, then restore it.
#[tauri::command]
pub async fn app_restore_animated(id: isize) {
    let Some(r) = shell().restore_rect(id) else { return shell().activate(id) };
    let (tx, rx) = std::sync::mpsc::channel();
    play(id, r, true, Box::new(move || { shell().activate(id); let _ = tx.send(()); }));
    let _ = tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(std::time::Duration::from_secs(3))).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Edge;
    use crate::platform::Rect;

    const FROM: Rect = Rect { x: 400, y: 100, w: 800, h: 600 };
    const TO: Rect = Rect { x: 900, y: 1020, w: 40, h: 40 };

    #[test]
    fn the_overlay_never_covers_a_whole_screen() {
        assert_eq!(overlay_rect(Rect { x: 0, y: 0, w: 1920, h: 1080 }), Rect { x: 1, y: 1, w: 1918, h: 1078 });
        assert_eq!(overlay_rect(Rect { x: -1920, y: 0, w: 3840, h: 1080 }), Rect { x: -1919, y: 1, w: 3838, h: 1078 });
    }

    #[test]
    fn big_windows_get_thinner_strips() {
        assert_eq!(strips_for(Rect { x: 0, y: 0, w: 300, h: 100 }, Edge::Bottom), 40);
        assert_eq!(strips_for(Rect { x: 0, y: 0, w: 300, h: 400 }, Edge::Top), 100);
        assert_eq!(strips_for(Rect { x: 0, y: 0, w: 1920, h: 1040 }, Edge::Bottom), 160);
        assert_eq!(strips_for(Rect { x: 0, y: 0, w: 400, h: 1000 }, Edge::Left), 100); // cut across its width
    }

    fn bbox(v: &[(Rect, Rect)]) -> Rect {
        let x0 = v.iter().map(|p| p.1.x).min().unwrap();
        let y0 = v.iter().map(|p| p.1.y).min().unwrap();
        let x1 = v.iter().map(|p| p.1.x + p.1.w).max().unwrap();
        let y1 = v.iter().map(|p| p.1.y + p.1.h).max().unwrap();
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }

    #[test]
    fn scale_goes_from_the_window_to_the_icon() {
        assert_eq!(frame(Effect::Scale, FROM, TO, Edge::Bottom, 0.0, 1), vec![(Rect { x: 0, y: 0, w: 800, h: 600 }, FROM)]);
        assert_eq!(frame(Effect::Scale, FROM, TO, Edge::Bottom, 1.0, 1)[0].1, TO);
    }

    #[test]
    fn genie_starts_as_the_whole_window() {
        let f = frame(Effect::Genie, FROM, TO, Edge::Bottom, 0.0, 30);
        assert_eq!(f.len(), 30);
        assert_eq!(bbox(&f), FROM);
        assert_eq!(f.iter().map(|p| p.1.h).sum::<i32>(), FROM.h);
        assert_eq!(f.iter().map(|p| p.0.h).sum::<i32>(), FROM.h);
        assert!(f.iter().all(|p| p.0.w == FROM.w && p.0.x == 0));
    }

    #[test]
    fn genie_ends_inside_the_icon() {
        for p in frame(Effect::Genie, FROM, TO, Edge::Bottom, 1.0, 30) {
            let d = p.1;
            assert!(d.x >= TO.x - 1 && d.y >= TO.y - 1 && d.x + d.w <= TO.x + TO.w + 1 && d.y + d.h <= TO.y + TO.h + 1, "{d:?}");
        }
    }

    #[test]
    fn genie_bends_the_side_nearest_the_dock_first() {
        let f = frame(Effect::Genie, FROM, TO, Edge::Bottom, 0.25, 30);
        let near = f.iter().max_by_key(|p| p.1.y).unwrap().1;
        let far = f.iter().min_by_key(|p| p.1.y).unwrap().1;
        assert!(near.w < far.w, "near {near:?} far {far:?}");
    }

    #[test]
    fn genie_on_a_side_dock_cuts_upright_strips() {
        let to = Rect { x: 10, y: 500, w: 40, h: 40 };
        let f = frame(Effect::Genie, FROM, to, Edge::Left, 0.0, 20);
        assert_eq!(bbox(&f), FROM);
        assert!(f.iter().all(|p| p.0.h == FROM.h && p.0.y == 0));
    }

    #[test]
    fn suck_also_ends_inside_the_icon() {
        for p in frame(Effect::Suck, FROM, TO, Edge::Bottom, 1.0, 30) {
            let d = p.1;
            assert!(d.x >= TO.x - 1 && d.x + d.w <= TO.x + TO.w + 1 && d.y >= TO.y - 1, "{d:?}");
        }
    }

    fn targets() -> Targets {
        let mut per_window = std::collections::HashMap::new();
        per_window.insert(7, Rect { x: 100, y: 1000, w: 40, h: 40 });
        Targets { effect: Effect::Genie, edge: Edge::Bottom, per_window, dock: Rect { x: 0, y: 990, w: 1000, h: 60 } }
    }

    #[test]
    fn a_window_flies_to_its_icon_else_the_dock_middle() {
        assert_eq!(pick_target(&targets(), 7), Rect { x: 100, y: 1000, w: 40, h: 40 });
        let mid = pick_target(&targets(), 9);
        assert_eq!((mid.x + mid.w / 2, mid.y + mid.h / 2), (500, 1020));
        assert!(mid.w > 0 && mid.w <= 60);
    }

    #[test]
    fn a_new_effect_for_the_same_window_cancels_the_old_one() {
        let mut r = Registry::default();
        let a = r.start(7);
        let other = r.start(8);
        let b = r.start(7);
        assert!(!r.current(7, a));
        assert!(r.current(7, b));
        assert!(r.current(8, other));
        r.end(7, a); // the cancelled run ending must not drop the new one
        assert!(r.current(7, b));
        r.end(7, b);
        assert!(!r.current(7, b));
    }

    #[test]
    fn each_strip_reads_its_own_slice_of_the_window() {
        let f = frame(Effect::Genie, FROM, TO, Edge::Bottom, 0.0, 30);
        let slices: Vec<Rect> = f.iter().map(|p| p.0).collect();
        // Source with 8 px invisible borders on the left, right and bottom.
        let r = source_rects(&slices, (FROM.w + 16, FROM.h + 8), (8, 0, 8, 8));
        assert_eq!(r[0], Rect { x: 8, y: 0, w: FROM.w, h: 20 });
        assert_eq!(r[29].y + r[29].h, FROM.h);
        // Source already the window's size: no borders to crop.
        let r = source_rects(&slices, (FROM.w, FROM.h), (8, 0, 8, 8));
        assert_eq!(r[1], Rect { x: 0, y: 20, w: FROM.w, h: 20 });
    }

    #[test]
    fn off_takes_no_time() {
        assert_eq!(duration_ms(Effect::None), 0);
        assert!(duration_ms(Effect::Genie) > duration_ms(Effect::Scale));
    }
}
