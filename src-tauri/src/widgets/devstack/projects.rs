//! JS projects (a package.json with a dev, serve or start script) in www and the extra folders.
//! Start opens a minimized console running `<pm> run <script>` there, so logs stay readable and
//! Ctrl+C works; running = a node/bun/deno process works in that folder.
use super::{ports, root_of, ini_of, www_of, NO_WINDOW};
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
pub struct Project {
    name: String,
    dir: String,
    /// "dev", "serve" (Vue CLI) or "start".
    script: String,
    /// npm | pnpm | yarn | bun.
    pm: String,
    running: bool,
    ports: Vec<u16>,
    /// What serves it (Vite, Next.js…), when running.
    label: String,
}

/// The script to run: `dev`, else `serve` (Vue CLI), else `start`.
pub fn script_of(package_json: &str) -> Option<&'static str> {
    let v: serde_json::Value = serde_json::from_str(package_json).ok()?;
    let s = v.get("scripts")?;
    ["dev", "serve", "start"].into_iter().find(|k| s.get(k).and_then(|x| x.as_str()).is_some())
}

/// The package manager, from the lockfile in `dir`.
pub fn pm_of(dir: &Path) -> &'static str {
    if dir.join("pnpm-lock.yaml").is_file() { "pnpm" }
    else if dir.join("yarn.lock").is_file() { "yarn" }
    else if dir.join("bun.lockb").is_file() || dir.join("bun.lock").is_file() { "bun" }
    else { "npm" }
}

/// www plus the user's extra folders (each scanned one level deep, and itself).
fn search_dirs(root: &str, extra: &[String]) -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(r) = root_of(root) { v.push(www_of(&r, &ini_of(&r))); }
    v.extend(extra.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).map(PathBuf::from));
    v
}

fn candidates(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut v = vec![];
    for d in dirs {
        if d.join("package.json").is_file() { v.push(d.clone()); }
        if let Ok(rd) = std::fs::read_dir(d) {
            v.extend(rd.flatten().map(|e| e.path()).filter(|p| p.join("package.json").is_file()));
        }
    }
    v.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    v.dedup_by_key(|p| p.to_string_lossy().to_lowercase());
    v
}

/// A `<pm> run dev|start` runner, or a known dev server. An editor's helpers working in the
/// folder (tsserver, eslint) are neither.
pub fn is_runner(cmd: &str) -> bool {
    let c = cmd.to_lowercase();
    [" run dev", " run serve", " run start", "yarn.js dev", "yarn.cjs dev", "yarn.js serve", "yarn.cjs serve"].iter().any(|s| c.contains(s)) || ports::framework("node.exe", &c).is_some()
}

/// The dev server's processes in `dir`: JS processes there that listen on a port or run the script.
fn servers(dir: &Path, js: &[(u32, PathBuf, String)], listening: &[u32]) -> Vec<u32> {
    js.iter().filter(|(pid, cwd, cmd)| ports::is_within(cwd, dir) && (listening.contains(pid) || is_runner(cmd))).map(|(p, ..)| *p).collect()
}

pub fn list(root: &str, extra: &[String]) -> Vec<Project> {
    let dirs = search_dirs(root, extra);
    let js = ports::js_processes();
    let listeners = ports::list(&dirs);
    let listening: Vec<u32> = listeners.iter().map(|l| l.pid).collect();
    candidates(&dirs).into_iter().filter_map(|dir| {
        let script = script_of(&std::fs::read_to_string(dir.join("package.json")).ok()?)?;
        let pids = servers(&dir, &js, &listening);
        let mine: Vec<&ports::Listener> = listeners.iter().filter(|l| pids.contains(&l.pid)).collect();
        let mut port_list: Vec<u16> = mine.iter().flat_map(|l| l.ports.iter().copied()).collect();
        port_list.sort_unstable();
        port_list.dedup();
        Some(Project {
            name: dir.file_name()?.to_string_lossy().into_owned(),
            dir: dir.display().to_string(),
            script: script.into(),
            pm: pm_of(&dir).into(),
            running: !pids.is_empty(),
            ports: port_list,
            label: mine.first().map(|l| l.label.clone()).unwrap_or_default(),
        })
    }).collect()
}

pub fn start(dir: &str) -> Result<(), String> {
    let d = Path::new(dir);
    let script = script_of(&std::fs::read_to_string(d.join("package.json")).map_err(|_| "No package.json there")?)
        .ok_or("package.json has no dev, serve or start script")?;
    let name = d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    // A minimized console of its own; GLASSBAR_DEV marks it so Stop may close it. The outer
    // quotes are there because cmd /c strips the first and last quote of what follows.
    let line = format!(r#"/c "start "{name} · {script}" /min /D "{dir}" cmd /k "set GLASSBAR_DEV=1&& {} run {script}"""#, pm_of(d));
    Command::new("cmd.exe").raw_arg(line).env("PATH", super::versions::fresh_path())
        .current_dir(d).creation_flags(NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn stop(dir: &str) -> Result<(), String> {
    let d = Path::new(dir);
    let listening: Vec<u32> = ports::list(&[]).iter().map(|l| l.pid).collect();
    let pids = servers(d, &ports::js_processes(), &listening);
    if pids.is_empty() { return Err("It isn't running".into()); }
    // The first stop usually takes the rest with it (same tree).
    for p in pids { let _ = ports::stop(p); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_a_server_from_an_editor_helper() {
        assert!(is_runner(r"node C:\Program Files\nodejs\node_modules\npm\bin\npm-cli.js run dev"));
        assert!(is_runner(r"node C:\x\node_modules\vite\bin\vite.js"));
        assert!(is_runner(r"node C:\Program Files\nodejs\node_modules\npm\bin\npm-cli.js run serve"));
        assert!(!is_runner(r"node C:\Users\me\.vscode\extensions\typescript\lib\tsserver.js --useInferredProjectPerProjectRoot"));
    }

    #[test]
    fn picks_the_script() {
        assert_eq!(script_of(r#"{"scripts":{"build":"vite build","dev":"vite"}}"#), Some("dev"));
        assert_eq!(script_of(r#"{"scripts":{"start":"node server.js"}}"#), Some("start"));
        assert_eq!(script_of(r#"{"scripts":{"start":"x","serve":"vue-cli-service serve"}}"#), Some("serve"));
        assert_eq!(script_of(r#"{"scripts":{"build":"tsc"}}"#), None);
        assert_eq!(script_of(r#"{"name":"x"}"#), None);
        assert_eq!(script_of("not json"), None);
    }

    #[test]
    fn picks_the_package_manager() {
        let d = std::env::temp_dir().join(format!("glassbar-pm-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(pm_of(&d), "npm");
        std::fs::write(d.join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(pm_of(&d), "pnpm");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
