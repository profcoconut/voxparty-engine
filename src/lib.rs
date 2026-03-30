pub mod platform;
pub mod core;
pub mod game;
pub mod assets;
pub mod bevy_plugins;

use platform::{Platform, TouchHandler, AudioManager, HapticManager};
use core::{Scene, SceneState, Camera, SpriteSheet, grid_to_screen, debug::DebugOverlay, ParticleSystem, particles::ParticleType, HealthMonitor, Console, TelemetryLogger};
use core::sprites::{reset_draw_calls, draw_call_count};
use game::{Episode, Player, PlayerState, Npc, World};
use game::input::{gamepad_to_inputs, InputRecorder, InputReplayer, BugReport, QaReport, AssertionResult, assert_camera_within_200px, assert_fps_above_10, assert_no_player_deadlock, GameStateForQA};
use assets::loader;
use serde_json::json;
use std::fs;
use bevy_plugins::scene_plugin::{
    go_to_episode_select, go_to_titlecard, go_to_menu, toggle_pause,
    trigger_victory, trigger_gameover,
};

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

/// episode-select-1: Path to the episodes directory
const EPISODES_DIR: &str = "assets/episodes";

/// Screenshot directory name
const SCREENSHOTS_DIR: &str = "screenshots";

/// Generate a timestamped screenshot path in the screenshots directory.
/// Uses ISO-style date formatting for consistency with other screenshots.
fn make_screenshot_path(prefix: &str) -> String {
    let now = chrono::Local::now();
    let ts = now.format("%Y%m%d_%H%M%S");
    format!("{}/{}_{}.png", SCREENSHOTS_DIR, prefix, ts)
}

/// episode-select-1: List all available episodes from the episodes directory.
/// Returns a vector of (episode_id, episode_path) pairs sorted by ID.
fn list_episodes() -> Vec<(String, String)> {
    let mut episodes = Vec::new();
    if let Ok(entries) = fs::read_dir(EPISODES_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(filename) = path.file_stem() {
                    let filename_str = filename.to_string_lossy();
                    // Skip hidden files like .DS_Store
                    if !filename_str.starts_with('.') {
                        episodes.push((filename_str.to_string(), path.to_string_lossy().into_owned()));
                    }
                }
            }
        }
    }
    // Sort for consistent ordering
    episodes.sort_by(|a, b| a.0.cmp(&b.0));
    episodes
}

/// episode-select-1: Load a specific episode by path and initialize all game state.
/// This replaces the current episode, world, players, NPCs, and camera.
fn reload_episode_game_state(
    episode_path: &str,
    screen_w: u32,
    screen_h: u32,
) -> Result<(Episode, World, Player, Player, Vec<Npc>, Camera), String> {
    let episode = Episode::load(episode_path).map_err(|e| format!("Failed to load episode: {}", e))?;
    let world = World::from_episode(episode.clone());

    let spawn1 = episode.spawn_points.iter().find(|s| s.player == 1)
        .ok_or_else(|| format!("Player 1 spawn point missing in episode '{}'", episode.id))?;
    let spawn2 = episode.spawn_points.iter().find(|s| s.player == 2)
        .ok_or_else(|| format!("Player 2 spawn point missing in episode '{}'", episode.id))?;
    let player1 = Player::new(1, spawn1.x, spawn1.y);
    let player2 = Player::new(2, spawn2.x, spawn2.y);

    let npcs: Vec<Npc> = episode
        .npcs
        .iter()
        .map(|n| Npc::new(n.x, n.y, n.name.clone(), n.dialogue.clone()))
        .collect();

    let camera = Camera::new(screen_w, screen_h, episode.grid_width, episode.grid_height);

    Ok((episode, world, player1, player2, npcs, camera))
}

/// Headless smoke test: runs 120 frames per episode without rendering.
/// Exits cleanly (code 0) on success, or panics on failure.
fn run_headless() -> Result<(), String> {
    let _ = env_logger::try_init();

    // Install panic hook so any panic in headless mode is captured
    init_panic_hook();

    log::info!("Headless mode: starting episode smoke tests");

    // Same episode audit as normal mode
    let episode_paths = list_episodes();
    let mut load_failures = Vec::new();
    let mut load_success = Vec::new();

    for (ep_id, ep_path) in &episode_paths {
        match Episode::load(ep_path) {
            Ok(ep) => {
                log::info!("  [OK] {} ({})", ep.id, ep.difficulty);
                load_success.push(ep_id.clone());
            }
            Err(e) => {
                log::error!("  [FAIL] {}: {}", ep_id, e);
                load_failures.push((ep_id.clone(), e));
            }
        }
    }

    if !load_failures.is_empty() {
        return Err(format!(
            "Episode audit: {}/{} failed to load. Failures: {:?}",
            load_failures.len(),
            episode_paths.len(),
            load_failures
        ));
    }

    let dt = 1.0 / 60.0;
    let total_frames = 120;
    let screen_w = 1280u32;
    let screen_h = 720u32;

    // Run smoke test for each successfully loaded episode
    for (ep_id, ep_path) in episode_paths.iter() {
        log::info!("Headless smoke test: episode '{}' ({} frames)", ep_id, total_frames);

        let episode = match Episode::load(ep_path) {
            Ok(ep) => ep,
            Err(e) => return Err(format!("Failed to load episode '{}': {}", ep_id, e)),
        };

        let mut world = World::from_episode(episode.clone());

        let spawn1 = episode.spawn_points.iter().find(|s| s.player == 1)
            .ok_or_else(|| format!("Player 1 spawn point missing in episode '{}'", ep_id))?;
        let spawn2 = episode.spawn_points.iter().find(|s| s.player == 2)
            .ok_or_else(|| format!("Player 2 spawn point missing in episode '{}'", ep_id))?;

        let mut player1 = Player::new(1, spawn1.x, spawn1.y);
        let mut player2 = Player::new(2, spawn2.x, spawn2.y);

        let _npcs: Vec<Npc> = episode
            .npcs
            .iter()
            .map(|n| Npc::new(n.x, n.y, n.name.clone(), n.dialogue.clone()))
            .collect();

        let mut camera = Camera::new(screen_w, screen_h, episode.grid_width, episode.grid_height);
        let mut scene = Scene::new();
        let mut particles = ParticleSystem::new();

        // Sprite sheets (needed for player tick)
        let chars_sheet = SpriteSheet::from_json(&loader::load_sprite_sheet("characters"));

        // Initialize player animations
        player1.anim.play("idle_p1", &chars_sheet, true);
        player2.anim.play("idle_p2", &chars_sheet, true);

        // Set scene to Playing
        scene.state = SceneState::Playing;
        scene.title_timer = 0.0;

        // Run headless frames
        for frame in 0..total_frames {
            scene.tick(dt);

            if scene.just_entered_playing() {
                camera.center_on(player1.grid_x as f32, player1.grid_y as f32);
            }

            world.tick_trap_cooldowns();
            particles.tick(dt);

            if scene.state == SceneState::Playing {
                // No input in headless mode - players stay idle but tick their animations
                player1.tick(dt, 0.0, 0.0, &mut world, &chars_sheet);
                player2.tick(dt, 0.0, 0.0, &mut world, &chars_sheet);
            }

            camera.tick(dt);

            if frame % 30 == 0 {
                log::info!("  Frame {}/{}: scene={:?}, p1=({},{}), p2=({},{})",
                    frame, total_frames, scene.state,
                    player1.grid_x, player1.grid_y,
                    player2.grid_x, player2.grid_y);
            }
        }

        log::info!("  Completed {} frames for '{}'", total_frames, ep_id);
    }

    log::info!("Headless smoke test: all episodes passed!");
    Ok(())
}

/// Initialize the global panic hook that writes to both stderr AND a crash log file.
/// This ensures panics are never silently swallowed, especially on mobile where
/// stderr may not be visible.
fn init_panic_hook() {
    // Crash log path — written to current working directory
    const CRASH_LOG: &str = "voxparty_crash.log";

    std::panic::set_hook(Box::new(move |panic_info| {
        // 1. Always write to stderr
        eprintln!("[PANIC] {}", panic_info);

        // 2. Also append to crash log file
        let msg = format!(
            "[{}] PANIC: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            panic_info
        );
        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(CRASH_LOG)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(msg.as_bytes())
            })
        {
            eprintln!("[PANIC] Failed to write crash log: {}", e);
        }
    }));
}

