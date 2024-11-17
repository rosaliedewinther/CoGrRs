use std::sync::OnceLock;

use gilrs::{Button, Gilrs};
use glam::{vec2, Vec2};

use parking_lot::Mutex;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

// keyboard
pub const MOVE_FORWARD: PhysicalKey = PhysicalKey::Code(KeyCode::KeyW);
pub const MOVE_BACKWARD: PhysicalKey = PhysicalKey::Code(KeyCode::KeyS);
pub const MOVE_LEFT: PhysicalKey = PhysicalKey::Code(KeyCode::KeyA);
pub const MOVE_RIGHT: PhysicalKey = PhysicalKey::Code(KeyCode::KeyD);
pub const KEYBOARD_JUMP: PhysicalKey = PhysicalKey::Code(KeyCode::Space);
pub const KEYBOARD_SPEEDUP: PhysicalKey = PhysicalKey::Code(KeyCode::ShiftLeft);
pub const KEYBOARD_ACTIVATE: PhysicalKey = PhysicalKey::Code(KeyCode::KeyR);
pub const KEYBOARD_PLACE_THING: PhysicalKey = PhysicalKey::Code(KeyCode::KeyT);
pub const KEYBOARD_INVENTORY: PhysicalKey = PhysicalKey::Code(KeyCode::KeyI);

// keyboard editor
pub const KEYBOARD_MOVE_UP: PhysicalKey = PhysicalKey::Code(KeyCode::KeyE);
pub const KEYBOARD_MOVE_DOWN: PhysicalKey = PhysicalKey::Code(KeyCode::KeyQ);
pub const KEYBOARD_SWITCH_CAMERA: PhysicalKey = PhysicalKey::Code(KeyCode::F1);
pub const KEYBOARD_ENABLE_MOVEMENT: PhysicalKey = PhysicalKey::Code(KeyCode::ControlLeft);

pub const KEYBOARD_CLOSE_WINDOW: PhysicalKey = PhysicalKey::Code(KeyCode::Escape);

// gamepad
pub const GAMEPAD_JUMP: Button = Button::South;
pub const GAMEPAD_SPEEDUP: Button = Button::East;
pub const GAMEPAD_ACTIVATE: Button = Button::West;
pub const GAMEPAD_PLACE_THING: Button = Button::North;
pub const GAMEPAD_CYCLE_POSITION_RIGHT: Button = Button::RightTrigger2;
pub const GAMEPAD_CYCLE_POSITION_LEFT: Button = Button::LeftTrigger2;
pub const GAMEPAD_INVENTORY: Button = Button::Start;

// gamepad editor
pub const GAMEPAD_MOVE_UP: Button = Button::RightTrigger;
pub const GAMEPAD_MOVE_DOWN: Button = Button::LeftTrigger;
pub const GAMEPAD_SWITCH_CAMERA: Button = Button::Mode;
pub const GAMEPAD_ENABLE_MOVEMENT: Button = Button::Select;

// Not set yet
//pub const GAMEPAD_CLOSE_WINDOW: Button;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ButtonState {
    #[default]
    Up,
    Pressed,
    Released,
    Down,
}

