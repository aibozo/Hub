use super::model::*;
use chrono::Utc;
use std::time::Duration;
use tokio::process::Command;

fn parse_first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

async fn run_and_capture(cmd: &str, args: &[&str]) -> Option<String> {
    let fut = async move {
        let output = Command::new(cmd).args(args).output().await.ok()?;
        if output.status.success() {
            let out = String::from_utf8_lossy(&output.stdout).to_string();
            Some(out)
        } else {
            None
        }
    };
    match tokio::time::timeout(Duration::from_millis(300), fut).await {
        Ok(v) => v,
        Err(_) => None,
    }
}

pub async fn scan_system() -> SystemMap {
    let mut map = SystemMap::default();
    map.scanned_at = Utc::now();

    // OS info
    let os_name = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| {
            let mut name: Option<String> = None;
            let mut version: Option<String> = None;
            for line in t.lines() {
                if line.starts_with("NAME=") {
                    name = Some(line.trim_start_matches("NAME=").trim_matches('"').to_string());
                } else if line.starts_with("VERSION=") {
                    version = Some(line.trim_start_matches("VERSION=").trim_matches('"').to_string());
                }
            }
            Some((name, version))
        })
        .and_then(|(name, version)| name.map(|n| (n, version)))
        .unwrap_or_else(|| ("Linux".to_string(), None));
    let kernel = run_and_capture("uname", &["-r"]).await.map(|s| parse_first_line(&s));
    let arch = run_and_capture("uname", &["-m"]).await.map(|s| parse_first_line(&s));
    map.os = OsInfo { name: os_name.0, version: os_name.1, kernel, arch };

    // Hardware
    let cpu_model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with("model name"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string()));
    let ram_gb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with("MemTotal"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<u64>().ok())
            .map(|kb| (kb as f64) / (1024.0 * 1024.0))
        );
    map.hardware = HardwareInfo { cpu_model, gpu_model: None, ram_gb };

    // Runtimes versions
    let python = match run_and_capture("python3", &["--version"]).await {
        Some(s) => Some(parse_first_line(&s)),
        None => run_and_capture("python", &["--version"]).await.map(|s| parse_first_line(&s)),
    };
    let node = run_and_capture("node", &["--version"]).await.map(|s| parse_first_line(&s));
    let rustc = run_and_capture("rustc", &["--version"]).await.map(|s| parse_first_line(&s));
    let cargo = run_and_capture("cargo", &["--version"]).await.map(|s| parse_first_line(&s));
    let java = run_and_capture("java", &["-version"]).await.map(|s| parse_first_line(&s));
    let cuda = run_and_capture("nvcc", &["--version"]).await.map(|s| parse_first_line(&s));
    map.runtimes = RuntimesInfo { python, node, rustc, cargo, java, cuda };

    // Package managers presence
    let mut pms = vec![];
    for (cmd, label) in [
        ("apt", "apt"),
        ("dnf", "dnf"),
        ("pacman", "pacman"),
        ("brew", "brew"),
        ("snap", "snap"),
        ("flatpak", "flatpak"),
        ("pip", "pip"),
        ("pip3", "pip3"),
        ("cargo", "cargo"),
        ("conda", "conda"),
    ] {
        if run_and_capture(cmd, &["--version"]).await.is_some() { pms.push(label.to_string()); }
    }
    pms.sort(); pms.dedup();
    map.package_managers = pms;

    // Apps (lightweight probes)
    let mut apps = vec![];
    for (cmd, label) in [
        ("steam", "steam"),
        ("docker", "docker"),
        ("podman", "podman"),
        ("code", "vscode"),
        ("emacs", "emacs"),
        ("vim", "vim"),
        ("nvim", "neovim"),
        ("git", "git"),
        ("gh", "github-cli"),
        ("make", "make"),
    ] {
        if run_and_capture(cmd, &["--version"]).await.is_some() { apps.push(label.to_string()); }
    }
    apps.sort(); apps.dedup();
    map.apps = apps;

    // Dev env
    let mut editors = vec![];
    for e in ["vscode", "emacs", "vim", "neovim"] {
        if map.apps.contains(&e.to_string()) { editors.push(e.to_string()); }
    }
    let vcs = if map.apps.iter().any(|a| a == "git") { vec!["git".to_string()] } else { vec![] };
    map.dev_env = DevEnvInfo { editors, vcs };

    // Network (light): hostname
    let hostname = run_and_capture("hostname", &[]).await.map(|s| parse_first_line(&s));
    map.network = NetworkInfo { hostname, interfaces: vec![] };

    map
}
