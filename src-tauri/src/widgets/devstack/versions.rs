//! PHP / Node / Python versions. Switching works like Laragon's menu: laragon.ini's choice,
//! Apache's fcgid.conf (PHP), and the folder in the user PATH (what new terminals run).
//! Every file and the old PATH are copied to <root>\usr\glassbar-backup first.
//! Node also comes from nvm-windows when it is installed (`nvm use`, which asks for admin).
use super::{ini_of, output, restart_if_running, root_of, running_names, version_dir, NO_WINDOW};
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
pub struct Kind { installed: Vec<String>, current: String }

#[derive(Serialize)]
pub struct Found {
    version: String,
    path: String,
    /// Something earlier in PATH wins over the version picked here.
    shadowed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Versions {
    php: Kind,
    nodejs: Kind,
    python: Kind,
    nvm: Option<Kind>,
    /// What `php` / `node` run in a new terminal.
    php_terminal: Option<Found>,
    node_terminal: Option<Found>,
    laragon_running: bool,
}

/// The program that tells a version folder of `kind` is usable.
fn key_exe(kind: &str) -> Option<&'static str> {
    Some(match kind { "php" => "php-cgi.exe", "nodejs" => "node.exe", "python" => "python.exe", _ => return None })
}

/// `name` with %VARS% replaced from this process' environment.
pub fn expand(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(a) = rest.find('%') {
        let Some(b) = rest[a + 1..].find('%') else { break };
        let var = &rest[a + 1..a + 1 + b];
        out.push_str(&rest[..a]);
        match std::env::var(var) { Ok(v) if !var.is_empty() => out.push_str(&v), _ => out.push_str(&rest[a..a + 2 + b]) }
        rest = &rest[a + 2 + b..];
    }
    out.push_str(rest);
    out
}

/// PATH as a newly opened terminal gets it (system, then user), not GlassBar's old copy.
pub fn fresh_path() -> String {
    let sh = crate::platform::shell();
    match (sh.machine_env("Path"), sh.user_env("Path")) {
        (None, None) => std::env::var("PATH").unwrap_or_default(),
        (m, u) => [m.unwrap_or_default(), u.map(|u| u.0).unwrap_or_default()].iter().map(|s| expand(s)).collect::<Vec<_>>().join(";"),
    }
}

fn which(exe: &str, path: &str) -> Option<PathBuf> {
    path.split(';').map(str::trim).filter(|d| !d.is_empty()).map(|d| Path::new(d).join(exe)).find(|p| p.is_file())
}

/// `[section] key=value` set in an ini file, every other byte kept.
pub fn set_ini_value(ini: &str, section: &str, key: &str, value: &str) -> String {
    let nl = if ini.contains("\r\n") { "\r\n" } else { "\n" };
    let kv = format!("{key}={value}");
    let (mut out, mut inside, mut done, mut seen) = (Vec::<String>::new(), false, false, false);
    for line in ini.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if inside && !done { out.push(kv.clone()); done = true; }
            inside = t.eq_ignore_ascii_case(&format!("[{section}]"));
            seen |= inside;
        } else if inside && !done && t.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key)) {
            out.push(kv.clone());
            done = true;
            continue;
        }
        out.push(line.to_string());
    }
    if inside && !done { out.push(kv.clone()); }
    if !seen { out.push(format!("[{section}]")); out.push(kv); }
    let mut s = out.join(nl);
    if ini.ends_with('\n') || !seen { s.push_str(nl); }
    s
}

/// Every `bin/<kind>/<version>` (either slash) in a config file pointed at `new`.
pub fn retarget(conf: &str, kind: &str, new: &str) -> String {
    let low = conf.to_ascii_lowercase();
    let pats = [format!("/bin/{kind}/"), format!("\\bin\\{kind}\\")];
    let (mut out, mut i) = (String::new(), 0);
    while let Some((start, len)) = pats.iter().filter_map(|p| low[i..].find(p.as_str()).map(|j| (i + j, p.len()))).min() {
        let seg = start + len;
        let end = conf[seg..].find(|c: char| matches!(c, '/' | '\\' | '"' | '\'' | ';') || c.is_whitespace()).map_or(conf.len(), |j| seg + j);
        out.push_str(&conf[i..seg]);
        out.push_str(new);
        i = end;
    }
    out.push_str(&conf[i..]);
    out
}

/// PATH with its `<base>\<version>[\sub]` entries moved to `new`; added if there is none.
pub fn swap_path_entry(path: &str, base: &str, new: &str) -> String {
    let b = format!("{}\\", base.trim_end_matches('\\'));
    let mut found = false;
    let mut parts: Vec<String> = path.split(';').map(|e| {
        match e.get(..b.len()) {
            Some(head) if head.replace('/', "\\").eq_ignore_ascii_case(&b) => {
                found = true;
                let rest = &e[b.len()..];
                let sub = rest.find(['\\', '/']).map_or("", |i| &rest[i..]);
                format!("{head}{new}{sub}")
            }
            _ => e.to_string(),
        }
    }).collect();
    if !found {
        if parts.last().is_some_and(|l| l.is_empty()) { parts.pop(); }
        parts.push(format!("{b}{new}"));
    }
    parts.join(";")
}