impl ButtonState {
    pub fn up(&self) -> bool {
        match self {
            ButtonState::Up => true,
            ButtonState::Pressed => false,
            ButtonState::Released => false,
            ButtonState::Down => false,
        }
    }
    pub fn pressed(&self) -> bool {
        match self {
            ButtonState::Up => false,
            ButtonState::Pressed => true,
            ButtonState::Released => false,
            ButtonState::Down => false,
        }
    }
    pub fn released(&self) -> bool {
        match self {
            ButtonState::Up => false,
            ButtonState::Pressed => false,
            ButtonState::Released => true,
            ButtonState::Down => false,
        }
    }
    pub fn down(&self) -> bool {
        match self {
            ButtonState::Up => false,
            ButtonState::Pressed => false,
            ButtonState::Released => false,
            ButtonState::Down => true,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GameControls {
    // Game controls
    pub move_forward: f32,
    pub move_backwards: f32,
    pub move_left: f32,
    pub move_right: f32,
    pub rotate_camera_y: f32,
    pub rotate_camera_x: f32,
    pub jump: ButtonState,
    pub speedup: ButtonState,
    pub activate: ButtonState,
    pub place_thing: ButtonState,
    pub cycle_position: i32,
    pub inventory: ButtonState,

    // Editor controls
    pub editor_move_up: f32,
    pub editor_move_down: f32,
    pub editor_switch_camera: ButtonState,
    pub editor_enable_movement: ButtonState,

    pub close_window: ButtonState,
}

pub const fn default_controls() -> GameControls {
    GameControls {
        move_forward: 0.0,
        move_backwards: 0.0,
        move_left: 0.0,
        move_right: 0.0,
        rotate_camera_y: 0.0,
        rotate_camera_x: 0.0,
        jump: ButtonState::Up,
        speedup: ButtonState::Up,
        activate: ButtonState::Up,
        place_thing: ButtonState::Up,
        cycle_position: 0,
        inventory: ButtonState::Up,
        editor_move_up: 0.0,
        editor_move_down: 0.0,
        editor_switch_camera: ButtonState::Up,
        editor_enable_movement: ButtonState::Up,
        close_window: ButtonState::Up,
    }
}

pub struct Input {
    pub sensitivity_modifier: f32,
    pub controls: GameControls,
    pub previous_mouse_pos: Vec2,
    pub using_controller: bool,
}

fn input() -> &'static Mutex<Input> {
    static INPUT: OnceLock<Mutex<Input>> = OnceLock::new();
    static GIL_RS: OnceLock<Mutex<Gilrs>> = OnceLock::new();

    INPUT.get_or_init(|| {
        Mutex::new(Input {
            sensitivity_modifier: 1.0,
            controls: default_controls(),
            previous_mouse_pos: Vec2::ZERO,
            using_controller: false,
        })
    })
}

fn gil_rs() -> &'static Mutex<Gilrs> {
    static GIL_RS: OnceLock<Mutex<Gilrs>> = OnceLock::new();

    GIL_RS.get_or_init(|| Mutex::new(Gilrs::new().unwrap()))
}

pub fn controls<'a>() -> GameControls {
    input().lock().controls.clone()
}

