//! Sprite rendering for VoxParty — TextureAtlas, GridPos, and isometric depth sorting.
//!
//! Provides:
//! - `GridPos` component: grid-space position for depth sorting
//! - `SpritePlugin`: registers atlas setup and depth sort system

use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{TextureDimension, TextureFormat};
use std::collections::HashMap;

use crate::assets::tile_gen::generate_tile_sprites;
use crate::core::isom::depth_key;

/// Grid-space position component for isometric depth sorting.
/// Entities with this component get their z-translation set by IsoDepthSystem.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Depth sort key: lower values are drawn first (behind).
    pub fn depth_key(&self) -> i32 {
        depth_key(self.x, self.y, self.z)
    }
}

/// Tag component for sprites that need depth sorting.
/// Any entity with both `GridPos` and this component is included in depth sort.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortedSprite;

/// The tile atlas texture index for each named tile type.
/// Matches the order in `assets/tile_gen.rs` `TILE_TYPES`.
const TILE_ATLAS_INDICES: &[(&str, usize)] = &[
    ("grass_passable", 0),
    ("grass_solid", 1),
    ("stone_solid", 2),
    ("lava_trap", 3),
    ("ice_passable", 4),
    ("bridge_passable", 5),
    ("snow_solid", 6),
    ("ice_trap", 7),
    ("checkpoint", 8),
    ("goal", 9),
];

/// Lookup from tile name string to atlas sprite index.
fn build_tile_name_to_index() -> HashMap<String, usize> {
    TILE_ATLAS_INDICES
        .iter()
        .map(|(name, idx)| (name.to_string(), *idx))
        .collect()
}

/// Sprite atlas resource holding the tile texture atlas handle and name→index lookup.
#[derive(Resource)]
pub struct VoxpartyAtlas {
    pub atlas_handle: Handle<Image>,
    /// Map from tile name (e.g. "grass_passable") → sprite index in the atlas
    pub tile_name_to_index: HashMap<String, usize>,
    /// Texture dimensions for grid_to_screen
    pub tex_width: f32,
    pub tex_height: f32,
}

/// Sprite plugin.
///
/// On `PreStartup`:
/// - Generates procedural tile sprites and character sprites
/// - Stores a `VoxpartyAtlas` resource
///
/// On `PostUpdate`:
/// - `iso_depth_sort_system`: sets transform.translation.z from GridPos depth key
pub struct SpritePlugin;

impl SpritePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, (build_tile_atlas,));
        app.add_systems(PostUpdate, iso_depth_sort_system);
    }
}

/// Build the tile TextureAtlas from procedural sprite bytes.
/// Runs in PreStartup so the atlas is ready before any rendering systems.
fn build_tile_atlas(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    // Generate tile sprites: 10 tiles × 64x32
    let (tile_w, tile_h, tile_bytes) = generate_tile_sprites();

    // Create Image from raw RGBA bytes using new_fill
    let extent = bevy::render::render_resource::Extent3d {
        width: tile_w,
        height: tile_h,
        depth_or_array_layers: 1,
    };
    let image = Image::new_fill(
        extent,
        TextureDimension::D2,
        &tile_bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let image_handle = images.add(image);

    let tile_name_to_index = build_tile_name_to_index();

    commands.insert_resource(VoxpartyAtlas {
        atlas_handle: image_handle,
        tile_name_to_index,
        tex_width: tile_w as f32,
        tex_height: tile_h as f32,
    });

    log::info!(
        "[Sprite] Created tile atlas: {}x{} px ({} bytes)",
        tile_w,
        tile_h,
        tile_w as usize * tile_h as usize * 4
    );
}

/// Isometric depth sort system.
/// Reads GridPos from all entities and sets transform.translation.z to the depth key.
/// This ensures correct back-to-front rendering in isometric projection.
pub fn iso_depth_sort_system(
    mut query: Query<(&GridPos, &mut Transform)>,
) {
    for (grid_pos, mut transform) in &mut query {
        transform.translation.z = grid_pos.depth_key() as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_pos_depth_key() {
        let pos = GridPos::new(3, 2, 0);
        assert_eq!(pos.depth_key(), 3 + 2 + 0 * 1000);
        assert_eq!(pos.depth_key(), 5);
    }

    #[test]
    fn test_grid_pos_depth_key_with_z() {
        let pos = GridPos::new(1, 1, 1);
        // z=1 should give depth > any z=0 regardless of x,y
        assert!(pos.depth_key() > GridPos::new(100, 100, 0).depth_key());
    }

    #[test]
    fn test_tile_name_to_index_contains_all_tiles() {
        let map = build_tile_name_to_index();
        assert_eq!(map.get("grass_passable"), Some(&0));
        assert_eq!(map.get("grass_solid"), Some(&1));
        assert_eq!(map.get("stone_solid"), Some(&2));
        assert_eq!(map.get("lava_trap"), Some(&3));
        assert_eq!(map.get("goal"), Some(&9));
    }
}
