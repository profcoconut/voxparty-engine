//! Isometric camera system for VoxParty Bevy migration.
//!
//! Provides:
//! - `IsoCameraBundle` — Camera2d + orthographic projection for the game
//! - `IsoCamera` resource — holds screen dimensions
//! - `iso_camera_follow_system` — centers camera on player grid position
//!
//! Depth sorting is handled by `sprite::iso_depth_sort_system` (not here).

use bevy::prelude::*;

use crate::core::isom::grid_to_screen;
use crate::bevy_plugins::sprite::GridPos;
use crate::bevy_plugins::sprite_spawn::PlayerTag;

/// `IsoCamera` resource — holds screen dimensions for the isometric camera system.
#[derive(Resource)]
pub struct IsoCamera {
    pub screen_width: f32,
    pub screen_height: f32,
}

impl IsoCamera {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }
}

/// Camera2d bundle configured for the isometric game.
/// Uses orthographic projection.
pub struct IsoCameraBundle;

impl IsoCameraBundle {
    /// Spawn the isometric camera bundle.
    pub fn spawn(
        commands: &mut Commands,
        _screen_width: f32,
        _screen_height: f32,
    ) -> Entity {
        // Camera2d is a Bundle that auto-adds Camera and OrthographicProjection.
        // Basic positioning is handled by iso_camera_follow_system.
        commands.spawn((Camera2d, Transform::default())).id()
    }
}

/// Plugin for the isometric camera system.
/// Spawns the camera, stores screen dimensions, and registers follow/depth systems.
pub struct IsoCameraPlugin {
    pub screen_width: f32,
    pub screen_height: f32,
}

impl IsoCameraPlugin {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }
}

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IsoCamera::new(self.screen_width, self.screen_height));

        // Spawn camera bundle during PreUpdate so it's ready for PostUpdate systems
        app.add_systems(PreUpdate, Self::spawn_camera_system);

        // Camera follow runs in PostUpdate after movement
        app.add_systems(PostUpdate, iso_camera_follow_system);
    }
}

impl IsoCameraPlugin {
    fn spawn_camera_system(mut commands: Commands, iso_cam: Res<IsoCamera>) {
        // Spawn the isometric camera
        IsoCameraBundle::spawn(&mut commands, iso_cam.screen_width, iso_cam.screen_height);
    }
}

/// `PostUpdate` system — camera follows the player grid position.
///
/// Reads `GridPos` of entities tagged with `PlayerTag` and positions the
/// camera so the player is centered on screen.
pub fn iso_camera_follow_system(
    mut cameras: Query<&mut Transform, With<Camera2d>>,
    players: Query<&GridPos, With<PlayerTag>>,
    _iso_cam: Res<IsoCamera>,
) {
    // Get player position (assume single player for now)
    let Ok(player_pos) = players.single() else {
        return;
    };

    // Compute player's screen position (world space when camera is at origin)
    let (sx, sy) = grid_to_screen(player_pos.x as f32, player_pos.y as f32, 0.0, 0.0);

    // Center player on screen by offsetting camera position negatively.
    // Camera at (-sx, -sy) means world position (sx, sy) appears at screen center.
    let cam_x = -sx;
    let cam_y = -sy;

    // Apply to camera transform — z=1000 keeps camera above all sprites
    for mut cam_transform in &mut cameras {
        cam_transform.translation.x = cam_x;
        cam_transform.translation.y = cam_y;
        cam_transform.translation.z = 1000.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_camera_new() {
        let cam = IsoCamera::new(1280.0, 720.0);
        assert_eq!(cam.screen_width, 1280.0);
        assert_eq!(cam.screen_height, 720.0);
    }
}
