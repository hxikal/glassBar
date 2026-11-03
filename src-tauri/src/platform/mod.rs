//! Platform abstraction: every OS call goes through these traits.
//! Windows lives in `platform/windows/`; a macOS module can be added later
//! behind `#[cfg(target_os = "macos")]` without touching widget code.
use crate::config::{Edge, TaskbarMode};
use serde::{Deserialize, Serialize};

pub type Res<T> = Result<T, String>;

/// Recycle Bin contents: total and per fixed drive (bytes, items, free space on the drive).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecycleInfo { pub size: u64, pub items: u64, pub drives: Vec<RecycleDrive> }
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecycleDrive { pub root: String, pub size: u64, pub items: u64, pub free: u64 }

/// One item in the Recycle Bin. `id` is its parsing name inside the bin (stable while it is
/// there); `path` is where it came from, `from` that folder, `deleted_at` Unix ms.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BinItem { pub id: String, pub name: String, pub path: String, pub from: String, pub deleted_at: i64, pub size: u64, pub folder: bool }

/// Files and the Recycle Bin (Downloads widget, System Monitor).
pub trait Files: Send + Sync {
    fn downloads_dir(&self) -> Option<String>;
    fn file_icon(&self, path: &str) -> Option<String>;
    fn reveal(&self, path: &str) -> Res<()>;
    fn open_with(&self, path: &str) -> Res<()>;
    /// Move to the Recycle Bin; never a permanent delete without Windows asking.
    fn recycle(&self, path: &str) -> Res<()>;
    fn drag_out(&self, hwnd: isize, path: &str) -> Res<()>;
    fn recycle_info(&self) -> Res<RecycleInfo>;
    fn recycle_empty(&self, root: Option<&str>) -> Res<()>;
    /// Empty the bin with Windows' own "are you sure" dialog (Err when the user said no).
    fn recycle_empty_ask(&self) -> Res<()>;
    /// Every item in the bin, newest deleted first.
    fn recycle_items(&self) -> Res<Vec<BinItem>>;
    /// Put items back where they came from (Windows' Restore). `keys` are item ids or
    /// original paths; returns how many were restored.
    fn recycle_restore(&self, keys: &[String]) -> Res<u32>;
    /// Calls `f` (debounced) whenever something enters or leaves a Recycle Bin.
    fn watch_recycle(&self, f: Box<dyn Fn() + Send>);
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppWindow { pub id: isize, pub title: String, pub exe: String, pub active: bool, pub minimized: bool,
    /// Store/packaged app identity (launch via shell:AppsFolder), None for normal programs.
    pub aumid: Option<String> }

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice { pub id: String, pub name: String, pub is_default: bool }

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioState { pub volume: f32, pub muted: bool, pub devices: Vec<AudioDevice> }

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSession { pub pid: u32, pub name: String, pub exe: String, pub volume: f32, pub muted: bool }

/// The default recording device: mute, level 0..1, and the recording devices to choose from.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MicState { pub muted: bool, pub volume: f32, pub devices: Vec<AudioDevice> }

/// An app using the microphone or camera now: its name, program path (empty for Store
/// apps) and since when (Unix ms).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsingApp { pub name: String, pub exe: String, pub since: i64 }

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub app: String, pub title: String, pub artist: String, pub album: String,
    /// data: URL of the album art, if the player provides one.
    pub art: Option<String>,
    pub playing: bool,
    pub position_ms: i64, pub duration_ms: i64,
    /// Unix ms at which `position_ms` was true (frontend interpolates from here).
    pub updated_at_ms: i64,
    pub can_next: bool, pub can_prev: bool,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MediaAction { Toggle, Next, Prev }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RadioKind { Wifi, Bluetooth, Other }

#[derive(Serialize, Clone, Debug)]
pub struct RadioInfo { pub kind: RadioKind, pub name: String, pub on: bool }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode { Internal, Clone, Extend, External }

/// OS events the bar must react to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SystemEvent {
    /// Resolution, monitor added/removed, projection mode changed.
    DisplayChanged,
    /// Explorer restarted: AppBar registration and taskbar state must be re-applied.
    ShellRestarted,
    /// A fullscreen app appeared (true) or left (false) on the bar's monitor.
    Fullscreen(bool),
    /// A USB device was connected (true) or removed (false).
    Device(bool),
    /// Something was copied (clipboard contents changed).
    Clipboard,
    /// The desktop work area changed (an app bar reserved or released space).
    WorkArea,
    /// Plugged in / unplugged / battery level changed (instant, from Windows).
    PowerChanged,
    /// The PC woke from sleep or hibernation.
    Resumed,
}

