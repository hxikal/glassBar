//! Localhost servers: every program listening on a local TCP port (netstat), named by what
//! it runs (Vite, Next.js, artisan serve…). Stop ends the whole `npm run dev` chain, never a
//! database: those are stopped from the Services tab, cleanly.
use super::{output, run};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Serialize, Clone)]
pub struct Listener {
    pub pid: u32,
    pub name: String,
    pub label: String,
    pub ports: Vec<u16>,
    /// Working folder (or the program's folder).
    pub dir: String,
    pub cmd: String,
    /// A dev server (node, python, php…, or a program inside a project folder).
    pub dev: bool,
    /// A database: Stop is refused here.
    pub db: bool,
}

/// Shown in the Services tab instead.
const LARAGON: &[&str] = &["httpd.exe", "nginx.exe", "mysqld.exe", "redis-server.exe", "memcached.exe", "php-cgi.exe"];
const DATABASES: &[&str] = &["mysqld.exe", "mariadbd.exe", "postgres.exe", "mongod.exe", "redis-server.exe", "memurai.exe", "sqlservr.exe", "memcached.exe"];
const RUNTIMES: &[&str] = &["node.exe", "bun.exe", "deno.exe", "python.exe", "pythonw.exe", "py.exe", "php.exe", "ruby.exe",
    "java.exe", "javaw.exe", "dotnet.exe", "go.exe", "air.exe", "hugo.exe", "caddy.exe", "ngrok.exe", "wslrelay.exe", "uvicorn.exe",
    "flask.exe", "gunicorn.exe", "rails.exe", "cargo.exe"];
/// Windows' own programs whose path we may not be allowed to read.
const SYSTEM: &[&str] = &["system", "svchost.exe", "lsass.exe", "wininit.exe", "services.exe", "spoolsv.exe", "smss.exe", "csrss.exe"];

/// (port, pid) of TCP sockets listening on this computer only or on all interfaces.
/// A listening row is the one whose foreign address is 0.0.0.0:0 / [::]:0 (the state
/// word is translated on non-English Windows).
pub fn parse_netstat(out: &str) -> Vec<(u16, u32)> {
    let mut v = vec![];
    for l in out.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 5 || !f[0].eq_ignore_ascii_case("TCP") || !matches!(f[2], "0.0.0.0:0" | "[::]:0") { continue; }
        let Some((host, port)) = f[1].rsplit_once(':') else { continue };
        if !matches!(host, "127.0.0.1" | "0.0.0.0" | "[::]" | "[::1]") { continue; }
        if let (Ok(port), Ok(pid)) = (port.parse(), f[f.len() - 1].parse()) { v.push((port, pid)); }
    }
    v
}

/// What a command line runs, for the known dev servers.
pub fn framework(name: &str, cmd: &str) -> Option<&'static str> {
    let c = cmd.to_lowercase().replace('\\', "/");
    let has = |s: &str| c.contains(s);
    Some(match () {
        _ if name == "wslrelay.exe" => "WSL",
        _ if name == "ngrok.exe" => "ngrok",
        _ if has("vitest") => "Vitest",
        _ if has("nuxt") || has("nuxi") => "Nuxt",
        _ if has("/next/") || has("next dev") || has("next start") => "Next.js",
        _ if has("astro") => "Astro",
        _ if has("@remix-run") || has("remix dev") => "Remix",
        _ if has("@angular/cli") || has("ng serve") => "Angular",
        _ if has("react-scripts") => "Create React App",
        _ if has("vue-cli-service") => "Vue CLI",
        _ if has("storybook") => "Storybook",
        _ if has("vite") => "Vite",
        _ if has("webpack") => "Webpack",
        _ if has("parcel") => "Parcel",
        _ if has("nodemon") => "Nodemon",
        _ if has("artisan") && has("serve") => "Laravel",
        _ if has("manage.py") && has("runserver") => "Django",
        _ if has("uvicorn") => "Uvicorn",
        _ if has("gunicorn") => "Gunicorn",
        _ if has("flask") => "Flask",
        _ if has("streamlit") => "Streamlit",
        _ if has("jupyter") => "Jupyter",
        _ if name == "php.exe" && c.split_whitespace().any(|a| a == "-s") => "PHP server",
        _ if has("rails") => "Rails",
        _ => return None,
    })
}