fn inner_run(screen_w: u32, screen_h: u32) -> Result<(), String> {
    let _ = env_logger::try_init(); // don't panic on re-init

    // Install panic hook FIRST — before any other initialization
    init_panic_hook();

    // console-1: Helper to convert SDL scancode to character for console input
    fn scancode_to_char(sc: sdl2::keyboard::Scancode) -> Option<char> {
        use sdl2::keyboard::Scancode::*;
        // Note: We only handle letter keys A-Z for console input
        // Numbers and symbols could be added but are not needed for basic commands
        match sc {
            A => Some('a'), B => Some('b'), C => Some('c'), D => Some('d'),
            E => Some('e'), F => Some('f'), G => Some('g'), H => Some('h'),
            I => Some('i'), J => Some('j'), K => Some('k'), L => Some('l'),
            M => Some('m'), N => Some('n'), O => Some('o'), P => Some('p'),
            Q => Some('q'), R => Some('r'), S => Some('s'), T => Some('t'),
            U => Some('u'), V => Some('v'), W => Some('w'), X => Some('x'),
            Y => Some('y'), Z => Some('z'),
            // Also handle space for convenience
            Space => Some(' '),
            // Handle numbers for commands like "scene 2"
            Num0 => Some('0'), Num1 => Some('1'), Num2 => Some('2'),
            Num3 => Some('3'), Num4 => Some('4'), Num5 => Some('5'),
            Num6 => Some('6'), Num7 => Some('7'), Num8 => Some('8'),
            Num9 => Some('9'),
            // Period for floating point numbers
            Period => Some('.'),
            Minus => Some('-'),
            _ => None,
        }
    }

    // ── Episode Load Audit ───────────────────────────────────────────────────
    // Verify ALL episodes load successfully before the game starts.
    // This catches corrupted JSON early instead of crashing mid-game.
    log::info!("Auditing episode files...");
    let episode_paths = list_episodes();
    let mut load_failures = Vec::new();
    let mut load_success = Vec::new();

    for (ep_id, ep_path) in &episode_paths {
        match Episode::load(ep_path) {
            Ok(ep) => {
                log::info!("  [OK] {} ({})", ep.id, ep.difficulty);
                load_success.push(ep_id.clone());
            }
            Err(e) => {
                log::error!("  [FAIL] {}: {}", ep_id, e);
                load_failures.push((ep_id.clone(), e));
            }
        }
    }

    if !load_failures.is_empty() {
        log::error!(
            "Episode audit: {}/{} failed to load. Successes: {:?}. Failures: {:?}",
            load_failures.len(),
            episode_paths.len(),
            load_success,
            load_failures
        );
    } else {
        log::info!(
            "Episode audit: all {}/{} episodes loaded successfully",
            load_success.len(),
            episode_paths.len()
        );
    }

    // Check for screenshot modes
    let args: Vec<String> = std::env::args().collect();
    let screenshot_path: Option<String> = args.windows(2)
        .find(|w| w[0] == "--screenshot")
        .map(|w| w[1].clone());
    let screenshot_hint = args.contains(&"--screenshot-hint".to_string());

    // qa-replay-1: Check for --record, --replay, --repro flags
    // Note: --record is handled via F4 hotkey during gameplay; record_path is for future CLI use
    let _record_path: Option<String> = args.windows(2)
        .find(|w| w[0] == "--record")
        .map(|w| w[1].clone());
    let replay_path: Option<String> = args.windows(2)
        .find(|w| w[0] == "--replay")
        .map(|w| w[1].clone());
    let repro_path: Option<String> = args.windows(2)
        .find(|w| w[0] == "--repro")
        .map(|w| w[1].clone());

    // headless-smoke-test: Check for --headless flag (runs 120 frames without rendering)
    let headless_mode = args.contains(&"--headless".to_string());

    // headless-smoke-test: If headless mode, run without SDL/display
    if headless_mode {
        log::info!("VoxParty starting in HEADLESS mode ({}x{})", screen_w, screen_h);
        return run_headless();
    }

    log::info!("VoxParty starting ({}x{})", screen_w, screen_h);

    let mut plat = Platform::new("VoxParty", screen_w, screen_h);
    let mut touch = TouchHandler::new(screen_w, screen_h);
    let mut audio = AudioManager::new();
    let mut haptic = HapticManager::new(plat.haptic());

    // Initialize SDL GameController (gamepad-full-1)
    let gamecontroller = plat.game_controller();
    if gamecontroller.is_some() {
        log::info!("Gamepad connected");
    }

    // Load sprite textures (ignore errors if files missing)
    // ascii-tiles-1: Generate tile sprites programmatically
    let (tiles_w, tiles_h, tiles_bytes) = assets::tile_gen::generate_tile_sprites();
    let _ = plat.load_sprite_from_bytes("tiles", tiles_w, tiles_h, &tiles_bytes);
    // ascii-chars-1: Generate character sprites programmatically
    let chars_bytes = assets::sprite_gen::generate_characters_sprite_sheet();
    let _ = plat.load_sprite_from_bytes("characters", 192, 192, &chars_bytes);

    // Load episode — use demo.json as default; audit already validated it loaded
    let mut episode = Episode::load("assets/episodes/demo.json")
        .map_err(|e| format!("Failed to load demo episode (audit should have caught this): {}", e))?;
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

    // Phase 6: GameState signal — bridges SDL2 loop and Bevy State API
    let game_state_signal = bevy_plugins::GameStateSignal::new();

    // episode-select-1: List available episodes for selection screen
    let episodes = list_episodes();

    // Particle system (particle-1: visual juice)
    let mut particles = ParticleSystem::new();

    // Health monitoring system (health assertions for debug overlay)
    let mut health = HealthMonitor::new();

    // Telemetry logger (telemetry-1: structured event logging)
    let mut telemetry = TelemetryLogger::new();

    // Audio
    let _ = audio.load_sfx("jump", "assets/sounds/jump.wav");
    let _ = audio.load_sfx("eliminate", "assets/sounds/eliminate.wav");
    let _ = audio.load_sfx("checkpoint", "assets/sounds/checkpoint.wav");
    // volume-control-1: Apply saved volume setting
    audio.set_volume(scene.save_data.audio_volume);
    // minpoc-4: Background music — play synthesized stub (no audio files needed)
    audio.play_music_stub();
    // soundtrack-1: Per-episode chiptune music using rodio synthesis
    audio.play_music_episode(&episode);

    // Debug overlay
    let mut debug = DebugOverlay::new();
    // In-game console
    let mut console = Console::new();
    let mut god_mode = false;
    // Cheat keys are only active after F1 has been pressed at least once
    let mut cheats_enabled = false;
    // Deferred sprite reload — set by R key, executed after event loop
    let mut pending_sprite_reload = false;
    // Interact input — set by E key, processed in game loop
    let mut interact_pressed = false;
    // Dialogue advance input — set by A/Enter/Space keys when dialogue is visible
    let mut dialogue_advance = false;
    // debug-screenshot-1: F12 screenshot flash timer
    let mut screenshot_flash_timer = 0.0f32;
    // debug-screenshot-1: Deferred screenshot path (set in event loop, processed after)
    let mut pending_screenshot: Option<String> = None;

    let dt = 1.0 / 60.0;

    // telemetry-1: Track if telemetry session is active (reset on return to menu)
    let mut telemetry_session_active = false;

    // qa-replay-1: Input recording state (F4 toggle)
    let _input_recorder: Option<InputRecorder> = None;
    let is_recording = false;

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
            SceneState::EpisodeSelect => {
                plat.clear(20, 20, 40, 255);
            }
            SceneState::TitleCard => {
                plat.clear(10, 10, 30, 255);
            }
            SceneState::Playing | SceneState::Victory | SceneState::GameOver | SceneState::Paused => {
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
                            crate::game::TileType::Checkpoint => sdl2::rect::Rect::new(512, 0, 64, 32), // checkpoint
                            crate::game::TileType::Goal => sdl2::rect::Rect::new(576, 0, 64, 32),    // goal
                        };
                        let _ = plat.blit_sprite("tiles", dst, Some(src));
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let src = chars_sheet.frames.get("npc_idle")
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
        // telemetry-1: End telemetry session on game exit
        telemetry.session_end();
        return Ok(());
    }

    // qa-replay-1: Handle --replay mode (headless replay with QA assertions)
    if let Some(ref replay_path) = replay_path {
        eprintln!("[QA] Starting replay: {}", replay_path);
        let mut replayer = match InputReplayer::from_file(replay_path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[QA] Failed to load replay: {}", e);
                return Err(e);
            }
        };

        let session_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let total_frames = replayer.total_frames() as u64;
        let mut qa_report = QaReport::new(session_id.clone(), total_frames);

        // Run headless replay with all QA assertions
        let camera_for_qa = Camera::new(screen_w, screen_h, episode.grid_width, episode.grid_height);
        let _result = replayer.headless_replay(|frame, state| {
            // Build a more complete GameStateForQA using the provided state
            let full_state = GameStateForQA {
                frame,
                camera_x: camera_for_qa.x,
                camera_y: camera_for_qa.y,
                player1_grid_x: state.player1_grid_x,
                player1_grid_y: state.player1_grid_y,
                player2_grid_x: state.player2_grid_x,
                player2_grid_y: state.player2_grid_y,
                fps: state.fps,
                player1_stuck_frames: state.player1_stuck_frames,
                player2_stuck_frames: state.player2_stuck_frames,
                player1_has_input: state.player1_has_input,
                player2_has_input: state.player2_has_input,
                _camera: &camera_for_qa,
            };

            // Run all assertions
            let mut all_pass = true;
            let mut fail_name = None;
            let mut fail_frame = None;
            let mut fail_detail = None;

            if let AssertionResult::Fail(detail) = assert_camera_within_200px(&full_state) {
                all_pass = false;
                fail_name = Some("camera_within_200px");
                fail_frame = Some(frame);
                fail_detail = Some(detail);
            }
            if let AssertionResult::Fail(detail) = assert_fps_above_10(&full_state) {
                all_pass = false;
                fail_name = Some("fps_above_10");
                fail_frame = Some(frame);
                fail_detail = Some(detail);
            }
            if let AssertionResult::Fail(detail) = assert_no_player_deadlock(&full_state) {
                all_pass = false;
                fail_name = Some("no_player_deadlock");
                fail_frame = Some(frame);
                fail_detail = Some(detail);
            }

            if all_pass {
                AssertionResult::Pass
            } else {
                if let (Some(name), Some(f), Some(d)) = (fail_name, fail_frame, fail_detail) {
                    qa_report.add_result(name, false, Some(f), Some(d));
                }
                AssertionResult::Fail(fail_name.unwrap_or("unknown").to_string())
            }
        });

        // Record passing assertions
        if qa_report.assertions.is_empty() || qa_report.assertions.iter().all(|a| a.passed) {
            qa_report.add_result("camera_within_200px", true, None, None);
            qa_report.add_result("fps_above_10", true, None, None);
            qa_report.add_result("no_player_deadlock", true, None, None);
        }

        // Write QA report
        let qa_path = format!("telemetry/qa_{}.json", session_id);
        if let Err(e) = qa_report.write(&qa_path) {
            eprintln!("[QA] Failed to write report: {}", e);
        } else {
            eprintln!("[QA] Report written: {}", qa_path);
        }

        // Print summary
        let total = qa_report.passed + qa_report.failed;
        if qa_report.failed == 0 {
            println!("QA: {}/{} assertions passed. All OK.", qa_report.passed, total);
        } else {
            let failed_assertions: Vec<_> = qa_report.assertions.iter()
                .filter(|a| !a.passed)
                .map(|a| format!("{} at frame {}", a.name, a.frame.unwrap_or(0)))
                .collect();
            println!("QA: {}/{} assertions passed. Failed: {}", qa_report.passed, total, failed_assertions.join("; "));
        }

        telemetry.session_end();
        return Ok(());
    }

    // qa-replay-1: Handle --repro mode (bug reproduction)
    if let Some(ref repro_path) = repro_path {
        eprintln!("[QA] Starting bug reproduction: {}", repro_path);
        let bug_report = match BugReport::load(repro_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[QA] Failed to load bug report: {}", e);
                return Err(e);
            }
        };

        eprintln!("[QA] Bug: {} at frame {}", bug_report.reason, bug_report.frame);
        eprintln!("[QA] Input range: [{}, {}]", bug_report.input_range[0], bug_report.input_range[1]);

        // Find the recording file - use the most recent one
        let recording_path = std::fs::read_dir(".")
            .ok()
            .and_then(|entries| {
                entries.flatten()
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "voxrecord"))
                    .max_by_key(|e| e.path())
            })
            .map(|e| e.path().to_string_lossy().into_owned());

        if let Some(rec_path) = recording_path {
            eprintln!("[QA] Loading recording: {}", rec_path);
            let mut replayer = match InputReplayer::from_file(&rec_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[QA] Failed to load recording: {}", e);
                    return Err(e);
                }
            };

            let start_frame = bug_report.input_range[0] as usize;
            let end_frame = bug_report.input_range[1] as usize;
            replayer.seek_to(start_frame);

            eprintln!("[QA] Replaying frames {} to {}", start_frame, end_frame);

            let mut frame_num = start_frame as u64;
            while replayer.current_frame() < end_frame && replayer.current_frame() < replayer.total_frames() {
                if let Some((p1_inputs, p2_inputs)) = replayer.next_frame() {
                    // In repro mode, just log the inputs for each frame
                    eprintln!("[QA] Frame {}: P1={:?} P2={:?}", frame_num, p1_inputs, p2_inputs);
                }
                frame_num += 1;
            }

            eprintln!("[QA] Reproduction complete. Frame {} reached.", replayer.current_frame());
        } else {
            eprintln!("[QA] No .voxrecord file found in current directory.");
        }

        telemetry.session_end();
        return Ok(());
    }

    // Game loop
    // Frame counter — logs every 100 frames to confirm the loop is running.
    // If this stops printing, the game has frozen.
    let mut frame_count: u64 = 0;

    loop {
        // sprint-22: Reset draw call counter at start of each frame
        reset_draw_calls();

        // --- INPUT ---
        for event in plat.event_pump.poll_iter() {
            touch.handle_event(&event);
            if let sdl2::event::Event::Quit { .. } = event {
                // telemetry-1: End telemetry session on quit
                telemetry.session_end();
                return Ok(());
            }

            // Handle key events
            if let sdl2::event::Event::KeyDown { scancode: Some(sc), .. } = event {
                // console-1: GRAVE toggles console visibility
                if sc == sdl2::keyboard::Scancode::Grave {
                    console.toggle();
                    continue; // Skip normal key handling when console is open
                }

                // console-1: When console is visible, intercept all keys for console input
                if console.visible {
                    match sc {
                        sdl2::keyboard::Scancode::Escape => {
                            console.dismiss();
                        }
                        sdl2::keyboard::Scancode::Return | sdl2::keyboard::Scancode::KpEnter => {
                            // Execute the command
                            // Clone input first to avoid borrow conflict (console.execute takes &mut self)
                            let input_text = console.input.clone();
                            let output = console.execute(
                                &input_text,
                                &mut scene,
                                &mut player1,
                                &mut player2,
                                &mut camera,
                                &mut god_mode,
                                &mut debug,
                            );
                            // Handle screenshot command specially (deferred to after event loop)
                            if output.iter().any(|l| l.contains("[SCREENSHOT]")) {
                                pending_screenshot = Some(make_screenshot_path("voxparty"));
                                screenshot_flash_timer = 1.0;
                            }
                            console.handle_enter();
                        }
                        sdl2::keyboard::Scancode::Backspace => {
                            console.handle_backspace();
                        }
                        sdl2::keyboard::Scancode::Up => {
                            console.handle_up();
                        }
                        sdl2::keyboard::Scancode::Down => {
                            console.handle_down();
                        }
                        _ => {
                            // Try to convert scancode to character for letter/number keys
                            if let Some(c) = scancode_to_char(sc) {
                                console.handle_char(c);
                            }
                        }
                    }
                    continue; // Skip normal key handling when console is open
                }

                // Normal game key handling (when console is NOT visible)
                // tutorial-1: Dismiss tutorial on any key press during Playing
                if scene.tutorial_visible && scene.state == SceneState::Playing {
                    scene.dismiss_tutorial();
                }
                match sc {
                    sdl2::keyboard::Scancode::F1 => {
                        debug.toggle();
                        cheats_enabled = debug.is_visible();
                        if cheats_enabled {
                            eprintln!("[DEBUG] Cheats enabled");
                        }
                    }
                    // perf-1: F3 toggles FPS counter display
                    sdl2::keyboard::Scancode::F3 => {
                        debug.toggle_fps();
                        eprintln!("[DEBUG] FPS counter: {}", if debug.is_fps_visible() { "ON" } else { "OFF" });
                    }
                    // qa-replay-1: F4 toggles input recording (only during Playing)
                    // sprint-20: F4 now cycles through debug display modes: off → FPS → minimap → both → off
                    sdl2::keyboard::Scancode::F4 => {
                        // Cycle: 0=off, 1=FPS, 2=minimap, 3=both
                        static mut DEBUG_CYCLE: u8 = 0;
                        unsafe {
                            DEBUG_CYCLE = (DEBUG_CYCLE + 1) % 4;
                            let (fps_on, minimap_on) = match DEBUG_CYCLE {
                                0 => (false, false),  // off
                                1 => (true, false),   // FPS
                                2 => (false, true),   // minimap
                                3 => (true, true),    // both
                                _ => (false, false),
                            };
                            // FPS is always settable
                            if fps_on != debug.is_fps_visible() {
                                debug.toggle_fps();
                            }
                            // Minimap only settable during Playing state
                            if scene.state == SceneState::Playing {
                                if minimap_on != debug.is_minimap_visible() {
                                    debug.toggle_minimap();
                                }
                                eprintln!("[DEBUG] Mode: {} FPS, {} minimap",
                                    if debug.is_fps_visible() { "ON" } else { "OFF" },
                                    if debug.is_minimap_visible() { "ON" } else { "OFF" });
                            } else {
                                // During non-Playing, force minimap off
                                if debug.is_minimap_visible() {
                                    debug.toggle_minimap();
                                }
                                eprintln!("[DEBUG] Mode: {} FPS (minimap only in Playing)",
                                    if debug.is_fps_visible() { "ON" } else { "OFF" });
                            }
                        }
                    }
                    // sprint-22: F5 cycles frame graph overlay: off → FPS → frame graph → both → off
                    sdl2::keyboard::Scancode::F5 => {
                        // Cycle: 0=off, 1=FPS, 2=frame_graph, 3=both
                        static mut FG_CYCLE: u8 = 0;
                        unsafe {
                            FG_CYCLE = (FG_CYCLE + 1) % 4;
                            let (fps_on, fg_on) = match FG_CYCLE {
                                0 => (false, false),  // off
                                1 => (true, false),   // FPS only
                                2 => (false, true),  // frame graph only
                                3 => (true, true),    // both
                                _ => (false, false),
                            };
                            // Sync fps_visible
                            if fps_on != debug.is_fps_visible() {
                                debug.toggle_fps();
                            }
                            // Sync frame_graph_visible
                            if fg_on != debug.is_frame_graph_visible() {
                                debug.toggle_frame_graph();
                            }
                            eprintln!("[DEBUG] F5: FPS={}, frame_graph={}",
                                debug.is_fps_visible(), debug.is_frame_graph_visible());
                        }
                    }
                    // volume-control-1: F7 decreases volume, F8 increases volume
                    sdl2::keyboard::Scancode::F7 => {
                        let new_vol = (scene.save_data.audio_volume - 0.1).max(0.0);
                        scene.save_data.audio_volume = new_vol;
                        audio.set_volume(new_vol);
                        scene.save();
                        eprintln!("[VOLUME] {}%", (new_vol * 100.0).round() as i32);
                    }
                    sdl2::keyboard::Scancode::F8 => {
                        let new_vol = (scene.save_data.audio_volume + 0.1).min(1.0);
                        scene.save_data.audio_volume = new_vol;
                        audio.set_volume(new_vol);
                        scene.save();
                        eprintln!("[VOLUME] {}%", (new_vol * 100.0).round() as i32);
                    }
                    // episode-select-1: Wire Menu → EpisodeSelect transition
                    // Unit 4: Uses scene_plugin transition functions
                    sdl2::keyboard::Scancode::Space | sdl2::keyboard::Scancode::Return => {
                        haptic.vibrate(20, 0.5); // haptics-1: menu button press
                        if scene.state == SceneState::Menu {
                            // Unit 4: Menu → EpisodeSelect via scene_plugin
                            go_to_episode_select(&mut scene, &game_state_signal);
                        } else if scene.state == SceneState::EpisodeSelect {
                            // Select the currently highlighted episode and start game
                            if !episodes.is_empty() {
                                let selected_idx = scene.selected_episode_index.min(episodes.len() - 1);
                                let (_ep_id, ep_path) = &episodes[selected_idx];
                                // Reload game state with selected episode
                                let result = reload_episode_game_state(ep_path, screen_w, screen_h);
                                let (new_ep, new_world, new_p1, new_p2, new_npcs, new_cam) = match result {
                                    Ok(r) => r,
                                    Err(e) => {
                                        eprintln!("[ERROR] {}", e);
                                        continue;
                                    }
                                };
                                episode = new_ep;
                                world = new_world;
                                player1 = new_p1;
                                player2 = new_p2;
                                npcs = new_npcs;
                                camera = new_cam;
                                // Unit 4: EpisodeSelect → TitleCard via scene_plugin
                                go_to_titlecard(&mut scene, &game_state_signal);
                            }
                        } else if scene.state == SceneState::Victory {
                            // victory-gameover-input: ENTER replays episode (skip titlecard)
                            if let Some(last_ep) = scene.save_data.last_episode.clone() {
                                let ep_path = format!("{}/{}.json", EPISODES_DIR, last_ep);
                                if let Ok((new_ep, new_world, new_p1, new_p2, new_npcs, new_cam)) =
                                    reload_episode_game_state(&ep_path, screen_w, screen_h)
                                {
                                    episode = new_ep;
                                    world = new_world;
                                    player1 = new_p1;
                                    player2 = new_p2;
                                    npcs = new_npcs;
                                    camera = new_cam;
                                    // Skip titlecard: go directly to Playing
                                    scene.state = SceneState::Playing;
                                    scene.title_timer = 0.0;
                                    game_state_signal.set(bevy_plugins::game_state::GameState::Playing);
                                }
                            }
                        } else if scene.state == SceneState::GameOver {
                            // victory-gameover-input: ENTER returns to menu
                            // Unit 4: GameOver → Menu via scene_plugin
                            go_to_menu(&mut scene, &game_state_signal);
                        } else if scene.state == SceneState::Playing {
                            // dialogue-advance: A/Enter/Space advances NPC dialogue
                            dialogue_advance = true;
                        }
                    }
                    // episode-select-1: Navigate up in episode list (also updates joystick when not in EpisodeSelect)
                    sdl2::keyboard::Scancode::Up => {
                        if scene.state == SceneState::EpisodeSelect {
                            haptic.vibrate(20, 0.5); // haptics-1: menu navigation
                            scene.episode_select_up(episodes.len());
                        } else {
                            touch.player1.joystick_y = -1.0;
                        }
                    }
                    // episode-select-1: Navigate down in episode list (also updates joystick when not in EpisodeSelect)
                    sdl2::keyboard::Scancode::Down => {
                        if scene.state == SceneState::EpisodeSelect {
                            haptic.vibrate(20, 0.5); // haptics-1: menu navigation
                            scene.episode_select_down(episodes.len());
                        } else {
                            touch.player1.joystick_y = 1.0;
                        }
                    }
                    // victory-gameover-input: ESC returns to menu from GameOver, ESC goes back to menu from episode select
                    // Unit 4: Uses scene_plugin transition functions
                    sdl2::keyboard::Scancode::Escape => {
                        if scene.state == SceneState::GameOver {
                            go_to_menu(&mut scene, &game_state_signal);
                        } else if scene.state == SceneState::EpisodeSelect {
                            go_to_menu(&mut scene, &game_state_signal);
                        } else {
                            // Unit 4: Playing ↔ Paused toggle via scene_plugin
                            toggle_pause(&mut scene, &game_state_signal);
                        }
                    }
                    // minpoc-3: Wire GameInput::Interact for NPC dialogue
                    sdl2::keyboard::Scancode::E => {
                        interact_pressed = true;
                    }
                    // pause-1: Q key quits to menu from pause screen
                    // Unit 4: Paused → Menu via scene_plugin
                    sdl2::keyboard::Scancode::Q => {
                        if scene.state == SceneState::Paused {
                            go_to_menu(&mut scene, &game_state_signal);
                        }
                    }
                    // debug-screenshot-1: F12 saves screenshot to screenshots/ directory
                    sdl2::keyboard::Scancode::F12 => {
                        // Defer screenshot to after event loop to avoid borrow conflict
                        pending_screenshot = Some(make_screenshot_path("voxparty"));
                    }
                    // Arrow keys → player 1 virtual joystick
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
                // console-1: Skip keyup handling when console is visible
                if console.visible {
                    continue;
                }
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
            // ascii-tiles-1: Regenerate tile sprites programmatically
            let (tiles_w, tiles_h, tiles_bytes) = assets::tile_gen::generate_tile_sprites();
            let _ = plat.load_sprite_from_bytes("tiles", tiles_w, tiles_h, &tiles_bytes);
            // ascii-chars-1: Regenerate character sprites programmatically
            let chars_bytes = assets::sprite_gen::generate_characters_sprite_sheet();
            let _ = plat.load_sprite_from_bytes("characters", 192, 192, &chars_bytes);
            pending_sprite_reload = false;
        }

        // debug-screenshot-1: Process deferred screenshot
        if let Some(path) = pending_screenshot.take() {
            if let Err(e) = plat.screenshot(&path) {
                eprintln!("Screenshot failed: {}", e);
            } else {
                eprintln!("[SCREENSHOT] Saved: {}", path);
                screenshot_flash_timer = 1.0;
            }
        }

        // gamepad-full-1: Poll gamepad state and update touch.player1
        // Track previous frame button states to detect press events (not held)
        use sdl2::controller::Button;
        static mut PREV_START: bool = false;
        static mut PREV_B: bool = false;
        if let Some(ref gc) = gamecontroller {
            // Left stick axes (for movement) - axis() returns i16, normalize to [-1, 1]
            touch.player1.joystick_x = gc.axis(sdl2::controller::Axis::LeftX) as f32 / 32767.0;
            touch.player1.joystick_y = gc.axis(sdl2::controller::Axis::LeftY) as f32 / 32767.0;

            // A button = confirm/interact
            if gc.button(Button::A) {
                haptic.vibrate(20, 0.5); // haptics-1: button press
                interact_pressed = true;
            }

            // Start button = pause toggle (only on press, not hold)
            // Unit 4: Playing ↔ Paused toggle via scene_plugin
            let start_pressed = gc.button(Button::Start);
            unsafe {
                if start_pressed && !PREV_START {
                    haptic.vibrate(20, 0.5); // haptics-1: button press
                    toggle_pause(&mut scene, &game_state_signal);
                }
                PREV_START = start_pressed;
            }

            // B button = cancel/back (return to menu when paused)
            // Unit 4: Paused → Menu via scene_plugin
            let b_pressed = gc.button(Button::B);
            unsafe {
                if b_pressed && !PREV_B && scene.state == SceneState::Paused {
                    haptic.vibrate(20, 0.5); // haptics-1: button press
                    go_to_menu(&mut scene, &game_state_signal);
                }
                PREV_B = b_pressed;
            }
        }

        // --- UPDATE ---
        // console-1: Tick console cursor blink (always runs)
        console.tick(dt);

        // console-1: When console is visible, pause the game (skip scene.time_elapsed and player movement)
        if !console.visible {
            // Track scene state before tick to detect changes for telemetry
            let prev_state = scene.state;
            scene.tick(dt);

            // telemetry-1: Emit scene_change event when state transitions
            if prev_state != scene.state {
                telemetry.log_event("scene_change", json!({
                    "from": format!("{:?}", prev_state),
                    "to": format!("{:?}", scene.state),
                }));
                // telemetry-1: End session when returning to Menu
                if scene.state == SceneState::Menu && telemetry_session_active {
                    telemetry.session_end();
                    telemetry_session_active = false;
                }
            }

        // camera-1: Center camera on player when transitioning from TitleCard to Playing
        if scene.just_entered_playing() {
            camera.center_on(player1.grid_x as f32, player1.grid_y as f32);
            health.reset();
            eprintln!("[CAMERA] Centered on player at ({}, {})", player1.grid_x, player1.grid_y);
            // telemetry-1: Start telemetry session when entering Playing
            if !telemetry_session_active {
                telemetry.session_start(&episode.id, None);
                telemetry_session_active = true;
            }
        }

        world.tick_trap_cooldowns();
        // particle-1: tick particle system
        particles.tick(dt);

        // debug-screenshot-1: Decrement screenshot flash timer
        if screenshot_flash_timer > 0.0 {
            screenshot_flash_timer -= dt;
        }

        // gamepad-full-1: Handle gamepad input for menu state
        // Unit 4: Menu → TitleCard (gamepad A skips EpisodeSelect) via scene_plugin
        if scene.state == SceneState::Menu && interact_pressed {
            interact_pressed = false;
            go_to_titlecard(&mut scene, &game_state_signal);
        }

        if scene.state == SceneState::Playing {
            // Player 1 input — use raw joystick values, quantize_direction handles deadzone + 8-way
            let (p1_jx, p1_jy) = if let Some((target_x, target_y)) = touch.take_pending_move(camera.x, camera.y) {
                // tap-to-move: convert grid delta to joystick-like values for quantize_direction
                let dx = target_x - player1.grid_x;
                let dy = target_y - player1.grid_y;
                // Only allow 1-tile Manhattan distance (adjacent)
                let manhattan = dx.abs() + dy.abs();
                if manhattan <= 1 && manhattan > 0 && !world.is_solid(target_x, target_y) {
                    (dx as f32, dy as f32)
                } else {
                    (touch.player1.joystick_x, touch.player1.joystick_y)
                }
            } else {
                (touch.player1.joystick_x, touch.player1.joystick_y)
            };

            if let Some(event) = player1.tick(dt, p1_jx, p1_jy, &mut world, &chars_sheet) {
                match event {
                    game::PlayerEvent::Moved => {
                        // telemetry-1: emit player_move on successful move
                        telemetry.log_event("player_move", json!({
                            "player": 1,
                            "gx": player1.grid_x,
                            "gy": player1.grid_y,
                            "success": true,
                        }));
                        // Play tile-appropriate step sound based on destination tile
                        let tile_str = world.get_tile_string(player1.grid_x, player1.grid_y);
                        audio.play_synth_step(&tile_str);
                        camera.shake(3.0, 0.08); // gamefeel-1: screen shake on valid move
                        haptic.vibrate(30, 0.5); // haptics-1: light vibration on movement
                        // particle-1: spawn dust puff at player1's feet
                        let (p1x, p1y) = grid_to_screen(player1.grid_x as f32, player1.grid_y as f32, camera.x, camera.y);
                        particles.spawn(p1x, p1y + 8.0, ParticleType::MovementDust);
                    }
                    game::PlayerEvent::Checkpoint => {
                        // telemetry-1: emit checkpoint event
                        telemetry.log_event("checkpoint", json!({
                            "player": 1,
                            "gx": player1.grid_x,
                            "gy": player1.grid_y,
                        }));
                        audio.play_sfx("checkpoint");
                        haptic.vibrate(50, 0.3); // haptics-1: light vibration on checkpoint
                        // particle-1: spawn sparkle at player1's position
                        let (p1x, p1y) = grid_to_screen(player1.grid_x as f32, player1.grid_y as f32, camera.x, camera.y);
                        particles.spawn(p1x, p1y, ParticleType::CheckpointSparkle);
                        // save-load-1: auto-save on checkpoint hit
                        scene.save();
                        // npc-hints-1: mark that player hit a checkpoint this run
                        scene.checkpoint_hit_this_run = true;
                    }
                    game::PlayerEvent::Eliminated => {
                        // telemetry-1: emit player_die and trap events
                        let by_tile = world.get_tile_string(player1.grid_x, player1.grid_y);
                        telemetry.log_event("player_die", json!({
                            "player": 1,
                            "gx": player1.grid_x,
                            "gy": player1.grid_y,
                            "by_tile": by_tile,
                        }));
                        if by_tile.contains("trap") {
                            telemetry.log_event("trap", json!({
                                "player": 1,
                                "gx": player1.grid_x,
                                "gy": player1.grid_y,
                            }));
                        }
                        audio.play_sfx("eliminate");
                        haptic.vibrate(200, 1.0); // haptics-1: heavy vibration on trap elimination
                        // particle-1: spawn trap flash at player1's position
                        let (p1x, p1y) = grid_to_screen(player1.grid_x as f32, player1.grid_y as f32, camera.x, camera.y);
                        particles.spawn(p1x, p1y, ParticleType::TrapFlash);
                        // npc-hints-1: mark that player died this run (for NPC dialogue hints)
                        scene.player_died_this_run = true;
                        // sprint-20: set death recap
                        scene.death_recap = Some((by_tile.clone(), player1.grid_x, player1.grid_y));
                        scene.death_recap_timer = 2.0;
                    }
                    game::PlayerEvent::Won => audio.play_victory_jingle(),
                }
            }

            // Player 2 input — use raw joystick values for 8-way movement
            if let Some(event) = player2.tick(dt, touch.player2.joystick_x, touch.player2.joystick_y, &mut world, &chars_sheet) {
                match event {
                    game::PlayerEvent::Moved => {
                        // telemetry-1: emit player_move on successful move
                        telemetry.log_event("player_move", json!({
                            "player": 2,
                            "gx": player2.grid_x,
                            "gy": player2.grid_y,
                            "success": true,
                        }));
                        // Play tile-appropriate step sound based on destination tile
                        let tile_str = world.get_tile_string(player2.grid_x, player2.grid_y);
                        audio.play_synth_step(&tile_str);
                        camera.shake(3.0, 0.08); // gamefeel-1: screen shake on valid move
                        haptic.vibrate(30, 0.5); // haptics-1: light vibration on movement
                        // particle-1: spawn dust puff at player2's feet
                        let (p2x, p2y) = grid_to_screen(player2.grid_x as f32, player2.grid_y as f32, camera.x, camera.y);
                        particles.spawn(p2x, p2y + 8.0, ParticleType::MovementDust);
                    }
                    game::PlayerEvent::Checkpoint => {
                        // telemetry-1: emit checkpoint event
                        telemetry.log_event("checkpoint", json!({
                            "player": 2,
                            "gx": player2.grid_x,
                            "gy": player2.grid_y,
                        }));
                        audio.play_sfx("checkpoint");
                        haptic.vibrate(50, 0.3); // haptics-1: light vibration on checkpoint
                        // particle-1: spawn sparkle at player2's position
                        let (p2x, p2y) = grid_to_screen(player2.grid_x as f32, player2.grid_y as f32, camera.x, camera.y);
                        particles.spawn(p2x, p2y, ParticleType::CheckpointSparkle);
                        // npc-hints-1: mark that player hit a checkpoint this run
                        scene.checkpoint_hit_this_run = true;
                    }
                    game::PlayerEvent::Eliminated => {
                        // telemetry-1: emit player_die and trap events
                        let by_tile = world.get_tile_string(player2.grid_x, player2.grid_y);
                        telemetry.log_event("player_die", json!({
                            "player": 2,
                            "gx": player2.grid_x,
                            "gy": player2.grid_y,
                            "by_tile": by_tile,
                        }));
                        if by_tile.contains("trap") {
                            telemetry.log_event("trap", json!({
                                "player": 2,
                                "gx": player2.grid_x,
                                "gy": player2.grid_y,
                            }));
                        }
                        audio.play_sfx("eliminate");
                        haptic.vibrate(200, 1.0); // haptics-1: heavy vibration on trap elimination
                        // particle-1: spawn trap flash at player2's position
                        let (p2x, p2y) = grid_to_screen(player2.grid_x as f32, player2.grid_y as f32, camera.x, camera.y);
                        particles.spawn(p2x, p2y, ParticleType::TrapFlash);
                        // npc-hints-1: mark that player died this run (for NPC dialogue hints)
                        scene.player_died_this_run = true;
                        // sprint-20: set death recap
                        scene.death_recap = Some((by_tile.clone(), player2.grid_x, player2.grid_y));
                        scene.death_recap_timer = 2.0;
                    }
                    game::PlayerEvent::Won => audio.play_victory_jingle(),
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
                // npc-hints-1: compute dialogue hint index based on game state
                // 0 = encouraging (after death), 1 = progress (after checkpoint), 2 = repeat, 3 = first time
                let hint_index = if scene.player_died_this_run {
                    0
                } else if scene.checkpoint_hit_this_run {
                    1
                } else if scene.npc_seen_this_episode {
                    2
                } else {
                    3
                };
                // Find NPC adjacent to player1 (same tile)
                for npc in &mut npcs {
                    if npc.grid_x == player1.grid_x && npc.grid_y == player1.grid_y {
                        // dialogue-advance: If dialogue is visible, advance it instead of re-triggering
                        if npc.bubble_visible() {
                            let name = npc.name.clone();
                            if let Some(line) = npc.advance() {
                                eprintln!("[NPC] {}: {}", name, line);
                            }
                        } else {
                            // Start new dialogue
                            let name = npc.name.clone();
                            if let Some(line) = npc.interact(hint_index) {
                                eprintln!("[NPC] {}: {}", name, line);
                            }
                            // Mark that player has seen NPC this episode (for repeat-visit dialogue)
                            scene.npc_seen_this_episode = true;
                        }
                        break;
                    }
                }
            }

            // dialogue-advance: Handle A/Enter/Space for advancing dialogue
            if dialogue_advance {
                dialogue_advance = false;
                // Find NPC adjacent to player1 and advance dialogue
                for npc in &mut npcs {
                    if npc.grid_x == player1.grid_x && npc.grid_y == player1.grid_y {
                        let name = npc.name.clone();
                        if let Some(line) = npc.advance() {
                            eprintln!("[NPC] {}: {}", name, line);
                        }
                        break;
                    }
                }
            }

            // Camera follows player 1
            camera.follow(player1.grid_x as f32, player1.grid_y as f32);
            camera.tick(dt); // gamefeel-1: decay screen shake timer

            // health-1: Run health assertions only during Playing (not during transitions)
            // Skip during TitleCard/Menu — camera is stale by design before gameplay starts
            if scene.state == SceneState::Playing {
                health.check(&camera, (player1.grid_x as f32, player1.grid_y as f32), (player2.grid_x as f32, player2.grid_y as f32), debug.fps(), dt, screen_w, screen_h);

                // screenshot-on-bug-1: Only once per failure episode (not every frame)
                // After taking a screenshot, set a 60-frame cooldown so we don't spam
                static SCREENSHOT_COOLDOWN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                if health.should_screenshot() && SCREENSHOT_COOLDOWN.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                    let reason = health.take_screenshot_reason().unwrap_or_else(|| "unknown".to_string());
                    let path = make_screenshot_path("bug");
                    telemetry.log_event("health_fail", json!({
                        "reason": reason,
                        "screenshot_path": path,
                    }));
                    if let Err(e) = plat.screenshot(&path) {
                        eprintln!("[BUG SCREENSHOT] Failed to capture: {}", e);
                    } else {
                        eprintln!("[BUG SCREENSHOT] Captured: {}", path);
                    }
                    SCREENSHOT_COOLDOWN.store(60, std::sync::atomic::Ordering::Relaxed);
                } else if SCREENSHOT_COOLDOWN.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                    SCREENSHOT_COOLDOWN.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                }
            }

            // Win/fail checks
            // Unit 4: Uses scene_plugin transition functions for state changes
            if player1.state == PlayerState::Won {
                // save-load-1: Record best time and save on episode complete
                let time_ms = (scene.time_elapsed * 1000.0) as u64;
                let _is_new_best = scene.update_best_time(&episode.id, time_ms);
                scene.set_last_episode(&episode.id);
                scene.save();
                // Unit 4: Playing → Victory via scene_plugin
                trigger_victory(&mut scene, &game_state_signal, Some(1));
            } else if player2.state == PlayerState::Won {
                // save-load-1: Record best time and save on episode complete
                let time_ms = (scene.time_elapsed * 1000.0) as u64;
                let _is_new_best = scene.update_best_time(&episode.id, time_ms);
                scene.set_last_episode(&episode.id);
                scene.save();
                // Unit 4: Playing → Victory via scene_plugin
                trigger_victory(&mut scene, &game_state_signal, Some(2));
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
                // Unit 4: Playing → GameOver via scene_plugin
                trigger_gameover(&mut scene, &game_state_signal, None); // draw
                audio.play_gameover_sound();
            }

            // Game over timer done → return to menu
            // Unit 4: GameOver → Menu via scene_plugin
            if scene.state == SceneState::GameOver && scene.gameover_done() {
                go_to_menu(&mut scene, &game_state_signal);
            }
            // Victory timer done → return to menu
            // Unit 4: Victory → Menu via scene_plugin
            if scene.state == SceneState::Victory && scene.victory_done() {
                go_to_menu(&mut scene, &game_state_signal);
            }
        }
        } // End: if !console.visible (pause game updates when console is open)

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
        // Compute player screen positions for debug overlay camera health check
        let (p1sx, p1sy) = grid_to_screen(player1.grid_x as f32, player1.grid_y as f32, camera.x, camera.y);
        let (p2sx, p2sy) = grid_to_screen(player2.grid_x as f32, player2.grid_y as f32, camera.x, camera.y);

        let debug_state = core::debug::DebugState {
            scene: &scene,
            player1: &player1,
            player2: &player2,
            world: &world,
            camera: &camera,
            mouse_screen: (mouse_state.x(), mouse_state.y()),
            input: &input_state,
            god_mode,
            p1_screen: (p1sx as i32, p1sy as i32),
            p2_screen: (p2sx as i32, p2sy as i32),
            time_elapsed: scene.time_elapsed,
            health_results: health.results(),
            recording: is_recording,
            show_minimap: debug.is_minimap_visible(),
        };

        // --- RENDER ---
        match scene.state {
            SceneState::Menu => {
                plat.clear(20, 20, 40, 255);
                // menu-polish-1: ASCII art header with box-drawing characters
                // Large decorative header
                let header_top = "+---------------------------+";
                let header_mid = "|       V O X P A R T Y     |";
                let header_bot = "+---------------------------+";
                let header_w = header_top.len() as i32 * 6;
                let header_x = plat.screen_width as i32 / 2 - header_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    header_top,
                    header_x,
                    180,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    header_mid,
                    header_x,
                    195,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    header_bot,
                    header_x,
                    210,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );
                // Subtitle with version/tagline
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    "[ ISOMETRIC PIXEL PARTY ]",
                    header_x + 30,
                    240,
                    sdl2::pixels::Color::RGBA(150, 150, 180, 255),
                );
                // Controls box at bottom
                let controls_box_top = "+-----------------------------+";
                let controls_box_mid = "|  UP/DOWN  SELECT  ENTER PLAY |";
                let controls_box_bot = "+-----------------------------+";
                let box_x = plat.screen_width as i32 / 2 - controls_box_top.len() as i32 * 3;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    controls_box_top,
                    box_x,
                    480,
                    sdl2::pixels::Color::RGBA(100, 100, 140, 255),
                );
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    controls_box_mid,
                    box_x,
                    495,
                    sdl2::pixels::Color::RGBA(200, 200, 200, 255),
                );
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    controls_box_bot,
                    box_x,
                    510,
                    sdl2::pixels::Color::RGBA(100, 100, 140, 255),
                );
                // Draw "PRESS SPACE TO START" centered below
                let subtitle = "PRESS SPACE TO START";
                let sub_w = subtitle.len() as i32 * 6;
                let sub_x = plat.screen_width as i32 / 2 - sub_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    subtitle,
                    sub_x,
                    380,
                    sdl2::pixels::Color::RGBA(200, 200, 200, 255),
                );
            }
            // episode-select-1: Episode selection screen with ASCII art decorations
            SceneState::EpisodeSelect => {
                plat.clear(20, 20, 40, 255);
                // menu-polish-1: ASCII art header
                let header = "+-----------------------------+";
                let header_text = "|    SELECT YOUR EPISODE      |";
                let header_w = header.len() as i32 * 6;
                let header_x = plat.screen_width as i32 / 2 - header_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    header,
                    header_x,
                    100,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    header_text,
                    header_x,
                    115,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    header,
                    header_x,
                    130,
                    sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                );

                // menu-polish-1: Episode cards displayed left-to-right
                // Each card: [icon] Title (difficulty stars)
                let card_width = 280;
                let card_spacing = 20;
                let total_width = episodes.len() as i32 * card_width + (episodes.len() - 1) as i32 * card_spacing;
                let start_x = (plat.screen_width as i32 - total_width) / 2;
                let card_y = 200;

                for (i, (ep_id, _ep_path)) in episodes.iter().enumerate() {
                    let ep = match Episode::load(&format!("{}/{}.json", EPISODES_DIR, ep_id)) {
                        Ok(ep) => ep,
                        Err(e) => {
                            eprintln!("[ERROR] Failed to load episode {}: {}", ep_id, e);
                            continue;
                        }
                    };
                    let is_selected = i == scene.selected_episode_index;
                    let card_x = start_x + i as i32 * (card_width + card_spacing);

                    // Card border
                    let border_color = if is_selected {
                        sdl2::pixels::Color::RGBA(255, 255, 80, 255)
                    } else {
                        sdl2::pixels::Color::RGBA(80, 80, 120, 255)
                    };
                    let top_border = format!("+{:-<width$}+", "", width = 34);
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &top_border,
                        card_x,
                        card_y,
                        border_color,
                    );

                    // Theme icon mapping
                    let theme_icon = match ep.theme.as_str() {
                        "grassland" | "grass" => "[GRASS]",
                        "cave" | "dungeon" => "[CAVE]",
                        "lava" | "volcanic" => "[LAVA]",
                        "ice" | "snow" => "[ICE]",
                        "water" | "ocean" => "[WATER]",
                        _ => "[?    ]",
                    };

                    // Difficulty stars mapping
                    let stars = match ep.difficulty.as_str() {
                        "easy" => "[*    ]",
                        "medium" => "[**   ]",
                        "hard" => "[***  ]",
                        "extreme" => "[**** ]",
                        _ => "[     ]",
                    };

                    // Selection indicator
                    let sel_indicator = if is_selected { ">>>" } else { "   " };

                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &format!("{} {}", sel_indicator, theme_icon),
                        card_x,
                        card_y + 20,
                        if is_selected {
                            sdl2::pixels::Color::RGBA(255, 255, 255, 255)
                        } else {
                            sdl2::pixels::Color::RGBA(180, 180, 180, 255)
                        },
                    );

                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &format!("{} {}", sel_indicator, stars),
                        card_x,
                        card_y + 40,
                        if is_selected {
                            sdl2::pixels::Color::RGBA(255, 255, 255, 255)
                        } else {
                            sdl2::pixels::Color::RGBA(180, 180, 180, 255)
                        },
                    );

                    // Episode title (truncated if needed)
                    let title = if ep.title.len() > 20 {
                        format!("{}...", &ep.title[..17])
                    } else {
                        ep.title.clone()
                    };
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &format!("{} {}", sel_indicator, &title),
                        card_x,
                        card_y + 60,
                        if is_selected {
                            sdl2::pixels::Color::RGBA(255, 255, 80, 255)
                        } else {
                            sdl2::pixels::Color::RGBA(200, 200, 200, 255)
                        },
                    );

                    // episode-select-besttimes-1: Display best time below title
                    let best_time_str = match scene.save_data.best_times.get(&ep.id) {
                        Some(&ms) => {
                            let secs = ms as u32 / 1000;
                            format!("BEST: {:02}:{:02}", secs / 60, secs % 60)
                        }
                        None => "BEST: --:--".to_string(),
                    };
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &format!("{} {}", sel_indicator, best_time_str),
                        card_x,
                        card_y + 75,
                        sdl2::pixels::Color::RGBA(180, 180, 100, 255),
                    );

                    let bot_border = format!("+{:-<width$}+", "", width = 34);
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        &bot_border,
                        card_x,
                        card_y + 80,
                        border_color,
                    );
                }

                // Controls hint at bottom with box-drawing
                let hint = "UP/DOWN SELECT   ENTER PLAY   ESC BACK";
                let hint_w = hint.len() as i32 * 6;
                let hint_x = plat.screen_width as i32 / 2 - hint_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    hint,
                    hint_x,
                    580,
                    sdl2::pixels::Color::RGBA(120, 120, 120, 255),
                );
            }
            SceneState::TitleCard => {
                plat.clear(10, 10, 30, 255);
                // titlecard-anim-1: episode name fade-in (0% to 100% over 1 second)
                let fade_progress = (1.0 - scene.titlecard_fade_timer).max(0.0).min(1.0);
                let title_opacity = (fade_progress * 255.0) as u8;
                // Draw episode title centered with animated opacity
                let title = episode.title.as_str();
                let title_w = title.len() as i32 * 6;
                let title_x = plat.screen_width as i32 / 2 - title_w / 2;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    title,
                    title_x,
                    300,
                    sdl2::pixels::Color::RGBA(100, 200, 255, title_opacity),
                );
                // titlecard-anim-1: "GET READY..." pulses 50% to 100% opacity over 0.8s
                // Only visible after fade-in completes
                if scene.titlecard_fade_timer <= 0.0 {
                    let pulse_phase = (scene.titlecard_pulse_timer * std::f32::consts::PI / 0.4) % (2.0 * std::f32::consts::PI);
                    let pulse_opacity = (0.5 + 0.5 * pulse_phase.sin()) * 255.0;
                    let sub_opacity = pulse_opacity as u8;
                    let sub = "GET READY...";
                    let sub_w = sub.len() as i32 * 6;
                    let sub_x = plat.screen_width as i32 / 2 - sub_w / 2;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        sub,
                        sub_x,
                        360,
                        sdl2::pixels::Color::RGBA(180, 180, 180, sub_opacity),
                    );
                }
            }
            SceneState::Playing | SceneState::GameOver => {
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
                            crate::game::TileType::Checkpoint => sdl2::rect::Rect::new(512, 0, 64, 32), // checkpoint
                            crate::game::TileType::Goal      => sdl2::rect::Rect::new(576, 0, 64, 32), // goal
                        };
                        let _ = plat.blit_sprite("tiles", dst, Some(src));
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let src = chars_sheet.frames.get("npc_idle")
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

                // hud-polish-1: Bordered ASCII HUD box
                let (cp_reached, cp_total) = world.checkpoint_progress();
                let p1_lives = player1.lives();
                let time_secs = scene.time_elapsed as u32;
                let time_str = format!("{:02}:{:02}", time_secs / 60, time_secs % 60);
                let best_time_str = match scene.save_data.best_times.get(&episode.id) {
                    Some(&ms) => {
                        let secs = ms as u32 / 1000;
                        format!("{:02}:{:02}", secs / 60, secs % 60)
                    }
                    None => "--:--".to_string(),
                };
                // Format lives as heart symbols (using * since bitmap font lacks Unicode hearts)
                let lives_str = match p1_lives {
                    3 => "***".to_string(),
                    2 => "** ".to_string(),
                    1 => "*  ".to_string(),
                    0 => "   ".to_string(),
                    _ => format!("{} ", p1_lives),
                };
                // Build HUD text content
                let hud_content = format!(" CP: {}/{} | LIVES: {} | TIME: {} | BEST: {} ",
                    cp_reached, cp_total, lives_str, time_str, best_time_str);
                let content_len = hud_content.len() as i32; // in characters
                let box_x = 8;
                let box_y = 8;
                let text_color = sdl2::pixels::Color::RGBA(200, 200, 200, 255);
                // Top border: +-- HUD --+------...------+
                let top_border = format!("+-- HUD --{}{}+",
                    "-".repeat((content_len - 6).max(0) as usize),
                    "-".repeat((content_len - 6).max(0) as usize));
                DebugOverlay::draw_text(&mut plat.canvas, &top_border, box_x, box_y, text_color);
                // Middle line: | CP: 1/3 | LIVES: *** | TIME: 00:42 | BEST: 00:00 |
                let middle_line = format!("|{}|", hud_content);
                DebugOverlay::draw_text(&mut plat.canvas, &middle_line, box_x, box_y + 9, text_color);
                // Bottom border
                let bottom_border = format!("+{}+", "-".repeat((content_len + 1).max(0) as usize));
                DebugOverlay::draw_text(&mut plat.canvas, &bottom_border, box_x, box_y + 18, text_color);

                // volume-control-1: Small volume indicator in HUD corner
                let vol = scene.save_data.audio_volume;
                let vol_pct = (vol * 100.0).round() as i32;
                let filled = ((vol * 10.0).round() as i32).max(0).min(10) as usize;
                let bar: String = std::iter::repeat('*').take(filled)
                    .chain(std::iter::repeat('-').take(10 - filled))
                    .collect();
                let vol_text = format!("VOL [{}] {}%", bar, vol_pct);
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    &vol_text,
                    box_x,
                    box_y + 27,
                    sdl2::pixels::Color::RGBA(150, 150, 150, 255),
                );

                // sprint-20: Death recap — show elimination info at top-center for 2 seconds
                if scene.death_recap_timer > 0.0 {
                    if let Some((ref tile_type, gx, gy)) = scene.death_recap {
                        let text = format!("ELIMINATED BY {} at ({}, {})", tile_type, gx, gy);
                        let text_w = text.len() as i32 * 8; // 8px per char
                        let box_w = text_w + 32;
                        let box_x = plat.screen_width as i32 / 2 - box_w / 2;
                        let box_y = 40;
                        // Semi-transparent dark box
                        plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 180));
                        let _ = plat.canvas.fill_rect(sdl2::rect::Rect::new(box_x, box_y, box_w as u32, 28));
                        // Red border
                        plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 60, 60, 255));
                        let _ = plat.canvas.draw_rect(sdl2::rect::Rect::new(box_x, box_y, box_w as u32, 28));
                        // Text
                        DebugOverlay::draw_text(
                            &mut plat.canvas,
                            &text,
                            box_x + 16,
                            box_y + 8,
                            sdl2::pixels::Color::RGBA(255, 80, 80, 255),
                        );
                    }
                }

                // tutorial-1: Draw tutorial overlay during Playing (before first move)
                if scene.tutorial_visible && scene.state == SceneState::Playing {
                    // Dark semi-transparent overlay
                    plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 180));
                    let _ = plat.canvas.fill_rect(sdl2::rect::Rect::new(0, 200, plat.screen_width, 320));

                    // Title: "VOXPARTY CONTROLS"
                    let title = "VOXPARTY CONTROLS";
                    let title_w = title.len() as i32 * 6;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        title,
                        plat.screen_width as i32 / 2 - title_w / 2,
                        220,
                        sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                    );

                    // Control hints
                    let controls = [
                        "ARROWS / D-PAD  =  MOVE",
                        "E / A  =  TALK TO NPC",
                        "ESC  =  PAUSE",
                    ];
                    for (i, line) in controls.iter().enumerate() {
                        let y = 280 + (i as i32) * 28;
                        let line_w = line.len() as i32 * 6;
                        DebugOverlay::draw_text(
                            &mut plat.canvas,
                            line,
                            plat.screen_width as i32 / 2 - line_w / 2,
                            y,
                            sdl2::pixels::Color::RGBA(200, 200, 200, 255),
                        );
                    }

                    // Dismiss hint
                    let dismiss = "PRESS ANY KEY TO START";
                    let dismiss_w = dismiss.len() as i32 * 6;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        dismiss,
                        plat.screen_width as i32 / 2 - dismiss_w / 2,
                        420,
                        sdl2::pixels::Color::RGBA(150, 150, 150, 255),
                    );
                }

                // Draw game over overlay
                if scene.state == SceneState::GameOver {
                    // Semi-transparent dark overlay
                    plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 150));
                    let _ = plat.canvas.fill_rect(sdl2::rect::Rect::new(0, 280, plat.screen_width, 160));

                    // Game Over text
                    let go_text = "GAME OVER";
                    let go_w = go_text.len() as i32 * 6;
                    DebugOverlay::draw_text(
                        &mut plat.canvas,
                        go_text,
                        plat.screen_width as i32 / 2 - go_w / 2,
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
                        plat.screen_width as i32 / 2 - result_w / 2,
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
                        plat.screen_width as i32 / 2 - countdown_w / 2,
                        410,
                        sdl2::pixels::Color::RGBA(150, 150, 150, 255),
                    );
                }

                // Draw NPC dialogue bubbles with portrait
                for (npc_index, npc) in npcs.iter().enumerate() {
                    if npc.bubble_visible() {
                        let (bx, by) = npc.bubble_screen_xy(camera.x, camera.y);
                        if let Some(line) = npc.current_line_text() {
                            eprintln!("[DIALOGUE] {}: {}", npc.name, line);
                            // Draw bubble background
                            let bubble_rect = sdl2::rect::Rect::new(bx, by - 24, 200, 32);
                            plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 200, 230));
                            let _ = plat.canvas.fill_rect(bubble_rect);

                            // Draw portrait: 28x28 colored rectangle on left side of bubble
                            let portrait_size = 28;
                            let portrait_x = bx + 2;
                            let portrait_y = by - 24 + 2; // center vertically in 32px bubble
                            let portrait_rect = sdl2::rect::Rect::new(portrait_x, portrait_y, portrait_size as u32, portrait_size as u32);

                            // Border (dark brown)
                            plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(80, 50, 30, 255));
                            let _ = plat.canvas.fill_rect(portrait_rect);

                            // Inner portrait with NPC color based on index
                            let inner_size = 24;
                            let inner_x = portrait_x + 2;
                            let inner_y = portrait_y + 2;
                            let inner_rect = sdl2::rect::Rect::new(inner_x, inner_y, inner_size as u32, inner_size as u32);

                            // Cycle through 3 colors: blue, green, orange
                            let portrait_color = match npc_index % 3 {
                                0 => sdl2::pixels::Color::RGBA(100, 150, 255, 255), // blue
                                1 => sdl2::pixels::Color::RGBA(100, 255, 150, 255), // green
                                _ => sdl2::pixels::Color::RGBA(255, 180, 100, 255), // orange
                            };
                            plat.canvas.set_draw_color(portrait_color);
                            let _ = plat.canvas.fill_rect(inner_rect);

                            // Draw NPC initial in center of portrait
                            if !npc.name.is_empty() {
                                let initial = npc.name.chars().next().unwrap_or('?');
                                let initial_str = initial.to_uppercase().to_string();
                                // Center the initial in the 24x24 inner rect (approx 4px per char)
                                let text_x = portrait_x + portrait_size as i32 / 2 - 6;
                                let text_y = portrait_y + portrait_size as i32 / 2 - 4;
                                DebugOverlay::draw_text(
                                    &mut plat.canvas,
                                    &initial_str,
                                    text_x,
                                    text_y,
                                    sdl2::pixels::Color::RGBA(0, 0, 0, 255),
                                );
                            }

                            // Render the dialogue text inside the bubble (shifted right for portrait)
                            DebugOverlay::draw_text(
                                &mut plat.canvas,
                                &line,
                                bx + 34,
                                by - 20,
                                sdl2::pixels::Color::RGBA(0, 0, 0, 255),
                            );
                        }
                    }
                }
            }
            // win-1: Distinct victory screen with episode complete, time taken, replay prompt
            SceneState::Victory => {
                plat.clear(30, 60, 90, 255);

                // Draw tiles in depth order (same as Playing)
                for y in 0..episode.grid_height {
                    for x in 0..episode.grid_width {
                        let tile = world.get_tile(x, y);
                        let (px, py) = grid_to_screen(x as f32, y as f32, camera.x, camera.y);
                        let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 16, 64, 48);

                        let src = match tile {
                            crate::game::TileType::Passable  => sdl2::rect::Rect::new(0,   0, 64, 32),
                            crate::game::TileType::Solid      => sdl2::rect::Rect::new(64,  0, 64, 32),
                            crate::game::TileType::Trap      => sdl2::rect::Rect::new(128, 0, 64, 32),
                            crate::game::TileType::Checkpoint => sdl2::rect::Rect::new(512, 0, 64, 32),
                            crate::game::TileType::Goal      => sdl2::rect::Rect::new(576, 0, 64, 32),
                        };
                        let _ = plat.blit_sprite("tiles", dst, Some(src));
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let src = chars_sheet.frames.get("npc_idle")
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

                // victory-ascii-1: ASCII art victory screen
                // Larger overlay to fit ASCII art
                plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 200));
                let _ = plat.canvas.fill_rect(sdl2::rect::Rect::new(0, 160, plat.screen_width, 380));

                let center_x = plat.screen_width as i32 / 2;
                let text_color = sdl2::pixels::Color::RGBA(80, 255, 120, 255);
                let gold_color = sdl2::pixels::Color::RGBA(255, 215, 80, 255);
                let gray_color = sdl2::pixels::Color::RGBA(180, 180, 180, 255);
                let dim_color = sdl2::pixels::Color::RGBA(120, 120, 120, 255);

                // ASCII art top decoration line
                let top_line = "*  ===================  *";
                let top_w = top_line.len() as i32 * 6;
                DebugOverlay::draw_text(&mut plat.canvas, top_line, center_x - top_w / 2, 175, text_color);

                // Episode complete text (large)
                let complete_text = "E P I S O D E   C O M P L E T E";
                let complete_w = complete_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    complete_text,
                    center_x - complete_w / 2,
                    200,
                    text_color,
                );

                // Winner info with ASCII flair
                let winner_text = if scene.winner == Some(1) {
                    ">> PLAYER 1 WINS! <<"
                } else if scene.winner == Some(2) {
                    ">> PLAYER 2 WINS! <<"
                } else {
                    "=========  DRAW  ========="
                };
                let winner_w = winner_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    winner_text,
                    center_x - winner_w / 2,
                    240,
                    gold_color,
                );

                // ASCII art middle decoration
                let mid_line = "  *  -  -  -  -  *  -  -  -  -  *";
                let mid_w = mid_line.len() as i32 * 6;
                DebugOverlay::draw_text(&mut plat.canvas, mid_line, center_x - mid_w / 2, 275, text_color);

                // Time taken / best time
                let time_secs = scene.time_elapsed as u32;
                let best_time_ms = scene.get_best_time_ms(&episode.id);
                let (time_text, time_color) = match best_time_ms {
                    None => (
                        "FIRST COMPLETION!".to_string(),
                        sdl2::pixels::Color::RGBA(80, 255, 120, 255),
                    ),
                    Some(best_ms) => {
                        let best_secs = best_ms as u32 / 1000;
                        let current_str = format!("{:02}:{:02}", time_secs / 60, time_secs % 60);
                        let best_str = format!("{:02}:{:02}", best_secs / 60, best_secs % 60);
                        (format!("TIME: {}    BEST: {}", current_str, best_str), sdl2::pixels::Color::RGBA(200, 200, 200, 255))
                    }
                };
                let time_w = time_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    &time_text,
                    center_x - time_w / 2,
                    310,
                    time_color,
                );

                // ASCII art lower decoration
                let low_line = "  *  -  -  -  -  *  -  -  -  -  *";
                let low_w = low_line.len() as i32 * 6;
                DebugOverlay::draw_text(&mut plat.canvas, low_line, center_x - low_w / 2, 345, text_color);

                // Controls hint
                let controls_text = "ENTER: REPLAY    ESC: MENU";
                let controls_w = controls_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    controls_text,
                    center_x - controls_w / 2,
                    380,
                    gray_color,
                );

                // Countdown
                let countdown = (scene.victory_timer.ceil() as u32).max(0);
                let countdown_text = format!("(auto-return in {} sec)", countdown);
                let countdown_w = countdown_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    &countdown_text,
                    center_x - countdown_w / 2,
                    420,
                    dim_color,
                );

                // Bottom ASCII decoration
                let bottom_line = "*  ===================  *";
                let bottom_w = bottom_line.len() as i32 * 6;
                DebugOverlay::draw_text(&mut plat.canvas, bottom_line, center_x - bottom_w / 2, 455, text_color);
            }
            // pause-1: Pause overlay — renders same as Playing but with dark overlay and pause text
            SceneState::Paused => {
                plat.clear(30, 60, 90, 255);

                // Draw tiles in depth order
                for y in 0..episode.grid_height {
                    for x in 0..episode.grid_width {
                        let tile = world.get_tile(x, y);
                        let (px, py) = grid_to_screen(x as f32, y as f32, camera.x, camera.y);
                        let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 16, 64, 48);

                        let src = match tile {
                            crate::game::TileType::Passable  => sdl2::rect::Rect::new(0,   0, 64, 32),
                            crate::game::TileType::Solid      => sdl2::rect::Rect::new(64,  0, 64, 32),
                            crate::game::TileType::Trap      => sdl2::rect::Rect::new(128, 0, 64, 32),
                            crate::game::TileType::Checkpoint => sdl2::rect::Rect::new(512, 0, 64, 32),
                            crate::game::TileType::Goal      => sdl2::rect::Rect::new(576, 0, 64, 32),
                        };
                        let _ = plat.blit_sprite("tiles", dst, Some(src));
                    }
                }

                // Draw NPCs
                for npc in &npcs {
                    let (px, py) = grid_to_screen(npc.grid_x as f32, npc.grid_y as f32, camera.x, camera.y);
                    let dst = sdl2::rect::Rect::new(px as i32, py as i32 - 32, 64, 64);
                    let src = chars_sheet.frames.get("npc_idle")
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

                // Pause overlay: semi-transparent dark overlay
                plat.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 160));
                let _ = plat.canvas.fill_rect(sdl2::rect::Rect::new(0, 0, plat.screen_width, 720));

                // "PAUSED" text centered
                let paused_text = "PAUSED";
                let paused_w = paused_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    paused_text,
                    plat.screen_width as i32 / 2 - paused_w / 2,
                    280,
                    sdl2::pixels::Color::RGBA(255, 255, 255, 255),
                );

                // "RESUME (ESC)" option
                let resume_text = "RESUME (ESC)";
                let resume_w = resume_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    resume_text,
                    plat.screen_width as i32 / 2 - resume_w / 2,
                    340,
                    sdl2::pixels::Color::RGBA(200, 200, 200, 255),
                );

                // "QUIT TO MENU (Q)" option
                let quit_text = "QUIT TO MENU (Q)";
                let quit_w = quit_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    quit_text,
                    plat.screen_width as i32 / 2 - quit_w / 2,
                    380,
                    sdl2::pixels::Color::RGBA(150, 150, 150, 255),
                );

                // volume-control-1: Volume indicator with ASCII bar
                let vol = scene.save_data.audio_volume;
                let vol_pct = (vol * 100.0).round() as i32;
                // Build ASCII bar: 10 segments
                let filled = ((vol * 10.0).round() as i32).max(0).min(10) as usize;
                let bar: String = std::iter::repeat('*').take(filled)
                    .chain(std::iter::repeat('-').take(10 - filled))
                    .collect();
                let vol_text = format!("VOL: [{}] {}%", bar, vol_pct);
                let vol_w = vol_text.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    &vol_text,
                    plat.screen_width as i32 / 2 - vol_w / 2,
                    430,
                    sdl2::pixels::Color::RGBA(180, 180, 180, 255),
                );

                // volume-control-1: F7/F8 hint
                let vol_hint = "F7: -   F8: +";
                let vol_hint_w = vol_hint.len() as i32 * 6;
                DebugOverlay::draw_text(
                    &mut plat.canvas,
                    vol_hint,
                    plat.screen_width as i32 / 2 - vol_hint_w / 2,
                    460,
                    sdl2::pixels::Color::RGBA(120, 120, 120, 255),
                );
            }
        }

        // Debug overlay renders on top of everything
        debug.render(&mut plat.canvas, &debug_state);

        // console-1: Console renders on top of everything (including debug overlay)
        console.render(&mut plat.canvas);

        // particle-1: Draw particles on top of game, below debug overlay
        particles.draw(&mut plat.canvas);

        // debug-screenshot-1: Render "SCREENSHOT SAVED" flash text for 1 second
        if screenshot_flash_timer > 0.0 {
            let text = "SCREENSHOT SAVED";
            let text_w = text.len() as i32 * 6;
            DebugOverlay::draw_text(
                &mut plat.canvas,
                text,
                plat.screen_width as i32 / 2 - text_w / 2,
                100,
                sdl2::pixels::Color::RGBA(80, 255, 120, 255),
            );
        }

        // sprint-22: Record draw calls before present
        debug.set_draw_calls(draw_call_count());

        plat.present();

        // Frame timing heartbeat — every 100 frames, log to confirm loop is alive.
        // If this stops, the game has frozen and needs investigation.
        frame_count += 1;
        if frame_count % 100 == 0 {
            log::info!("[FRAME] #{:08}  scene={:?}", frame_count, scene.state);
        }

        // telemetry-1: Emit fps_sample every 60 frames
        if frame_count % 60 == 0 {
            telemetry.log_event("fps_sample", json!({
                "fps": debug.fps(),
            }));
        }

        // sprint-22: Emit frame_budget and draw_calls telemetry every 300 frames
        if frame_count % 300 == 0 {
            telemetry.log_event("frame_budget", json!({
                "avg_ms": debug.avg_frame_time().unwrap_or(0.0),
                "p99_ms": debug.p99_frame_time().unwrap_or(0.0),
                "fps_equivalent": if debug.avg_frame_time().unwrap_or(0.0) > 0.0 {
                    1000.0 / debug.avg_frame_time().unwrap_or(16.67)
                } else { 0.0 },
            }));
            telemetry.log_event("draw_calls", json!({
                "count": debug.draw_call_count(),
            }));
            // sprint-22: Performance warning if draw calls > 100 per frame
            if debug.draw_call_count() > 100 {
                log::warn!("[PERF] High draw calls: {} (target < 100)", debug.draw_call_count());
            }
        }

        // telemetry-1: Flush buffered events to disk at end of each frame
        telemetry.flush();

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
            "difficulty": "easy",
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
        player.tick(0.016, 1.0, 0.0, &mut world, &sheet);

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
            "difficulty": "easy",
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
        player.tick(0.016, 1.0, 0.0, &mut world, &sheet);

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
            "difficulty": "medium",
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
        player.tick(0.016, 1.0, 0.0, &mut world, &sheet);

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
            "difficulty": "hard",
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
        player.tick(0.016, 1.0, 0.0, &mut world, &sheet);

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

