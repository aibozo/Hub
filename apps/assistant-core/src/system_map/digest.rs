use super::model::SystemMap;

pub fn compute_digest(map: &SystemMap) -> String {
    // Build a compact, deterministic summary; sort lists to keep stable
    let mut pms = map.package_managers.clone(); pms.sort();
    let mut apps = map.apps.clone(); apps.sort();
    let runt = &map.runtimes;
    let os = &map.os;
    let hw = &map.hardware;

    let mut parts: Vec<String> = vec![];
    parts.push(format!(
        "System: {}{}; kernel {}; arch {}.",
        os.name,
        os.version.as_ref().map(|v| format!(" {}", v)).unwrap_or_default(),
        os.kernel.as_deref().unwrap_or("unknown"),
        os.arch.as_deref().unwrap_or("unknown"),
    ));
    if hw.cpu_model.is_some() || hw.ram_gb.is_some() {
        parts.push(format!(
            "Hardware: CPU {}; RAM {:.1} GB.",
            hw.cpu_model.as_deref().unwrap_or("unknown"),
            hw.ram_gb.unwrap_or(0.0)
        ));
    }
    let mut rt_parts = vec![];
    if let Some(v) = &runt.python { rt_parts.push(format!("python({})", v)); }
    if let Some(v) = &runt.node { rt_parts.push(format!("node({})", v)); }
    if let Some(v) = &runt.rustc { rt_parts.push(format!("rustc({})", v)); }
    if let Some(v) = &runt.cargo { rt_parts.push(format!("cargo({})", v)); }
    if let Some(v) = &runt.java { rt_parts.push(format!("java({})", v)); }
    if let Some(v) = &runt.cuda { rt_parts.push(format!("cuda({})", v)); }
    if !rt_parts.is_empty() {
        parts.push(format!("Runtimes: {}.", rt_parts.join(", ")));
    }
    if !pms.is_empty() {
        parts.push(format!("Package managers: {}.", pms.join(", ")));
    }
    if !apps.is_empty() {
        let list = if apps.len() > 8 { let mut a = apps[..8].to_vec(); a.push("…".to_string()); a } else { apps };
        parts.push(format!("Apps: {}.", list.join(", ")));
    }
    parts.push("Rules: prefer local CLI; apt/installs require approval; no external network scans; expand details via map://packages, map://emulators, map://worktrees.".to_string());

    let digest = parts.join(" ");

    // Trim to ~400 tokens (approximate by words)
    let words: Vec<&str> = digest.split_whitespace().collect();
    let max_tokens = 400;
    let trimmed = if words.len() > max_tokens { words[..max_tokens].join(" ") } else { digest };
    trimmed
}

