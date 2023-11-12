use cogrrs::{anyhow::Result, main_loop_run, CoGr, Game, Input};

pub struct HelloWorld {}

impl Game for HelloWorld {
    fn on_init(_gpu: &mut CoGr) -> Result<Self> {
        Ok(Self {})
    }

    fn on_render(&mut self, gpu: &mut CoGr, _input: &Input, dt: f32) -> Result<()> {
        let mut encoder = gpu.get_encoder_for_draw()?;
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
    main_loop_run::<HelloWorld>(10f32)?;
    Ok(())
}
