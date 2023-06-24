use crate::input::Input;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub trait Game: Sized {
    fn on_init(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Result<Self>;
    fn on_tick(&mut self, dt: f32) -> Result<()>;
    fn on_render(&mut self, input: &mut Input, dt: f32) -> Result<()>;
    fn on_window_event(&mut self, event: &WindowEvent) -> Result<()>;
}

pub fn main_loop_run<T>(window_width: u32, window_height: u32, ticks_per_s: f32) -> Result<()>
where
    T: 'static + Game,
{
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(window_width, window_height))
        .with_resizable(false);
    let window = Arc::new(
        window_builder
            .build(&event_loop)
            .expect("unable to build window"),
    );
    let mut game = T::on_init(&window, &event_loop)?;
    let mut window_input = Input::new();
    let mut on_tick_timer = Instant::now();
    let mut on_render_timer = Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if let Err(err) = game.on_window_event(event) {
                println!("{}", err);
                *control_flow = ControlFlow::Exit;
            }
            match event {
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

                _ => {}
            }
        }
        Event::RedrawRequested(_) => {
            let dt = on_render_timer.elapsed().as_secs_f32();
            on_render_timer = Instant::now();
            match game.on_render(&mut window_input, dt) {
                Ok(_) => {
                    window_input.update();
                }
                Err(err) => {
                    println!("{}", err);
                    *control_flow = ControlFlow::Exit;
                }
            };
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {
            if on_tick_timer.elapsed().as_secs_f32() * ticks_per_s > 1f32 {
                if let Err(err) = game.on_tick(on_tick_timer.elapsed().as_secs_f32()) {
                    println!("{}", err);
                    *control_flow = ControlFlow::Exit;
                }
                on_tick_timer = Instant::now();
            }
        }
    });
}
