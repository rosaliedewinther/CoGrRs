use cogrrs::{anyhow::Result, main_loop_run, CoGr, Game, Input, ResourceHandle};

pub struct HelloWorld {
    _buffer_handle: ResourceHandle,
    first_print: u32,
}

impl Game for HelloWorld {
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        // create many buffers that wont be used
        gpu.buffer("buffer_1", 256usize, 8);
        gpu.buffer("buffer_2", 256usize, 8);
        gpu.buffer("buffer_3", 256usize, 8);
        gpu.buffer("buffer_4", 256usize, 8);
        let buffer_5 = gpu.buffer("buffer_5", 256usize, 8);
        gpu.buffer("buffer_6", 256usize, 8);
        gpu.buffer("buffer_7", 256usize, 8);

        Ok(HelloWorld {
            _buffer_handle: buffer_5,
            first_print: 0,
        })
    }

    fn on_render(&mut self, gpu: &mut CoGr, _input: &Input, dt: f32) -> Result<()> {
        if self.first_print < 2 {
            // after a get_encoder call, all buffer handles that no longer exist will be deleted
            self.first_print += 1;
        }
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
