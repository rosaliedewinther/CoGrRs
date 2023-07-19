use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use cogrrs::TextureRes::FullRes;
use cogrrs::{div_ceil, main_loop_run, Input};
use cogrrs::{CoGr, Game, Pipeline, ResourceHandle};

pub struct HelloSine {
    pub to_draw_texture: ResourceHandle,
    pub draw_pipeline: Pipeline,
    pub time: f32,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct GpuData {
    time: f32,
    width: u32,
    height: u32,
}

impl Game for HelloSine {
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        let to_draw_texture = gpu.texture("to_draw", FullRes, gpu.config.format);
        let draw_pipeline = gpu.pipeline("examples/hello_sine/sine.hlsl")?;
        Ok(HelloSine {
            to_draw_texture,
            draw_pipeline,
            time: 0f32,
        })
    }

    fn on_render(&mut self, gpu: &mut CoGr, _input: &Input, dt: f32) -> Result<()> {
        let width = gpu.config.width;
        let height = gpu.config.height;
        let mut encoder = gpu.get_encoder_for_draw()?;

        self.time += dt;
        let gpu_data = GpuData {
            time: self.time,
            width: encoder.width(),
            height: encoder.height(),
        };
        encoder.dispatch_pipeline(
            &mut self.draw_pipeline,
            (div_ceil(width, 32), div_ceil(height, 32), 1),
            &gpu_data,
            &[&self.to_draw_texture],
        )?;
        encoder.to_screen(&self.to_draw_texture)?;

        Ok(())
    }

    fn on_tick(&mut self, _gpu: &mut CoGr, _dt: f32) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<HelloSine>(10f32)?;
    Ok(())
}
