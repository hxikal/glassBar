//! Environment variables as stored in the registry (user: HKCU\Environment, system:
//! HKLM\...\Session Manager\Environment), unexpanded, so %VARS% in PATH survive a rewrite.
use super::super::Res;
use super::wide;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, WPARAM};
use windows::Win32::System::Registry::*;
use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE};

const USER: &str = "Environment";
const MACHINE: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

/// (value, is REG_EXPAND_SZ).
pub fn get(machine: bool, name: &str) -> Option<(String, bool)> {
    let (root, sub) = if machine { (HKEY_LOCAL_MACHINE, MACHINE) } else { (HKEY_CURRENT_USER, USER) };
    let (s, n) = (wide(sub), wide(name));
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND;
    let mut size = 0u32;
    let mut kind = REG_VALUE_TYPE::default();
    unsafe {
        if RegGetValueW(root, PCWSTR(s.as_ptr()), PCWSTR(n.as_ptr()), flags, Some(&mut kind), None, Some(&mut size)) != ERROR_SUCCESS { return None; }
        let mut buf = vec![0u16; size as usize / 2 + 1];
        if RegGetValueW(root, PCWSTR(s.as_ptr()), PCWSTR(n.as_ptr()), flags, Some(&mut kind), Some(buf.as_mut_ptr() as _), Some(&mut size)) != ERROR_SUCCESS { return None; }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some((String::from_utf16_lossy(&buf[..len]), kind == REG_EXPAND_SZ))
    }
}

pub fn set(name: &str, value: &str, expand: bool) -> Res<()> {
    let (s, n, v) = (wide(USER), wide(name), wide(value));
    let kind = if expand { REG_EXPAND_SZ } else { REG_SZ };
    let r = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(s.as_ptr()), PCWSTR(n.as_ptr()), kind.0, Some(v.as_ptr() as _), (v.len() * 2) as u32) };
    if r != ERROR_SUCCESS { return Err(format!("couldn't save {name} ({})", r.0)); }
    // New terminals and Explorer pick the change up after this broadcast.
    let env = wide("Environment");
    unsafe {
        let _ = SendMessageTimeoutW(HWND(0xffff as _), WM_SETTINGCHANGE, WPARAM(0), LPARAM(env.as_ptr() as isize), SMTO_ABORTIFHUNG, 3000, None);
    }
    Ok(())
}