pub fn input_window_event(event: &WindowEvent) {
    //let mut locked_input = INPUT.lock();
    let mut input = input().lock();
    while let Some(event) = gil_rs().lock().next_event() {
        input_controller_event(event);
    }

    match event {
        WindowEvent::CursorMoved { position, .. } => {
            input.using_controller = false;
            input.controls.rotate_camera_x = position.x as f32 - input.previous_mouse_pos.x;
            input.controls.rotate_camera_y = position.y as f32 - input.previous_mouse_pos.y;
            input.previous_mouse_pos = vec2(position.x as f32, position.y as f32);
        }
        WindowEvent::CursorEntered { .. } => {}
        WindowEvent::CursorLeft { .. } => {}
        WindowEvent::MouseInput { .. } => {}
        WindowEvent::MouseWheel { delta, .. } => {
            input.using_controller = false;
            input.controls.cycle_position += match delta {
                winit::event::MouseScrollDelta::LineDelta(vertical, ..) => *vertical as i32,
                winit::event::MouseScrollDelta::PixelDelta(pos) => pos.x as i32,
            };
        }
        WindowEvent::KeyboardInput { event, .. } => {
            input.using_controller = false;
            match (event.state, event.physical_key) {
                (ElementState::Pressed, MOVE_FORWARD) => input.controls.move_forward = 1.0,
                (ElementState::Released, MOVE_FORWARD) => input.controls.move_forward = 0.0,
                (ElementState::Pressed, MOVE_BACKWARD) => input.controls.move_backwards = 1.0,
                (ElementState::Released, MOVE_BACKWARD) => input.controls.move_backwards = 0.0,
                (ElementState::Pressed, MOVE_LEFT) => input.controls.move_left = 1.0,
                (ElementState::Released, MOVE_LEFT) => input.controls.move_left = 0.0,
                (ElementState::Pressed, MOVE_RIGHT) => input.controls.move_right = 1.0,
                (ElementState::Released, MOVE_RIGHT) => input.controls.move_right = 0.0,
                (ElementState::Pressed, KEYBOARD_JUMP) => {
                    input.controls.jump = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_JUMP) => {
                    input.controls.jump = ButtonState::Released
                }
                (ElementState::Pressed, KEYBOARD_SPEEDUP) => {
                    input.controls.speedup = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_SPEEDUP) => {
                    input.controls.speedup = ButtonState::Released
                }
                (ElementState::Pressed, KEYBOARD_ACTIVATE) => {
                    input.controls.activate = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_ACTIVATE) => {
                    input.controls.activate = ButtonState::Released
                }
                (ElementState::Pressed, KEYBOARD_PLACE_THING) => {
                    input.controls.place_thing = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_PLACE_THING) => {
                    input.controls.place_thing = ButtonState::Released
                }
                (ElementState::Pressed, KEYBOARD_INVENTORY) => {
                    input.controls.inventory = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_INVENTORY) => {
                    input.controls.inventory = ButtonState::Released
                }

                (ElementState::Pressed, KEYBOARD_MOVE_UP) => input.controls.editor_move_up = 1.0,
                (ElementState::Released, KEYBOARD_MOVE_UP) => input.controls.editor_move_up = 0.0,
                (ElementState::Pressed, KEYBOARD_MOVE_DOWN) => {
                    input.controls.editor_move_down = 1.0
                }
                (ElementState::Released, KEYBOARD_MOVE_DOWN) => {
                    input.controls.editor_move_down = 0.0
                }
                (ElementState::Pressed, KEYBOARD_SWITCH_CAMERA) => {
                    input.controls.editor_switch_camera = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_SWITCH_CAMERA) => {
                    input.controls.editor_switch_camera = ButtonState::Released
                }
                (ElementState::Pressed, KEYBOARD_ENABLE_MOVEMENT) => {
                    input.controls.editor_enable_movement = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_ENABLE_MOVEMENT) => {
                    input.controls.editor_enable_movement = ButtonState::Released
                }
                (ElementState::Pressed, KEYBOARD_CLOSE_WINDOW) => {
                    input.controls.close_window = ButtonState::Pressed
                }
                (ElementState::Released, KEYBOARD_CLOSE_WINDOW) => {
                    input.controls.close_window = ButtonState::Released
                }
                _ => {}
            };
        }
        _ => {}
    }
}

