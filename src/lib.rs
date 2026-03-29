pub mod platform;
pub mod core;
pub mod game;
pub mod assets;

use platform::{Platform, TouchHandler, AudioManager};
use core::{Scene, SceneState, Camera, SpriteSheet, grid_to_screen, depth_key};
use game::{Episode, Player, PlayerState, Npc, World};
use game::input::gamepad_to_inputs;
use assets::loader;

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("VoxParty starting");

    let mut plat = Platform::new("VoxParty", 1280, 720);
    let mut touch = TouchHandler::new(1280, 720);
    let mut audio = AudioManager::new();

    // Load sprite textures (ignore errors if files missing)
    let _ = plat.load_sprite("tiles", "assets/sprites/tiles.png");
    let _ = plat.load_sprite("characters", "assets/sprites/characters.png");

    // Load episode
    let episode = Episode::load("assets/episodes/demo.json");
    let mut world = World::from_episode(episode.clone());

    // Sprite sheets
    let tiles_sheet = SpriteSheet::from_json(&loader::load_sprite_sheet("tiles"));
    let chars_sheet = SpriteSheet::from_json(&loader::load_sprite_sheet("characters"));
    let _ = &tiles_sheet; // suppress unused warning

    // Players
    let spawn1 = episode.spawn_points.iter().find(|s| s.player == 1).unwrap();
    let spawn2 = episode.spawn_points.iter().find(|s| s.player == 2).unwrap();
    let mut player1 = Player::new(1, spawn1.x, spawn1.y);
    let mut player2 = Player::new(2, spawn2.x, spawn2.y);

    // NPCs
    let mut npcs: Vec<Npc> = episode
        .npcs
        .iter()
        .map(|n| Npc::new(n.x, n.y, n.name.clone(), n.dialogue.clone()))
        .collect();

    // Camera
    let mut camera = Camera::new(1280, 720, episode.grid_width, episode.grid_height);

    // Scene
    let mut scene = Scene::new();

    // Audio
    let _ = audio.load_sfx("jump", "assets/sounds/jump.wav");
    let _ = audio.load_sfx("eliminate", "assets/sounds/eliminate.wav");
    let _ = audio.load_sfx("checkpoint", "assets/sounds/checkpoint.wav");
    let _ = &audio;

    let dt = 1.0 / 60.0;

    // Game loop
    loop {
        // --- INPUT ---
        for event in plat.event_pump.poll_iter() {
            touch.handle_event(&event);
            if let sdl2::event::Event::Quit { .. } = event {
                return;
            }
        }

        // --- UPDATE ---
        scene.tick(dt);
        world.tick_trap_cooldowns();

        if scene.state == SceneState::Playing {
            // Player 1 input
            let p1_inputs = gamepad_to_inputs(touch.player1.joystick_x, touch.player1.joystick_y);
            player1.tick(dt, &p1_inputs, &mut world, &chars_sheet);

            // Player 2 input
            let p2_inputs = gamepad_to_inputs(touch.player2.joystick_x, touch.player2.joystick_y);
            player2.tick(dt, &p2_inputs, &mut world, &chars_sheet);

            // NPCs
            for npc in &mut npcs {
                npc.tick(dt);
            }

            // Camera follows player 1
            camera.follow(player1.grid_x as f32, player1.grid_y as f32);

            // Win/fail checks
            if player1.state == PlayerState::Won {
                scene.trigger_gameover(Some(1));
            } else if player2.state == PlayerState::Won {
                scene.trigger_gameover(Some(2));
            } else if player1.state == PlayerState::Eliminated {
                scene.trigger_gameover(Some(2));
            } else if player2.state == PlayerState::Eliminated {
                scene.trigger_gameover(Some(1));
            }

            // Game over timer done → return to menu
            if scene.state == SceneState::GameOver && scene.gameover_done() {
                scene.return_to_menu();
            }
        }

        // --- RENDER ---
        match scene.state {
            SceneState::Menu => {
                plat.clear(20, 20, 40, 255);
            }
            SceneState::TitleCard => {
                plat.clear(10, 10, 30, 255);
            }
            SceneState::Playing | SceneState::GameOver => {
                plat.clear(30, 60, 90, 255);

                // Draw tiles in depth order
                for depth in 0..=(episode.grid_width + episode.grid_height) * 2 {
                    for y in 0..episode.grid_height {
                        for x in 0..episode.grid_width {
                            if depth_key(x, y, 0) == depth {
                                let tile = world.get_tile(x, y);
                                if tile != crate::game::TileType::Passable {
                                    let (px, py) = grid_to_screen(x as f32, y as f32, camera.x, camera.y);
                                    let src = sdl2::rect::Rect::new(0, 0, 64, 32);
                                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 16, 64, 48);
                                    let _ = plat.blit_sprite("tiles", dst, Some(src));
                                }
                            }
                        }
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let _ = (npc, dst);
                }

                // Draw players
                for player in &[&player1, &player2] {
                    let (px, py) = grid_to_screen(player.grid_x as f32, player.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let _ = (player, dst);
                }

                // Draw game over overlay
                if scene.state == SceneState::GameOver {
                    plat.clear(0, 0, 0, 180);
                }
            }
        }

        plat.present();
        plat.delay(16);
    }
}
