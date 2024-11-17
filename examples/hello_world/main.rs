use cogrrs::{anyhow::Result, controls, main_loop_run, CoGr, Game, GameState};

pub struct HelloWorld {}

impl Game for HelloWorld {
    fn on_init(_cogr: &mut CoGr) -> Result<Self> {
        Ok(Self {})
    }

    fn on_render(&mut self, cogr: &mut CoGr, dt: f32) -> Result<GameState> {
        if controls().close_window.pressed() {
            return Ok(GameState::Break);
        }

        let mut encoder = cogr.get_encoder_for_draw()?;
        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("fps: {}", 1f32 / dt));
            });
        })?;

        Ok(GameState::Continue)
    }

    fn on_resize(&mut self, _cogr: &mut CoGr, _new_dimensions: glam::UVec2) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<HelloWorld>()?;
    Ok(())
}
