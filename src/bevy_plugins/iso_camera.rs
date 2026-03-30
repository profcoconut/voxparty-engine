//! Isometric camera system for VoxParty Bevy migration.
//!
//! Provides:
//! - `IsoCameraBundle` — Camera2d + orthographic projection for the game
//! - `IsoCamera` resource — holds screen dimensions
//! - `iso_camera_follow_system` — centers camera on player grid position with lerp
//! - `CameraShake` component — camera shake state (intensity, duration, elapsed)
//! - `camera_shake_system` — applies random shake offset to camera transform
//!
//! Depth sorting is handled by `sprite::iso_depth_sort_system` (not here).

use bevy::prelude::*;

use crate::core::isom::grid_to_screen;
use crate::core::isom::{TILE_H, TILE_W};
use crate::bevy_plugins::sprite::GridPos;
use crate::bevy_plugins::sprite_spawn::PlayerTag;
use crate::bevy_plugins::player::PlayerComponent;

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

/// Camera shake state component.
/// Attach to the camera entity to enable screen shake effects.
/// Trigger via `CameraShake { intensity, duration, elapsed: 0.0 }`.
#[derive(Component, Default)]
pub struct CameraShake {
    /// Maximum pixel offset magnitude.
    pub intensity: f32,
    /// Total duration of the shake in seconds.
    pub duration: f32,
    /// Elapsed time since shake started.
    pub elapsed: f32,
}

/// Resource tracking the camera position from the previous frame.
/// Used for smooth lerp-based following.
#[derive(Resource, Default)]
pub struct PreviousCamPos {
    pub x: f32,
    pub y: f32,
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
        app.insert_resource(PreviousCamPos::default());

        // Spawn camera bundle during PreUpdate so it's ready for PostUpdate systems
        app.add_systems(PreUpdate, Self::spawn_camera_system);

        // Camera follow runs before shake so shake can offset the followed position
        // Both run in PostUpdate after movement
        app.add_systems(PostUpdate, (
            iso_camera_follow_system.before(camera_shake_system),
            camera_shake_system,
        ));
    }
}

impl IsoCameraPlugin {
    fn spawn_camera_system(mut commands: Commands, iso_cam: Res<IsoCamera>) {
        // Spawn the isometric camera
        IsoCameraBundle::spawn(&mut commands, iso_cam.screen_width, iso_cam.screen_height);
    }
}

/// `PostUpdate` system — camera follows player 1's grid position with smooth lerp.
///
/// Reads `PlayerComponent` and `GridPos` of entities tagged with `PlayerTag` and
/// positions the camera so player 1 is centered on screen. Uses lerp for smooth
/// following. Camera shake is applied via the separate `camera_shake_system`.
///
/// Runs before `camera_shake_system` in PostUpdate so shake can offset the followed position.
pub fn iso_camera_follow_system(
    mut cameras: Query<&mut Transform, With<Camera2d>>,
    mut prev: ResMut<PreviousCamPos>,
    iso_cam: Res<IsoCamera>,
    // Query PlayerComponent to find player 1 (player.player_id == 1)
    players: Query<(&PlayerComponent, &GridPos), With<PlayerTag>>,
) {
    // Find player 1's entity by filtering for player_id == 1
    let Some((player, _grid_pos)) = players.iter().find(|(p, _)| p.player_id == 1) else {
        return;
    };

    // Use grid position from PlayerComponent
    let (sx, sy) = grid_to_screen(player.grid_x as f32, player.grid_y as f32, 0.0, 0.0);

    // Compute target camera position (centered on player)
    let target_screen_x = sx + TILE_W / 2.0;
    let target_screen_y = sy + TILE_H / 2.0;
    let target_x = target_screen_x - iso_cam.screen_width / 2.0;
    let target_y = target_screen_y - iso_cam.screen_height / 2.0;

    // Lerp from previous position toward target (stored back into prev for next frame)
    let lerp_speed = 0.15;
    prev.x += (target_x - prev.x) * lerp_speed;
    prev.y += (target_y - prev.y) * lerp_speed;

    // Apply lerped position to camera (shake is applied separately by camera_shake_system)
    for mut cam_transform in &mut cameras {
        // Clamp to world bounds (approximate — world bounds come from episode data)
        let max_cam_x = f32::MAX;
        let max_cam_y = f32::MAX;
        cam_transform.translation.x = prev.x.clamp(-max_cam_x, max_cam_x);
        cam_transform.translation.y = prev.y.clamp(-max_cam_y, max_cam_y);
        cam_transform.translation.z = 1000.0;
    }
}

/// `PostUpdate` system — applies camera shake effect.
///
/// Decays the shake over time and applies random offset to camera position.
pub fn camera_shake_system(
    mut cameras: Query<&mut Transform, With<Camera2d>>,
    mut shake: Query<&mut CameraShake>,
    time: Res<Time>,
) {
    let Ok(mut s) = shake.single_mut() else {
        return;
    };

    if s.elapsed >= s.duration {
        return;
    }

    s.elapsed += time.delta_secs();
    if s.elapsed >= s.duration {
        s.elapsed = s.duration;
        return;
    }

    // Decay shake intensity as time progresses
    let remaining = (s.duration - s.elapsed) / s.duration;
    let intensity = s.intensity * remaining;

    for mut cam_transform in &mut cameras {
        let shake_x = (rand_simple() * 2.0 - 1.0) * intensity;
        let shake_y = (rand_simple() * 2.0 - 1.0) * intensity;
        cam_transform.translation.x += shake_x;
        cam_transform.translation.y += shake_y;
    }
}

/// Deterministic pseudo-random for shake jitter — no external dependencies.
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos as f32 * 0.618034).fract()
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
