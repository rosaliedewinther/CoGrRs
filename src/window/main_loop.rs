use anyhow::Result;
use glam::UVec2;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoop;
use winit::window::Fullscreen;
use winit::window::Window;
use winit::window::WindowAttributes;

use crate::controls;
use crate::CoGr;

pub enum GameState {
    Continue,
    Break,
}

pub trait Game: Sized {
    fn on_init(cogr: &mut CoGr) -> Result<Self>;
    fn on_render(&mut self, cogr: &mut CoGr, dt: f32) -> Result<GameState>;
    fn on_resize(&mut self, cogr: &mut CoGr, new_dimensions: UVec2) -> Result<()>;
}

struct GameApplicationHandler<T: Game> {
    window: Option<Arc<Window>>,
    game: Option<T>,
    cogr: Option<CoGr>,
    on_render_timer: Option<Instant>,
}

impl<T: Game> Default for GameApplicationHandler<T> {
    fn default() -> Self {
        Self {
            window: None,
            game: None,
            cogr: None,
            on_render_timer: None,
        }
    }
}

impl<T: Game> ApplicationHandler for GameApplicationHandler<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            info!("Init window");
            self.window = Some(Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_fullscreen(Some(Fullscreen::Borderless(None))),
                    )
                    .unwrap(),
            ));
            info!("Created window");
        }
        if self.cogr.is_none() {
            info!("Init cogr");
            self.cogr = Some(CoGr::new(self.window.clone().unwrap()).unwrap());
            info!("Created cogr");
        }
        if self.game.is_none() {
            info!("Init game");
            self.game = Some(T::on_init(self.cogr.as_mut().unwrap()).unwrap());
            info!("Created game");
        }

        if self.on_render_timer.is_none() {
            self.on_render_timer = Some(Instant::now());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        crate::window::input_window_event(&event);
        if let Some(cogr) = &mut self.cogr {
            cogr.handle_window_event(&event);
        }

        match event {
            WindowEvent::Resized(size) => {
                puffin::profile_scope!("Resize");
                if let Some(game) = &mut self.game {
                    if let Some(cogr) = &mut self.cogr {
                        game.on_resize(
                            cogr,
                            UVec2 {
                                x: size.width,
                                y: size.height,
                            },
                        )
                        .unwrap()
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                puffin::GlobalProfiler::lock().new_frame();
                puffin::profile_scope!("Render");

                let dt = if let Some(timer) = self.on_render_timer {
                    timer.elapsed().as_secs_f32()
                } else {
                    // To prevent we set dt to 0 when we don't know the time
                    0.0
                };
                self.on_render_timer = Some(Instant::now());

                if let Some(game) = &mut self.game {
                    if let Some(cogr) = &mut self.cogr {
                        let result = game.on_render(cogr, dt).unwrap();
                        if matches!(result, GameState::Break) {
                            event_loop.exit();
                        }
                    }
                    crate::window::input::input_update();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {
                //dbg!(event);
            }
        }
    }
}

pub fn main_loop_run<T>() -> Result<()>
where
    T: 'static + Game,
{
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let event_loop = EventLoop::new().unwrap();
    info!("created event loop");

    event_loop
        .run_app(&mut GameApplicationHandler::<T>::default())
        .map_err(|err| anyhow::Error::from(err))
}
