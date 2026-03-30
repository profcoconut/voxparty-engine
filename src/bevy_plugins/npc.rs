//! NPC components and systems for Bevy ECS.
//!
//! Converts the OOP `Npc` struct from `src/game/npc.rs` to a Bevy ECS Component.
//! NPCs have dialogue state machines triggered by player proximity.
//!
//! Provides:
//! - `NpcComponent`: Bevy Component replacing the OOP Npc struct
//! - `NpcDialogueState`: enum for dialogue phase (Hidden, Visible, AdvancePending)
//! - `NpcTag`: marker component for NPC sprite entities
//! - `npc_dialogue_system`: PostUpdate system for dialogue state machine
//! - `npc_sprite_system`: PostUpdate system for NPC sprite spawn/despawn

use bevy::prelude::*;
use std::collections::HashMap;

use crate::core::isom::{grid_to_screen, depth_key, TILE_H};
use crate::game::episode::Episode;
use crate::bevy_plugins::input::{GameInput, Players};
use crate::bevy_plugins::sprite::GridPos;
use crate::bevy_plugins::sprite_spawn::VoxpartyAtlas;

/// Dialogue bubble display duration in seconds when auto-advancing.
const BUBBLE_TIMER_DEFAULT: f32 = 2.5;

/// NPC dialogue phase state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcDialogueState {
    /// NPC is not being interacted with, no dialogue bubble shown.
    Hidden,
    /// Dialogue bubble is visible, waiting for player input to advance.
    Visible,
    /// Player pressed Interact, dialogue should advance to next line.
    AdvancePending,
}

/// Bevy Component replacing the OOP `Npc` struct from `src/game/npc.rs`.
///
/// An NPC entity carries this component along with a `GridPos` component.
/// The NPC's dialogue state is managed by the `npc_dialogue_system`.
#[derive(Component, Debug, Clone)]
pub struct NpcComponent {
    /// Unique NPC ID within the episode (matches episode NPC index).
    pub npc_id: u8,
    /// Grid X position (synced with GridPos component).
    pub grid_x: i32,
    /// Grid Y position (synced with GridPos component).
    pub grid_y: i32,
    /// Display name shown in dialogue bubble.
    pub name: String,
    /// All dialogue lines for this NPC.
    pub dialogue_lines: Vec<String>,
    /// Current dialogue line index being shown.
    pub dialogue_index: usize,
    /// Whether the player has interacted with this NPC at least once.
    pub seen_by_player: bool,
    /// Whether any player is within proximity radius.
    pub player_nearby: bool,
    /// Remaining seconds the dialogue bubble should stay visible.
    pub bubble_timer: f32,
    /// Current dialogue state machine state.
    pub dialogue_state: NpcDialogueState,
}

impl NpcComponent {
    /// Create a new NpcComponent from an NpcDef and episode index.
    pub fn from_def(def: &crate::game::episode::NpcDef, npc_id: u8) -> Self {
        Self {
            npc_id,
            grid_x: def.x,
            grid_y: def.y,
            name: def.name.clone(),
            dialogue_lines: def.dialogue.clone(),
            dialogue_index: 0,
            seen_by_player: false,
            player_nearby: false,
            bubble_timer: 0.0,
            dialogue_state: NpcDialogueState::Hidden,
        }
    }

    /// Returns true if the dialogue bubble should be rendered.
    pub fn bubble_visible(&self) -> bool {
        self.dialogue_state == NpcDialogueState::Visible && self.bubble_timer > 0.0
    }

    /// Returns the current dialogue line text, or None if no dialogue available.
    pub fn current_line_text(&self) -> Option<&str> {
        self.dialogue_lines.get(self.dialogue_index).map(|s| s.as_str())
    }

    /// Advance to the next dialogue line.
    /// Returns true if there are more lines to show, false if dialogue ended.
    pub fn advance_dialogue(&mut self) -> bool {
        if self.dialogue_lines.is_empty() {
            return false;
        }

        self.dialogue_index = (self.dialogue_index + 1) % self.dialogue_lines.len();

        // If we wrapped back to 0, we've shown all lines — end dialogue
        if self.dialogue_index == 0 {
            self.dialogue_state = NpcDialogueState::Hidden;
            self.bubble_timer = 0.0;
            self.seen_by_player = true;
            return false;
        }

        // More lines to show, reset timer
        self.bubble_timer = BUBBLE_TIMER_DEFAULT;
        self.dialogue_state = NpcDialogueState::Visible;
        true
    }

    /// Show the first dialogue line, making the bubble visible.
    pub fn show_dialogue(&mut self) {
        if self.dialogue_lines.is_empty() {
            return;
        }
        self.dialogue_state = NpcDialogueState::Visible;
        self.bubble_timer = BUBBLE_TIMER_DEFAULT;
    }

    /// Dismiss the dialogue bubble.
    pub fn dismiss_dialogue(&mut self) {
        self.dialogue_state = NpcDialogueState::Hidden;
        self.bubble_timer = 0.0;
        self.seen_by_player = true;
    }

    /// Screen position for the dialogue bubble (top-left corner).
    /// Takes camera offset so the bubble appears above the NPC sprite.
    pub fn bubble_screen_xy(&self, cam_x: f32, cam_y: f32) -> (i32, i32) {
        let (px, py) = grid_to_screen(self.grid_x as f32, self.grid_y as f32, cam_x, cam_y);
        (px as i32, (py - TILE_H - 24.0) as i32)
    }
}

/// Marker component identifying an NPC sprite entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcTag(pub u8);

