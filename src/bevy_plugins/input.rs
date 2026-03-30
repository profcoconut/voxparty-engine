//! Virtual gamepad and input handling for Bevy.
//!
//! Replaces `platform/touch.rs` `TouchHandler` / `VirtualGamepad`.
//! Keyboard and gamepad are wired to Bevy's input system.
//! Touch input (virtual gamepad on mobile) will be added in a later phase.
//!
//! The `VirtualGamepad` resource holds the current input state for each player.
//! Systems read from these resources rather than polling hardware directly.

use bevy::prelude::*;
use std::collections::HashMap;

/// Input commands that the game logic consumes.
/// Mirrors the current `GameInput` enum from `platform/touch.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameInput {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Interact,
    Confirm,
    Back,
    Pause,
}

/// A player's virtual gamepad state — analog joystick + held buttons.
/// Mirrors the current `VirtualGamepad` struct from `platform/touch.rs`.
#[derive(Debug, Clone)]
pub struct VirtualGamepad {
    /// Which player this is (1 or 2)
    pub player_id: u8,
    /// Analog joystick X axis (-1.0 to 1.0)
    pub joystick_x: f32,
    /// Analog joystick Y axis (-1.0 to 1.0)
    pub joystick_y: f32,
    /// Currently held input commands
    pub held: HashMap<GameInput, bool>,
}

impl VirtualGamepad {
    pub fn new(player_id: u8) -> Self {
        Self {
            player_id,
            joystick_x: 0.0,
            joystick_y: 0.0,
            held: HashMap::new(),
        }
    }

    /// Returns true if the given input is currently held.
    pub fn is_held(&self, input: GameInput) -> bool {
        self.held.get(&input).copied().unwrap_or(false)
    }
}

/// `Players` resource holds both players' virtual gamepads.
#[derive(Resource)]
pub struct Players {
    pub player1: VirtualGamepad,
    pub player2: VirtualGamepad,
}

impl Default for Players {
    fn default() -> Self {
        Self {
            player1: VirtualGamepad::new(1),
            player2: VirtualGamepad::new(2),
        }
    }
}

/// Game input plugin — wires keyboard and gamepad to `Players` resource.
pub struct VoxpartyInputPlugin;

impl Plugin for VoxpartyInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Players>();
        app.add_systems(
            bevy::prelude::Update,
            (
                keyboard_input_system,
                gamepad_input_system,
            ),
        );
    }
}

/// Keyboard input system.
/// Maps arrow keys and WASD to player 1's virtual gamepad.
fn keyboard_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut players: ResMut<Players>,
) {
    // Player 1: Arrow keys OR WASD
    let left = keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);
    let right = keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let up = keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
    let down = keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);

    *players.player1.held.entry(GameInput::MoveLeft).or_insert(false) = left;
    *players.player1.held.entry(GameInput::MoveRight).or_insert(false) = right;
    *players.player1.held.entry(GameInput::MoveUp).or_insert(false) = up;
    *players.player1.held.entry(GameInput::MoveDown).or_insert(false) = down;

    // Space = Confirm/Jump
    *players.player1.held.entry(GameInput::Confirm).or_insert(false) =
        keys.any_pressed([KeyCode::Space, KeyCode::Enter]);
    *players.player1.held.entry(GameInput::Jump).or_insert(false) =
        keys.any_pressed([KeyCode::Space]);

    // E = Interact
    *players.player1.held.entry(GameInput::Interact).or_insert(false) =
        keys.any_pressed([KeyCode::KeyE]);

    // Escape = Pause / Back
    *players.player1.held.entry(GameInput::Pause).or_insert(false) =
        keys.any_pressed([KeyCode::Escape]);
    *players.player1.held.entry(GameInput::Back).or_insert(false) =
        keys.any_pressed([KeyCode::Escape]);
}

/// Gamepad input system.
/// In Bevy 0.18, `Gamepad` is a component — query it directly.
/// `ButtonInput<GamepadButton>` tracks button state globally (any gamepad).
fn gamepad_input_system(
    // Query all entities with a Gamepad component
    gamepad_query: Query<(Entity, &Gamepad)>,
    // Access button state via ButtonInput<GamepadButton>
    button_input: Res<ButtonInput<GamepadButton>>,
    mut players: ResMut<Players>,
) {
    // Track whether any gamepad was processed this frame
    let mut found_gamepad = false;

    for (_entity, gamepad) in &gamepad_query {
        found_gamepad = true;

        // --- Left stick axes ---
        let deadzone = 0.3;
        let stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let stick_y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);

        // Update analog joystick values
        let jx = if stick_x.abs() > deadzone { stick_x } else { 0.0 };
        let jy = if stick_y.abs() > deadzone { stick_y } else { 0.0 };
        players.player1.joystick_x = jx;
        players.player1.joystick_y = jy;

        // --- Digital input from D-pad + left stick ---
        // D-pad buttons (no gamepad arg needed — ButtonInput is global)
        let dpad_left = GamepadButton::DPadLeft;
        let dpad_right = GamepadButton::DPadRight;
        let dpad_up = GamepadButton::DPadUp;
        let dpad_down = GamepadButton::DPadDown;

        let left = button_input.pressed(dpad_left) || jx < -deadzone;
        let right = button_input.pressed(dpad_right) || jx > deadzone;
        let up = button_input.pressed(dpad_up) || jy < -deadzone;
        let down = button_input.pressed(dpad_down) || jy > deadzone;

        *players.player1.held.entry(GameInput::MoveLeft).or_insert(false) = left;
        *players.player1.held.entry(GameInput::MoveRight).or_insert(false) = right;
        *players.player1.held.entry(GameInput::MoveUp).or_insert(false) = up;
        *players.player1.held.entry(GameInput::MoveDown).or_insert(false) = down;

        // South button (A on Xbox) = Confirm + Jump
        let a_pressed = button_input.pressed(GamepadButton::South);
        *players.player1.held.entry(GameInput::Confirm).or_insert(false) = a_pressed;
        *players.player1.held.entry(GameInput::Jump).or_insert(false) = a_pressed;

        // East button (B on Xbox) = Interact
        *players.player1.held.entry(GameInput::Interact).or_insert(false) =
            button_input.pressed(GamepadButton::East);

        // Start = Pause
        *players.player1.held.entry(GameInput::Pause).or_insert(false) =
            button_input.pressed(GamepadButton::Start);
    }

    // If no gamepad found, clear joystick values (avoid stale state)
    if !found_gamepad {
        players.player1.joystick_x = 0.0;
        players.player1.joystick_y = 0.0;
    }
}
