//! Windows implementation of the platform traits.
mod appbar;
mod audio;
mod awake;
mod btbattery;
mod display;
mod effects;
mod files;
mod gpu;
mod procmem;
mod hotspot;
mod lang;
mod osd;
mod power;
mod privacy;
mod packages;
mod powermode;
mod promoted;
mod location;
mod media;
mod radios;
mod recent;
mod shortcuts;
mod snap;
mod system;
mod thermal;
mod thumbs;
mod toggles;
pub(crate) mod tray;
mod windows_list;
mod wireless;

use super::*;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

pub struct WinShell;
pub struct WinAudio;
pub struct WinMedia;
pub struct WinRadios;
pub struct WinToggles;
pub struct WinDisplay;
pub struct WinLocation;
pub struct WinPower;
pub struct WinGpu;
pub struct WinScreen;
pub struct WinTray;
impl Tray for WinTray {
    fn start(&self, on_change: Box<dyn Fn() + Send>) { tray::start(on_change) }
    fn stop(&self) { tray::stop() }
    fn running(&self) -> bool { tray::running() }
    fn refresh(&self) { tray::refresh() }
    fn icons(&self) -> Vec<TrayIcon> { let p = promoted::load(); tray::icons(&|exe| promoted::is_promoted(&p, exe)) }
    fn click(&self, id: &str, button: &str) -> bool { tray::click(id, button) }
}
pub struct WinWireless;
impl Wireless for WinWireless {
    fn wifi_list(&self, scan: bool) -> Res<(Option<String>, Vec<WifiNetwork>)> { wireless::wifi_list(scan) }
    fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> Res<()> { wireless::wifi_connect(ssid, password) }
    fn wifi_disconnect(&self) -> Res<()> { wireless::wifi_disconnect() }
    fn bt_list(&self, discover: bool) -> Res<Vec<BtDevice>> { wireless::bt_list(discover) }
    fn bt_pair(&self, id: &str) -> Res<()> { wireless::bt_pair(id) }
    fn bt_unpair(&self, id: &str) -> Res<()> { wireless::bt_unpair(id) }
    fn watch_network(&self, f: Box<dyn Fn() + Send + Sync>) { wireless::watch_network(f) }
}
impl Screen for WinScreen {
    fn capture_at_cursor(&self) -> Option<Shot> { snap::capture_at_cursor() }
    fn capture_rect(&self, x: i32, y: i32, w: i32, h: i32) -> Option<Shot> { snap::capture_rect(x, y, w, h) }
    fn window_rects(&self, area: (i32, i32, i32, i32)) -> Vec<WinRect> { snap::window_rects(area) }
    fn unobscured(&self, id: isize, r: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) { snap::unobscured(id, r) }
    fn scroll_window(&self, id: isize, at: (i32, i32), notches: i32, focus: bool) { snap::scroll_window(id, at, notches, focus) }
    fn ocr(&self, bgra: &[u8], w: u32, h: u32) -> Res<Vec<Vec<Word>>> { snap::ocr(bgra, w, h) }
}

/// Join this thread to the COM multithreaded apartment (no-op if already joined).
pub(crate) fn com_init() {
    unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
}

pub(crate) fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }

pub(crate) fn err(e: impl std::fmt::Display) -> String { e.to_string() }

