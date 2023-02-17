use gpu::{wgpu_impl::CoGrWGPU, CoGr};
use ui::UI;
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::window::Window,
};

pub struct HelloWorld {
    pub gpu_context: CoGrWGPU,
    pub ui: ui::imgui::MainGui,
}

impl Game for HelloWorld {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = CoGrWGPU::new(window, "to_draw_texture", "");

        gpu_context.texture("to_draw_texture", (1280, 720, 1), gpu_context.config.format);

        let ui = ui::imgui::MainGui::new(&gpu_context, window);

        HelloWorld { gpu_context, ui }
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.ui.text("fps", &(1f32 / dt).to_string());

        self.ui.draw(&mut encoder, window);

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