/// Fallback name: "node.exe" → "Node".
fn pretty(name: &str) -> String {
    let base = name.trim_end_matches(".exe");
    match base {
        "node" => "Node".into(), "python" | "pythonw" | "py" => "Python".into(), "php" => "PHP".into(),
        "java" | "javaw" => "Java".into(), "dotnet" => ".NET".into(), "bun" => "Bun".into(), "deno" => "Deno".into(),
        _ => base.to_string(),
    }
}

fn within(p: &Path, dirs: &[PathBuf]) -> bool {
    let p = p.to_string_lossy().to_lowercase();
    dirs.iter().any(|d| {
        let d = d.to_string_lossy().to_lowercase();
        let d = d.trim_end_matches('\\');
        !d.is_empty() && (p == d || p.starts_with(&format!("{d}\\")))
    })
}

fn full() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_cwd(UpdateKind::Always).with_exe(UpdateKind::Always)
}

fn cmd_of(p: &sysinfo::Process) -> String {
    p.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" ")
}

/// Listening programs. `dev_dirs` = project folders (www…): anything running from them is a dev server.
pub fn list(dev_dirs: &[PathBuf]) -> Vec<Listener> {
    let out = output("netstat.exe", &["-ano"]).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let mut by_pid: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    for (port, pid) in parse_netstat(&out) {
        let ports = by_pid.entry(pid).or_default();
        if !ports.contains(&port) { ports.push(port); }
    }
    // Not Windows' own, and not GlassBar itself (Stop would close the dock).
    let me = std::process::id();
    by_pid.retain(|&pid, _| pid > 4 && pid != me);
    let pids: Vec<Pid> = by_pid.keys().map(|&p| Pid::from_u32(p)).collect();
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, full());
    let mut v: Vec<Listener> = by_pid.into_iter().filter_map(|(pid, mut ports)| {
        let p = sys.process(Pid::from_u32(pid))?;
        let name = p.name().to_string_lossy().to_lowercase();
        let exe = p.exe().map(Path::to_path_buf).unwrap_or_default();
        let exe_s = exe.to_string_lossy().to_lowercase();
        if LARAGON.contains(&name.as_str()) || SYSTEM.contains(&name.as_str()) || exe_s.starts_with("c:\\windows\\") { return None; }
        let cmd = cmd_of(p);
        let dir = p.cwd().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf)
            .or_else(|| exe.parent().map(Path::to_path_buf)).unwrap_or_default();
        let known = framework(&name, &cmd);
        let dev = known.is_some() || RUNTIMES.contains(&name.as_str()) || within(&exe, dev_dirs) || within(&dir, dev_dirs)
            || exe_s.contains("\\target\\debug\\") || exe_s.contains("\\target\\release\\");
        ports.sort_unstable();
        Some(Listener {
            pid, label: known.map(String::from).unwrap_or_else(|| pretty(&name)), ports,
            dir: dir.display().to_string(), cmd, dev, db: DATABASES.contains(&name.as_str()), name,
        })
    }).collect();
    v.sort_by_key(|l| (!l.dev, l.ports.first().copied().unwrap_or(0)));
    v
}

/// Can a parent be stopped together with the server? Script runners yes; a person's own
/// terminal (an interactive cmd, PowerShell, VS Code) never.
pub fn part_of_chain(name: &str, cmd: &str) -> bool {
    match name {
        "node.exe" | "bun.exe" | "deno.exe" => true,
        // npm runs scripts through `cmd /d /s /c …`; our Start consoles carry GLASSBAR_DEV.
        "cmd.exe" => { let c = cmd.to_lowercase(); c.contains("glassbar_dev") || c.split_whitespace().any(|a| a == "/c") }
        _ => false,
    }
}

/// End `pid` with its children, and its npm/cmd parents that only exist to run it.
pub fn stop(pid: u32) -> Result<(), String> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always));
    let p = sys.process(Pid::from_u32(pid)).ok_or("It has already stopped")?;
    let name = p.name().to_string_lossy().to_lowercase();
    if DATABASES.contains(&name.as_str()) { return Err("Databases are stopped from the Services tab, so no data is lost".into()); }
    if pid <= 4 || SYSTEM.contains(&name.as_str()) { return Err("That's part of Windows".into()); }
    let mut top = pid;
    let mut cur = p;
    while let Some(parent) = cur.parent().and_then(|pp| sys.process(pp)) {
        if !part_of_chain(&parent.name().to_string_lossy().to_lowercase(), &cmd_of(parent)) { break; }
        top = parent.pid().as_u32();
        cur = parent;
    }
    run(Path::new("taskkill.exe"), &["/F", "/T", "/PID", &top.to_string()], Path::new("C:\\"), true)
}

