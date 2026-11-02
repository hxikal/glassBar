//! What GlassBar uses: glassbar.exe plus the WebView2 helpers it started, process by process.

/// `root` plus every msedgewebview2.exe below it, from `(pid, parent pid, is WebView2)`.
/// Only WebView2 helpers: an app launched from the dock is a child of glassbar.exe too, and
/// once got every open Opera tab counted as GlassBar's. The visited check stops cycles (PID
/// reuse can make a process look like its own ancestor).
pub fn ours(root: u32, procs: &[(u32, u32, bool)]) -> Vec<u32> {
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let p = out[i];
        for &(pid, parent, webview) in procs {
            if parent == p && webview && !out.contains(&pid) { out.push(pid); }
        }
        i += 1;
    }
    out
}

/// Task Manager's name for one of GlassBar's processes, from its command line.
pub fn label(is_root: bool, cmd: &str) -> String {
    if is_root { return "GlassBar".into(); }
    let flag = |f: &str| cmd.split_whitespace().find_map(|a| a.strip_prefix(f)).map(|v| v.trim_matches('"'));
    let what = match flag("--type=") {
        None => return "WebView2 Manager".into(),
        Some("gpu-process") => "GPU Process".into(),
        Some("renderer") => "Renderer".into(),
        Some("crashpad-handler") => "Crashpad".into(),
        // "network.mojom.NetworkService" -> "Utility: Network Service"
        Some("utility") => match flag("--utility-sub-type=").and_then(|s| s.rsplit('.').next()) {
            Some(svc) => format!("Utility: {}", svc.chars().enumerate()
                .fold(String::new(), |mut o, (i, c)| { if i > 0 && c.is_uppercase() { o.push(' '); } o.push(c); o })),
            None => "Utility".into(),
        },
        Some(t) => t.into(),
    };
    format!("WebView2: {what}")
}

/// One of GlassBar's processes, as the Performance page lists it.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcUsage { pub pid: u32, pub name: String, pub ram_mb: f32, pub cpu: f32, pub gpu: Option<f32> }

/// One reading of GlassBar's processes, plus the system load Auto decides from.
pub struct Snapshot {
    /// Private working set of them all, MB (Task Manager's Memory).
    pub ram_mb: f32,
    /// % of the whole CPU (Task Manager's scale), summed.
    pub cpu: f32,
    /// 3D engine %, summed; None when GPU counters are unavailable.
    pub gpu: Option<f32>,
    /// Each process, GlassBar first and then its helpers by name; they add up to the totals.
    pub procs: Vec<ProcUsage>,
    pub sys_mem_pct: f32,
    pub sys_cpu_pct: f32,
}

pub struct Meter { sys: sysinfo::System }

impl Meter {
    pub fn new() -> Self { Self { sys: sysinfo::System::new() } }

    /// CPU is measured since the previous call, so the first reading is ~0.
    pub fn sample(&mut self) -> Snapshot { self.sample_of(std::process::id()) }

