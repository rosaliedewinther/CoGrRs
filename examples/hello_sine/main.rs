use bytemuck::{Pod, Zeroable};
use gpu::Context;
use gpu::Execution::PerPixel2D;
use ui::{imgui::MainGui, UI};
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::window::Window,
};

pub struct HelloSine {
    pub gpu_context: Context,
    pub ui: MainGui,
    pub time: f32,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct GpuData {
    time: f32,
}

impl Game for HelloSine {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = Context::new(window, "to_draw_texture", "examples/hello_sine/");

        gpu_context.texture("to_draw_texture", (1280, 720, 1), gpu_context.config.format);

        let ui = MainGui::new(&gpu_context, window);

        HelloSine { gpu_context, ui, time: 0f32 }
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32, _window: &Window) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.time += dt;
        let gpu_data = GpuData { time: self.time };
        encoder
            .gpu_context
            .dispatch_pipeline("sine", PerPixel2D, encoder.encoder.as_mut().unwrap(), &gpu_data);
        Context::image_buffer_to_screen(&mut encoder);

        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}

    fn on_tick(&mut self, _dt: f32) -> UpdateResult {
        UpdateResult::Continue
    }
}

fn main() {
    main_loop_run::<HelloSine>(1280, 720, 10f32);
}
