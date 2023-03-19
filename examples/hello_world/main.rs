use std::sync::Arc;

use gpu::{egui, CoGr, CoGrEncoder, Renderer};
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult},
    winit::{event::WindowEvent, event_loop::EventLoop, window::Window},
};

pub struct HelloWorld {
    pub gpu_context: Renderer,
}

impl Game for HelloWorld {
    fn on_init(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Self {
        let gpu_context = Renderer::new(window, "", event_loop);
        HelloWorld { gpu_context }
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("fps: {}", 1f32 / dt));
            });
        });

        RenderResult::Continue
    }

    fn on_tick(&mut self, _dt: f32) {}
    fn on_window_event(&mut self, event: &WindowEvent) {
        self.gpu_context.handle_window_event(event);
    }
}

fn main() {
    main_loop_run::<HelloWorld>(1280, 720, 10f32);
}