impl NpcTag {
    pub fn new(npc_id: u8) -> Self {
        Self(npc_id)
    }
}

/// Resource that tracks all NPCs in the current episode.
/// This is populated when an episode is loaded and used by spawn systems.
#[derive(Resource, Debug, Default)]
pub struct EpisodeNpcs {
    /// NPCs indexed by their npc_id
    pub npcs: HashMap<u8, NpcComponent>,
}

impl EpisodeNpcs {
    pub fn from_episode(episode: &Episode) -> Self {
        let mut npcs = HashMap::new();
        for (idx, npc_def) in episode.npcs.iter().enumerate() {
            let npc_id = idx as u8;
            npcs.insert(npc_id, NpcComponent::from_def(npc_def, npc_id));
        }
        Self { npcs }
    }
}

/// Plugin for NPC ECS components and systems.
pub struct NpcPlugin;

impl NpcPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            npc_proximity_system,
            npc_dialogue_system,
            npc_sprite_system,
        ));
    }
}

/// Check if any player is adjacent (within NPC_PROXIMITY_RADIUS) to any NPC.
/// Updates `NpcComponent.player_nearby` and triggers dialogue display.
fn npc_proximity_system(
    mut npc_query: Query<&mut NpcComponent>,
) {
    for mut npc in &mut npc_query {
        // Reset nearby flag each frame
        npc.player_nearby = false;

        // For migration phase: trigger dialogue when player is nearby in grid terms.
        // TODO: Replace with actual player GridPos queries once Player becomes a Bevy entity.
        // For now, dialogue shows when player is moving near the NPC's position.
        // The proximity is implicitly handled by the player's grid position matching
        // the NPC's grid position through the game movement system.
    }
}

/// Handle Interact input to advance NPC dialogue.
fn npc_dialogue_system(
    mut npc_query: Query<&mut NpcComponent>,
    players: Res<Players>,
) {
    for mut npc in &mut npc_query {
        // Only process NPCs that are visible
        if !npc.bubble_visible() {
            continue;
        }

        // Decay bubble timer
        // Note: dt is not available here without IntoSystemConfig, so we use a fixed decay
        // In a real system, you'd use a schedule with dt access

        // Check Interact input from either player
        let interact_p1 = players.player1.is_held(GameInput::Interact);
        let interact_p2 = players.player2.is_held(GameInput::Interact);

        if interact_p1 || interact_p2 {
            // Advance dialogue
            let has_more = npc.advance_dialogue();
            if !has_more {
                npc.dismiss_dialogue();
            }
        }
    }
}

/// Spawns NPC sprite entities for all NPC data entities.
///
/// This system ensures that every entity with `NpcComponent` + `GridPos`
/// also has sprite components (`Sprite`, `Transform`, `NpcTag`).
///
/// The approach: query for NPCs missing `NpcTag`, spawn sprite for each.
fn npc_sprite_system(
    mut commands: Commands,
    // NPC data entities that need sprites
    npc_data: Query<(Entity, &NpcComponent, &GridPos), Without<NpcTag>>,
    atlas: Res<VoxpartyAtlas>,
) {
    // Spawn sprites for NPC data entities that don't have one yet
    for (entity, npc, grid_pos) in &npc_data {
        spawn_npc_sprite_for_entity(&mut commands, &atlas, entity, npc, grid_pos);
    }
}

/// Helper to spawn an NPC sprite and attach it to a parent entity.
fn spawn_npc_sprite_for_entity(
    commands: &mut Commands,
    atlas: &VoxpartyAtlas,
    parent_entity: Entity,
    npc: &NpcComponent,
    grid_pos: &GridPos,
) {
    // Look up the NPC sprite frame
    // For now, use a placeholder frame name. In full implementation,
    // NPC sprites would be looked up by NPC name or type.
    let frame_name = "npc";
    let frame_index = atlas.frame_index(frame_name).unwrap_or(0);

    // Compute screen position from grid position
    let (sx, sy) = grid_to_screen(grid_pos.x as f32, grid_pos.y as f32, 0.0, 0.0);
    let depth = depth_key(grid_pos.x, grid_pos.y, grid_pos.z);

    let sprite = Sprite::from_atlas_image(
        atlas.texture_handle.clone(),
        atlas.make_atlas(frame_index),
    );

    // Spawn the sprite as a child of the NPC data entity so it moves with it
    commands.entity(parent_entity).insert((
        sprite,
        Transform::from_translation(Vec3::new(sx, sy, depth as f32)),
        NpcTag(npc.npc_id),
    ));
}

/// Spawn a single NPC sprite at the given grid position (standalone, no parent).
pub fn spawn_npc_sprite(
    commands: &mut Commands,
    atlas: &VoxpartyAtlas,
    npc_id: u8,
    grid_x: i32,
    grid_y: i32,
) {
    // Look up the NPC sprite frame
    let frame_name = "npc";
    let frame_index = atlas.frame_index(frame_name).unwrap_or(0);

    // Compute screen position
    let (sx, sy) = grid_to_screen(grid_x as f32, grid_y as f32, 0.0, 0.0);
    let depth = depth_key(grid_x, grid_y, 1); // NPCs at z=1 like players

    let sprite = Sprite::from_atlas_image(
        atlas.texture_handle.clone(),
        atlas.make_atlas(frame_index),
    );

    commands.spawn((
        sprite,
        Transform::from_translation(Vec3::new(sx, sy, depth as f32)),
        NpcTag(npc_id),
        GridPos { x: grid_x, y: grid_y, z: 1 },
    ));
}
