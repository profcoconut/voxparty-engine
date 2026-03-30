//! Headless smoke test binary for VoxParty engine.
//!
//! Runs 120 frames of game loop per episode without rendering.
//! Exit 0: clean. Exit 1: panic or assertion failure.
//!
//! Usage: cargo run --bin smoke_test

use voxparty::core::{Scene, SceneState, Camera, SpriteSheet, grid_to_screen};
use voxparty::game::{Episode, Player, World, Npc};
use voxparty::assets::loader;

// Headless episodes directory
const EPISODES_DIR: &str = "assets/episodes";

fn headless_episode_test(episode_id: &str) -> Result<(), String> {
    let episode_path = format!("{}/{}.json", EPISODES_DIR, episode_id);
    let episode = Episode::load(&episode_path)
        .map_err(|e| format!("Failed to load episode '{}': {}", episode_id, e))?;

    let mut world = World::from_episode(episode.clone());

    let spawn1 = episode.spawn_points.iter().find(|s| s.player == 1)
        .ok_or_else(|| format!("Player 1 spawn point missing in episode '{}'", episode_id))?;
    let spawn2 = episode.spawn_points.iter().find(|s| s.player == 2)
        .ok_or_else(|| format!("Player 2 spawn point missing in episode '{}'", episode_id))?;

    let mut player1 = Player::new(1, spawn1.x, spawn1.y);
    let mut player2 = Player::new(2, spawn2.x, spawn2.y);

    let npcs: Vec<Npc> = episode
        .npcs
        .iter()
        .map(|n| Npc::new(n.x, n.y, n.name.clone(), n.dialogue.clone()))
        .collect();

    let screen_w = 1280;
    let screen_h = 720;
    let mut camera = Camera::new(screen_w, screen_h, episode.grid_width, episode.grid_height);
    let mut scene = Scene::new();
    let mut particles = voxparty::core::ParticleSystem::new();

    // Sprite sheets (needed for player tick)
    let chars_sheet = SpriteSheet::from_json(&loader::load_sprite_sheet("characters"));

    // Initialize player animations
    player1.anim.play("idle_p1", &chars_sheet, true);
    player2.anim.play("idle_p2", &chars_sheet, true);

    // Set scene to Playing to simulate normal gameplay
    scene.state = SceneState::Playing;
    scene.title_timer = 0.0;

    // Run 120 frames at 60fps
    let dt = 1.0 / 60.0;
    let total_frames = 120;

    for frame in 0..total_frames {
        // Update scene
        scene.tick(dt);

        // Center camera on player 1 when entering Playing state
        if scene.just_entered_playing() {
            camera.center_on(player1.grid_x as f32, player1.grid_y as f32);
        }

        // Update world trap cooldowns
        world.tick_trap_cooldowns();

        // Update particles
        particles.tick(dt);

        // Update player 1 (no input in headless mode, stays idle)
        if scene.state == SceneState::Playing {
            player1.tick(dt, 0.0, 0.0, &mut world, &chars_sheet);
            player2.tick(dt, 0.0, 0.0, &mut world, &chars_sheet);
        }

        // Update camera (smooth follow player 1)
        camera.follow(player1.grid_x as f32, player1.grid_y as f32);

        // Decay camera shake timer
        camera.tick(dt);

        if frame % 20 == 0 {
            println!("  Frame {}/{}: scene={:?}, p1=({},{}), p2=({},{})",
                frame, total_frames, scene.state,
                player1.grid_x, player1.grid_y,
                player2.grid_x, player2.grid_y);
        }
    }

    println!("  Completed {} frames for episode '{}'", total_frames, episode_id);
    Ok(())
}

fn main() {
    println!("VoxParty Headless Smoke Test");
    println!("==============================");

    let episodes = ["demo", "episode2", "episode3"];
    let mut all_passed = true;

    for ep in &episodes {
        println!("\nSmoke testing episode: {}", ep);
        match headless_episode_test(ep) {
            Ok(()) => {
                println!("  [OK] {} passed smoke test", ep);
            }
            Err(e) => {
                eprintln!("  [FAIL] {}: {}", ep, e);
                all_passed = false;
            }
        }
    }

    println!("\n==============================");
    if all_passed {
        println!("All episodes passed smoke test!");
        std::process::exit(0);
    } else {
        eprintln!("Some episodes failed smoke test!");
        std::process::exit(1);
    }
}
