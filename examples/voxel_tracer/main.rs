use camera::Camera;
use cogrrs::{Game, CoGr, Input, anyhow::Result, main_loop_run, TextureRes, ResourceHandle};
use trace::Trace;
use wgpu::TextureFormat;

mod key_mapping;
mod camera;
mod trace;

#[derive(Debug, PartialEq)]
enum RenderMode{
    Trace,
    Directions,
}

pub struct VoxelTracer {
    camera: Camera,
    trace: Trace,
    time: f32,
    render_mode: RenderMode,
    to_screen: ResourceHandle,
}

impl Game for VoxelTracer {
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        let camera = Camera::new(gpu);
        let trace = Trace::new(gpu);
        let to_screen = gpu.texture("to_screen", TextureRes::FullRes, TextureFormat::Rgba16Float);
        Ok(Self {camera, trace, time: 0.0, render_mode: RenderMode::Trace, to_screen})
    }

    fn on_render(&mut self, gpu: &mut CoGr, input: &Input, dt: f32) -> Result<()> {
        let mut encoder = gpu.get_encoder_for_draw()?;
        self.time += dt;

        self.camera.update(input, dt);
        let camera_results = self.camera.dispatch(&mut encoder);

        match self.render_mode{
            RenderMode::Trace => {self.trace.dispatch(&mut encoder, self.time, &camera_results, &self.to_screen, camera_results.camera.position);},
            RenderMode::Directions =>self.camera.debug_ray_direction(&mut encoder, &self.to_screen),
        }
        
        encoder.to_screen(&self.to_screen)?;

        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("fps: {}", 1f32 / dt));
                egui::ComboBox::from_label("Select one!")
                    .selected_text(format!("{:?}", self.render_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.render_mode, RenderMode::Trace, "Trace");
                        ui.selectable_value(&mut self.render_mode, RenderMode::Directions, "Directions");
                    }
                );
                self.camera.draw_ui(ui);
                self.trace.draw_ui(ui);
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
