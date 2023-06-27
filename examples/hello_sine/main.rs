use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use gpu::compute_pipeline::ComputePipeline;
use gpu::resources::ResourceHandle::T;
use gpu::resources::TextureHandle;
use gpu::resources::TextureRes::FullRes;
use gpu::CoGr;
use window::{main_loop_run, Game, Input};

pub struct HelloSine {
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
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        let to_draw_texture = gpu.texture("to_draw", FullRes, gpu.config.format);
        let draw_pipeline = gpu.init_pipeline("examples/hello_sine/sine.hlsl")?;
        Ok(HelloSine {
            to_draw_texture,
            draw_pipeline,
            time: 0f32,
        })
    }

    fn on_render(&mut self, gpu: &mut CoGr, _input: &mut Input, dt: f32) -> Result<()> {
        let mut encoder = gpu.get_encoder_for_draw()?;

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

    fn on_tick(&mut self, _gpu: &mut CoGr, _dt: f32) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<HelloSine>(1280, 720, 10f32)?;
    Ok(())
}
