/// Load a sprite sheet JSON descriptor from assets/sprites/<name>.json
pub fn load_sprite_sheet(name: &str) -> String {
    let path = format!("assets/sprites/{}.json", name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("Sprite sheet not found: {}", path))
}

/// Load an episode JSON file from assets/episodes/<name>.json
pub fn load_episode(name: &str) -> String {
    let path = format!("assets/episodes/{}.json", name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("Episode not found: {}", path))
}
