use bytemuck::{Pod, Zeroable};
use gpu::shader::Execution::PerPixel2D;
use gpu::{CoGr, CoGrEncoder, Renderer};
use window::winit::event_loop::{EventLoop};
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult},
    winit::window::Window,
};

pub struct HelloSine {
    pub gpu_context: Renderer,
    pub time: f32,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct GpuData {
    time: f32,
}

impl Game for HelloSine {
    fn on_init(window: &Window, _event_loop: &EventLoop<()>) -> Self {
        let mut gpu_context = Renderer::new(window, "to_draw_texture", "examples/hello_sine/");
        gpu_context.texture("to_draw_texture", (1280, 720, 1), gpu_context.config.format);
        HelloSine { gpu_context, time: 0f32 }
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32, _window: &Window) -> RenderResult {
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.time += dt;
        let gpu_data = GpuData { time: self.time };
        encoder.dispatch_pipeline("sine", PerPixel2D, &gpu_data);
        encoder.image_buffer_to_screen();

        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}
    fn on_tick(&mut self, _dt: f32) {}
    fn on_window_event(&mut self, _event: &window::winit::event::WindowEvent) {}
}

fn main() {
    main_loop_run::<HelloSine>(1280, 720, 10f32);
}
