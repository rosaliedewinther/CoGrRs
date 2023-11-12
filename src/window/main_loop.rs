use crate::CoGr;
use crate::Input;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use winit::dpi::PhysicalPosition;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

pub trait Game: Sized {
    fn on_init(gpu: &mut CoGr) -> Result<Self>;
    fn on_tick(&mut self, gpu: &mut CoGr, dt: f32) -> Result<()>;
    fn on_render(&mut self, gpu: &mut CoGr, input: &Input, dt: f32) -> Result<()>;
}

pub fn main_loop_run<T>(ticks_per_s: f32) -> Result<()>
where
    T: 'static + Game,
{
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    puffin::set_scopes_on(true);
    let event_loop = EventLoop::new();
    info!("created event loop");
    let monitor = event_loop
        .primary_monitor()
        .expect("We don't support having no monitors");
    info!("created monitor");
    let window_builder = WindowBuilder::new()
        .with_resizable(false)
        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(monitor))));
    info!("created window builder");
    let window = Arc::new(
        window_builder
            .build(&event_loop)
            .expect("unable to build window"),
    );
    info!("created window");
    let mut window_input = Input::new();
    info!("created window input");
    let mut on_tick_timer = Instant::now();
    let mut on_render_timer = Instant::now();
    let mut gpu = CoGr::new(&window, &event_loop)?;
    info!("created gpu");
    let mut game = T::on_init(&mut gpu)?;
    info!("created game");

    event_loop.run(move |event, _, control_flow| {
        puffin::profile_function!();
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                gpu.handle_window_event(event);
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
                puffin::profile_scope!("Render");
                puffin::GlobalProfiler::lock().new_frame();
                let dt = on_render_timer.elapsed().as_secs_f32();
                on_render_timer = Instant::now();
                match game.on_render(&mut gpu, &window_input, dt) {
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
                    puffin::profile_scope!("Tick");
                    if let Err(err) = game.on_tick(&mut gpu, on_tick_timer.elapsed().as_secs_f32())
                    {
                        println!("{}", err);
                        *control_flow = ControlFlow::Exit;
                    }
                    on_tick_timer = Instant::now();
                }
            }
        }
    });
}
