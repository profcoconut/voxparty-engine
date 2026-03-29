use crate::platform::GameInput;

/// Convert virtual gamepad state to a list of game inputs.
pub fn gamepad_to_inputs(jx: f32, jy: f32) -> Vec<GameInput> {
    let deadzone = 0.3;
    let mut inputs = Vec::new();
    if jx < -deadzone { inputs.push(GameInput::MoveLeft); }
    if jx > deadzone  { inputs.push(GameInput::MoveRight); }
    if jy < -deadzone { inputs.push(GameInput::MoveUp); }
    if jy > deadzone  { inputs.push(GameInput::MoveDown); }
    inputs
}
