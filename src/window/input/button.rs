#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ButtonState {
    #[default]
    Up,
    Pressed,
    Released,
    Down,
}

impl From<ButtonState> for bool {
    fn from(value: ButtonState) -> Self {
        match value {
            ButtonState::Up => false,
            ButtonState::Pressed => true,
            ButtonState::Released => false,
            ButtonState::Down => true,
        }
    }
}
