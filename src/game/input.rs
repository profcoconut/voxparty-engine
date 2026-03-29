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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_joystick_no_input() {
        let inputs = gamepad_to_inputs(0.0, 0.0);
        assert!(inputs.is_empty());
    }

    #[test]
    fn test_move_left() {
        let inputs = gamepad_to_inputs(-1.0, 0.0);
        assert_eq!(inputs, vec![GameInput::MoveLeft]);
    }

    #[test]
    fn test_move_right() {
        let inputs = gamepad_to_inputs(1.0, 0.0);
        assert_eq!(inputs, vec![GameInput::MoveRight]);
    }

    #[test]
    fn test_move_up() {
        let inputs = gamepad_to_inputs(0.0, -1.0);
        assert_eq!(inputs, vec![GameInput::MoveUp]);
    }

    #[test]
    fn test_move_down() {
        let inputs = gamepad_to_inputs(0.0, 1.0);
        assert_eq!(inputs, vec![GameInput::MoveDown]);
    }

    #[test]
    fn test_deadzone_below_threshold() {
        let inputs = gamepad_to_inputs(0.29, 0.0);
        assert!(inputs.is_empty(), "jx=0.29 should be in deadzone");
    }

    #[test]
    fn test_deadzone_at_threshold() {
        // At exactly -deadzone, should NOT trigger (jx < -deadzone is false at -0.3)
        let inputs = gamepad_to_inputs(-0.3, 0.0);
        assert!(inputs.is_empty(), "jx=-0.3 is at boundary, in deadzone");
    }

    #[test]
    fn test_deadzone_above_triggers() {
        let inputs = gamepad_to_inputs(-0.31, 0.0);
        assert_eq!(inputs, vec![GameInput::MoveLeft]);
    }

    #[test]
    fn test_positive_deadzone_above_triggers() {
        let inputs = gamepad_to_inputs(0.31, 0.0);
        assert_eq!(inputs, vec![GameInput::MoveRight]);
    }

    #[test]
    fn test_diagonal_right_up() {
        let inputs = gamepad_to_inputs(1.0, -1.0);
        assert_eq!(inputs, vec![GameInput::MoveRight, GameInput::MoveUp]);
    }

    #[test]
    fn test_diagonal_left_down() {
        let inputs = gamepad_to_inputs(-1.0, 1.0);
        assert_eq!(inputs, vec![GameInput::MoveLeft, GameInput::MoveDown]);
    }
}