fn kind_of(root: &Path, kind: &str) -> Kind {
    let exe = key_exe(kind).unwrap();
    let mut installed: Vec<String> = std::fs::read_dir(root.join("bin").join(kind)).map(|d| d.flatten()
        .filter(|e| e.path().join(exe).is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    installed.sort_by_key(|s| std::cmp::Reverse(s.to_lowercase()));
    let current = version_dir(root, &ini_of(root), kind).and_then(|d| Some(d.file_name()?.to_string_lossy().into_owned())).unwrap_or_default();
    Kind { installed, current }
}

fn nvm_home() -> Option<PathBuf> {
    std::env::var("NVM_HOME").ok().or_else(|| Some(expand(&crate::platform::shell().user_env("NVM_HOME")?.0)))
        .map(PathBuf::from).filter(|p| p.join("nvm.exe").is_file())
}

fn nvm_link() -> Option<PathBuf> {
    std::env::var("NVM_SYMLINK").ok().or_else(|| Some(expand(&crate::platform::shell().user_env("NVM_SYMLINK")?.0))).map(PathBuf::from)
}

/// "18.8.0" → (18, 8, 0) for sorting.
fn semver(v: &str) -> (u32, u32, u32) {
    let mut n = v.trim_start_matches('v').split('.').map(|x| x.parse().unwrap_or(0));
    (n.next().unwrap_or(0), n.next().unwrap_or(0), n.next().unwrap_or(0))
}

fn nvm() -> Option<Kind> {
    let home = nvm_home()?;
    let mut installed: Vec<String> = std::fs::read_dir(&home).ok()?.flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('v') && home.join(n).join("node.exe").is_file())
        .map(|n| n.trim_start_matches('v').to_string()).collect();
    installed.sort_by_key(|v| std::cmp::Reverse(semver(v)));
    let current = nvm_link().and_then(|l| std::fs::read_link(l).ok())
        .and_then(|t| Some(t.file_name()?.to_string_lossy().trim_start_matches('v').to_string())).unwrap_or_default();
    Some(Kind { installed, current })
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_string();
    n(a).eq_ignore_ascii_case(&n(b))
}

pub fn versions(root: &str) -> Result<Versions, String> {
    let r = root_of(root)?;
    let path = fresh_path();
    let (php, nodejs, python, nvm) = (kind_of(&r, "php"), kind_of(&r, "nodejs"), kind_of(&r, "python"), nvm());
    let php_terminal = which("php.exe", &path).map(|p| {
        let dir = p.parent().unwrap().to_path_buf();
        Found {
            version: dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            shadowed: !php.current.is_empty() && !same_dir(&dir, &r.join("bin/php").join(&php.current)),
            path: dir.display().to_string(),
        }
    });
    let node_terminal = which("node.exe", &path).map(|p| {
        let dir = p.parent().unwrap().to_path_buf();
        let version = output(&p.display().to_string(), &["-v"]).map(|o| String::from_utf8_lossy(&o).trim().trim_start_matches('v').to_string()).unwrap_or_default();
        let wanted = match (&nvm, nvm_link()) {
            (Some(_), Some(link)) => Some(link),
            _ if !nodejs.current.is_empty() => Some(r.join("bin/nodejs").join(&nodejs.current)),
            _ => None,
        };
        Found { version, shadowed: wanted.is_some_and(|w| !same_dir(&dir, &w)), path: dir.display().to_string() }
    });
    Ok(Versions { php, nodejs, python, nvm, php_terminal, node_terminal, laragon_running: running_names().iter().any(|n| n == "laragon.exe") })
}

fn backup(dir: &Path, name: &str, content: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    std::fs::write(dir.join(format!("{stamp}-{name}")), content).map_err(|e| format!("Backup failed, nothing changed: {e}"))
}

/// Switch Laragon's `kind` (php | nodejs | python) to `version`. Returns a note for the user.
pub fn switch(root: &str, kind: &str, version: &str) -> Result<String, String> {
    let r = root_of(root)?;
    let exe = key_exe(kind).ok_or("unknown kind")?;
    if version.is_empty() || version.contains(['\\', '/']) || version.contains("..") { return Err("bad version".into()); }
    if !r.join("bin").join(kind).join(version).join(exe).is_file() { return Err(format!("{version} isn't installed")); }
    let bak = r.join("usr/glassbar-backup");
    let sh = crate::platform::shell();

    // Read everything and back it up before writing anything.
    let ini_path = r.join("usr/laragon.ini");
    let ini = std::fs::read_to_string(&ini_path).map_err(|e| format!("laragon.ini: {e}"))?;
    let fcgid_path = r.join("etc/apache2/fcgid.conf");
    let fcgid = if kind == "php" { std::fs::read_to_string(&fcgid_path).ok() } else { None };
    let user_path = sh.user_env("Path");
    backup(&bak, "laragon.ini", &ini)?;
    if let Some(f) = &fcgid { backup(&bak, "fcgid.conf", f)?; }
    if let Some((p, _)) = &user_path { backup(&bak, "user-path.txt", p)?; }

    std::fs::write(&ini_path, set_ini_value(&ini, kind, "Version", version)).map_err(|e| format!("laragon.ini: {e}"))?;
    if let Some(f) = &fcgid { std::fs::write(&fcgid_path, retarget(f, kind, version)).map_err(|e| format!("fcgid.conf: {e}"))?; }
    let (p, expand_sz) = user_path.unwrap_or_default();
    let base = r.join("bin").join(kind).display().to_string();
    sh.set_user_env("Path", &swap_path_entry(&p, &base, version), expand_sz || p.contains('%'))?;

    if kind == "php" {
        restart_if_running(root, "apache")?;
        restart_if_running(root, "nginx")?;
    }
    Ok(if running_names().iter().any(|n| n == "laragon.exe") {
        format!("Laragon is open: if it switches back, pick {version} in its menu too.")
    } else { String::new() })
}

/// `nvm use <version>`; nvm asks Windows for admin to move its link.
pub fn nvm_use(version: &str) -> Result<(), String> {
    let home = nvm_home().ok_or("nvm isn't installed")?;
    if version.is_empty() || version.contains(['\\', '/']) || version.contains("..") { return Err("bad version".into()); }
    if !home.join(format!("v{version}")).join("node.exe").is_file() { return Err(format!("Node {version} isn't installed")); }
    let out = Command::new(home.join("nvm.exe")).args(["use", version]).current_dir(&home).creation_flags(NO_WINDOW)
        .output().map_err(|e| e.to_string())?;
    if nvm().is_some_and(|k| k.current == version) { return Ok(()); }
    let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Err(if msg.is_empty() { "Node didn't switch (was the admin prompt declined?)".into() } else { msg })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_one_ini_line() {
        let ini = "[nodejs]\r\nVersion=node-v24\r\n\r\n[php]\r\nVersion=php-8.4\r\nQuickSettings=xdebug\r\n\r\n[apache]\r\nUse=-1\r\n";
        assert_eq!(set_ini_value(ini, "php", "Version", "php-8.2"),
            "[nodejs]\r\nVersion=node-v24\r\n\r\n[php]\r\nVersion=php-8.2\r\nQuickSettings=xdebug\r\n\r\n[apache]\r\nUse=-1\r\n");
        assert_eq!(set_ini_value("[php]\nX=1\n[a]\n", "php", "Version", "p"), "[php]\nX=1\nVersion=p\n[a]\n");
        assert_eq!(set_ini_value("[a]\nX=1\n", "python", "Version", "py"), "[a]\nX=1\n[python]\nVersion=py\n");
    }

    #[test]
    fn retargets_fcgid() {
        let conf = "FcgidInitialEnv PATH \"C:/laragon/bin/php/php-8.4.16-nts;C:/Windows;\"\nFcgidInitialEnv PHPRC \"C:/laragon/bin/php/php-8.4.16-nts\"\nFcgidWrapper \"C:/laragon/bin/php/php-8.4.16-nts/php-cgi.exe\" .php\n";
        assert_eq!(retarget(conf, "php", "php-8.2"),
            "FcgidInitialEnv PATH \"C:/laragon/bin/php/php-8.2;C:/Windows;\"\nFcgidInitialEnv PHPRC \"C:/laragon/bin/php/php-8.2\"\nFcgidWrapper \"C:/laragon/bin/php/php-8.2/php-cgi.exe\" .php\n");
        assert_eq!(retarget("no php here", "php", "x"), "no php here");
    }

    #[test]
    fn swaps_the_path_entry() {
        let p = r"C:\laragon\bin;C:\laragon\bin\php\php-8.4;C:\laragon\bin\python\python-3.10\Scripts;%USERPROFILE%\bin";
        assert_eq!(swap_path_entry(p, r"C:\laragon\bin\php", "php-8.2"), r"C:\laragon\bin;C:\laragon\bin\php\php-8.2;C:\laragon\bin\python\python-3.10\Scripts;%USERPROFILE%\bin");
        assert_eq!(swap_path_entry(p, r"C:\laragon\bin\python", "python-3.12"), r"C:\laragon\bin;C:\laragon\bin\php\php-8.4;C:\laragon\bin\python\python-3.12\Scripts;%USERPROFILE%\bin");
        assert_eq!(swap_path_entry(r"C:\a;", r"C:\laragon\bin\php\", "php-8.2"), r"C:\a;C:\laragon\bin\php\php-8.2");
        // C:\laragon\bin itself is not a php entry
        assert_eq!(swap_path_entry(r"C:\laragon\bin", r"C:\laragon\bin\php", "x"), r"C:\laragon\bin;C:\laragon\bin\php\x");
    }

    #[test]
    fn expands_and_sorts() {
        std::env::set_var("GB_TEST_VAR", "X");
        assert_eq!(expand("%GB_TEST_VAR%\\bin;%NOPE_GB%;100%"), "X\\bin;%NOPE_GB%;100%");
        assert!(semver("24.18.0") > semver("18.8.0"));
    }
}
