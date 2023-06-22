use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use gpu::compute_pipeline::ComputePipeline;
use gpu::resources::ResourceHandle::T;
use gpu::resources::TextureHandle;
use gpu::resources::TextureRes::FullRes;
use gpu::CoGr;
use std::path::Path;
use std::sync::Arc;
use window::winit::event::WindowEvent;
use window::winit::event_loop::EventLoop;
use window::{
    input::Input,
    main_loop::{main_loop_run, Game},
    winit::window::Window,
};

pub struct HelloSine {
    pub gpu_context: CoGr,
    pub to_draw_texture: TextureHandle,
    pub draw_pipeline: ComputePipeline,
    pub time: f32,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct GpuData {
    time: f32,
}

impl Game for HelloSine {
    fn on_init(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Result<Self> {
        let mut gpu_context = CoGr::new(window, event_loop)?;
        let to_draw_texture = gpu_context.texture("to_draw", FullRes, gpu_context.config.format);
        let draw_pipeline =
            gpu_context.init_pipeline(Path::new("examples/hello_sine/sine.hlsl"))?;
        Ok(HelloSine {
            gpu_context,
            to_draw_texture,
            draw_pipeline,
            time: 0f32,
        })
    }

    fn on_render(&mut self, _input: &mut Input, dt: f32) -> Result<()> {
        let mut encoder = self.gpu_context.get_encoder_for_draw()?;

        self.time += dt;
        let gpu_data = GpuData { time: self.time };
        encoder.dispatch_pipeline(
            &mut self.draw_pipeline,
            (1280 / 32, 720 / 32, 1),
            &gpu_data,
            &[T(&self.to_draw_texture)],
        )?;
        encoder.to_screen(&self.to_draw_texture)?;

        Ok(())
    }

    fn on_tick(&mut self, _dt: f32) -> Result<()> {
        Ok(())
    }
    fn on_window_event(&mut self, _event: &WindowEvent) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<HelloSine>(1280, 720, 10f32)?;
    Ok(())
}
