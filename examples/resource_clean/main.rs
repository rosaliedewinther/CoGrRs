use anyhow::Result;
use std::sync::Arc;

use gpu::{egui, resources::BufferHandle, CoGr};
use window::{
    input::Input,
    main_loop::{main_loop_run, Game},
    winit::{event::WindowEvent, event_loop::EventLoop, window::Window},
};

pub struct HelloWorld {
    pub gpu_context: CoGr,
    _buffer_handle: BufferHandle,
    first_print: u32,
}

impl Game for HelloWorld {
    fn on_init(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Result<Self> {
        let mut gpu_context = CoGr::new(window, event_loop)?;

        // create many buffers that wont be used
        gpu_context.buffer("buffer_1", 256usize, 8);
        gpu_context.buffer("buffer_2", 256usize, 8);
        gpu_context.buffer("buffer_3", 256usize, 8);
        gpu_context.buffer("buffer_4", 256usize, 8);
        let buffer_5 = gpu_context.buffer("buffer_5", 256usize, 8);
        gpu_context.buffer("buffer_6", 256usize, 8);
        gpu_context.buffer("buffer_7", 256usize, 8);

        gpu_context.log_state();

        Ok(HelloWorld {
            gpu_context,
            _buffer_handle: buffer_5,
            first_print: 0,
        })
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32) -> Result<()> {
        if self.first_print < 2 {
            // after a get_encoder call, all buffer handles that no longer exist will be deleted
            self.gpu_context.log_state();
            self.first_print += 1;
        }

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
