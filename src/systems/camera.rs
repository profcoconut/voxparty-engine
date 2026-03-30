use bevy::prelude::*;
use crate::player::IsPlayer;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, camera_follow_system);
    }
}

fn camera_follow_system(
    player: Query<&Transform, With<IsPlayer>>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<IsPlayer>)>,
) {
    if let Ok(player_transform) = player.single() {
        if let Ok(mut cam_transform) = camera.single_mut() {
            // Smooth lerp toward player
            let target_x = player_transform.translation.x;
            let target_y = player_transform.translation.y;

            cam_transform.translation.x += (target_x - cam_transform.translation.x) * 0.1;
            cam_transform.translation.y += (target_y - cam_transform.translation.y) * 0.1;
        }
    }
}
