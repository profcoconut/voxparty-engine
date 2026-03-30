use bevy::prelude::*;

#[derive(Component)]
pub struct Tile;

#[derive(Component, Clone)]
pub enum TileType {
    Grass,
    Wall,
}

impl TileType {
    pub fn color(&self) -> Color {
        match self {
            TileType::Grass => Color::srgb(0.227, 0.549, 0.227), // #3a8c3a
            TileType::Wall => Color::srgb(0.478, 0.478, 0.478),  // #7a7a7a
        }
    }
}

#[derive(Component)]
pub struct TilePosition {
    pub x: i32,
    pub y: i32,
}

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_grid);
    }
}

pub const GRID_SIZE: i32 = 10;

fn spawn_grid(mut commands: Commands) {
    use crate::iso::grid_to_screen;

    for y in 0..GRID_SIZE {
        for x in 0..GRID_SIZE {
            // Determine tile type: walls on border, grass inside
            let tile_type = if x == 0 || y == 0 || x == GRID_SIZE - 1 || y == GRID_SIZE - 1 {
                TileType::Wall
            } else {
                TileType::Grass
            };

            let (sx, sy) = grid_to_screen(x, y);
            let z = (x + y) as f32; // depth sorting key

            commands.spawn((
                Tile,
                tile_type.clone(),
                TilePosition { x, y },
                Sprite {
                    color: tile_type.color(),
                    custom_size: Some(Vec2::new(32.0, 16.0)),
                    ..default()
                },
                Transform::from_xyz(sx, sy, z),
            ));
        }
    }
}
