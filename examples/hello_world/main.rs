use anyhow::Result;
use std::sync::Arc;

use gpu::{egui, CoGr};
use window::{
    input::Input,
    main_loop::{main_loop_run, Game},
    winit::{event::WindowEvent, event_loop::EventLoop, window::Window},
};

pub struct HelloWorld {
    pub gpu_context: CoGr,
}

impl Game for HelloWorld {
    fn on_init(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Result<Self> {
        let gpu_context = CoGr::new(window, event_loop)?;
        Ok(HelloWorld { gpu_context })
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32) -> Result<()> {
        let mut encoder = self.gpu_context.get_encoder_for_draw()?;

        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("fps: {}", 1f32 / dt));
            });
        })?;

        Ok(())
    }

    fn on_tick(&mut self, _dt: f32) -> Result<()> {
        Ok(())
    }
    fn on_window_event(&mut self, event: &WindowEvent) -> Result<()> {
        self.gpu_context.handle_window_event(event);
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<HelloWorld>(1280, 720, 10f32)?;
    Ok(())
}
