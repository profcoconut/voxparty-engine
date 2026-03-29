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
