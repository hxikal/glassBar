//! Outside Laragon: database / Docker Windows services (start/stop through Windows' admin
//! prompt, so they shut down cleanly) and Docker Desktop's containers.
use super::{output, running_names, NO_WINDOW};
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Debug, PartialEq)]
pub struct WinService { pub name: String, pub display: String, pub running: bool, pub pending: bool }

#[derive(Serialize)]
pub struct Container { id: String, name: String, image: String, running: bool, status: String, ports: String }

#[derive(Serialize)]
pub struct Docker { running: bool, containers: Vec<Container> }

#[derive(Serialize)]
pub struct Other { services: Vec<WinService>, docker: Option<Docker> }

const WATCHED: &[&str] = &["postgres", "mongo", "mysql", "mariadb", "redis", "memurai", "mssql", "sqlserver", "docker", "elasticsearch", "rabbitmq", "couchdb", "neo4j", "influx"];

/// `sc query` output → services. The STATE number is the same in every language.
pub fn parse_sc(out: &str) -> Vec<WinService> {
    let mut v: Vec<WinService> = vec![];
    for l in out.lines().map(str::trim) {
        let Some((k, val)) = l.split_once(':') else { continue };
        let val = val.trim();
        match k.trim() {
            "SERVICE_NAME" => v.push(WinService { name: val.into(), display: val.into(), running: false, pending: false }),
            "DISPLAY_NAME" => if let Some(s) = v.last_mut() { s.display = val.into() },
            "STATE" => if let Some(s) = v.last_mut() {
                let n: u32 = val.split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(1);
                s.running = n == 4;
                s.pending = matches!(n, 2 | 3 | 5 | 6);
            },
            _ => {}
        }
    }
    v
}

fn watched(s: &WinService) -> bool {
    let n = format!("{} {}", s.name, s.display).to_lowercase();
    WATCHED.iter().any(|w| n.contains(w))
}

pub fn services() -> Vec<WinService> {
    let out = output("sc.exe", &["query", "type=", "service", "state=", "all"]).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    parse_sc(&out).into_iter().filter(watched).collect()
}

/// `sc start|stop` behind the admin prompt. Only for a service we list.
pub fn service(name: &str, start: bool) -> Result<(), String> {
    if !services().iter().any(|s| s.name == name) { return Err("unknown service".into()); }
    crate::platform::shell().launch_elevated("sc.exe", &format!("{} \"{name}\"", if start { "start" } else { "stop" }), false)
}

fn docker_exe() -> Option<PathBuf> {
    let installed = PathBuf::from(r"C:\Program Files\Docker\Docker\resources\bin\docker.exe");
    if installed.is_file() { return Some(installed); }
    super::versions::fresh_path().split(';').map(|d| PathBuf::from(d.trim()).join("docker.exe")).find(|p| p.is_file())
}

fn docker() -> Option<Docker> {
    let exe = docker_exe()?;
    let names = running_names();
    let running = names.iter().any(|n| n == "com.docker.backend.exe" || n == "dockerd.exe");
    let mut containers = vec![];
    if running {
        let out = output(&exe.display().to_string(), &["ps", "-a", "--format", "{{json .}}"]).unwrap_or_default();
        for l in String::from_utf8_lossy(&out).lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(l) else { continue };
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
            containers.push(Container { id: s("ID"), name: s("Names"), image: s("Image"), running: s("State") == "running", status: s("Status"), ports: s("Ports") });
        }
    }
    Some(Docker { running, containers })
}

/// "desktop" (start Docker Desktop) | "start" | "stop" a container by id.
pub fn docker_action(action: &str, id: &str) -> Result<(), String> {
    let exe = docker_exe().ok_or("Docker isn't installed")?;
    match action {
        "desktop" => {
            let app = r"C:\Program Files\Docker\Docker\Docker Desktop.exe";
            crate::platform::shell().launch(app, "")
        }
        "start" | "stop" if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) => {
            let out = Command::new(exe).args([action, id]).creation_flags(NO_WINDOW).output().map_err(|e| e.to_string())?;
            if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
        }
        _ => Err("unknown action".into()),
    }
}

pub fn other() -> Other { Other { services: services(), docker: docker() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sc_query() {
        let out = "\r\nSERVICE_NAME: postgresql-x64-18\r\nDISPLAY_NAME: postgresql-x64-18 - PostgreSQL Server 18\r\n        TYPE               : 10  WIN32_OWN_PROCESS\r\n        STATE              : 4  RUNNING\r\n                                (STOPPABLE, PAUSABLE, ACCEPTS_SHUTDOWN)\r\n\r\nSERVICE_NAME: Spooler\r\nDISPLAY_NAME: Print Spooler\r\n        STATE              : 1  STOPPED\r\n";
        let v = parse_sc(out);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], WinService { name: "postgresql-x64-18".into(), display: "postgresql-x64-18 - PostgreSQL Server 18".into(), running: true, pending: false });
        assert!(watched(&v[0]) && !watched(&v[1]));
    }
}
