use std::time::Instant;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use crate::input::Input;

pub enum RenderResult {
    Continue,
    Exit,
}
pub enum UpdateResult {
    Continue,
    Exit,
}

pub trait Game {
    fn on_init(window: &Window) -> Self;
    fn on_tick(&mut self, dt: f32) -> UpdateResult;
    fn on_render(&mut self, input: &mut Input, dt: f32, window: &Window) -> RenderResult;
    fn on_resize(&mut self, new_size: (u32, u32));
}

pub fn main_loop_run<T>(window_width: u32, window_height: u32, ticks_per_s: f32)
where
    T: 'static + Game,
{
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(window_width, window_height))
        .with_resizable(false);
    let window = window_builder
        .build(&event_loop)
        .expect("unable to build window");
    let mut game = T::on_init(&window);
    let mut window_input = Input::new();
    let mut on_tick_timer = Instant::now();
    let mut on_render_timer = Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CursorMoved { position, .. } => {
                window_input.update_cursor_moved(&PhysicalPosition::<f32> {
                    x: position.x as f32,
                    y: position.y as f32,
                });
            }
            WindowEvent::CursorEntered { .. } => {
                window_input.update_cursor_entered();
            }
            WindowEvent::CursorLeft { .. } => {
                window_input.update_cursor_left();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                window_input.update_mouse_input(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                window_input.update_mouse_wheel(delta);
            }
            WindowEvent::KeyboardInput { input, .. } => {
                window_input.update_keyboard_input(input, control_flow);
            }
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                game.on_resize((physical_size.width, physical_size.height));
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                game.on_resize((new_inner_size.width, new_inner_size.height));
            }

            _ => {}
        },
        Event::RedrawRequested(_) => {
            let dt = on_render_timer.elapsed().as_secs_f32();
            on_render_timer = Instant::now();
            match game.on_render(&mut window_input, dt, &window) {
                RenderResult::Continue => {
                    window_input.update();
                }
                RenderResult::Exit => *control_flow = ControlFlow::Exit,
            };
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {
            if on_tick_timer.elapsed().as_secs_f32() * ticks_per_s > 1f32 {
                match game.on_tick(on_tick_timer.elapsed().as_secs_f32()) {
                    UpdateResult::Continue => {}
                    UpdateResult::Exit => *control_flow = ControlFlow::Exit,
                }
                on_tick_timer = Instant::now();
            }
        }
    });
}
