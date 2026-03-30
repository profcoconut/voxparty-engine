// Smoke test: verify all modules compile and the app can be constructed.
// Does NOT run the full game loop (requires window/scheduler infrastructure).

use bevy_poc::tile::TilePlugin;
use bevy_poc::player::PlayerPlugin;
use bevy_poc::systems::CameraPlugin;

fn main() {
    // Verify app builds and all plugins register without panic
    let _app = bevy::app::App::new()
        .add_plugins(TilePlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(CameraPlugin);

    eprintln!("Smoke test passed - app constructs cleanly");
}
