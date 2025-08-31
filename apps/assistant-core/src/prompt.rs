pub fn base_system_prompt() -> String {
    // Local context and tool usage policy for emulator + Steam flows
    let roms_root = "/home/kil/games/roms";
    let ds_emulator = "/home/kil/games/emulators/melonDS-x86_64.AppImage";
    let gb_cmd = "mgba-qt";
    let mut s = String::new();
    s.push_str("Local Context:\n");
    s.push_str(&format!("- Game ROMs folder: {}\n", roms_root));
    s.push_str("- Emulators:\n");
    s.push_str(&format!("  • Nintendo DS: {} (launch without arguments)\n", ds_emulator));
    s.push_str(&format!("  • Game Boy / Game Boy Advance: {} (usage: mgba-qt /path/to/game)\n", gb_cmd));

    // List available games by console (best effort)
    if let Some(by_console) = scan_roms_root(roms_root) {
        if !by_console.is_empty() {
            s.push_str("\nAvailable games by console:\n");
            for (console, titles) in by_console {
                if titles.is_empty() { continue; }
                s.push_str(&format!("- {}: ", console));
                let joined = titles.join(", ");
                s.push_str(&joined);
                s.push('\n');
            }
        }
    }

    // Steam quick-launch mapping from config/steamgames.toml
    let steam_games = crate::api::load_steamgames_user_list();
    if !steam_games.is_empty() {
        s.push_str("\nSteam Games: appid\n");
        for (name, appid) in steam_games.iter().take(20) {
            s.push_str(&format!("- {}: {}\n", name, appid));
        }
        if steam_games.len() > 20 { s.push_str(&format!("- … {} more\n", steam_games.len() - 20)); }
        s.push_str("\nLaunch policy:\n");
        s.push_str("- Prefer the steam.launch tool with the AppID.\n");
        s.push_str("- Or use shell.exec: steam -applaunch <APPID> (allowed).\n");
    }

    s.push_str("\nTool usage policy:\n");
    s.push_str("- When launching emulators:\n");
    s.push_str(&format!("  • DS: run the emulator by itself: {} (no ROM path).\n", ds_emulator));
    s.push_str(&format!("  • Game Boy: launch with the ROM file: {} /path/to/game.\n", gb_cmd));
    s.push_str(&format!("  • For Game Boy, list files in {} first to find the exact ROM path requested.\n", roms_root));
    s.push_str("  • If a game title appears under exactly one console above, you can infer the console. If ambiguous, ask.\n");
    s.push_str("  • Do not attempt other shell commands. Only the above patterns are allowed.\n");

    s.push_str("\nSafety & Policy:\n");
    s.push_str("- Risky actions require approval; propose the action and wait.\n");
    s.push_str("- Use available tools via the provided schemas; avoid guessing paths.\n");
    s
}

fn scan_roms_root(root: &str) -> Option<Vec<(String, Vec<String>)>> {
    use std::fs;
    use std::path::Path;
    let mut out: Vec<(String, Vec<String>)> = vec![];
    let root_path = Path::new(root);
    if !root_path.exists() { return None; }
    let entries = fs::read_dir(root_path).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            let console = entry.file_name().to_string_lossy().to_string();
            let mut titles: Vec<String> = vec![];
            if let Ok(mut games) = fs::read_dir(&path) {
                while let Some(Ok(g)) = games.next() {
                    let gp = g.path();
                    if gp.is_file() {
                        if let Some(name) = gp.file_stem().and_then(|s| s.to_str()) {
                            let title = name.replace('_', " ");
                            titles.push(title);
                        }
                    }
                }
            }
            titles.sort();
            out.push((console, titles));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Some(out)
}

pub fn voice_mode_suffix() -> String {
    // Additional guidance when running in realtime voice mode
    let mut s = String::new();
    s.push_str("\nVoice Mode:\n");
    s.push_str("- Keep responses concise and conversational.\n");
    s.push_str("- Prefer stepwise guidance; avoid long lists unless asked.\n");
    s
}
