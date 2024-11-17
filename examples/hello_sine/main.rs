use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use cogrrs::{
    anyhow::Result, div_ceil, main_loop_run, tracing::info, CoGr, Game, GameState, Input, Pipeline,
    ResourceHandle, TextureFormat, TextureRes,
};

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
        let to_draw_texture =
            gpu.texture("to_draw", TextureRes::FullRes, TextureFormat::Rgba32Float);
        let draw_pipeline = gpu.pipeline("examples/hello_sine/sine.hlsl")?;
        Ok(HelloSine {
            to_draw_texture,
            draw_pipeline,
            time: 0f32,
        })
    }

    fn on_render(&mut self, gpu: &mut CoGr, dt: f32) -> Result<GameState> {
        let width = gpu.config.width;
        let height = gpu.config.height;
        let render_data_buffer = gpu.buffer("render data", 1, size_of::<GpuData>());
        let mut encoder = gpu.get_encoder_for_draw()?;

        info!("hello");

        self.time += dt;

        encoder.set_buffer_data(
            &render_data_buffer,
            &[GpuData {
                time: self.time,
                width,
                height,
            }],
        )?;
        encoder.dispatch_pipeline(
            &mut self.draw_pipeline,
            (div_ceil(width, 32), div_ceil(height, 32), 1),
            &[&self.to_draw_texture],
        )?;
        encoder.to_screen(&self.to_draw_texture, TextureFormat::Rgba32Float)?;

        Ok(GameState::Continue)
    }

    fn on_resize(&mut self, cogr: &mut CoGr, new_dimensions: glam::UVec2) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<HelloSine>()?;
    Ok(())
}
