//! Bevy plugin modules for VoxParty engine migration.
//!
//! Phase 1: App scaffold, window, input
//! Phase 2: Sprite rendering
//! Phase 3: Game entities (ECS)
//! Phase 4: Audio
//! Phase 5: Mobile haptics
//! Phase 6: Scene states
//! Phase 7: Mobile cross-compile

pub mod game_state;
pub mod input;

// ─── Phase 2: Sprite rendering ───────────────────────────────────────────────

pub mod sprite;       // TextureAtlas + GridPos + IsoDepthSystem
pub mod iso_camera;   // IsoCameraBundle + follow system
pub mod sprite_spawn; // Tile/player spawn systems
pub mod world;        // WorldComponent + VoxpartyWorld resource + tick systems
pub mod player;       // PlayerComponent + movement systems
pub mod npc;          // NpcComponent + NpcTag + dialogue/sprite systems
pub mod particle;     // Particle ECS components + spawner + systems

// ─── Phase 4: Audio ──────────────────────────────────────────────────────────

pub mod audio;        // AudioManager Bevy resource + AudioEvent + playback system

use bevy::prelude::*;

// Re-export types for convenience
pub use sprite::{GridPos, SortedSprite, SpritePlugin};
pub use sprite_spawn::{PlayerTag, TileTag, SpriteSpawnPlugin};
pub use iso_camera::{CameraShake, IsoCamera, IsoCameraBundle, IsoCameraPlugin, PreviousCamPos, camera_shake_system};
pub use player::{PlayerComponent, PlayerState, PlayerPlugin};
pub use npc::{NpcComponent, NpcDialogueState, NpcTag, EpisodeNpcs, NpcPlugin};
pub use particle::{Particle, ParticleType, ParticleColor, ParticleSpawner, ParticlePlugin};
pub use world::{WorldComponent, VoxpartyWorld, WorldPlugin};
pub use audio::{AudioEvent, AudioPlugin, audio_playback_observer};

/// Combined sprite rendering plugin for Phase 2.
/// Wires together: atlas creation, isometric camera, tile/player spawning, depth sorting.
///
/// System ordering (PostUpdate, in registration order):
/// 1. `SpritePlugin` systems: atlas setup (PreStartup), iso_depth_sort (PostUpdate)
/// 2. `IsoCameraPlugin` systems: iso_camera_follow, iso_depth_sort
/// 3. `SpriteSpawnPlugin` systems: tile_spawn, player_spawn
///
/// The camera follow runs before spawn systems, and depth sort runs after both,
/// ensuring correct back-to-front rendering.
pub struct BevySpritePlugin;

impl BevySpritePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for BevySpritePlugin {
    fn build(&self, app: &mut App) {
        // Add all sprite-related plugins
        // Phase 2: Sprite rendering
        app.add_plugins((
            SpritePlugin::new(),
            IsoCameraPlugin::new(1280.0, 720.0),
            SpriteSpawnPlugin::new(),
        ));
        // Phase 3: Game entities (ECS)
        app.add_plugins((
            WorldPlugin::new(),
            PlayerPlugin::new(),
            NpcPlugin::new(),
            ParticlePlugin::new(),
        ));
        // Phase 4: Audio
        app.add_plugins(AudioPlugin::new());
    }
}