impl Shell for WinShell {
    fn style_window(&self, win: &tauri::WebviewWindow, g: &Glass) -> &'static str { appbar::style_window(win, g) }
    fn os_build(&self) -> u32 { appbar::os_build() }
    fn hide_window(&self, win: &tauri::WebviewWindow) {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        if let Ok(h) = win.hwnd() { unsafe { let _ = ShowWindow(windows::Win32::Foundation::HWND(h.0), SW_HIDE); } }
    }
    fn no_animation(&self, win: &tauri::WebviewWindow) {
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED};
        if let Ok(h) = win.hwnd() {
            let on: windows::core::BOOL = true.into();
            unsafe { let _ = DwmSetWindowAttribute(windows::Win32::Foundation::HWND(h.0), DWMWA_TRANSITIONS_FORCEDISABLED, &on as *const _ as *const _, 4); }
        }
    }
    fn raise_top(&self, win: &tauri::WebviewWindow) {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE};
        if let Ok(h) = win.hwnd() {
            unsafe { let _ = SetWindowPos(windows::Win32::Foundation::HWND(h.0), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); }
        }
    }
    fn init(&self, dir: &std::path::Path, f: Box<dyn Fn(SystemEvent) + Send>) {
        appbar::set_state_dir(dir);
        system::start(f);
    }
    fn appbar_set(&self, edge: Edge, strip: Rect) -> Rect { appbar::set(edge, strip) }
    fn appbar_remove(&self) { appbar::remove() }
    fn appbar_set_window(&self, win: isize, edge: Edge, strip: Rect) -> Rect { appbar::set_for(win, edge, strip) }
    fn appbar_remove_window(&self, win: isize) { appbar::remove_for(win) }
    fn appbar_remove_windows(&self) { appbar::remove_extras() }
    fn thumbs_show(&self, dest: isize, items: &[(isize, Rect)]) -> Vec<isize> { thumbs::show(dest, items) }
    fn thumbs_clear(&self, dest: isize) { thumbs::clear(dest) }
    fn clipboard_is_private(&self) -> bool { system::clipboard_is_private() }
    fn press_keys(&self, keys: &[u16]) {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        let key = |vk: u16, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, ..Default::default() } },
        };
        let inputs: Vec<INPUT> = keys.iter().map(|&k| key(k, false)).chain(keys.iter().rev().map(|&k| key(k, true))).collect();
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
    }
    fn type_text(&self, text: &str) {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        let key = |c: u16, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wScan: c, dwFlags: KEYEVENTF_UNICODE | if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, ..Default::default() } },
        };
        let inputs: Vec<INPUT> = text.encode_utf16().flat_map(|c| [key(c, false), key(c, true)]).collect();
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
    }
    fn keyboard_layouts(&self) -> Vec<KeyboardLayout> { lang::list() }
    fn keyboard_layout_set(&self, id: isize) -> bool { lang::set(id) }
    fn time_12h(&self) -> Option<bool> { lang::time_12h() }
    fn idle_ms(&self) -> u64 {
        use windows::Win32::System::SystemInformation::GetTickCount;
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        unsafe {
            if !GetLastInputInfo(&mut li).as_bool() { return 0; }
            GetTickCount().wrapping_sub(li.dwTime) as u64
        }
    }
    fn foreground_exe(&self) -> Option<String> { windows_list::foreground_exe() }
    fn recent_for(&self, exe: &str, limit: usize) -> Vec<RecentItem> { recent::recent_for(exe, limit) }
    fn transparency_effects(&self) -> bool {
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
        let (mut v, mut size) = (1u32, 4u32);
        let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, windows::core::w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            windows::core::w!("EnableTransparency"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut _ as *mut _), Some(&mut size)) };
        r.is_err() || v != 0
    }
    fn mouse_down(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
        unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0 }
    }
    fn cursor_pos(&self) -> Option<(i32, i32)> {
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut p = windows::Win32::Foundation::POINT::default();
        unsafe { GetCursorPos(&mut p).ok()? };
        Some((p.x, p.y))
    }
    fn menu_open(&self) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};
        // "#32768" is the window class of every popup menu (ours and tray apps').
        unsafe { FindWindowW(windows::core::w!("#32768"), windows::core::PCWSTR::null()).map(|h| IsWindowVisible(h).as_bool()).unwrap_or(false) }
    }
    fn set_taskbar(&self, mode: TaskbarMode) { appbar::set_taskbar(mode) }
    fn wait_for_exit(&self, pid: u32, ms: u64) {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::Foundation::WAIT_TIMEOUT;
        use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE};
        unsafe {
            if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) {
                // Still running after the wait: if it really is GlassBar (not a reused PID),
                // it's our own stuck copy, so end it.
                let ours = std::env::current_exe().ok().map(|e| e.to_string_lossy().to_lowercase())
                    == Some(windows_list::exe_path(pid).to_lowercase());
                if WaitForSingleObject(h, ms as u32) == WAIT_TIMEOUT && ours {
                    let _ = TerminateProcess(h, 1);
                    let _ = WaitForSingleObject(h, 3_000);
                    crate::applog::log("health: the previous GlassBar didn't quit: ended it");
                }
                let _ = CloseHandle(h);
            }
        }
    }
    fn set_starts_with_windows(&self, on: bool) { appbar::KEEP_ON_RESTART.store(on, std::sync::atomic::Ordering::Relaxed) }
    fn list_windows(&self) -> Vec<AppWindow> { windows_list::list() }
    fn watch_windows(&self, f: Box<dyn Fn() + Send>) { windows_list::watch(f) }
    fn watch_minimize(&self, f: Box<dyn Fn(isize, Rect) + Send>) { effects::watch(f) }
    fn restore_rect(&self, id: isize) -> Option<Rect> { effects::visible_rect(id) }
    fn effect_play(&self, src: isize, run: u64, frames: Box<dyn Fn(f64) -> Vec<(Rect, Rect)> + Send>, ms: u32, then: Box<dyn FnOnce() + Send>) { effects::play(src, run, frames, ms, then) }
    fn suppress_native_animations(&self, on: bool) { effects::suppress(on) }
    fn windows_osd(&self, hide: bool) { osd::set(hide) }
    fn windows_osd_nudge(&self) { osd::nudge() }
    fn windows_osd_restore(&self) { osd::restore_now() }
    fn activate(&self, id: isize) { windows_list::activate(id) }
    fn close(&self, id: isize) { windows_list::close(id) }
    fn launch(&self, path: &str, args: &str) -> Res<()> { windows_list::launch(path, args) }
    fn launch_aumid(&self, aumid: &str) -> Res<()> { windows_list::launch(&format!("shell:AppsFolder\\{aumid}"), "") }
    fn aumid_for_path(&self, path: &str) -> Option<String> { packages::aumid_for_path(path) }
    fn aumid_icon(&self, aumid: &str) -> Option<String> { windows_list::aumid_icon(aumid) }
    fn resolve_shortcut(&self, lnk: &str) -> Option<(String, String)> { shortcuts::resolve(lnk) }
    fn installed_apps(&self) -> Vec<InstalledApp> {
        // Start menu shortcuts, plus Store apps (Spotify, WhatsApp…), which have none.
        let mut apps = shortcuts::installed();
        apps.extend(packages::installed());
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }
    fn exe_icon(&self, exe: &str) -> Option<String> { windows_list::exe_icon(exe) }
    fn window_icon(&self, id: isize) -> Option<String> { windows_list::window_icon(id) }
}

