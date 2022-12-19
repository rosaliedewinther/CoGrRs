use gpu::gpu_context::GpuContext;
use gpu::wgpu::TextureFormat::Rgba8Unorm;
use ui::main_gui::MainGui;
use window::{
    input::{button::ButtonState, Input},
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::{dpi::PhysicalSize, window::Window},
};

pub struct HelloWorld {
    pub gpu_context: GpuContext,
    pub ui: MainGui,
}

impl Game for HelloWorld {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = GpuContext::new(window, "to_draw_texture");

        gpu_context.texture("to_draw_texture", (1280, 720, 1), Rgba8Unorm);

        let ui = MainGui::new(
            &gpu_context.device,
            &gpu_context.config.format,
            window,
            &gpu_context.queue,
        );

        HelloWorld { gpu_context, ui }
    }

    fn on_render(&mut self, input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();
        self.gpu_context.image_buffer_to_screen(&mut encoder);

        self.ui.text("welcome_message", "hello world");

        self.ui.draw(
            &self.gpu_context,
            &mut encoder,
            window,
            self.gpu_context.surface_texture_view.as_ref().unwrap(),
            input.mouse_state.mouse_location,
            input.mouse_state.get_left_button() == ButtonState::Pressed,
        );

        self.gpu_context.execute_encoder(encoder);
        RenderResult::Continue
    }

    fn on_resize(&mut self, physical_size: PhysicalSize<u32>) {}

    fn on_tick(&mut self, _dt: f32) -> UpdateResult {
        UpdateResult::Continue
    }
}

fn main() {
    main_loop_run::<HelloWorld>(1280, 720, 10f32);
}
