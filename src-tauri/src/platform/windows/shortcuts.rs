//! .lnk shortcuts: resolve to the real program, and list the Start menu's apps.
use super::super::InstalledApp;
use windows::core::{Interface, HSTRING};
use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

/// (target, arguments) of a .lnk, or None (e.g. Store-app shortcuts have no file target).
pub fn resolve(lnk: &str) -> Option<(String, String)> {
    super::com_init();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>().ok()?.Load(&HSTRING::from(lnk), STGM_READ).ok()?;
        let mut path = [0u16; 1024];
        link.GetPath(&mut path, std::ptr::null_mut(), 0).ok()?;
        let mut args = [0u16; 1024];
        let _ = link.GetArguments(&mut args);
        let s = |b: &[u16]| String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]);
        let target = s(&path);
        (!target.is_empty()).then(|| (target, s(&args)))
    }
}

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: u8) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && depth < 4 { walk(&p, out, depth + 1); }
        else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) { out.push(p); }
    }
}

/// Programs in the Start menu (all users + this user) that point at an .exe.
pub fn installed() -> Vec<InstalledApp> {
    let mut lnks = Vec::new();
    for var in ["ProgramData", "APPDATA"] {
        if let Ok(base) = std::env::var(var) {
            walk(&std::path::Path::new(&base).join(r"Microsoft\Windows\Start Menu\Programs"), &mut lnks, 0);
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut apps: Vec<InstalledApp> = lnks.iter().filter_map(|l| {
        let name = l.file_stem()?.to_string_lossy().into_owned();
        let low = name.to_lowercase();
        if ["uninstall", "readme", "help", "documentation", "website", "release notes"].iter().any(|w| low.contains(w)) { return None; }
        let (path, args) = resolve(&l.to_string_lossy())?;
        if !path.to_lowercase().ends_with(".exe") || !std::path::Path::new(&path).exists() { return None; }
        seen.insert(format!("{}|{}", path.to_lowercase(), args)).then_some(InstalledApp { name, path, args, aumid: None })
    }).collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}