impl Audio for WinAudio {
    fn state(&self) -> Res<AudioState> { audio::state() }
    fn set_volume(&self, v: f32) -> Res<()> { audio::set_volume(v) }
    fn set_mute(&self, m: bool) -> Res<()> { audio::set_mute(m) }
    fn set_default(&self, id: &str) -> Res<()> { audio::set_default(id) }
    fn sessions(&self) -> Res<Vec<AudioSession>> { audio::sessions() }
    fn set_session_volume(&self, pid: u32, v: f32) -> Res<()> { audio::set_session_volume(pid, v) }
    fn set_session_mute(&self, pid: u32, m: bool) -> Res<()> { audio::set_session_mute(pid, m) }
    fn watch(&self, f: Box<dyn Fn(AudioState) + Send>, level: Box<dyn Fn(f32, bool) + Send>) { audio::watch(f, level) }
    fn peak(&self) -> Option<f32> { audio::peak() }
    fn loopback(&self, f: &mut dyn FnMut(&[f32], u32) -> bool) -> Res<()> { audio::loopback(f) }
    fn mic_state(&self) -> Res<MicState> { audio::mic_state() }
    fn mic_set_mute(&self, m: bool) -> Res<()> { audio::mic_set_mute(m) }
    fn mic_set_volume(&self, v: f32) -> Res<()> { audio::mic_set_volume(v) }
    fn watch_mic(&self, f: Box<dyn Fn() + Send>) { audio::watch_mic(f) }
    fn using(&self, what: &str) -> Vec<UsingApp> { privacy::in_use(what) }
    fn watch_using(&self, f: Box<dyn Fn() + Send>) { privacy::watch(f) }
}

