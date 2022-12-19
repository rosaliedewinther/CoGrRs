pub mod button;
pub mod keyboard;
pub mod mouse;

use crate::input::button::ButtonState;
use crate::input::keyboard::KeyboardState;
use crate::input::mouse::MouseState;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode};
use winit::event_loop::ControlFlow;

#[derive(Default)]
pub struct Input {
    pub sensitivity_modifier: f32,
    pub mouse_state: MouseState,
    pub cursor_in_screen: bool,
    pub keyboard_state: KeyboardState,
}

impl Input {
    pub fn new() -> Input {
        Input {
            sensitivity_modifier: 0.8,
            mouse_state: MouseState::new(),
            keyboard_state: KeyboardState::new(),
            cursor_in_screen: true,
        }
    }
    pub fn update(&mut self) {
        self.keyboard_state.update();
        self.mouse_state.mouse_delta = [0.0, 0.0];
        self.mouse_state.scroll_delta = 0.0;
    }
    pub fn update_cursor_moved(&mut self, pos: &PhysicalPosition<f32>) {
        self.mouse_state.mouse_delta = [
            (pos.x as f32 - self.mouse_state.mouse_location[0]) * self.sensitivity_modifier,
            (pos.y as f32 - self.mouse_state.mouse_location[1]) * self.sensitivity_modifier,
        ];
        self.mouse_state.mouse_location = [pos.x as f32, pos.y as f32];
    }
    pub fn update_cursor_entered(&mut self) {
        self.cursor_in_screen = true;
    }
    pub fn update_cursor_left(&mut self) {
        self.cursor_in_screen = false;
    }
    pub fn update_mouse_input(&mut self, state: &ElementState, button: &MouseButton) {
        match state {
            ElementState::Pressed => match button {
                MouseButton::Left => self.mouse_state.left_button_pressed(),
                MouseButton::Right => self.mouse_state.right_button_pressed(),
                _ => {}
            },
            ElementState::Released => match button {
                MouseButton::Left => self.mouse_state.left_button_released(),
                MouseButton::Right => self.mouse_state.right_button_released(),
                _ => {}
            },
        }
    }
    pub fn update_mouse_wheel(&mut self, delta: &MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_, scrolled) => {
                self.mouse_state.scroll_delta = *scrolled as f32;
                self.mouse_state.scroll_location += *scrolled as f32;
            }
            MouseScrollDelta::PixelDelta(_) => {}
        }
    }
    pub fn update_keyboard_input(&mut self, input: &KeyboardInput, control_flow: &mut ControlFlow) {
        if input.state == ElementState::Pressed && input.virtual_keycode.is_some() {
            self.keyboard_state.pressed(input.virtual_keycode.unwrap());
        }
        if input.state == ElementState::Released && input.virtual_keycode.is_some() {
            self.keyboard_state.released(input.virtual_keycode.unwrap());
        }

        if let KeyboardInput {
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::Escape),
            ..
        } = input
        {
            *control_flow = ControlFlow::Exit
        }
    }
    pub fn mouse_pressed(&self, button: MouseButton) -> ButtonState {
        if button == MouseButton::Left {
            return self.mouse_state.get_left_button();
        } else if button == MouseButton::Right {
            return self.mouse_state.get_right_button();
        }
        ButtonState::Up
    }
    pub fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keyboard_state.down(key)
    }
    pub fn mouse_change(&self) -> [f32; 2] {
        self.mouse_state.mouse_delta
    }
    pub fn any_change(&self) -> bool {
        self.keyboard_state.any_down() || self.mouse_state.mouse_delta[0] != 0.0 || self.mouse_state.mouse_delta[1] != 0.0
    }
}
