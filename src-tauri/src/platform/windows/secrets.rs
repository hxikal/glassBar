//! Secrets in Windows Credential Manager (generic credentials, this user only). Windows
//! encrypts them with the user's login; they never touch config.json or the database.
use super::super::Res;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

pub fn get(target: &str) -> Option<String> {
    let t = wide(target);
    let mut p: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut p).ok()?;
        let c = &*p;
        let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
        let units: Vec<u16> = bytes.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        let s = String::from_utf16(&units).ok();
        CredFree(p as *const _);
        s
    }
}

pub fn set(target: &str, user: &str, secret: &str) -> Res<()> {
    let mut t = wide(target);
    let mut u = wide(user);
    let mut blob: Vec<u8> = secret.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    let c = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(t.as_mut_ptr()),
        Comment: PWSTR::null(),
        LastWritten: FILETIME::default(),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: PWSTR::null(),
        UserName: PWSTR(u.as_mut_ptr()),
    };
    let r = unsafe { CredWriteW(&c, 0) }.map_err(|e| format!("couldn't save to Windows Credential Manager: {e}"));
    blob.fill(0);
    r
}

pub fn delete(target: &str) -> Res<()> {
    let t = wide(target);
    match unsafe { CredDeleteW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
