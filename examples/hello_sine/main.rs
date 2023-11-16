use bytemuck::{Pod, Zeroable};
use cogrrs::{
    anyhow::Result, div_ceil, main_loop_run, tracing::info, CoGr, Game, Input, Pipeline,
    ResourceHandle, TextureFormat, TextureRes,
};

pub struct HelloSine {
    pub to_draw_texture: ResourceHandle,
    pub uniform_buffer: ResourceHandle,
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
            gpu.texture("to_draw", TextureRes::FullRes, TextureFormat::Rgba8Unorm);
        let uniform_buffer = gpu.buffer("gpu data", 1, std::mem::size_of::<GpuData>());
        let draw_pipeline = gpu.pipeline("examples/hello_sine/sine.hlsl")?;
        Ok(HelloSine {
            to_draw_texture,
            uniform_buffer,
            draw_pipeline,
            time: 0f32,
        })
    }

    fn on_render(&mut self, gpu: &mut CoGr, _input: &Input, dt: f32) -> Result<()> {
        info!("on_render");
        let width = gpu.config.width;
        let height = gpu.config.height;

        let mut encoder = gpu.get_encoder_for_draw()?;

        self.time += dt;
        let gpu_data = GpuData {
            time: self.time,
            width: encoder.width(),
            height: encoder.height(),
        };
        encoder.set_buffer_data(&self.uniform_buffer, [gpu_data])?;
        encoder.dispatch_pipeline(
            &mut self.draw_pipeline,
            (div_ceil(width, 16), div_ceil(height, 16), 1),
            &[&self.to_draw_texture, &self.uniform_buffer],
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
