//! Dev Stack: Laragon's services (running = its program is running), start/stop each one,
//! and the sites in its www folder (http://<folder>.test, Laragon's auto virtual hosts).
//! Laragon has no command line, so services are started the way Laragon starts them.
//! MySQL is only ever stopped with `mysqladmin shutdown`, never killed, so no data is lost.
//! The other tabs live in: ports (localhost servers), projects (npm dev servers),
//! versions (PHP / Node / Python), other (Windows services, Docker).
pub mod other;
pub mod ports;
pub mod projects;
pub mod versions;

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const NO_WINDOW: u32 = 0x0800_0000;

struct Svc { id: &'static str, name: &'static str, exe: &'static str, port: u16 }

const SERVICES: &[Svc] = &[
    Svc { id: "apache", name: "Apache", exe: "bin/httpd.exe", port: 80 },
    Svc { id: "nginx", name: "Nginx", exe: "nginx.exe", port: 80 },
    Svc { id: "mysql", name: "MySQL", exe: "bin/mysqld.exe", port: 3306 },
    Svc { id: "redis", name: "Redis", exe: "redis-server.exe", port: 6379 },
    Svc { id: "memcached", name: "Memcached", exe: "memcached.exe", port: 11211 },
];

#[derive(Serialize)]
pub struct Service { id: String, name: String, version: String, port: u16, running: bool, enabled: bool }

#[derive(Serialize)]
pub struct Status { found: bool, root: String, www: String, services: Vec<Service>, sites: Vec<String> }

/// `key` in `[section]` of an ini file (case-insensitive).
pub fn ini_value(ini: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for l in ini.lines().map(str::trim) {
        if l.starts_with('[') { inside = l.eq_ignore_ascii_case(&format!("[{section}]")); continue; }
        if !inside { continue; }
        if let Some((k, v)) = l.split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) { return Some(v.trim().trim_matches('"').to_string()); }
        }
    }
    None
}

fn root_of(root: &str) -> Result<PathBuf, String> {
    let r = PathBuf::from(if root.trim().is_empty() { "C:\\laragon" } else { root.trim() });
    if r.join("laragon.exe").is_file() { Ok(r) } else { Err(format!("Laragon isn't in {}", r.display())) }
}

fn ini_of(root: &Path) -> String { std::fs::read_to_string(root.join("usr/laragon.ini")).unwrap_or_default() }

/// The version folder Laragon uses: laragon.ini's choice, else the newest installed one.
fn version_dir(root: &Path, ini: &str, kind: &str) -> Option<PathBuf> {
    let base = root.join("bin").join(kind);
    if let Some(v) = ini_value(ini, kind, "Version") { if base.join(&v).is_dir() { return Some(base.join(v)); } }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&base).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    dirs.pop()
}

/// The www folder: Apache's DocumentRoot from laragon.ini, else <root>\www.
fn www_of(root: &Path, ini: &str) -> PathBuf {
    ini_value(ini, "apache", "DocumentRoot").map(PathBuf::from).filter(|p| p.is_dir()).unwrap_or(root.join("www"))
}

fn exe_name(s: &Svc) -> &'static str { s.exe.rsplit('/').next().unwrap() }

fn running_names() -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    let mut sys = sysinfo::System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).collect()
}

fn status(root: &str) -> Status {
    let Ok(r) = root_of(root) else { return Status { found: false, root: root.into(), www: String::new(), services: vec![], sites: vec![] } };
    let ini = ini_of(&r);
    let names = running_names();
    let services = SERVICES.iter().filter_map(|s| {
        let dir = version_dir(&r, &ini, s.id)?;
        if !dir.join(s.exe).is_file() { return None; }
        let port = if s.id == "mysql" {
            std::fs::read_to_string(dir.join("my.ini")).ok()
                .and_then(|m| ini_value(&m, "mysqld", "port")).and_then(|p| p.parse().ok()).unwrap_or(s.port)
        } else { s.port };
        Some(Service {
            id: s.id.into(), name: s.name.into(), version: dir.file_name()?.to_string_lossy().into(), port,
            running: names.iter().any(|n| n == exe_name(s)),
            enabled: ini_value(&ini, s.id, "Use").as_deref() == Some("-1"),
        })
    }).collect();
    let www = www_of(&r, &ini);
    let mut sites: Vec<String> = std::fs::read_dir(&www).map(|d| d.flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect()).unwrap_or_default();
    sites.sort_by_key(|s| s.to_lowercase());
    Status { found: true, root: r.display().to_string(), www: www.display().to_string(), services, sites }
}