#[derive(Serialize, Clone, Debug)]
pub struct RecentItem { pub name: String, pub path: String }

/// How a window's glass is drawn. `material`: acrylic | mica | blur. `radius`: the
/// appearance's outer corner radius (Windows can only round a window by 0, 4 or 8 px).
pub struct Glass { pub native: bool, pub dark: bool, pub material: String, pub radius: u32 }

pub trait Shell: Send + Sync {
    /// Hide a window at the OS level as well as through Tauri (belt and braces: a preview
    /// must never stay on screen as an empty box).
    fn hide_window(&self, win: &tauri::WebviewWindow);
    /// Show and hide a window instantly, without Windows' fade.
    fn no_animation(&self, _win: &tauri::WebviewWindow) {}
    /// Put a window back at the top of the always-on-top layer without activating it.
    fn raise_top(&self, _win: &tauri::WebviewWindow) {}
    /// Native blur + rounded corners (or plain transparent window for CSS glass).
    /// Returns the material Windows really draws ("none" if none could be applied).
    fn style_window(&self, win: &tauri::WebviewWindow, glass: &Glass) -> &'static str;
    /// Windows build number (0 if unknown): decides which materials exist.
    fn os_build(&self) -> u32 { 0 }
    /// Reserve `strip` on `edge` so maximized windows stop short of the bar.
    /// Start the system event source and set where to keep state that must survive a crash.
    /// Call once at startup, before any other Shell method.
    fn init(&self, data_dir: &std::path::Path, on_event: Box<dyn Fn(SystemEvent) + Send>);
    /// Returns the strip Windows actually granted (it may shift to avoid other bars).
    fn appbar_set(&self, edge: Edge, strip: Rect) -> Rect;
    fn appbar_remove(&self);
    /// No-op when the mode is unchanged. Restores the user's original state on `Show`.
    fn set_taskbar(&self, mode: TaskbarMode);
    /// Wait (at most `ms`) until process `pid` has exited.
    fn wait_for_exit(&self, _pid: u32, ms: u64) { std::thread::sleep(std::time::Duration::from_millis(ms.min(3_000))); }
    /// Whether GlassBar starts with Windows (then a hidden taskbar stays hidden across restarts).
    fn set_starts_with_windows(&self, _on: bool) {}
    /// Live thumbnails of windows drawn into `dest` (client pixels). Returns the windows
    /// that got one; the caller shows a fallback for the rest.
    fn thumbs_show(&self, dest: isize, items: &[(isize, Rect)]) -> Vec<isize>;
    /// Reserve `strip` for one of the extra docks, registered with that dock's own window.
    fn appbar_set_window(&self, _win: isize, _edge: Edge, strip: Rect) -> Rect { strip }
    fn appbar_remove_window(&self, _win: isize) {}
    /// Release every extra dock's reserved strip (quitting).
    fn appbar_remove_windows(&self) {}
    /// Remove every thumbnail (must run whenever the preview hides).
    fn thumbs_clear(&self, dest: isize);
    /// The current clipboard contents were marked private by the app that copied them.
    fn clipboard_is_private(&self) -> bool;
    /// Press keys together (in order) and release them: e.g. [LWIN] opens Start.
    fn press_keys(&self, keys: &[u16]);
    /// Type text into whatever has the keyboard (Unicode, so any language).
    fn type_text(&self, _text: &str) {}
    /// Installed input languages; `active` = the one in the last app you used.
    fn keyboard_layouts(&self) -> Vec<KeyboardLayout>;
    /// Windows' regional time format: Some(true) = 12-hour.
    fn time_12h(&self) -> Option<bool>;
    /// Switch the last-used app to this layout; false if that wasn't possible.
    fn keyboard_layout_set(&self, id: isize) -> bool;
    /// Is the left mouse button down right now (used to detect the end of a window drag)?
    fn mouse_down(&self) -> bool;
    /// Windows "Transparency effects" (Personalization › Colors). Off: native blur draws nothing.
    fn transparency_effects(&self) -> bool { true }
    /// Mouse pointer position in physical desktop pixels.
    fn cursor_pos(&self) -> Option<(i32, i32)>;
    /// A context menu (any app's) is open right now.
    fn menu_open(&self) -> bool;
    /// Milliseconds since the last keyboard/mouse input.
    fn idle_ms(&self) -> u64;
    /// Full path of the foreground window's program, if any.
    fn foreground_exe(&self) -> Option<String>;
    /// Recently used files that open with `exe`, newest first.
    fn recent_for(&self, exe: &str, limit: usize) -> Vec<RecentItem>;
    fn list_windows(&self) -> Vec<AppWindow>;
    /// Boost: drop in-memory caches (window and exe icons); they refill on demand.
    fn free_caches(&self) {}
    /// Calls `on_change` (debounced) whenever top-level windows change.
    fn watch_windows(&self, on_change: Box<dyn Fn() + Send>);
    /// Hide (or stop hiding) Windows' own volume / brightness pop-up (windows/osd.rs).
    fn windows_osd(&self, _hide: bool) {}
    /// The volume or brightness just changed: catch Windows' pop-up as it appears.
    fn windows_osd_nudge(&self) {}
    /// Closing: give Windows its pop-up back now.
    fn windows_osd_restore(&self) {}
    /// Calls `f(window, rect)` when a window starts to minimize; `rect` = where it was.
    fn watch_minimize(&self, _f: Box<dyn Fn(isize, Rect) + Send>) {}
    /// Where a minimized window will appear when restored (screen px).
    fn restore_rect(&self, _id: isize) -> Option<Rect> { None }
    /// Draw a minimize/restore effect of window `src` on top of the screen: `frames(t)` gives
    /// the (source slice, screen rect) pairs at t = 0..1 over `ms`, then `then` runs. `run`
    /// is the effect's id in `effects::RUNS`; it stops early once that is no longer current.
    fn effect_play(&self, _src: isize, _run: u64, _frames: Box<dyn Fn(f64) -> Vec<(Rect, Rect)> + Send>, _ms: u32, then: Box<dyn FnOnce() + Send>) { then() }
    /// Switch Windows' own minimize/restore animation off (or back on) for app windows, so
    /// it doesn't play under GlassBar's effect. Best effort.
    fn suppress_native_animations(&self, _on: bool) {}
    /// Focus, restore, or minimize if it is already the active window.
    fn activate(&self, id: isize);
    fn close(&self, id: isize);
    fn launch(&self, path: &str, args: &str) -> Res<()>;
    /// `launch` behind Windows' admin prompt (UAC).
    fn launch_elevated(&self, _path: &str, _args: &str) -> Res<()> { Err("not supported".into()) }
    /// A user environment variable as stored (unexpanded), and whether it is REG_EXPAND_SZ.
    fn user_env(&self, _name: &str) -> Option<(String, bool)> { None }
    /// A system environment variable as stored (unexpanded).
    fn machine_env(&self, _name: &str) -> Option<String> { None }
    /// Save a user environment variable and tell running programs it changed.
    fn set_user_env(&self, _name: &str, _value: &str, _expand: bool) -> Res<()> { Err("not supported".into()) }
    /// Start a Store/packaged app by its AppUserModelID, the way the Start menu does.
    fn launch_aumid(&self, aumid: &str) -> Res<()>;
    /// AppUserModelID for a program file inside C:\\Program Files\\WindowsApps (any version).
    fn aumid_for_path(&self, path: &str) -> Option<String>;
    fn aumid_icon(&self, aumid: &str) -> Option<String>;
    /// PNG data: URL for an executable's icon.
    fn exe_icon(&self, exe: &str) -> Option<String>;
    /// (target, args) of a .lnk shortcut; None if it has no file target.
    fn resolve_shortcut(&self, lnk: &str) -> Option<(String, String)>;
    /// Programs listed in the Start menu.
    fn installed_apps(&self) -> Vec<InstalledApp>;
    /// Icon for a window (Store apps, or programs whose file has no icon).
    fn window_icon(&self, id: isize) -> Option<String>;
}