impl Media for WinMedia {
    fn info(&self) -> Res<Option<MediaInfo>> { media::info() }
    fn control(&self, a: MediaAction) -> Res<()> { media::control(a) }
    fn seek(&self, ms: i64) -> Res<()> { media::seek(ms) }
    fn pause_playing(&self) -> Res<Vec<String>> { media::pause_playing() }
    fn resume(&self, ids: &[String]) -> Res<()> { media::resume(ids) }
    fn watch(&self, f: Box<dyn Fn(Option<MediaInfo>) + Send>) { media::watch(f) }
    fn source_id(&self) -> Option<String> { media::source_id() }
}

impl Radios for WinRadios {
    fn list(&self) -> Res<Vec<RadioInfo>> { radios::list() }
    fn set(&self, kind: Option<RadioKind>, on: bool) -> Res<()> { radios::set(kind, on) }
    fn hotspot(&self) -> Res<Hotspot> { hotspot::state() }
    fn set_hotspot(&self, on: bool) -> Res<()> { hotspot::set(on) }
}

impl Toggles for WinToggles {
    fn night_light(&self) -> Res<bool> { toggles::night_light() }
    fn set_night_light(&self, on: bool) -> Res<()> { toggles::set_night_light(on) }
    fn nearby(&self) -> Res<u32> { toggles::nearby() }
    fn set_nearby(&self, mode: u32) -> Res<()> { toggles::set_nearby(mode) }
    fn cast(&self) -> Res<CastState> { toggles::cast() }
}

impl Display for WinDisplay {
    fn mode(&self) -> Option<DisplayMode> { display::mode() }
    fn set_mode(&self, m: DisplayMode) -> Res<()> { display::set_mode(m) }
    fn connected(&self) -> u32 { display::connected() }
    fn brightness(&self) -> Option<u32> { display::brightness() }
    fn set_brightness(&self, v: u32) -> Res<()> { display::set_brightness(v) }
    fn watch_brightness(&self, f: Box<dyn Fn(u32) + Send>) { display::watch_brightness(f) }
}

impl Location for WinLocation {
    fn locate(&self) -> Res<Fix> { location::locate() }
}

impl Power for WinPower {
    fn battery(&self) -> Option<Battery> { power::battery() }
    fn power_mode(&self) -> Option<PowerMode> { powermode::mode() }
    fn set_power_mode(&self, m: PowerMode) -> Res<()> { powermode::set_mode(m) }
    fn saver_on(&self) -> Option<bool> { powermode::saver_on() }
    fn other_plan(&self) -> Option<String> { powermode::other_plan() }
    fn use_balanced_plan(&self) -> Res<()> { powermode::use_balanced_plan() }
    fn keep_awake(&self, display: bool) -> Res<()> { awake::set(display) }
    fn allow_sleep(&self) { awake::clear() }
}

impl Gpu for WinGpu {
    fn usage(&self) -> Option<f32> { gpu::usage() }
    fn usage_each(&self, pids: &[u32]) -> Option<Vec<f32>> { gpu::usage_each(pids) }
    fn private_ws(&self, pid: u32) -> Option<u64> { procmem::private_ws(pid) }
    fn cpu_temp_c(&self) -> Option<f32> { thermal::cpu_temp_c() }
}

pub struct WinFiles;
impl Files for WinFiles {
    fn downloads_dir(&self) -> Option<String> { files::downloads_dir() }
    fn file_icon(&self, path: &str) -> Option<String> { files::file_icon(path) }
    fn reveal(&self, path: &str) -> Res<()> { files::reveal(path) }
    fn open_with(&self, path: &str) -> Res<()> { files::open_with(path) }
    fn recycle(&self, path: &str) -> Res<()> { files::recycle(path) }
    fn drag_out(&self, hwnd: isize, path: &str) -> Res<()> { files::drag_out(hwnd, path) }
    fn recycle_info(&self) -> Res<RecycleInfo> { files::recycle_info() }
    fn recycle_empty(&self, root: Option<&str>) -> Res<()> { files::recycle_empty(root) }
    fn recycle_empty_ask(&self) -> Res<()> { files::recycle_empty_ask() }
    fn recycle_items(&self) -> Res<Vec<BinItem>> { files::recycle_items() }
    fn recycle_restore(&self, keys: &[String]) -> Res<u32> { files::recycle_restore(keys) }
    fn watch_recycle(&self, f: Box<dyn Fn() + Send>) { files::watch_recycle(f) }
}
