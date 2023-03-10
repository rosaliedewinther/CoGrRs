use gpu::{CoGr, Renderer, Ui, UI};
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult},
    winit::{event_loop::EventLoop, window::Window},
};

pub struct HelloWorld {
    pub gpu_context: Renderer,
    pub ui: Ui,
}

impl Game for HelloWorld {
    fn on_init(window: &Window, event_loop: &EventLoop<()>) -> Self {
        let mut gpu_context = Renderer::new(window, "");
        gpu_context.texture("to_draw_texture", (1280, 720, 1), gpu_context.config.format);
        let ui = Ui::new(&gpu_context, window, event_loop);
        HelloWorld { gpu_context, ui }
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.ui.draw(&mut encoder, window, |ui| {
            ui.label(format!("fps: {}", 1f32 / dt));
        });

        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}
    fn on_tick(&mut self, _dt: f32) {}
    fn on_window_event(&mut self, event: &window::winit::event::WindowEvent) {
        self.ui.handle_window_event(event);
    }
}

fn main() {
    main_loop_run::<HelloWorld>(1280, 720, 10f32);
}
