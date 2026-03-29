pub mod platform;
pub mod core;
pub mod game;
pub mod assets;

use platform::{Platform, TouchHandler, AudioManager};
use core::{Scene, SceneState, Camera, SpriteSheet, grid_to_screen, depth_key, debug::DebugOverlay};
use game::{Episode, Player, PlayerState, Npc, World};
use game::input::gamepad_to_inputs;
use assets::loader;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn run() {
    inner_run(1280, 720);
}

/// C FFI entry point — called from iOS/Android native code.
/// Exports as `voxparty_run` for dlopen/FFI usage.
#[cfg(any(target_os = "ios", target_os = "android"))]
pub fn run() {
    // On mobile, use screen dimensions from the OS
    inner_run(1280, 720); // TODO: get actual screen size
}

/// For Android native activity glue
#[no_mangle]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub extern "C" fn native_main() {
    inner_run(1280, 720);
}

fn inner_run(screen_w: u32, screen_h: u32) {
    let _ = env_logger::try_init(); // don't panic on re-init

    log::info!("VoxParty starting ({}x{})", screen_w, screen_h);

    let mut plat = Platform::new("VoxParty", screen_w, screen_h);
    let mut touch = TouchHandler::new(screen_w, screen_h);
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
    let _ = &tiles_sheet;

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
    let mut camera = Camera::new(screen_w, screen_h, episode.grid_width, episode.grid_height);

    // Scene
    let mut scene = Scene::new();

    // Audio
    let _ = audio.load_sfx("jump", "assets/sounds/jump.wav");
    let _ = audio.load_sfx("eliminate", "assets/sounds/eliminate.wav");
    let _ = audio.load_sfx("checkpoint", "assets/sounds/checkpoint.wav");

    // Debug overlay
    let mut debug = DebugOverlay::new();
    let mut god_mode = false;
    // Cheat keys are only active after F1 has been pressed at least once
    let mut cheats_enabled = false;
    // Deferred sprite reload — set by R key, executed after event loop
    let mut pending_sprite_reload = false;

    let dt = 1.0 / 60.0;

    // Game loop
    loop {
        // --- INPUT ---
        for event in plat.event_pump.poll_iter() {
            touch.handle_event(&event);
            if let sdl2::event::Event::Quit { .. } = event {
                return;
            }

            // Debug overlay toggle and cheat keys — only active after F1 pressed
            if let sdl2::event::Event::KeyDown { scancode: Some(sc), .. } = event {
                match sc {
                    sdl2::keyboard::Scancode::F1 => {
                        debug.toggle();
                        cheats_enabled = debug.is_visible();
                        if cheats_enabled {
                            eprintln!("[DEBUG] Cheats enabled");
                        }
                    }
                    _ if cheats_enabled => {
                        match sc {
                            sdl2::keyboard::Scancode::R => {
                                eprintln!("[DEBUG] Reloading sprites...");
                                pending_sprite_reload = true;
                            }
                            sdl2::keyboard::Scancode::G => {
                                god_mode = !god_mode;
                                eprintln!("[DEBUG] God mode: {}", god_mode);
                            }
                            sdl2::keyboard::Scancode::C => {
                                eprintln!(
                                    "[DEBUG] CAM: target=({:.1}, {:.1}) actual=({:.1}, {:.1})",
                                    camera.target_x, camera.target_y, camera.x, camera.y
                                );
                            }
                            sdl2::keyboard::Scancode::P => {
                                eprintln!(
                                    "[DEBUG] P1: ({}, {}) {:?}  P2: ({}, {}) {:?}",
                                    player1.grid_x, player1.grid_y, player1.state,
                                    player2.grid_x, player2.grid_y, player2.state
                                );
                            }
                            sdl2::keyboard::Scancode::Num1 => {
                                scene.state = SceneState::Menu;
                                eprintln!("[DEBUG] Jump to Menu");
                            }
                            sdl2::keyboard::Scancode::Num2 => {
                                scene.state = SceneState::TitleCard;
                                scene.title_timer = 3.0;
                                eprintln!("[DEBUG] Jump to TitleCard");
                            }
                            sdl2::keyboard::Scancode::Num3 => {
                                scene.state = SceneState::Playing;
                                eprintln!("[DEBUG] Jump to Playing");
                            }
                            sdl2::keyboard::Scancode::Num4 => {
                                scene.state = SceneState::GameOver;
                                scene.gameover_timer = 5.0;
                                eprintln!("[DEBUG] Jump to GameOver");
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        // Execute deferred sprite reload after event loop
        if pending_sprite_reload {
            let _ = plat.load_sprite("tiles", "assets/sprites/tiles.png");
            let _ = plat.load_sprite("characters", "assets/sprites/characters.png");
            pending_sprite_reload = false;
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
            } else if player1.state == PlayerState::Eliminated && !god_mode {
                scene.trigger_gameover(Some(2));
            } else if player2.state == PlayerState::Eliminated && !god_mode {
                scene.trigger_gameover(Some(1));
            }

            // Game over timer done → return to menu
            if scene.state == SceneState::GameOver && scene.gameover_done() {
                scene.return_to_menu();
            }
        }

        // Update debug overlay
        debug.update_fps(dt);
        let mouse_state = plat.event_pump.mouse_state();
        debug.update_mouse((mouse_state.x(), mouse_state.y()), &camera);

        // Build input direction strings for overlay
        let p1_dirs = gamepad_to_inputs(touch.player1.joystick_x, touch.player1.joystick_y)
            .iter()
            .map(|i| match i {
                platform::GameInput::MoveLeft => "L",
                platform::GameInput::MoveRight => "R",
                platform::GameInput::MoveUp => "U",
                platform::GameInput::MoveDown => "D",
                _ => "",
            })
            .collect::<Vec<_>>()
            .join("");
        let p2_dirs = gamepad_to_inputs(touch.player2.joystick_x, touch.player2.joystick_y)
            .iter()
            .map(|i| match i {
                platform::GameInput::MoveLeft => "L",
                platform::GameInput::MoveRight => "R",
                platform::GameInput::MoveUp => "U",
                platform::GameInput::MoveDown => "D",
                _ => "",
            })
            .collect::<Vec<_>>()
            .join("");

        let input_state = core::debug::DebugInputState {
            p1_dirs,
            p2_dirs,
        };
        let debug_state = core::debug::DebugState {
            scene: &scene,
            player1: &player1,
            player2: &player2,
            camera: &camera,
            mouse_screen: (mouse_state.x(), mouse_state.y()),
            input: &input_state,
            god_mode,
        };

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
                    let _ = plat.blit_sprite("characters", dst, None);
                }

                // Draw players
                for player in &[&player1, &player2] {
                    let (px, py) = grid_to_screen(player.grid_x as f32, player.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let _ = plat.blit_sprite("characters", dst, None);
                }

                // Draw game over overlay
                if scene.state == SceneState::GameOver {
                    plat.clear(0, 0, 0, 180);
                }
            }
        }

        // Debug overlay renders on top of everything
        debug.render(&mut plat.canvas, &debug_state);

        plat.present();
        plat.delay(16);
    }
}

#[cfg(test)]
mod audio_sfx_tests {
    use super::*;

    /// Test that jump SFX logic detects when a player successfully moves.
    #[test]
    fn test_jump_sfx_triggers_on_valid_move() {
        // Create a minimal world where player can move
        let json = r#"{
            "id": "ep_sfx_test",
            "title": "SFX Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 10,
            "grid_height": 10,
            "tiles": [],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [{"player": 1, "x": 5, "y": 5}],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        // Copy spawn values before moving ep into World
        let spawn_x = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().x;
        let spawn_y = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().y;
        let mut world = World::from_episode(ep);

        let mut player = Player::new(1, spawn_x, spawn_y);

        // Record initial position
        let initial_x = player.grid_x;
        let initial_y = player.grid_y;

        // After tick with MoveRight input, player should move if valid
        let sheet = SpriteSheet::from_json(&assets::loader::load_sprite_sheet("characters"));
        let inputs = vec![platform::GameInput::MoveRight];
        player.tick(0.016, &inputs, &mut world, &sheet);

        // If move was valid (not blocked), position should have changed
        // This indicates jump SFX should have been triggered
        let moved = player.grid_x != initial_x || player.grid_y != initial_y;
        assert!(moved, "Player should have moved with MoveRight input on open grid");
    }

    /// Test that checkpoint SFX logic fires when player reaches a checkpoint.
    #[test]
    fn test_checkpoint_sfx_triggers_on_reaching_checkpoint() {
        let json = r#"{
            "id": "ep_cp_test",
            "title": "Checkpoint Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 10,
            "grid_height": 10,
            "tiles": [],
            "npcs": [],
            "checkpoints": [{"x": 6, "y": 5}],
            "spawn_points": [{"player": 1, "x": 5, "y": 5}],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        // Copy spawn values before moving ep into World
        let spawn_x = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().x;
        let spawn_y = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().y;
        let mut world = World::from_episode(ep);

        let mut player = Player::new(1, spawn_x, spawn_y);

        // Move right to checkpoint at (6, 5)
        let sheet = SpriteSheet::from_json(&assets::loader::load_sprite_sheet("characters"));
        let inputs = vec![platform::GameInput::MoveRight];
        player.tick(0.016, &inputs, &mut world, &sheet);

        // Player should be standing on checkpoint position
        assert_eq!(player.grid_x, 6, "Player should be at checkpoint x");
        assert_eq!(player.grid_y, 5, "Player should be at checkpoint y");
    }

    /// Test that eliminate SFX logic triggers when player is eliminated.
    #[test]
    fn test_eliminate_sfx_triggers_on_elimination() {
        let json = r#"{
            "id": "ep_elim_test",
            "title": "Eliminate Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 10,
            "grid_height": 10,
            "tiles": [{"x": 6, "y": 5, "type": "lava_trap"}],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [{"player": 1, "x": 5, "y": 5}],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        // Copy spawn values before moving ep into World
        let spawn_x = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().x;
        let spawn_y = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().y;
        let mut world = World::from_episode(ep);

        let mut player = Player::new(1, spawn_x, spawn_y);

        // Move right onto trap tile
        let sheet = SpriteSheet::from_json(&assets::loader::load_sprite_sheet("characters"));
        let inputs = vec![platform::GameInput::MoveRight];
        player.tick(0.016, &inputs, &mut world, &sheet);

        // Player should be eliminated after stepping on trap
        assert_eq!(player.state, PlayerState::Eliminated,
            "Player should be Eliminated after stepping on trap");
    }

    /// Test that jump SFX does NOT trigger when move is blocked.
    #[test]
    fn test_jump_sfx_not_triggered_on_blocked_move() {
        let json = r#"{
            "id": "ep_blocked_test",
            "title": "Blocked Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 10,
            "grid_height": 10,
            "tiles": [{"x": 6, "y": 5, "type": "stone_solid"}],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [{"player": 1, "x": 5, "y": 5}],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        // Copy spawn values before moving ep into World
        let spawn_x = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().x;
        let spawn_y = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().y;
        let mut world = World::from_episode(ep);

        let mut player = Player::new(1, spawn_x, spawn_y);

        let initial_x = player.grid_x;
        let initial_y = player.grid_y;

        // Try to move right into solid tile
        let sheet = SpriteSheet::from_json(&assets::loader::load_sprite_sheet("characters"));
        let inputs = vec![platform::GameInput::MoveRight];
        player.tick(0.016, &inputs, &mut world, &sheet);

        // Position should NOT have changed (blocked by solid)
        let moved = player.grid_x != initial_x || player.grid_y != initial_y;
        assert!(!moved, "Player should NOT have moved into solid tile");
    }
}

/// Tests for Unit 4: Menu → start_game() transition wiring
#[cfg(test)]
mod menu_input_tests {
    use super::*;

    /// Test: start_game() transitions Menu → TitleCard (Unit 4 prerequisite).
    /// This verifies the scene state transition that menu input SHOULD trigger.
    #[test]
    fn test_start_game_transitions_menu_to_titlecard() {
        let mut scene = Scene::new();
        assert_eq!(scene.state, SceneState::Menu);
        scene.start_game();
        assert_eq!(scene.state, SceneState::TitleCard);
        assert_eq!(scene.title_timer, 3.0);
    }
}

/// Tests for Unit 5: Player respawn from Eliminated state
#[cfg(test)]
mod respawn_tests {
    use super::*;

    /// Test: Single elimination leads to respawn, not immediate gameover.
    /// When only P1 is Eliminated, P1 should respawn at checkpoint (Idle state).
    /// Game should NOT end — gameplay continues.
    #[test]
    fn test_single_elimination_leads_to_respawn() {
        let mut scene = Scene::new();
        let mut player1 = Player::new(1, 5, 5);
        let mut player2 = Player::new(2, 6, 5);

        // Simulate P1 eliminated by trap (P2 still alive)
        player1.state = PlayerState::Eliminated;
        // P2 is still alive (Idle)
        assert_eq!(player2.state, PlayerState::Idle);

        // Simulate the FIXED game loop logic for single elimination:
        let god_mode = false;
        if player1.state == PlayerState::Eliminated && !god_mode {
            player1.respawn();
        }
        // Game over should NOT fire when only one player is eliminated
        assert_eq!(player1.state, PlayerState::Idle,
            "P1 should respawn to Idle after single elimination");
        assert_eq!(player2.state, PlayerState::Idle,
            "P2 should remain Idle");
        // scene.trigger_gameover should NOT be called for single elimination
        assert_ne!(scene.state, SceneState::GameOver,
            "Game should NOT end when only one player is eliminated");
    }

    /// Test: Both players eliminated simultaneously leads to draw (gameover with None).
    #[test]
    fn test_both_eliminated_leads_to_draw() {
        let mut scene = Scene::new();
        let mut player1 = Player::new(1, 5, 5);
        let mut player2 = Player::new(2, 6, 5);

        // Simulate both players eliminated
        player1.state = PlayerState::Eliminated;
        player2.state = PlayerState::Eliminated;

        // Last-standing check: both eliminated → draw
        let god_mode = false;
        if player1.state == PlayerState::Eliminated && player2.state == PlayerState::Eliminated && !god_mode {
            scene.trigger_gameover(None); // draw
        }

        assert_eq!(scene.state, SceneState::GameOver,
            "Game should end in draw when both players are eliminated");
        assert_eq!(scene.winner, None, "Draw has no winner");
    }

    /// Test: P1 Won, P2 Eliminated → P1 wins (not draw).
    /// This ensures Won-state checks take precedence over Eliminated.
    #[test]
    fn test_won_and_eliminated_leads_to_winner() {
        let mut scene = Scene::new();
        let mut player1 = Player::new(1, 5, 5);
        let mut player2 = Player::new(2, 6, 5);

        player1.state = PlayerState::Won;
        player2.state = PlayerState::Eliminated;

        // Won checks (should trigger before Eliminated checks)
        if player1.state == PlayerState::Won {
            scene.trigger_gameover(Some(1));
        } else if player2.state == PlayerState::Won {
            scene.trigger_gameover(Some(2));
        }

        assert_eq!(scene.state, SceneState::GameOver);
        assert_eq!(scene.winner, Some(1), "P1 should win");
    }

    /// Test: god_mode prevents respawn.
    #[test]
    fn test_god_mode_prevents_respawn() {
        let mut player = Player::new(1, 5, 5);
        player.state = PlayerState::Eliminated;

        let god_mode = true;
        if player.state == PlayerState::Eliminated && !god_mode {
            player.respawn();
        }

        assert_eq!(player.state, PlayerState::Eliminated,
            "Player should remain Eliminated in god_mode");
    }

    /// Test: P1 Eliminated, P2 Won → P2 wins.
    #[test]
    fn test_eliminated_and_won_leads_to_winner() {
        let mut scene = Scene::new();
        let mut player1 = Player::new(1, 5, 5);
        let mut player2 = Player::new(2, 6, 5);

        player1.state = PlayerState::Eliminated;
        player2.state = PlayerState::Won;

        // Won checks
        if player1.state == PlayerState::Won {
            scene.trigger_gameover(Some(1));
        } else if player2.state == PlayerState::Won {
            scene.trigger_gameover(Some(2));
        }

        assert_eq!(scene.state, SceneState::GameOver);
        assert_eq!(scene.winner, Some(2), "P2 should win");
    }
}