pub fn input_controller_event(event: gilrs::Event) {
    let mut input = input().lock();
    input.using_controller = true;
    match event.event {
        gilrs::EventType::ButtonPressed(button, _) => match button {
            GAMEPAD_JUMP => input.controls.jump = ButtonState::Pressed,
            GAMEPAD_SPEEDUP => input.controls.speedup = ButtonState::Pressed,
            GAMEPAD_ACTIVATE => input.controls.activate = ButtonState::Pressed,
            GAMEPAD_PLACE_THING => input.controls.place_thing = ButtonState::Pressed,
            GAMEPAD_INVENTORY => input.controls.inventory = ButtonState::Pressed,
            GAMEPAD_CYCLE_POSITION_LEFT => input.controls.cycle_position -= 1,
            GAMEPAD_CYCLE_POSITION_RIGHT => input.controls.cycle_position += 1,

            GAMEPAD_MOVE_UP => input.controls.editor_move_up = 1.0,
            GAMEPAD_MOVE_DOWN => input.controls.editor_move_down = 1.0,
            GAMEPAD_SWITCH_CAMERA => input.controls.editor_switch_camera = ButtonState::Pressed,
            GAMEPAD_ENABLE_MOVEMENT => input.controls.editor_enable_movement = ButtonState::Pressed,
            _ => {}
        },
        gilrs::EventType::ButtonReleased(button, _) => match button {
            GAMEPAD_JUMP => input.controls.jump = ButtonState::Released,
            GAMEPAD_SPEEDUP => input.controls.speedup = ButtonState::Released,
            GAMEPAD_ACTIVATE => input.controls.activate = ButtonState::Released,
            GAMEPAD_PLACE_THING => input.controls.place_thing = ButtonState::Released,
            GAMEPAD_INVENTORY => input.controls.inventory = ButtonState::Released,

            GAMEPAD_MOVE_UP => input.controls.editor_move_up = 0.0,
            GAMEPAD_MOVE_DOWN => input.controls.editor_move_down = 0.0,
            GAMEPAD_SWITCH_CAMERA => input.controls.editor_switch_camera = ButtonState::Released,
            GAMEPAD_ENABLE_MOVEMENT => {
                input.controls.editor_enable_movement = ButtonState::Released
            }
            _ => {}
        },
        gilrs::EventType::AxisChanged(axis, value, _) => match axis {
            gilrs::Axis::LeftStickX if value >= 0.0 => {
                input.controls.move_right = value;
                input.controls.move_left = 0.0;
            }
            gilrs::Axis::LeftStickX if value < 0.0 => {
                input.controls.move_left = -value;
                input.controls.move_right = 0.0
            }
            gilrs::Axis::LeftStickY if value >= 0.0 => {
                input.controls.move_forward = value;
                input.controls.move_backwards = 0.0
            }
            gilrs::Axis::LeftStickY if value < 0.0 => {
                input.controls.move_backwards = -value;
                input.controls.move_forward = 0.0
            }
            gilrs::Axis::RightStickX => {
                input.controls.rotate_camera_x = value * input.sensitivity_modifier * 8.0
            }
            gilrs::Axis::RightStickY => {
                input.controls.rotate_camera_y = -value * input.sensitivity_modifier * 8.0
            }
            _ => {}
        },
        _ => {}
    }
}

/// call at the end of each frame
pub fn input_update() {
    let mut input = input().lock();
    match input.controls.jump {
        ButtonState::Pressed => input.controls.jump = ButtonState::Down,
        ButtonState::Released => input.controls.jump = ButtonState::Up,
        _ => {}
    }
    match input.controls.speedup {
        ButtonState::Pressed => input.controls.speedup = ButtonState::Down,
        ButtonState::Released => input.controls.speedup = ButtonState::Up,
        _ => {}
    }
    match input.controls.activate {
        ButtonState::Pressed => input.controls.activate = ButtonState::Down,
        ButtonState::Released => input.controls.activate = ButtonState::Up,
        _ => {}
    }
    match input.controls.place_thing {
        ButtonState::Pressed => input.controls.place_thing = ButtonState::Down,
        ButtonState::Released => input.controls.place_thing = ButtonState::Up,
        _ => {}
    }
    match input.controls.inventory {
        ButtonState::Pressed => input.controls.inventory = ButtonState::Down,
        ButtonState::Released => input.controls.inventory = ButtonState::Up,
        _ => {}
    }

    match input.controls.editor_switch_camera {
        ButtonState::Pressed => input.controls.editor_switch_camera = ButtonState::Down,
        ButtonState::Released => input.controls.editor_switch_camera = ButtonState::Up,
        _ => {}
    }
    match input.controls.editor_enable_movement {
        ButtonState::Pressed => input.controls.editor_enable_movement = ButtonState::Down,
        ButtonState::Released => input.controls.editor_enable_movement = ButtonState::Up,
        _ => {}
    }
    match input.controls.close_window {
        ButtonState::Pressed => input.controls.close_window = ButtonState::Down,
        ButtonState::Released => input.controls.close_window = ButtonState::Up,
        _ => {}
    }

    // only when using keyboard
    if !input.using_controller {
        input.controls.rotate_camera_x = 0.0;
        input.controls.rotate_camera_y = 0.0;
    }
}