pub trait Audio: Send + Sync {
    fn state(&self) -> Res<AudioState>;
    fn set_volume(&self, v: f32) -> Res<()>;
    fn set_mute(&self, m: bool) -> Res<()>;
    fn set_default(&self, id: &str) -> Res<()>;
    fn sessions(&self) -> Res<Vec<AudioSession>>;
    fn set_session_volume(&self, pid: u32, v: f32) -> Res<()>;
    fn set_session_mute(&self, pid: u32, m: bool) -> Res<()>;
    /// `on_change`: the volume, mute or device list changed (any source). `on_level`: the
    /// master volume or mute changed from *outside* GlassBar (keys, Windows, another app), as
    /// it happens -- (volume 0..1, muted).
    fn watch(&self, on_change: Box<dyn Fn(AudioState) + Send>, on_level: Box<dyn Fn(f32, bool) + Send>);
    /// Current output peak level 0..1 (None if no output device).
    fn peak(&self) -> Option<f32>;
    /// Capture what the speakers play, as mono chunks (see windows/audio.rs `loopback`).
    fn loopback(&self, on_chunk: &mut dyn FnMut(&[f32], u32) -> bool) -> Res<()>;
    fn mic_state(&self) -> Res<MicState>;
    fn mic_set_mute(&self, m: bool) -> Res<()>;
    fn mic_set_volume(&self, v: f32) -> Res<()>;
    /// Mic mute/level or the default recording device changed (any source).
    fn watch_mic(&self, f: Box<dyn Fn() + Send>);
    /// Apps using "microphone" or "webcam" right now (Windows' privacy store).
    fn using(&self, what: &str) -> Vec<UsingApp>;
    /// An app started or stopped using the microphone or camera.
    fn watch_using(&self, f: Box<dyn Fn() + Send>);
}

