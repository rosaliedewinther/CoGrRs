use camera::Camera;
use cogrrs::{Game, CoGr, Input, anyhow::Result, main_loop_run, TextureRes, ResourceHandle};
use wgpu::TextureFormat;

mod key_mapping;
mod camera;


pub struct VoxelTracer {
    camera: Camera,
    to_screen: ResourceHandle
}

impl Game for VoxelTracer {
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        let camera = Camera::new(gpu);
        let to_screen = gpu.texture("to_screen", TextureRes::FullRes, TextureFormat::Rgba16Float);
        Ok(Self {camera, to_screen})
    }

    fn on_render(&mut self, gpu: &mut CoGr, input: &Input, dt: f32) -> Result<()> {
        let mut encoder = gpu.get_encoder_for_draw()?;

        self.camera.update(input, dt);
        let _camera_results = self.camera.dispatch(&mut encoder);
        self.camera.debug_ray_direction(&mut encoder, &self.to_screen);

        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("fps: {}", 1f32 / dt));
            });
        })?;

        Ok(())
    }

    fn on_tick(&mut self, _gpu: &mut CoGr, _dt: f32) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<VoxelTracer>(10f32)?;
    Ok(())
}
