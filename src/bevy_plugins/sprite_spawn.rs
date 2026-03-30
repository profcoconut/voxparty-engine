//! Sprite spawn systems for tiles and players using Bevy TextureAtlas.
//!
//! Provides:
//! - `PlayerTag` component: marks player sprite entities
//! - `TileTag` component: marks tile sprite entities
//! - `tile_spawn_system`: spawns tile sprites from World episode data
//! - `player_spawn_system`: spawns player sprites from grid positions

use bevy::prelude::*;
use bevy::image::TextureAtlas;
use crate::core::sprites::SpriteSheet;
use crate::core::isom::{grid_to_screen, depth_key};
use crate::bevy_plugins::input::Players;
use crate::bevy_plugins::sprite::GridPos;
use crate::bevy_plugins::world::VoxpartyWorld;
use crate::bevy_plugins::player::PlayerComponent;

/// Marker component identifying a player sprite entity.
/// The u8 is the player_id (1 or 2).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTag(pub u8);

/// Marker component identifying a tile sprite entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileTag;

/// Resource that holds the sprite sheet data for the current episode.
/// This is populated by the asset loading system and used by spawn systems.
#[derive(Resource, Debug, Clone)]
pub struct VoxpartyAtlas {
    /// The sprite sheet with frame definitions
    pub sheet: SpriteSheet,
    /// Handle to the texture image for this sprite sheet
    pub texture_handle: Handle<Image>,
    /// The texture atlas for sprite indexing
    pub atlas: TextureAtlas,
}

impl VoxpartyAtlas {
    /// Look up a frame rect by name. Returns the frame index (0-based)
    /// within the texture atlas, or None if not found.
    pub fn frame_index(&self, name: &str) -> Option<usize> {
        self.sheet.frames.get(name).map(|f| {
            (f.x / 64) as usize
        })
    }

    /// Create a TextureAtlas with the given frame index.
    pub fn make_atlas(&self, frame_index: usize) -> TextureAtlas {
        self.atlas.clone().with_index(frame_index)
    }
}

/// Plugin that registers sprite spawn systems.
pub struct SpriteSpawnPlugin;

impl SpriteSpawnPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for SpriteSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            tile_spawn_system,
            player_spawn_system,
        ));
    }
}

/// Spawns tile sprites from the current World episode data.
/// Runs in PostUpdate to ensure entities are spawned after World is updated.
pub fn tile_spawn_system(
    mut commands: Commands,
    tiles: Query<Entity, With<TileTag>>,
    world: Res<VoxpartyWorld>,
    atlas: Res<VoxpartyAtlas>,
) {
    // Despawn all existing tile entities to avoid duplicates
    // This is a simple approach suitable for the migration phase
    for entity in &tiles {
        commands.entity(entity).despawn();
    }

    // Spawn new tile entities for each tile in the episode
    // Note: VoxpartyWorld.world.episode.tiles is Vec<TileDef> with {x, y, tile_type}
    for tile in &world.world.episode.tiles {
        let gx = tile.x;
        let gy = tile.y;
        let tile_type = &tile.tile_type;

        // Get the frame index for this tile type
        let frame_index = match atlas.frame_index(tile_type) {
            Some(idx) => idx,
            None => {
                // Fallback: try the passable variant or skip
                atlas.frame_index(&format!("{}_passable", tile_type.split('_').next().unwrap_or("grass")))
                    .unwrap_or(0)
            }
        };

        // Compute screen position with zero camera offset
        // Camera offset will be handled by a separate camera system
        let (sx, sy) = grid_to_screen(gx as f32, gy as f32, 0.0, 0.0);
        let depth = depth_key(gx, gy, 0);

        let sprite = Sprite::from_atlas_image(
            atlas.texture_handle.clone(),
            atlas.make_atlas(frame_index),
        );

        commands.spawn((sprite, Transform::from_translation(Vec3::new(sx, sy, depth as f32)), TileTag, GridPos { x: gx, y: gy, z: 0 }));
    }
}

/// Spawns player sprites at their current grid positions.
/// Player positions come from the Players resource (virtual gamepad state).
/// Note: For the migration phase, this reads player grid positions from
/// the Players resource. In a full ECS approach, players would be
/// Bevy entities with GridPos components.
pub fn player_spawn_system(
    mut commands: Commands,
    players: Query<Entity, With<PlayerTag>>,
    _players_resource: Res<Players>,
    atlas: Res<VoxpartyAtlas>,
) {
    // Despawn all existing player sprite entities
    for entity in &players {
        commands.entity(entity).despawn();
    }

    // Spawn player 1 sprite at default position
    // Note: Player grid positions will be bridged from the game module
    // once Player becomes a Bevy component. For now, use spawn point (1, 1).
    spawn_player_sprite(&mut commands, &atlas, 1, 1, 1);

    // Spawn player 2 sprite at default position
    spawn_player_sprite(&mut commands, &atlas, 2, 1, 3);
}

/// Helper to spawn a single player sprite entity.
fn spawn_player_sprite(
    commands: &mut Commands,
    atlas: &VoxpartyAtlas,
    player_id: u8,
    grid_x: i32,
    grid_y: i32,
) {
    // Look up the idle frame for this player
    let frame_name = if player_id == 1 { "player1_idle" } else { "player2_idle" };
    let frame_index = atlas.frame_index(frame_name).unwrap_or(0);

    // Compute screen position
    let (sx, sy) = grid_to_screen(grid_x as f32, grid_y as f32, 0.0, 0.0);
    let depth = depth_key(grid_x, grid_y, 1); // Players are at z=1 for foreground

    let sprite = Sprite::from_atlas_image(
        atlas.texture_handle.clone(),
        atlas.make_atlas(frame_index),
    );

    commands.spawn((
        sprite,
        Transform::from_translation(Vec3::new(sx, sy, depth as f32)),
        PlayerTag(player_id),
        PlayerComponent::new(player_id, grid_x, grid_y),
        GridPos { x: grid_x, y: grid_y, z: 1 },
    ));
}