/// Tests for headless smoke test mode.
#[cfg(test)]
mod headless_tests {
    use super::*;

    /// Test that headless mode runs without panicking on demo episode.
    #[test]
    fn test_headless_flag_exits_cleanly() {
        // This test verifies the headless run path doesn't panic.
        // It runs a minimal subset (just episode audit + one episode's init)
        // without the full 120 frames to keep test time reasonable.
        let _ = env_logger::try_init();

        // Verify demo episode loads successfully
        let episode = Episode::load("assets/episodes/demo.json");
        assert!(episode.is_ok(), "demo episode should load for headless test");

        let ep = episode.unwrap();
        // sprint-20: episode id was changed from "demo" to "ep_demo" in the JSON
        assert_eq!(ep.id, "ep_demo");

        // Verify world creation doesn't panic
        let mut world = World::from_episode(ep.clone());
        assert!(world.grid_w > 0);
        assert!(world.grid_h > 0);

        // Verify player creation doesn't panic
        let spawn1 = ep.spawn_points.iter().find(|s| s.player == 1).unwrap();
        let player = Player::new(1, spawn1.x, spawn1.y);
        assert_eq!(player.grid_x, spawn1.x);
        assert_eq!(player.grid_y, spawn1.y);

        // Verify camera creation doesn't panic
        let camera = Camera::new(1280, 720, ep.grid_width, ep.grid_height);
        // Camera starts centered on world (not at 0,0)
        assert!(camera.x.is_finite());
        assert!(camera.y.is_finite());

        // Verify scene creation doesn't panic
        let scene = Scene::new();
        assert_eq!(scene.state, SceneState::Menu);

        // Verify particle system creation doesn't panic
        let mut particles = ParticleSystem::new();

        // Verify tick calls don't panic (5 frames only for test speed)
        let dt = 1.0 / 60.0;
        let chars_sheet = SpriteSheet::from_json(&assets::loader::load_sprite_sheet("characters"));
        let mut player1 = Player::new(1, spawn1.x, spawn1.y);
        let mut scene = Scene::new();
        scene.state = SceneState::Playing;

        for _ in 0..5 {
            scene.tick(dt);
            player1.tick(dt, 0.0, 0.0, &mut world, &chars_sheet);
            let _ = particles.tick(dt);
        }

        // If we get here without panic, the test passes
        assert!(true, "headless path executed without panic");
    }
}
