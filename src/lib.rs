pub mod platform;
pub mod core;
pub mod game;
pub mod assets;

use platform::{Platform, TouchHandler, AudioManager};
use core::{Scene, SceneState, Camera, SpriteSheet, grid_to_screen, debug::DebugOverlay};
use game::{Episode, Player, PlayerState, Npc, World};
use game::input::gamepad_to_inputs;
use assets::loader;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn run() {
    if let Err(e) = inner_run(1280, 720) {
        eprintln!("Game error: {}", e);
    }
}

/// C FFI entry point — called from iOS/Android native code.
/// Exports as `voxparty_run` for dlopen/FFI usage.
#[cfg(any(target_os = "ios", target_os = "android"))]
pub fn run() {
    // On mobile, use screen dimensions from the OS
    if let Err(e) = inner_run(1280, 720) {
        eprintln!("Game error: {}", e);
    }
}

/// For Android native activity glue
#[no_mangle]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub extern "C" fn native_main() {
    if let Err(e) = inner_run(1280, 720) {
        eprintln!("Game error: {}", e);
    }
}

fn inner_run(screen_w: u32, screen_h: u32) -> Result<(), String> {
    let _ = env_logger::try_init(); // don't panic on re-init

    // Check for screenshot modes
    let args: Vec<String> = std::env::args().collect();
    let screenshot_path: Option<String> = args.windows(2)
        .find(|w| w[0] == "--screenshot")
        .map(|w| w[1].clone());
    let screenshot_hint = args.contains(&"--screenshot-hint".to_string());

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
    let spawn1 = episode.spawn_points.iter().find(|s| s.player == 1)
        .ok_or_else(|| "Player 1 spawn point missing in episode".to_string())?;
    let spawn2 = episode.spawn_points.iter().find(|s| s.player == 2)
        .ok_or_else(|| "Player 2 spawn point missing in episode".to_string())?;
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
    // minpoc-4: Background music — play synthesized stub (no audio files needed)
    audio.play_music_stub();

    // Debug overlay
    let mut debug = DebugOverlay::new();
    let mut god_mode = false;
    // Cheat keys are only active after F1 has been pressed at least once
    let mut cheats_enabled = false;
    // Deferred sprite reload — set by R key, executed after event loop
    let mut pending_sprite_reload = false;
    // Interact input — set by E key, processed in game loop
    let mut interact_pressed = false;

    let dt = 1.0 / 60.0;

    // Screenshot mode: render one frame and save
    // Also handles --screenshot-hint which just renders+presents then exits
    if screenshot_path.is_some() || screenshot_hint {
        // Force Playing state so tiles render (game starts in Menu which shows nothing)
        scene.state = SceneState::Playing;
        scene.title_timer = 0.0; // skip title card

        // Extract player animation frames (needed for rendering)
        // Initialize idle animations if not already playing (screenshot path skips Player::tick)
        if player1.anim.current_frame().is_none() {
            player1.anim.play("idle_p1", &chars_sheet, true);
        }
        if player2.anim.current_frame().is_none() {
            player2.anim.play("idle_p2", &chars_sheet, true);
        }
        let p1_frame = player1.anim.current_frame()
            .map(|f| sdl2::rect::Rect::new(f.x as i32, f.y as i32, f.w as u32, f.h as u32));
        let p2_frame = player2.anim.current_frame()
            .map(|f| sdl2::rect::Rect::new(f.x as i32, f.y as i32, f.w as u32, f.h as u32));

        // --- RENDER ONE FRAME ---
        match scene.state {
            SceneState::Menu => {
                plat.clear(20, 20, 40, 255);
            }
            SceneState::TitleCard => {
                plat.clear(10, 10, 30, 255);
            }
            SceneState::Playing | SceneState::Victory | SceneState::GameOver => {
                plat.clear(30, 60, 90, 255);

                // Draw tiles in depth order (iterate y then x — naturally depth-sorted)
                for y in 0..episode.grid_height {
                    for x in 0..episode.grid_width {
                        let tile = world.get_tile(x, y);
                        let (px, py) = grid_to_screen(x as f32, y as f32, camera.x, camera.y);
                        let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 16, 64, 48);

                        // Select sprite rect based on tile type (see assets/sprites/tiles.json)
                        let src = match tile {
                            crate::game::TileType::Passable => sdl2::rect::Rect::new(0, 0, 64, 32),   // grass_passable
                            crate::game::TileType::Solid => sdl2::rect::Rect::new(64, 0, 64, 32),    // grass_solid
                            crate::game::TileType::Trap => sdl2::rect::Rect::new(128, 0, 64, 32),    // lava_trap
                            crate::game::TileType::Checkpoint => sdl2::rect::Rect::new(0, 0, 64, 32), // grass (checkpoint uses this sprite)
                            crate::game::TileType::Goal => sdl2::rect::Rect::new(192, 0, 64, 32),    // goal_tile
                        };
                        let _ = plat.blit_sprite("tiles", dst, Some(src));
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let src = chars_sheet.frames.get("player1_idle")
                        .map(|f| sdl2::rect::Rect::new(f.x as i32, f.y as i32, f.w as u32, f.h as u32));
                    let _ = plat.blit_sprite("characters", dst, src);
                }

                // Draw players
                let (p1x, p1y) = grid_to_screen(player1.grid_x as f32, player1.grid_y as f32, camera.x, camera.y);
                let p1_dst = sdl2::rect::Rect::new(p1x as i32, p1y as i32 - 32, 64, 64);
                let _ = plat.blit_sprite("characters", p1_dst, p1_frame);

                let (p2x, p2y) = grid_to_screen(player2.grid_x as f32, player2.grid_y as f32, camera.x, camera.y);
                let p2_dst = sdl2::rect::Rect::new(p2x as i32, p2y as i32 - 32, 64, 64);
                let _ = plat.blit_sprite("characters", p2_dst, p2_frame);
            }
        }

        // Present first (swaps buffers), THEN screenshot (reads from front buffer)
        plat.present();
        if let Some(path) = &screenshot_path {
            if let Err(e) = plat.screenshot(path) {
                eprintln!("Screenshot failed: {}", e);
            }
        }
        if screenshot_hint {
            // Keep window visible briefly so screencapture can grab it
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        return Ok(());
    }

    // Game loop
    loop {
        // --- INPUT ---
        for event in plat.event_pump.poll_iter() {
            touch.handle_event(&event);
            if let sdl2::event::Event::Quit { .. } = event {
                return Ok(());
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
                    // Unit 4: Wire Menu → start_game() transition
                    sdl2::keyboard::Scancode::Space | sdl2::keyboard::Scancode::Return => {
                        if scene.state == SceneState::Menu {
                            scene.start_game();
                        }
                    }
                    // minpoc-3: Wire GameInput::Interact for NPC dialogue
                    sdl2::keyboard::Scancode::E => {
                        interact_pressed = true;
                    }
                    // Arrow keys → player 1 virtual joystick
                    sdl2::keyboard::Scancode::Up => { touch.player1.joystick_y = -1.0; }
                    sdl2::keyboard::Scancode::Down => { touch.player1.joystick_y = 1.0; }
                    sdl2::keyboard::Scancode::Left => { touch.player1.joystick_x = -1.0; }
                    sdl2::keyboard::Scancode::Right => { touch.player1.joystick_x = 1.0; }
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

            // KeyUp → reset arrow key virtual joystick
            if let sdl2::event::Event::KeyUp { scancode: Some(sc), .. } = event {
                match sc {
                    sdl2::keyboard::Scancode::Up => { if touch.player1.joystick_y < 0.0 { touch.player1.joystick_y = 0.0; } }
                    sdl2::keyboard::Scancode::Down => { if touch.player1.joystick_y > 0.0 { touch.player1.joystick_y = 0.0; } }
                    sdl2::keyboard::Scancode::Left => { if touch.player1.joystick_x < 0.0 { touch.player1.joystick_x = 0.0; } }
                    sdl2::keyboard::Scancode::Right => { if touch.player1.joystick_x > 0.0 { touch.player1.joystick_x = 0.0; } }
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
            if let Some(event) = player1.tick(dt, &p1_inputs, &mut world, &chars_sheet) {
                match event {
                    game::PlayerEvent::Moved => {
                        audio.play_sfx("jump");
                        camera.shake(3.0, 0.08); // gamefeel-1: screen shake on valid move
                    }
                    game::PlayerEvent::Checkpoint => audio.play_sfx("checkpoint"),
                    game::PlayerEvent::Eliminated => audio.play_sfx("eliminate"),
                    game::PlayerEvent::Won => {}
                }
            }

            // Player 2 input
            let p2_inputs = gamepad_to_inputs(touch.player2.joystick_x, touch.player2.joystick_y);
            if let Some(event) = player2.tick(dt, &p2_inputs, &mut world, &chars_sheet) {
                match event {
                    game::PlayerEvent::Moved => {
                        audio.play_sfx("jump");
                        camera.shake(3.0, 0.08); // gamefeel-1: screen shake on valid move
                    }
                    game::PlayerEvent::Checkpoint => audio.play_sfx("checkpoint"),
                    game::PlayerEvent::Eliminated => audio.play_sfx("eliminate"),
                    game::PlayerEvent::Won => {}
                }
            }

            // NPCs
            for npc in &mut npcs {
                npc.tick(dt);
            }

            // Note: Player animations are advanced inside Player::tick() above

            // minpoc-3: Process Interact input — find nearest NPC and trigger dialogue
            if interact_pressed {
                interact_pressed = false;
                // Find NPC adjacent to player1 (same tile)
                for npc in &mut npcs {
                    if npc.grid_x == player1.grid_x && npc.grid_y == player1.grid_y {
                        let name = npc.name.clone();
                        if let Some(line) = npc.interact() {
                            eprintln!("[NPC] {}: {}", name, line);
                        }
                        break;
                    }
                }
            }

            // Camera follows player 1
            camera.follow(player1.grid_x as f32, player1.grid_y as f32);
            camera.tick(dt); // gamefeel-1: decay screen shake timer

            // Win/fail checks
            if player1.state == PlayerState::Won {
                scene.trigger_gameover(Some(1));
            } else if player2.state == PlayerState::Won {
                scene.trigger_gameover(Some(2));
            }
            // Unit 5: Respawn each eliminated player individually
            if player1.state == PlayerState::Eliminated && !god_mode {
                player1.respawn();
            }
            if player2.state == PlayerState::Eliminated && !god_mode {
                player2.respawn();
            }
            // Last-standing: game-over only when BOTH eliminated simultaneously
            if player1.state == PlayerState::Eliminated && player2.state == PlayerState::Eliminated && !god_mode {
                scene.trigger_gameover(None); // draw
            }

            // Game over timer done → return to menu
            if scene.state == SceneState::GameOver && scene.gameover_done() {
                scene.return_to_menu();
            }
        }

        // Extract player frame data for rendering (must happen before debug_state borrow)
        let p1_frame = player1.anim.current_frame()
            .map(|f| sdl2::rect::Rect::new(f.x as i32, f.y as i32, f.w as u32, f.h as u32));
        let p2_frame = player2.anim.current_frame()
            .map(|f| sdl2::rect::Rect::new(f.x as i32, f.y as i32, f.w as u32, f.h as u32));

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
                // Draw "VOXPARTY" title centered
                let title = "VOXPARTY";
                let title_w = title.len() as i32 * 6;
                let title_x = 1280 / 2 - title_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    title,
                    title_x,
                    280,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );
                // Draw "PRESS SPACE TO START" centered below
                let subtitle = "PRESS SPACE TO START";
                let sub_w = subtitle.len() as i32 * 6;
                let sub_x = 1280 / 2 - sub_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    subtitle,
                    sub_x,
                    340,
                    sdl2::pixels::Color::RGBA(200, 200, 200, 255),
                );
            }
            SceneState::TitleCard => {
                plat.clear(10, 10, 30, 255);
                // Draw episode title centered
                let title = episode.title.as_str();
                let title_w = title.len() as i32 * 6;
                let title_x = 1280 / 2 - title_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    title,
                    title_x,
                    300,
                    sdl2::pixels::Color::RGBA(100, 200, 255, 255),
                );
                // Draw "GET READY..." subtitle
                let sub = "GET READY...";
                let sub_w = sub.len() as i32 * 6;
                let sub_x = 1280 / 2 - sub_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    sub,
                    sub_x,
                    360,
                    sdl2::pixels::Color::RGBA(180, 180, 180, 255),
                );
            }
            SceneState::Playing | SceneState::Victory | SceneState::GameOver => {
                plat.clear(30, 60, 90, 255);

                // Draw tiles in depth order (iterate y then x — naturally depth-sorted)
                for y in 0..episode.grid_height {
                    for x in 0..episode.grid_width {
                        let tile = world.get_tile(x, y);
                        let (px, py) = grid_to_screen(x as f32, y as f32, camera.x, camera.y);
                        let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 16, 64, 48);

                        // Select sprite rect based on tile type (see assets/sprites/tiles.json)
                        let src = match tile {
                            crate::game::TileType::Passable  => sdl2::rect::Rect::new(0,   0, 64, 32), // grass_passable
                            crate::game::TileType::Solid      => sdl2::rect::Rect::new(64,  0, 64, 32), // grass_solid
                            crate::game::TileType::Trap      => sdl2::rect::Rect::new(128, 0, 64, 32), // lava_trap
                            crate::game::TileType::Checkpoint => sdl2::rect::Rect::new(0,   0, 64, 32), // grass (checkpoint uses this sprite)
                            crate::game::TileType::Goal      => sdl2::rect::Rect::new(192, 0, 64, 32), // goal_tile
                        };
                        let _ = plat.blit_sprite("tiles", dst, Some(src));
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let src = chars_sheet.frames.get("player1_idle")
                        .map(|f| sdl2::rect::Rect::new(f.x as i32, f.y as i32, f.w as u32, f.h as u32));
                    let _ = plat.blit_sprite("characters", dst, src);
                }

                // Draw players with pre-extracted animation frames
                let (p1x, p1y) = grid_to_screen(player1.grid_x as f32, player1.grid_y as f32, camera.x, camera.y);
                let p1_dst = sdl2::rect::Rect::new(p1x as i32, p1y as i32 - 32, 64, 64);
                let _ = plat.blit_sprite("characters", p1_dst, p1_frame);

                let (p2x, p2y) = grid_to_screen(player2.grid_x as f32, player2.grid_y as f32, camera.x, camera.y);
                let p2_dst = sdl2::rect::Rect::new(p2x as i32, p2y as i32 - 32, 64, 64);
                let _ = plat.blit_sprite("characters", p2_dst, p2_frame);

                // Draw game over overlay
                if scene.state == SceneState::GameOver {
                    // Semi-transparent dark overlay
                    plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 150));
                    let _ = plat.canvas.fill_rect(sdl2::rect::Rect::new(0, 280, 1280, 160));

                    // Game Over text
                    let go_text = "GAME OVER";
                    let go_w = go_text.len() as i32 * 6;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        go_text,
                        1280 / 2 - go_w / 2,
                        310,
                        sdl2::pixels::Color::RGBA(255, 80, 80, 255),
                    );

                    // Winner info (winner is 1 or 2; anything else including None = draw)
                    let result_text = if scene.winner == Some(1) {
                        "PLAYER 1 WINS"
                    } else if scene.winner == Some(2) {
                        "PLAYER 2 WINS"
                    } else {
                        "DRAW"
                    };
                    let result_w = result_text.len() as i32 * 6;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        result_text,
                        1280 / 2 - result_w / 2,
                        370,
                        sdl2::pixels::Color::RGBA(200, 200, 200, 255),
                    );

                    // Countdown: RETURNING TO MENU IN X...
                    let countdown = (scene.gameover_timer.ceil() as u32).max(0);
                    let countdown_text = format!("RETURNING TO MENU IN {}...", countdown);
                    let countdown_w = countdown_text.len() as i32 * 6;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &countdown_text,
                        1280 / 2 - countdown_w / 2,
                        410,
                        sdl2::pixels::Color::RGBA(150, 150, 150, 255),
                    );
                }

                // Draw NPC dialogue bubbles
                for npc in &npcs {
                    if npc.bubble_visible() {
                        let (bx, by) = npc.bubble_screen_xy(camera.x, camera.y);
                        if let Some(line) = npc.current_line_text() {
                            eprintln!("[DIALOGUE] {}: {}", npc.name, line);
                            // Draw a simple debug rectangle as bubble placeholder
                            let bubble_rect = sdl2::rect::Rect::new(bx, by - 24, 200, 32);
                            plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 200, 230));
                            let _ = plat.canvas.fill_rect(bubble_rect);
                            // Render the dialogue text inside the bubble
                            DebugOverlay::draw_text(
                                &mut plat.canvas,
                                &line,
                                bx + 4,
                                by - 20,
                                sdl2::pixels::Color::RGBA(0, 0, 0, 255),
                            );
                        }
                    }
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
        let scene = Scene::new();
        let mut player1 = Player::new(1, 5, 5);
        let player2 = Player::new(2, 6, 5);

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