    fn sample_of(&mut self, root: u32) -> Snapshot {
        use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
        let s = &mut self.sys;
        s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu());
        s.refresh_cpu_usage();
        s.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        let procs: Vec<(u32, u32, bool)> = s.processes().iter()
            .filter_map(|(pid, p)| Some((pid.as_u32(), p.parent()?.as_u32(), p.name().eq_ignore_ascii_case("msedgewebview2.exe"))))
            .collect();
        let pids = ours(root, &procs);
        let cores = s.cpus().len().max(1) as f32;
        let cpu: Vec<f32> = pids.iter().map(|&p| s.process(Pid::from_u32(p)).map_or(0.0, |p| p.cpu_usage() / cores)).collect();
        // Command lines (for the names), read once per process and only for these.
        let sp: Vec<Pid> = pids.iter().map(|&p| Pid::from_u32(p)).collect();
        s.refresh_processes_specifics(ProcessesToUpdate::Some(&sp), false, ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet));
        let hw = crate::platform::gpu();
        let gpu = hw.usage_each(&pids);
        let mut rows: Vec<ProcUsage> = pids.iter().enumerate().map(|(i, &pid)| {
            let cmd = s.process(Pid::from_u32(pid))
                .map(|p| p.cmd().join(std::ffi::OsStr::new(" ")).to_string_lossy().into_owned()).unwrap_or_default();
            ProcUsage { pid, name: label(pid == root, &cmd), ram_mb: hw.private_ws(pid).unwrap_or(0) as f32 / 1_048_576.0,
                        cpu: cpu[i], gpu: gpu.as_ref().map(|g| g[i]) }
        }).collect();
        rows[1..].sort_by(|a, b| (&a.name, a.pid).cmp(&(&b.name, b.pid)));
        Snapshot {
            ram_mb: rows.iter().map(|r| r.ram_mb).sum(),
            cpu: rows.iter().map(|r| r.cpu).sum(),
            gpu: gpu.map(|g| g.iter().sum::<f32>().min(100.0)),
            procs: rows,
            sys_mem_pct: if s.total_memory() > 0 { s.used_memory() as f32 * 100.0 / s.total_memory() as f32 } else { 0.0 },
            sys_cpu_pct: s.global_cpu_usage(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ours_is_root_and_its_webview2_helpers_only() {
        // 20, 21, 30: our WebView2 helpers. 40: an app launched from the dock, 41/42 its children.
        // 98: another app's WebView2.
        let procs = [(20, 10, true), (21, 10, true), (30, 20, true), (40, 10, false), (41, 40, false),
                     (42, 40, true), (99, 1, false), (98, 99, true)];
        let mut t = ours(10, &procs); t.sort();
        assert_eq!(t, vec![10, 20, 21, 30]);
    }

    #[test]
    fn ours_survives_a_parent_cycle() {
        // PID reuse can make a process look like its own ancestor.
        let mut t = ours(10, &[(10, 30, true), (20, 10, true), (30, 20, true)]); t.sort();
        assert_eq!(t, vec![10, 20, 30]);
    }

    #[test]
    fn labels_match_task_manager() {
        let exe = r#""C:\Program Files (x86)\Microsoft\EdgeWebView\Application\156.0\msedgewebview2.exe""#;
        assert_eq!(label(true, "glassbar.exe"), "GlassBar");
        assert_eq!(label(false, &format!("{exe} --embedded-browser-webview=1 --webview-exe-name=glassbar.exe")), "WebView2 Manager");
        assert_eq!(label(false, &format!("{exe} --type=gpu-process --no-sandbox")), "WebView2: GPU Process");
        assert_eq!(label(false, &format!("{exe} --type=renderer --lang=en-US")), "WebView2: Renderer");
        assert_eq!(label(false, &format!("{exe} --type=crashpad-handler")), "WebView2: Crashpad");
        assert_eq!(label(false, &format!("{exe} --type=utility --utility-sub-type=network.mojom.NetworkService")),
                   "WebView2: Utility: Network Service");
        assert_eq!(label(false, &format!("{exe} --type=utility")), "WebView2: Utility");
        assert_eq!(label(false, ""), "WebView2 Manager"); // command line unreadable
    }

    /// What one sample costs (the monitor must stay cheap). By hand:
    /// cargo test sample_cost -- --ignored --nocapture
    #[test]
    #[ignore]
    fn sample_cost() {
        let mut m = Meter::new();
        m.sample();
        let t = std::time::Instant::now();
        for _ in 0..10 { m.sample(); }
        println!("Meter::sample average: {:?}", t.elapsed() / 10);
    }

    /// Another GlassBar's rows, to hold up against Task Manager. By hand:
    /// $env:GLASSBAR_PID=1234; cargo test live_rows -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_rows() {
        let pid = std::env::var("GLASSBAR_PID").expect("GLASSBAR_PID").parse().unwrap();
        let mut m = Meter::new();
        m.sample_of(pid);
        std::thread::sleep(std::time::Duration::from_secs(5));
        let s = m.sample_of(pid);
        for p in &s.procs { println!("{:>6} {:<36} {:>8.1} MB {:>5.1}% gpu {:?}", p.pid, p.name, p.ram_mb, p.cpu, p.gpu); }
        println!("total {:.1} MB, {:.1}% CPU, gpu {:?}", s.ram_mb, s.cpu, s.gpu);
    }
}
