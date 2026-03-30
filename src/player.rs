use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub grid_x: i32,
    pub grid_y: i32,
    pub cooldown: f32,
}

#[derive(Component)]
pub struct IsPlayer;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub is_player: IsPlayer,
    pub intended_direction: IntendedDirection,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl PlayerBundle {
    pub fn new(grid_x: i32, grid_y: i32) -> Self {
        use crate::iso::grid_to_screen;
        let (sx, sy) = grid_to_screen(grid_x, grid_y);
        let z = (grid_x + grid_y) as f32 + 0.5; // draw above tiles

        PlayerBundle {
            player: Player {
                grid_x,
                grid_y,
                cooldown: 0.0,
            },
            is_player: IsPlayer,
            intended_direction: IntendedDirection::default(),
            sprite: Sprite {
                color: Color::srgb(0.9, 0.3, 0.3), // red player
                custom_size: Some(Vec2::new(28.0, 14.0)),
                ..default()
            },
            transform: Transform::from_xyz(sx, sy, z),
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (player_input_system, player_move_system).chain());
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn(PlayerBundle::new(5, 5));
}

// Direction the player intends to move
#[derive(Component, Default)]
pub struct IntendedDirection {
    pub dx: i32,
    pub dy: i32,
}

fn player_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut dir_query: Query<&mut IntendedDirection, With<IsPlayer>>,
) {
    if let Ok(mut dir) = dir_query.single_mut() {
        let up = keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp);
        let down = keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown);
        let left = keyboard.just_pressed(KeyCode::KeyA) || keyboard.just_pressed(KeyCode::ArrowLeft);
        let right = keyboard.just_pressed(KeyCode::KeyD) || keyboard.just_pressed(KeyCode::ArrowRight);

        dir.dx = 0;
        dir.dy = 0;

        if up {
            dir.dy = -1;
        } else if down {
            dir.dy = 1;
        } else if left {
            dir.dx = -1;
        } else if right {
            dir.dx = 1;
        }
    }
}

fn player_move_system(
    time: Res<Time>,
    intended_dir: Query<&IntendedDirection, With<IsPlayer>>,
    mut player: Query<(&mut Player, &mut Transform), With<IsPlayer>>,
    tiles: Query<(&crate::tile::TilePosition, &crate::tile::TileType)>,
) {
    if let Ok((mut player_comp, mut transform)) = player.single_mut() {
        let Ok(dir) = intended_dir.single() else {
            return;
        };

        // Decrease cooldown
        if player_comp.cooldown > 0.0 {
            player_comp.cooldown -= time.delta_secs();
            return;
        }

        // No direction = no move
        if dir.dx == 0 && dir.dy == 0 {
            return;
        }

        let new_x = player_comp.grid_x + dir.dx;
        let new_y = player_comp.grid_y + dir.dy;

        // Check tile at destination
        let mut is_wall = false;
        for (tile_pos, tile_type) in tiles.iter() {
            if tile_pos.x == new_x && tile_pos.y == new_y {
                if matches!(tile_type, crate::tile::TileType::Wall) {
                    is_wall = true;
                }
                break;
            }
        }

        if is_wall {
            // Can't walk into wall - reset cooldown anyway
            player_comp.cooldown = 0.2;
            return;
        }

        // Apply move
        player_comp.grid_x = new_x;
        player_comp.grid_y = new_y;
        player_comp.cooldown = 0.2;

        // Update transform
        let (sx, sy) = crate::iso::grid_to_screen(new_x, new_y);
        transform.translation.x = sx;
        transform.translation.y = sy;
        transform.translation.z = (new_x + new_y) as f32 + 0.5;
    }
}
