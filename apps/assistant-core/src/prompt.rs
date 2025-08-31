pub fn base_system_prompt() -> String {
    // Pinned local instructions for emulators and ROMs + dynamic game list
    let roms_root = "/home/kil/games/roms";
    let ds_emulator = "/home/kil/games/emulators/melonDS-x86_64.AppImage";
    let gb_cmd = "mgba-qt";
    let mut s = String::new();
    s.push_str("Local Context:\n");
    s.push_str(&format!("- Game ROMs folder: {}\n", roms_root));
    s.push_str("- Emulators:\n");
    s.push_str(&format!("  • Nintendo DS: {} (launch without arguments)\n", ds_emulator));
    s.push_str(&format!("  • Game Boy / Game Boy Advance: {} (usage: mgba-qt /path/to/game)\n", gb_cmd));

    // Inject Steam games list from config if available
    let games = crate::api::load_steamgames_user_list();
    if !games.is_empty() {
        s.push_str("- Steam quick-launch: use steam.launch {\"appid\":\"<ID>\"}\n");
        s.push_str("  • Installed titles:\n");
        for (name, appid) in games.iter().take(12) {
            s.push_str(&format!("    - {} = {}\n", name, appid));
        }
        if games.len() > 12 { s.push_str(&format!("    - … {} more\n", games.len() - 12)); }
    }

    s.push_str("\nSafety & Policy:\n");
    s.push_str("- Risky actions require approval; propose the action and wait.\n");
    s.push_str("- Use available tools via the provided schemas; avoid guessing paths.\n");
    s
}

pub fn voice_mode_suffix() -> String {
    // Additional guidance when running in realtime voice mode
    let mut s = String::new();
    s.push_str("\nVoice Mode:\n");
    s.push_str("- Keep responses concise and conversational.\n");
    s.push_str("- Prefer stepwise guidance; avoid long lists unless asked.\n");
    s
}