pub trait Media: Send + Sync {
    fn info(&self) -> Res<Option<MediaInfo>>;
    fn control(&self, a: MediaAction) -> Res<()>;
    fn seek(&self, position_ms: i64) -> Res<()>;
    /// Pause all playing sessions (for the adzan); returns their ids for `resume`.
    fn pause_playing(&self) -> Res<Vec<String>>;
    fn resume(&self, ids: &[String]) -> Res<()>;
    fn watch(&self, on_change: Box<dyn Fn(Option<MediaInfo>) + Send>);
    /// The app behind the current media session: Windows' id for it ("Spotify.exe",
    /// "Chrome", "MSEdge", a Store app id). None when nothing is playing.
    fn source_id(&self) -> Option<String> { None }
}

pub trait Radios: Send + Sync {
    fn list(&self) -> Res<Vec<RadioInfo>>;
    fn set(&self, kind: Option<RadioKind>, on: bool) -> Res<()>;
    /// Mobile hotspot; Err(reason) when this PC/connection can't host one.
    fn hotspot(&self) -> Res<Hotspot>;
    fn set_hotspot(&self, on: bool) -> Res<()>;
}

/// Quick Settings toggles Windows has no public API for (windows/toggles.rs). Each reads the
/// real state back; an error means "open the Windows page instead".
pub trait Toggles: Send + Sync {
    fn night_light(&self) -> Res<bool>;
    fn set_night_light(&self, on: bool) -> Res<()>;
    /// Nearby sharing: 0 off, 1 my devices only, 2 everyone nearby.
    fn nearby(&self) -> Res<u32>;
    fn set_nearby(&self, mode: u32) -> Res<()>;
    /// Wireless displays nearby, and whether this PC is casting to one now.
    fn cast(&self) -> Res<CastState>;
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastState { pub casting: bool, pub displays: Vec<CastDisplay> }
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CastDisplay { pub id: String, pub name: String }

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Hotspot { pub on: bool, pub ssid: String, pub clients: u32, pub max_clients: u32 }

/// A screenshot: BGRA pixels of one monitor at desktop position (x, y), physical pixels.
pub struct Shot { pub x: i32, pub y: i32, pub w: i32, pub h: i32, pub bgra: Vec<u8> }

/// A window on screen (absolute physical pixels), for picking one to screenshot.
#[derive(Serialize, Clone, Debug)]
pub struct WinRect { pub id: isize, pub title: String, pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

/// One recognised word: text, left edge and width in image pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Word { pub text: String, pub x: f32, pub w: f32 }

pub trait Screen: Send + Sync {
    /// Screenshot of the monitor under the mouse.
    fn capture_at_cursor(&self) -> Option<Shot>;
    /// Live pixels of a desktop rectangle.
    fn capture_rect(&self, x: i32, y: i32, w: i32, h: i32) -> Option<Shot>;
    /// Visible app windows within (x, y, w, h), front-most first.
    fn window_rects(&self, area: (i32, i32, i32, i32)) -> Vec<WinRect>;
    /// The part of a window's rectangle (x, y, w, h) not covered by bars in front of it.
    fn unobscured(&self, _id: isize, r: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) { r }
    /// Scroll a window by mouse-wheel notches at screen point `at`.
    fn scroll_window(&self, id: isize, at: (i32, i32), notches: i32, focus: bool);
    /// OCR with the OS engine: lines in reading order.
    fn ocr(&self, bgra: &[u8], w: u32, h: u32) -> Res<Vec<Vec<Word>>>;
}

/// An app from the Start menu. Store apps carry their AppUserModelID (`aumid`): that's how
/// they're started, and `path` is their .exe so pins match their running windows.
#[derive(Serialize, Clone, Debug)]
pub struct InstalledApp { pub name: String, pub path: String, pub args: String, pub aumid: Option<String> }

#[derive(Serialize, Clone, Debug)]
pub struct WifiNetwork { pub ssid: String, pub bars: u8, pub secured: bool, pub security: String, pub connected: bool, pub saved: bool }

#[derive(Serialize, Clone, Debug)]
pub struct BtDevice { pub id: String, pub name: String, pub kind: String, pub paired: bool, pub connected: bool,
    /// Battery % when the connected device reports one to Windows.
    pub battery: Option<u8> }

pub trait Wireless: Send + Sync {
    /// (connected SSID, networks). `scan` asks the adapter for a fresh scan first.
    fn wifi_list(&self, scan: bool) -> Res<(Option<String>, Vec<WifiNetwork>)>;
    fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> Res<()>;
    fn wifi_disconnect(&self) -> Res<()>;
    fn bt_list(&self, discover: bool) -> Res<Vec<BtDevice>>;
    fn bt_pair(&self, id: &str) -> Res<()>;
    fn bt_unpair(&self, id: &str) -> Res<()>;
    /// Call `f` whenever Windows' network status changes (connect, disconnect, new network).
    fn watch_network(&self, f: Box<dyn Fn() + Send + Sync>);
}

/// A notification-area icon hosted by GlassBar (see platform/windows/tray.rs).
#[derive(Serialize, Clone, Debug)]
pub struct TrayIcon { pub id: String, pub tip: String, pub icon: Option<String>, pub exe: String,
    /// Windows 11 "always show" for this program (Settings › Personalization › Taskbar).
    pub promoted: bool, pub rev: u64 }

pub trait Tray: Send + Sync {
    fn start(&self, on_change: Box<dyn Fn() + Send>);
    fn stop(&self);
    fn running(&self) -> bool;
    /// Ask all apps to re-send their tray icons.
    fn refresh(&self);
    fn icons(&self) -> Vec<TrayIcon>;
    /// "left" | "right" | "double"; forwarded to the owning app as Explorer would.
    fn click(&self, id: &str, button: &str) -> bool;
}

/// An installed keyboard input language. `id` is the Windows HKL.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardLayout { pub id: isize, pub code: String, pub name: String, pub locale: String, pub active: bool }

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Battery { pub percent: u8, pub charging: bool, pub on_ac: bool, pub secs_left: Option<u32> }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode { Efficiency, Balanced, Performance }

pub trait Power: Send + Sync {
    /// None on PCs without a battery.
    fn battery(&self) -> Option<Battery>;
    fn power_mode(&self) -> Option<PowerMode>;
    fn set_power_mode(&self, m: PowerMode) -> Res<()>;
    /// Battery/energy saver on? None if unknown.
    fn saver_on(&self) -> Option<bool>;
    /// Name of the active power plan when it is NOT Balanced (power modes only work on
    /// Balanced), e.g. "High performance". None when Balanced or unknown.
    fn other_plan(&self) -> Option<String> { None }
    /// Switch to Windows' Balanced power plan (so power modes work again).
    fn use_balanced_plan(&self) -> Res<()> { Err("Not supported".into()) }
    /// Keep the PC awake (and the screen on, with `display`) until `allow_sleep`. Belongs to
    /// this process: it ends when GlassBar quits or crashes.
    fn keep_awake(&self, _display: bool) -> Res<()> { Err("Not supported".into()) }
    fn allow_sleep(&self) {}
}

pub trait Gpu: Send + Sync {
    /// GPU utilization 0..100, None if counters are unavailable.
    fn usage(&self) -> Option<f32>;
    /// GPU utilization of each of these processes (same order), 0..100; None if counters are unavailable.
    fn usage_each(&self, pids: &[u32]) -> Option<Vec<f32>>;
    /// A process's private working set (Task Manager's Memory) in bytes; None if it can't be read.
    fn private_ws(&self, pid: u32) -> Option<u64>;
    /// CPU/thermal-zone temperature in °C; None when the PC exposes no sensor.
    fn cpu_temp_c(&self) -> Option<f32>;
}

/// A browser-installed web app (Chrome/Edge PWA: "Chrome._crx_…", "MSEdge._crx_…") runs as
/// the browser's exe but is its own app: its window carries the app's own ID.
pub fn is_web_app_id(id: &str) -> bool { id.contains("._crx_") }

/// A position from the OS location service.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix { pub lat: f64, pub lon: f64, pub accuracy_m: Option<f64>, pub via: &'static str }

pub trait Location: Send + Sync {
    /// Device position from the OS location service (GPS, Wi-Fi, cell or IP, as Windows decides).
    fn locate(&self) -> Res<Fix>;
}

pub trait Display: Send + Sync {
    fn mode(&self) -> Option<DisplayMode>;
    fn set_mode(&self, m: DisplayMode) -> Res<()>;
    /// Displays plugged in (also ones switched off by "PC only").
    fn connected(&self) -> u32 { 1 }
    fn brightness(&self) -> Option<u32>;
    fn set_brightness(&self, v: u32) -> Res<()>;
    /// The built-in screen's brightness changed from outside GlassBar (0..100).
    fn watch_brightness(&self, _on_change: Box<dyn Fn(u32) + Send>) {}
}

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod win;

#[cfg(windows)]
pub fn shell() -> &'static dyn Shell { &win::WinShell }
#[cfg(windows)]
pub fn audio() -> &'static dyn Audio { &win::WinAudio }
#[cfg(windows)]
pub fn media() -> &'static dyn Media { &win::WinMedia }
#[cfg(windows)]
pub fn radios() -> &'static dyn Radios { &win::WinRadios }
#[cfg(windows)]
pub fn toggles() -> &'static dyn Toggles { &win::WinToggles }
#[cfg(windows)]
pub fn display() -> &'static dyn Display { &win::WinDisplay }
#[cfg(windows)]
pub fn location() -> &'static dyn Location { &win::WinLocation }
#[cfg(windows)]
pub fn power() -> &'static dyn Power { &win::WinPower }
#[cfg(windows)]
pub fn gpu() -> &'static dyn Gpu { &win::WinGpu }
#[cfg(windows)]
pub fn screen() -> &'static dyn Screen { &win::WinScreen }
#[cfg(windows)]
pub fn wireless() -> &'static dyn Wireless { &win::WinWireless }
#[cfg(windows)]
pub fn files() -> &'static dyn Files { &win::WinFiles }
#[cfg(windows)]
pub fn tray() -> &'static dyn Tray { &win::WinTray }
