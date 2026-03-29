use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnPoint {
    pub x: i32,
    pub y: i32,
    pub player: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpcDef {
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub dialogue: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileDef {
    pub x: i32,
    pub y: i32,
    #[serde(rename = "type")]
    pub tile_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckpointDef {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WinCondition {
    #[serde(rename = "type")]
    pub cond_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FailCondition {
    #[serde(rename = "type")]
    pub cond_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Episode {
    pub id: String,
    pub title: String,
    pub mode: String,
    pub theme: String,
    pub difficulty: String,
    pub duration_target_seconds: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub grid_width: i32,
    pub grid_height: i32,
    pub tiles: Vec<TileDef>,
    pub npcs: Vec<NpcDef>,
    pub checkpoints: Vec<CheckpointDef>,
    pub spawn_points: Vec<SpawnPoint>,
    pub win_condition: WinCondition,
    pub fail_condition: FailCondition,
}

impl Episode {
    pub fn load(path: &str) -> Self {
        let data = std::fs::read_to_string(path).expect("Failed to load episode");
        serde_json::from_str(&data).expect("Failed to parse episode JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_demo_episode_fields() {
        let ep = Episode::load("assets/episodes/demo.json");
        assert_eq!(ep.id, "ep_demo");
        assert_eq!(ep.title, "Demo Island");
        assert_eq!(ep.mode, "solo");
        assert_eq!(ep.theme, "grassland");
        assert_eq!(ep.duration_target_seconds, 120);
        assert_eq!(ep.tile_width, 64);
        assert_eq!(ep.tile_height, 32);
        assert_eq!(ep.grid_width, 16);
        assert_eq!(ep.grid_height, 16);
    }

    #[test]
    fn test_load_demo_episode_has_content() {
        let ep = Episode::load("assets/episodes/demo.json");
        // Demo episode now has tiles (walls, trap, checkpoint, goal)
        assert!(!ep.tiles.is_empty(), "demo episode should have tiles");
        // Demo episode may have NPCs for dialogue/interaction
        // Demo episode now has a checkpoint
        assert!(!ep.checkpoints.is_empty(), "demo episode should have checkpoints");
    }

    #[test]
    fn test_load_demo_episode_spawn_points() {
        let ep = Episode::load("assets/episodes/demo.json");
        assert_eq!(ep.spawn_points.len(), 2);
        assert_eq!(ep.spawn_points[0].x, 1);
        assert_eq!(ep.spawn_points[0].y, 1);
        assert_eq!(ep.spawn_points[0].player, 1);
        assert_eq!(ep.spawn_points[1].x, 1);
        assert_eq!(ep.spawn_points[1].y, 3);
        assert_eq!(ep.spawn_points[1].player, 2);
    }

    #[test]
    fn test_load_demo_episode_conditions() {
        let ep = Episode::load("assets/episodes/demo.json");
        assert_eq!(ep.win_condition.cond_type, "reach_goal");
        assert_eq!(ep.fail_condition.cond_type, "fall_off_map");
    }

    #[test]
    fn test_spawn_point_deserialization() {
        let json = r#"{"x": 5, "y": 10, "player": 2}"#;
        let sp: SpawnPoint = serde_json::from_str(json).unwrap();
        assert_eq!(sp.x, 5);
        assert_eq!(sp.y, 10);
        assert_eq!(sp.player, 2);
    }

    #[test]
    fn test_npc_def_deserialization() {
        let json = r#"{"x": 3, "y": 4, "name": "Mayor", "dialogue": ["Hello!", "Bye"]}"#;
        let npc: NpcDef = serde_json::from_str(json).unwrap();
        assert_eq!(npc.x, 3);
        assert_eq!(npc.y, 4);
        assert_eq!(npc.name, "Mayor");
        assert_eq!(npc.dialogue, vec!["Hello!", "Bye"]);
    }

    #[test]
    fn test_tile_def_deserialization() {
        let json = r#"{"x": 1, "y": 2, "type": "grass_solid"}"#;
        let tile: TileDef = serde_json::from_str(json).unwrap();
        assert_eq!(tile.x, 1);
        assert_eq!(tile.y, 2);
        assert_eq!(tile.tile_type, "grass_solid");
    }

    #[test]
    fn test_win_condition_deserialization() {
        let json = r#"{"type": "reach_goal"}"#;
        let cond: WinCondition = serde_json::from_str(json).unwrap();
        assert_eq!(cond.cond_type, "reach_goal");
    }

    #[test]
    fn test_fail_condition_deserialization() {
        let json = r#"{"type": "time_limit"}"#;
        let cond: FailCondition = serde_json::from_str(json).unwrap();
        assert_eq!(cond.cond_type, "time_limit");
    }

    #[test]
    fn test_episode_deserialization_minimal() {
        let json = r#"{
            "id": "ep_test",
            "title": "Test",
            "mode": "solo",
            "theme": "cave",
            "difficulty": "hard",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 8,
            "grid_height": 8,
            "tiles": [],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        assert_eq!(ep.id, "ep_test");
        assert_eq!(ep.title, "Test");
        assert_eq!(ep.mode, "solo");
        assert_eq!(ep.theme, "cave");
        assert_eq!(ep.difficulty, "hard");
        assert_eq!(ep.duration_target_seconds, 60);
        assert_eq!(ep.grid_width, 8);
        assert_eq!(ep.grid_height, 8);
        assert!(ep.tiles.is_empty());
        assert!(ep.spawn_points.is_empty());
    }
}
