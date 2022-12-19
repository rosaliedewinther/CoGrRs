use crate::input::button::ButtonState;

#[derive(Default)]
pub struct MouseState {
    pub mouse_location: [f32; 2],
    pub mouse_delta: [f32; 2],
    pub scroll_location: f32,
    pub scroll_delta: f32,
    left: ButtonState,
    right: ButtonState,
}
impl MouseState {
    pub fn new() -> MouseState {
        MouseState {
            mouse_location: [0.0, 0.0],
            mouse_delta: [0.0, 0.0],
            scroll_location: 0.0,
            scroll_delta: 0.0,
            left: ButtonState::Up,
            right: ButtonState::Up,
        }
    }
    pub fn update(&mut self) {
        if self.left == ButtonState::Pressed {
            self.left = ButtonState::Down;
        }
        if self.right == ButtonState::Pressed {
            self.right = ButtonState::Down;
        }
        if self.left == ButtonState::Released {
            self.left = ButtonState::Up;
        }
        if self.right == ButtonState::Released {
            self.right = ButtonState::Up;
        }
    }
    pub fn left_button_pressed(&mut self) {
        self.left = ButtonState::Pressed;
    }
    pub fn left_button_released(&mut self) {
        self.left = ButtonState::Released
    }
    pub fn right_button_pressed(&mut self) {
        self.right = ButtonState::Pressed;
    }
    pub fn right_button_released(&mut self) {
        self.right = ButtonState::Released
    }
    pub fn get_left_button(&self) -> ButtonState {
        self.left
    }
    pub fn get_right_button(&self) -> ButtonState {
        self.right
    }
}