/// (pid, cwd, command line) of JS runtimes, for matching dev servers to project folders.
pub fn js_processes() -> Vec<(u32, PathBuf, String)> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let pids: Vec<Pid> = sys.processes().iter()
        .filter(|(_, p)| matches!(p.name().to_string_lossy().to_lowercase().as_str(), "node.exe" | "bun.exe" | "deno.exe"))
        .map(|(pid, _)| *pid).collect();
    if pids.is_empty() { return vec![]; }
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), false,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always).with_cmd(UpdateKind::Always));
    pids.iter().filter_map(|pid| { let p = sys.process(*pid)?; Some((pid.as_u32(), p.cwd()?.to_path_buf(), cmd_of(p))) }).collect()
}

pub(super) fn is_within(p: &Path, dir: &Path) -> bool { within(p, std::slice::from_ref(&dir.to_path_buf())) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_listening_rows() {
        let out = "\r\nActive Connections\r\n\r\n  Proto  Local Address          Foreign Address        State           PID\r\n\
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1180\r\n\
  TCP    127.0.0.1:5173         0.0.0.0:0              ABHÖREN         9012\r\n\
  TCP    192.168.1.5:139        0.0.0.0:0              LISTENING       4\r\n\
  TCP    127.0.0.1:5173         127.0.0.1:50211        ESTABLISHED     9012\r\n\
  TCP    [::1]:3000             [::]:0                 LISTENING       777\r\n\
  TCP    [::]:5173              [::]:0                 LISTENING       9012\r\n\
  UDP    0.0.0.0:5353           *:*                                    2222\r\n";
        assert_eq!(parse_netstat(out), vec![(135, 1180), (5173, 9012), (3000, 777), (5173, 9012)]);
    }

    #[test]
    fn names_frameworks() {
        assert_eq!(framework("node.exe", r"node C:\x\node_modules\vite\bin\vite.js"), Some("Vite"));
        assert_eq!(framework("node.exe", "node node_modules/.bin/../vitest/vitest.mjs --ui"), Some("Vitest"));
        assert_eq!(framework("node.exe", r"node C:\x\node_modules\next\dist\bin\next dev"), Some("Next.js"));
        assert_eq!(framework("node.exe", "node node_modules/nuxi/bin/nuxi.mjs dev"), Some("Nuxt"));
        assert_eq!(framework("php.exe", "php artisan serve"), Some("Laravel"));
        assert_eq!(framework("node.exe", r"node C:\x\node_modules\@vue\cli-service\bin\vue-cli-service.js serve"), Some("Vue CLI"));
        assert_eq!(framework("php.exe", "php -S localhost:8000"), Some("PHP server"));
        assert_eq!(framework("python.exe", "python manage.py runserver"), Some("Django"));
        assert_eq!(framework("wslrelay.exe", ""), Some("WSL"));
        assert_eq!(framework("node.exe", "node server.js"), None);
        assert_eq!(pretty("node.exe"), "Node");
    }

    #[test]
    fn keeps_a_persons_terminal() {
        assert!(part_of_chain("node.exe", "node npm-cli.js run dev"));
        assert!(part_of_chain("cmd.exe", r#"C:\WINDOWS\system32\cmd.exe /d /s /c "vite""#));
        assert!(part_of_chain("cmd.exe", r#"cmd /k "set GLASSBAR_DEV=1&& npm run dev""#));
        assert!(!part_of_chain("cmd.exe", r"C:\WINDOWS\system32\cmd.exe"));
        assert!(!part_of_chain("powershell.exe", "powershell"));
        assert!(!part_of_chain("code.exe", "code"));
    }

    #[test]
    fn folder_containment() {
        let dirs = [PathBuf::from(r"C:\laragon\www\")];
        assert!(within(Path::new(r"C:\Laragon\www\app"), &dirs));
        assert!(!within(Path::new(r"C:\laragon\www2\app"), &dirs));
    }
}
