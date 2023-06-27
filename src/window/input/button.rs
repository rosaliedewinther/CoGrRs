#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ButtonState {
    #[default]
    Up,
    Pressed,
    Released,
    Down,
}
