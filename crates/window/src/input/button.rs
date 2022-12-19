#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ButtonState {
    Up,
    Pressed,
    Released,
    Down,
}

impl Default for ButtonState {
    fn default() -> Self {
        ButtonState::Up
    }
}
