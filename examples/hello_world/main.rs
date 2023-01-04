use gpu::Context;
use ui::MainGui;
use window::{
    input::{button::ButtonState, Input},
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::window::Window,
};

pub struct HelloWorld {
    pub gpu_context: Context,
    pub ui: MainGui,
}

impl Game for HelloWorld {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = Context::new(window, "to_draw_texture", "");

        gpu_context.texture("to_draw_texture", (1280, 720, 1), gpu_context.config.format);

        let ui = MainGui::new(&gpu_context, window);

        HelloWorld { gpu_context, ui }
    }

    fn on_render(&mut self, input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.ui.text("fps", &(1f32 / dt).to_string());

        self.ui.draw(
            &self.gpu_context,
            &mut encoder,
            window,
            input.mouse_state.mouse_location,
            input.mouse_state.get_left_button() == ButtonState::Pressed,
        );

        self.gpu_context.execute_encoder(encoder);
        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}

    fn on_tick(&mut self, _dt: f32) -> UpdateResult {
        UpdateResult::Continue
    }
}

fn main() {
    main_loop_run::<HelloWorld>(1280, 720, 10f32);
}