fn run(exe: &Path, args: &[&str], cwd: &Path, wait: bool) -> Result<(), String> {
    let mut c = Command::new(exe);
    c.args(args).current_dir(cwd).creation_flags(NO_WINDOW);
    if !wait { return c.spawn().map(|_| ()).map_err(|e| e.to_string()); }
    let out = c.output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// Output of a hidden command (stdout), or None if it couldn't run.
fn output(exe: &str, args: &[&str]) -> Option<Vec<u8>> {
    Command::new(exe).args(args).creation_flags(NO_WINDOW).output().ok().map(|o| o.stdout)
}

fn kill(exe: &str) -> Result<(), String> {
    run(Path::new("taskkill.exe"), &["/F", "/T", "/IM", exe], Path::new("C:\\"), true)
}

/// Ports Nginx sends PHP requests to (etc/nginx/php_upstream.conf), 9001 if unreadable.
pub fn upstream_ports(conf: &str) -> Vec<u16> {
    let v: Vec<u16> = conf.lines().map(str::trim).filter(|l| l.starts_with("server "))
        .filter_map(|l| l.split(':').nth(1)?.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()).collect();
    if v.is_empty() { vec![9001] } else { v }
}

/// Nginx has no PHP of its own: Laragon runs php-cgi on the upstream ports. Do the same.
fn start_php_cgi(r: &Path) -> Result<(), String> {
    let php = version_dir(r, &ini_of(r), "php").ok_or("PHP isn't installed")?;
    let conf = std::fs::read_to_string(r.join("etc/nginx/php_upstream.conf")).unwrap_or_default();
    for port in upstream_ports(&conf) {
        Command::new(php.join("php-cgi.exe")).args(["-b", &format!("127.0.0.1:{port}")])
            .env("PHP_FCGI_MAX_REQUESTS", "0").current_dir(&php).creation_flags(NO_WINDOW)
            .spawn().map_err(|e| format!("PHP for Nginx didn't start: {e}"))?;
    }
    Ok(())
}

fn toggle(root: &str, id: &str, start: bool) -> Result<(), String> {
    let r = root_of(root)?;
    let s = SERVICES.iter().find(|s| s.id == id).ok_or("unknown service")?;
    let dir = version_dir(&r, &ini_of(&r), s.id).ok_or(format!("{} isn't installed", s.name))?;
    let exe = dir.join(s.exe);
    let exe_dir = exe.parent().unwrap().to_path_buf();
    match (s.id, start) {
        ("mysql", true) => {
            let arg = format!("--defaults-file={}", dir.join("my.ini").display());
            let args: &[&str] = if dir.join("my.ini").is_file() { &[&arg] } else { &[] };
            run(&exe, args, &exe_dir, false)
        }
        ("mysql", false) => run(&exe_dir.join("mysqladmin.exe"), &["-u", "root", "shutdown"], &exe_dir, true)
            .map_err(|_| "MySQL didn't stop (the root user may have a password). Stop it from Laragon.".to_string()),
        ("nginx", true) => { start_php_cgi(&r)?; run(&exe, &[], &dir, false) }
        ("nginx", false) => {
            let res = run(&exe, &["-s", "quit"], &dir, true);
            // Apache's mod_fcgid runs php-cgi too; leave those alone.
            if !running_names().iter().any(|n| n == "httpd.exe") { let _ = kill("php-cgi.exe"); }
            res
        }
        ("redis", true) => run(&exe, &["redis.windows.conf"], &dir, false),
        ("redis", false) => run(&dir.join("redis-cli.exe"), &["shutdown"], &dir, true).or_else(|_| kill(exe_name(s))),
        // Apache and Memcached don't mind where they start from.
        (_, true) => run(&exe, &[], &dir, false),
        (_, false) => kill(exe_name(s)),
    }
}

/// Restart a Laragon service if it is running (after a PHP switch).
fn restart_if_running(root: &str, id: &str) -> Result<(), String> {
    let s = SERVICES.iter().find(|s| s.id == id).unwrap();
    if !running_names().iter().any(|n| n == exe_name(s)) { return Ok(()); }
    toggle(root, id, false)?;
    std::thread::sleep(std::time::Duration::from_millis(800));
    toggle(root, id, true)
}

#[tauri::command]
pub async fn devstack_status(root: String) -> Result<Status, String> {
    crate::widgets::blocking(move || status(&root)).await
}

#[tauri::command]
pub async fn devstack_toggle(root: String, id: String, start: bool) -> Result<(), String> {
    crate::widgets::blocking(move || toggle(&root, &id, start)).await?
}

#[tauri::command]
pub fn devstack_open_laragon(root: String) -> Result<(), String> {
    let r = root_of(&root)?;
    crate::platform::shell().launch(&r.join("laragon.exe").display().to_string(), "")
}

/// Laragon's own tools: "terminal" (Cmder at `path`), "heidisql", "phpini" (current PHP's php.ini).
#[tauri::command]
pub fn devstack_open_tool(root: String, tool: String, path: String) -> Result<(), String> {
    let r = root_of(&root)?;
    let sh = crate::platform::shell();
    match tool.as_str() {
        "terminal" => {
            let cmder = r.join("bin/cmder/Cmder.exe");
            if cmder.is_file() { sh.launch(&cmder.display().to_string(), &format!("/START \"{path}\"")) }
            else { sh.launch("cmd.exe", &format!("/K cd /d \"{path}\"")) }
        }
        "heidisql" => sh.launch(&r.join("bin/heidisql/heidisql.exe").display().to_string(), ""),
        "phpini" => {
            let php = version_dir(&r, &ini_of(&r), "php").ok_or("PHP isn't installed")?;
            sh.launch(&php.join("php.ini").display().to_string(), "")
        }
        _ => Err("unknown tool".into()),
    }
}

use crate::widgets::blocking;

/// Programs listening on localhost ports; `dirs` = extra project folders.
#[tauri::command]
pub async fn devstack_ports(root: String, dirs: Vec<String>) -> Result<Vec<ports::Listener>, String> {
    blocking(move || {
        let mut d: Vec<PathBuf> = dirs.iter().filter(|s| !s.trim().is_empty()).map(PathBuf::from).collect();
        if let Ok(r) = root_of(&root) { d.push(www_of(&r, &ini_of(&r))); }
        ports::list(&d)
    }).await
}

#[tauri::command]
pub async fn devstack_stop_pid(pid: u32) -> Result<(), String> { blocking(move || ports::stop(pid)).await? }

#[tauri::command]
pub async fn devstack_projects(root: String, dirs: Vec<String>) -> Result<Vec<projects::Project>, String> {
    blocking(move || projects::list(&root, &dirs)).await
}

#[tauri::command]
pub async fn devstack_project(dir: String, start: bool) -> Result<(), String> {
    blocking(move || if start { projects::start(&dir) } else { projects::stop(&dir) }).await?
}

#[tauri::command]
pub async fn devstack_versions(root: String) -> Result<versions::Versions, String> { blocking(move || versions::versions(&root)).await? }

#[tauri::command]
pub async fn devstack_switch(root: String, kind: String, version: String) -> Result<String, String> {
    blocking(move || versions::switch(&root, &kind, &version)).await?
}

#[tauri::command]
pub async fn devstack_nvm_use(version: String) -> Result<(), String> { blocking(move || versions::nvm_use(&version)).await? }

#[tauri::command]
pub async fn devstack_other() -> Result<other::Other, String> { blocking(other::other).await }

#[tauri::command]
pub async fn devstack_service(name: String, start: bool) -> Result<(), String> { blocking(move || other::service(&name, start)).await? }

#[tauri::command]
pub async fn devstack_docker(action: String, id: String) -> Result<(), String> { blocking(move || other::docker_action(&action, &id)).await? }

#[cfg(test)]
mod tests {
    #[test]
    fn reads_ini_sections() {
        let ini = "[php]\nVersion=8\n[apache]\nVersion=httpd-2.4\nUse=-1\n[mysqld]\ndatadir= \"C:/x\"\nport = 3307\n";
        assert_eq!(super::ini_value(ini, "apache", "Version").as_deref(), Some("httpd-2.4"));
        assert_eq!(super::ini_value(ini, "apache", "use").as_deref(), Some("-1"));
        assert_eq!(super::ini_value(ini, "mysqld", "port").as_deref(), Some("3307"));
        assert_eq!(super::ini_value(ini, "mysqld", "datadir").as_deref(), Some("C:/x"));
        assert_eq!(super::ini_value(ini, "php", "Use"), None);
    }

    #[test]
    fn reads_upstream_ports() {
        let conf = "# comment\nupstream php_upstream {\n\tserver 127.0.0.1:9001 weight=1 max_fails=1 fail_timeout=1;\n\tserver 127.0.0.1:9002 weight=1;\n}\n";
        assert_eq!(super::upstream_ports(conf), vec![9001, 9002]);
        assert_eq!(super::upstream_ports(""), vec![9001]);
    }
}
