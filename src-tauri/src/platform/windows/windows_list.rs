//! Top-level window list (what a taskbar shows), live updates, launch/focus, exe icons.
use super::super::{AppWindow, Res};
use super::wide;
use base64::Engine;
use image::ImageEncoder;
use std::collections::HashMap;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use windows::core::{BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::Shell::{SHDefExtractIconW, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Last foreground window that is not ours (clicking the bar steals focus).
static LAST_FG: AtomicIsize = AtomicIsize::new(0);
/// The last foreground window that isn't GlassBar's.
pub fn last_foreground() -> isize { LAST_FG.load(Ordering::Relaxed) }
static TX: OnceLock<Sender<()>> = OnceLock::new();
static ICONS: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

pub fn free_icons() { *ICONS.lock().unwrap() = None; }

fn h(id: isize) -> HWND { HWND(id as *mut _) }

pub(crate) fn pid_of(w: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(w, Some(&mut pid)) };
    pid
}

pub fn foreground_exe() -> Option<String> {
    let w = unsafe { GetForegroundWindow() };
    if w.is_invalid() { return None; }
    Some(exe_path(pid_of(w))).filter(|p| !p.is_empty())
}

pub(crate) fn exe_path(pid: u32) -> String {
    unsafe {
        let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(proc, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(proc);
        if ok { String::from_utf16_lossy(&buf[..len as usize]) } else { String::new() }
    }
}

fn class_of(w: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(w, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

pub(crate) fn title_of(w: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(w, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

/// Same rules the classic taskbar uses.
pub(crate) fn is_taskbar_window(w: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(w).as_bool() { return false; }
        let ex = GetWindowLongPtrW(w, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0 != 0 { return false; }
        let owned = GetWindow(w, GW_OWNER).map(|o| !o.is_invalid()).unwrap_or(false);
        if owned && ex & WS_EX_APPWINDOW.0 == 0 { return false; }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(w, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        if cloaked != 0 { return false; }
    }
    if title_of(w).is_empty() { return false; }
    !matches!(class_of(w).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
}

pub fn list() -> Vec<AppWindow> {
    unsafe extern "system" fn each(w: HWND, out: LPARAM) -> BOOL {
        unsafe { (*(out.0 as *mut Vec<HWND>)).push(w) };
        true.into()
    }
    let mut all: Vec<HWND> = Vec::new();
    unsafe { let _ = EnumWindows(Some(each), LPARAM(&mut all as *mut _ as isize)); }
    let me = std::process::id();
    let fg = LAST_FG.load(Ordering::Relaxed);
    all.into_iter()
        .filter(|&w| is_taskbar_window(w))
        .filter_map(|w| {
            super::effects::on_listed(w);
            let pid = pid_of(w);
            if pid == me { return None; }
            // Store-style apps (Settings, Calculator…) live inside ApplicationFrameHost:
            // report the hosted app, so it gets its own icon and identity.
            let pid = hosted_pid(w, pid);
            Some(AppWindow {
                id: w.0 as isize,
                title: title_of(w),
                exe: exe_path(pid),
                active: w.0 as isize == fg,
                minimized: unsafe { IsIconic(w).as_bool() },
                // Store apps: the package's ID. Installed web apps: the window's own ID.
                aumid: aumid_of(pid).or_else(|| window_aumid(w).filter(|a| crate::platform::is_web_app_id(a))),
            })
        })
        .collect()
}

unsafe extern "system" fn on_event(_: HWINEVENTHOOK, event: u32, w: HWND, obj: i32, child: i32, _: u32, _: u32) {
    if obj != OBJID_WINDOW.0 || child != 0 || w.is_invalid() { return; }
    if event == EVENT_OBJECT_SHOW { super::appbar::rehide_if_taskbar(w); }
    if event == EVENT_SYSTEM_MINIMIZESTART { super::effects::minimizing(w); }
    if event == EVENT_SYSTEM_FOREGROUND {
        if pid_of(w) != std::process::id() { LAST_FG.store(w.0 as isize, Ordering::Relaxed); }
    } else if event != EVENT_OBJECT_DESTROY && unsafe { GetAncestor(w, GA_ROOT) } != w {
        return;
    }
    if let Some(tx) = TX.get() { let _ = tx.send(()); }
}

pub fn watch(on_change: Box<dyn Fn() + Send>) {
    LAST_FG.store(unsafe { GetForegroundWindow() }.0 as isize, Ordering::Relaxed);
    let (tx, rx) = channel();
    let _ = TX.set(tx);
    std::thread::spawn(move || unsafe {
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        for (lo, hi) in [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
            (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
        ] {
            SetWinEventHook(lo, hi, None, Some(on_event), 0, 0, flags);
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    // Debounce: one update per burst, at most every 120 ms.
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            std::thread::sleep(Duration::from_millis(120));
            while rx.try_recv().is_ok() {}
            on_change();
        }
    });
}

pub fn activate(id: isize) {
    let w = h(id);
    unsafe {
        if IsIconic(w).as_bool() {
            let _ = ShowWindow(w, SW_RESTORE);
            let _ = SetForegroundWindow(w);
        } else if LAST_FG.load(Ordering::Relaxed) == id {
            let _ = ShowWindow(w, SW_MINIMIZE);
        } else {
            let _ = SetForegroundWindow(w);
        }
    }
}

pub fn close(id: isize) {
    unsafe { let _ = PostMessageW(Some(h(id)), WM_CLOSE, WPARAM(0), LPARAM(0)); }
}

pub fn launch(path: &str, args: &str) -> Res<()> { shell_exec("open", path, args, SW_SHOWNORMAL) }

/// Like `launch`, but Windows asks for admin first (the UAC prompt), and no window shows
/// (it runs commands such as `sc stop`).
pub fn launch_elevated(path: &str, args: &str) -> Res<()> { shell_exec("runas", path, args, SW_HIDE) }

fn shell_exec(verb: &str, path: &str, args: &str, show: SHOW_WINDOW_CMD) -> Res<()> {
    let (op, file, params) = (wide(verb), wide(path), wide(args));
    let r = unsafe {
        ShellExecuteW(None, PCWSTR(op.as_ptr()), PCWSTR(file.as_ptr()), PCWSTR(params.as_ptr()), PCWSTR::null(), show)
    };
    if r.0 as isize > 32 { Ok(()) } else { Err(format!("could not open {path}")) }
}

pub fn exe_icon(exe: &str) -> Option<String> {
    let key = exe.to_lowercase();
    if let Some(hit) = ICONS.lock().unwrap().get_or_insert_with(HashMap::new).get(&key) {
        return hit.clone();
    }
    let icon = extract(exe, 48);
    ICONS.lock().unwrap().as_mut().unwrap().insert(key, icon.clone());
    icon
}

pub(super) fn data_url(png: Vec<u8>) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

fn extract(exe: &str, size: u32) -> Option<String> {
    let path = wide(exe);
    let mut icon = HICON::default();
    unsafe {
        SHDefExtractIconW(PCWSTR(path.as_ptr()), 0, 0, Some(&mut icon), None, size).ok().ok()?;
        let png = icon_to_png(icon);
        let _ = DestroyIcon(icon);
        png.map(data_url)
    }
}

/// Icon for a window whose program file has none. Microsoft Store apps (WhatsApp,
/// Calculator, ...) keep their icons in the app package, so ask the shell for the
/// app's registered icon by its AppUserModelID; otherwise use the window's own icon.
pub fn window_icon(id: isize) -> Option<String> {
    let w = h(id);
    let pid = hosted_pid(w, pid_of(w));
    aumid_of(pid).and_then(|a| package_icon(&a)).or_else(|| hwnd_icon(w)).map(data_url)
}

/// The AppUserModelID a window sets for itself (its taskbar identity), if any.
fn window_aumid(w: HWND) -> Option<String> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
    use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
    // PKEY_AppUserModel_ID = {9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}, 5
    let key = windows::Win32::Foundation::PROPERTYKEY { fmtid: windows::core::GUID::from_u128(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3), pid: 5 };
    super::com_init();
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(w).ok()?;
        let v = store.GetValue(&key).ok()?;
        let p = PropVariantToStringAlloc(&v).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s.filter(|s| !s.is_empty())
    }
}

/// Frame window -> the app process it hosted the last time we could see it.
static HOSTED: Mutex<Option<HashMap<isize, u32>>> = Mutex::new(None);

/// For an ApplicationFrameHost window, the pid of the app it hosts (its CoreWindow).
/// Open: the CoreWindow is a child of the frame. Minimised: Windows detaches it into a
/// top-level CoreWindow with the same title, so look for that, then for the last match.
/// Without this a minimised Settings/Calculator window showed up as some other app.
fn hosted_pid(w: HWND, pid: u32) -> u32 {
    if !exe_path(pid).to_lowercase().ends_with("\\applicationframehost.exe") { return pid; }
    unsafe extern "system" fn each(c: HWND, out: LPARAM) -> BOOL {
        let st = unsafe { &mut *(out.0 as *mut (u32, u32)) };
        let p = pid_of(c);
        if p != st.0 { st.1 = p; return false.into(); }
        true.into()
    }
    let mut st = (pid, 0u32);
    unsafe { let _ = EnumChildWindows(Some(w), Some(each), LPARAM(&mut st as *mut _ as isize)); }
    let found = (st.1 != 0).then_some(st.1).or_else(|| detached_core_pid(w, pid));
    let mut cache = HOSTED.lock().unwrap();
    let map = cache.get_or_insert_with(HashMap::new);
    match found {
        Some(p) => { map.insert(w.0 as isize, p); p }
        None => map.get(&(w.0 as isize)).copied().filter(|&p| !exe_path(p).is_empty()).unwrap_or(pid),
    }
}

/// A top-level CoreWindow from another process with the frame's title (a minimised Store app).
fn detached_core_pid(frame: HWND, host: u32) -> Option<u32> {
    let title = title_of(frame);
    if title.is_empty() { return None; }
    let class = wide("Windows.UI.Core.CoreWindow");
    let mut after: Option<HWND> = None;
    for _ in 0..64 {
        let c = unsafe { FindWindowExW(None, after, PCWSTR(class.as_ptr()), PCWSTR::null()) }.ok()?;
        let p = pid_of(c);
        if p != host && title_of(c) == title { return Some(p); }
        after = Some(c);
    }
    None
}

/// AppUserModelID of a packaged process, None for normal programs.
pub(crate) fn aumid_of(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let r = GetApplicationUserModelId(proc, &mut len, Some(PWSTR(buf.as_mut_ptr())));
        let _ = CloseHandle(proc);
        (r == ERROR_SUCCESS && len >= 2).then(|| String::from_utf16_lossy(&buf[..len as usize - 1]))
    }
}

pub fn aumid_icon(aumid: &str) -> Option<String> { package_icon(aumid).map(data_url) }

fn package_icon(aumid: &str) -> Option<Vec<u8>> {
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY};
    unsafe {
        super::com_init();
        let path = wide(&format!("shell:AppsFolder\\{aumid}"));
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(PCWSTR(path.as_ptr()), None).ok()?;
        let hbm = factory.GetImage(SIZE { cx: 48, cy: 48 }, SIIGBF_ICONONLY).ok()?;
        let png = bitmap_to_png(hbm);
        let _ = DeleteObject(hbm.into());
        png
    }
}

fn hwnd_icon(w: HWND) -> Option<Vec<u8>> {
    unsafe {
        let mut res = 0usize;
        let _ = SendMessageTimeoutW(w, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0), SMTO_ABORTIFHUNG, 200, Some(&mut res));
        let raw = if res != 0 { res as isize } else { GetClassLongPtrW(w, GCLP_HICON) as isize };
        if raw == 0 { return None; }
        icon_to_png(HICON(raw as *mut _)) // shared icon owned by the window: don't destroy
    }
}

pub(super) unsafe fn icon_to_png(icon: HICON) -> Option<Vec<u8>> {
    let mut info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut info).ok()? };
    let png = unsafe { bitmap_to_png(info.hbmColor) };
    unsafe {
        let _ = DeleteObject(info.hbmColor.into());
        let _ = DeleteObject(info.hbmMask.into());
    }
    png
}

/// 32-bit bitmap of any size -> PNG (BGRA -> RGBA; opaque if it has no alpha).
unsafe fn bitmap_to_png(hbm: HBITMAP) -> Option<Vec<u8>> {
    let mut bm = BITMAP::default();
    if unsafe { GetObjectW(hbm.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) } == 0 { return None; }
    let (w, ht) = (bm.bmWidth, bm.bmHeight.abs());
    if w <= 0 || ht <= 0 || w > 512 || ht > 512 { return None; }
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w, biHeight: -ht, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut px = vec![0u8; (w * ht * 4) as usize];
    let n = unsafe {
        let dc = GetDC(None);
        let n = GetDIBits(dc, hbm, 0, ht as u32, Some(px.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
        ReleaseDC(None, dc);
        n
    };
    if n <= 0 { return None; }
    let has_alpha = px.chunks(4).any(|p| p[3] != 0);
    for p in px.chunks_mut(4) {
        p.swap(0, 2);
        if !has_alpha { p[3] = 255; }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&px, w as u32, ht as u32, image::ExtendedColorType::Rgba8).ok()?;
    Some(out)
}

/// A (copied) HICON as a PNG data URL, for tray icons.
pub(crate) fn hicon_data_url(icon: isize) -> Option<String> {
    unsafe { icon_to_png(HICON(icon as *mut _)) }.map(data_url)
}

#[cfg(test)]
mod tests {
    #[test]
    fn web_app_ids_are_told_apart_from_the_browser() {
        assert!(crate::platform::is_web_app_id("Chrome._crx_agimnkijcaahngcdmfeangaknmldooml"));
        assert!(crate::platform::is_web_app_id("MSEdge._crx_cifhbcnohmdccbgoicgdjpfamggdegmo"));
        assert!(!crate::platform::is_web_app_id("Chrome"));
        assert!(!crate::platform::is_web_app_id("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"));
    }
}
