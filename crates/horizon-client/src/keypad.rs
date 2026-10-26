//! Optional physical-keypad preset, using the existing controller motion path.
use crate::gamepad::{Button, PadState};
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct Keypad(u32);
impl Keypad {
    pub const fn empty() -> Self {
        Self(0)
    }
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    pub fn key(&mut self, key: KeyCode, down: bool) -> bool {
        let bit = match key {
            KeyCode::ArrowUp => 0,
            KeyCode::ArrowDown => 1,
            KeyCode::ArrowLeft => 2,
            KeyCode::ArrowRight => 3,
            KeyCode::Digit2 | KeyCode::Numpad2 => 4,
            KeyCode::Digit8 | KeyCode::Numpad8 => 5,
            KeyCode::Digit4 | KeyCode::Numpad4 => 6,
            KeyCode::Digit6 | KeyCode::Numpad6 => 7,
            KeyCode::Digit1 | KeyCode::Numpad1 => 8,
            KeyCode::Digit3 | KeyCode::Numpad3 => 9,
            KeyCode::Digit5 | KeyCode::Numpad5 | KeyCode::Enter => 10,
            KeyCode::Digit0 | KeyCode::Numpad0 => 11,
            KeyCode::Digit7 | KeyCode::Numpad7 => 12,
            KeyCode::Digit9 | KeyCode::Numpad9 => 13,
            _ => return false,
        };
        if down {
            self.0 |= 1 << bit;
        } else {
            self.0 &= !(1 << bit);
        }
        true
    }
    pub fn state(&self) -> PadState {
        let held = |i: u32| -> bool { self.0 & (1u32 << i) != 0u32 };
        let axis = |positive, negative| {
            f32::from(u8::from(held(positive))) - f32::from(u8::from(held(negative)))
        };
        let mut left = [axis(3, 2), axis(0, 1)];
        let length = (left[0] * left[0] + left[1] * left[1]).sqrt().max(1.0);
        left = left.map(|v| v / length);
        let mut state = PadState {
            left,
            right: [axis(7, 6), axis(4, 5)],
            triggers: [f32::from(u8::from(held(9))), f32::from(u8::from(held(8)))],
            ..Default::default()
        };
        for (bit, button) in [
            (10, Button::South),
            (11, Button::RightStick),
            (12, Button::North),
            (13, Button::RightBumper),
        ] {
            state.set(button, held(bit));
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_release_and_camera_are_independent() {
        let mut keypad = Keypad::default();
        keypad.key(KeyCode::ArrowUp, true);
        keypad.key(KeyCode::ArrowRight, true);
        keypad.key(KeyCode::Digit6, true);
        let s = keypad.state();
        assert!((s.left[0].hypot(s.left[1]) - 1.0).abs() < 0.0001);
        assert_eq!(s.right, [1.0, 0.0]);
        keypad.key(KeyCode::Digit6, false);
        assert_eq!(keypad.state().right, [0.0, 0.0]);
        keypad.clear();
        assert_eq!(keypad.state(), PadState::default());
    }
    #[test]
    fn attack_and_use_are_analog_triggers_with_independent_releases() {
        let mut keypad = Keypad::default();
        let mut edges = crate::gamepad::Edges::default();
        keypad.key(KeyCode::Digit1, true);
        keypad.key(KeyCode::Digit3, true);
        assert_eq!(keypad.state().triggers, [1.0, 1.0]);
        let pressed = edges.update(&keypad.state(), 0.4);
        assert!(pressed.contains(&(Button::RightTrigger, true)));
        assert!(pressed.contains(&(Button::LeftTrigger, true)));
        keypad.key(KeyCode::Digit1, false);
        assert_eq!(
            edges.update(&keypad.state(), 0.4),
            [(Button::RightTrigger, false)]
        );
        keypad.clear();
        assert_eq!(
            edges.update(&keypad.state(), 0.4),
            [(Button::LeftTrigger, false)]
        );
    }
}
