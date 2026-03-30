use bevy::prelude::*;
use bevy_poc::tile::TilePlugin;
use bevy_poc::player::PlayerPlugin;
use bevy_poc::systems::CameraPlugin;

fn main() {
    eprintln!("POC running");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (640, 360).into(),
                title: "Bevy POC".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TilePlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(CameraPlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, debug_text_system)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 100.0)));

    // Spawn debug text UI
    commands.spawn((
        // Text string
        bevy::text::TextSpan("FPS: -- | Player: (--, --)".to_string()),
        // Font settings
        bevy::text::TextFont {
            font_size: 14.0,
            ..default()
        },
        // Layout
        bevy::text::TextLayout::default(),
        // Position
        bevy::ui::Node {
            position_type: bevy::ui::PositionType::Absolute,
            top: bevy::ui::Val::Px(10.0),
            left: bevy::ui::Val::Px(10.0),
            ..default()
        },
        DebugText,
    ));
}

fn debug_text_system(
    player: Query<&bevy_poc::player::Player, With<bevy_poc::player::IsPlayer>>,
    mut text: Query<&mut bevy::text::TextSpan, With<DebugText>>,
    time: Res<Time>,
    mut last_fps: Local<f32>,
) {
    let fps = 1.0 / time.delta_secs();
    if (fps - *last_fps).abs() > 0.5 {
        *last_fps = fps;

        if let Ok(player) = player.single() {
            for mut text in &mut text {
                text.0 = format!("FPS: {:.0} | Player: ({}, {})", fps, player.grid_x, player.grid_y);
            }
        }
    }
}

#[derive(Component)]
struct DebugText;
