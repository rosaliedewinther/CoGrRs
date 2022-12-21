use std::cell::UnsafeCell;

use bvh::{normalize, Point, BVH};
use gpu::wgpu::TextureFormat::R32Float;
use gpu::Context;
use gpu::Execution::PerPixel2D;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use ui::MainGui;
use window::{
    input::{button::ButtonState, Input},
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::window::Window,
};

use crate::bvh::Ray;

mod bvh;

pub struct HelloWorld {
    pub gpu_context: Context,
    pub ui: MainGui,
    pub bvh: BVH,
    pub screen_buffer: Vec<f32>,
}

impl Game for HelloWorld {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = Context::new(
            window,
            "to_draw_texture",
            "crates/CoGrRs/examples/ray_tracer/",
        );

        gpu_context.texture("to_draw_texture", (1280, 720, 1), gpu_context.config.format);
        gpu_context.texture("depth_buffer", (1280, 720, 1), R32Float);
        gpu_context.pipeline("draw", [], PerPixel2D);

        let screen_buffer = vec![0f32; 1280 * 720];

        let mut bvh = BVH::construct("crates/CoGrRs/examples/ray_tracer/teapot.obj");
        bvh.build_bvh();

        let ui = MainGui::new(&gpu_context, window);

        HelloWorld {
            gpu_context,
            ui,
            bvh,
            screen_buffer,
        }
    }

    fn on_render(&mut self, input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        self.screen_buffer = (0..720 * 1280)
            .into_par_iter()
            .map(|index| {
                let x = index % 1280;
                let y = index / 1280;
                let screen_plane_y = ((y * -1 + 720) as f32 - (720f32 / 2f32)) / (720f32 / 2f32);
                let screen_plane_x = (x as f32 - (1280f32 / 2f32)) / (1280f32 / 2f32) * 1.7777777;

                let ray_origin = Point::new(0f32, 0f32, -10f32);
                let ray_direction =
                    normalize(&(Point::new(screen_plane_x, screen_plane_y, 0f32) - ray_origin));
                let ray_r_direction = Point::new(
                    1f32 / ray_direction.pos[0],
                    1f32 / ray_direction.pos[1],
                    1f32 / ray_direction.pos[2],
                );
                let mut ray = Ray {
                    o: ray_origin,
                    d: ray_direction,
                    d_r: ray_r_direction,
                    t: f32::MAX,
                    prim: u32::MAX,
                    _padding1: 0,
                    _padding2: 0,
                };

                self.bvh.fast_intersect(&mut ray);

                ray.t
            })
            .collect();

        println!("done with screen buffer");

        self.gpu_context.set_texture_data(
            "depth_buffer",
            self.screen_buffer.as_slice(),
            (1280, 720, 1),
        );
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.gpu_context
            .dispatch_pipeline("draw", &mut encoder, &[0; 0]);

        self.gpu_context.image_buffer_to_screen(&mut encoder);

        self.ui.text("fps", &(1f32 / dt).to_string());

        self.ui.draw(
            &self.gpu_context,
            &mut encoder,
            window,
            input.mouse_state.mouse_location,
            input.mouse_state.get_left_button() == ButtonState::Pressed,
        );

        self.gpu_context.execute_encoder(encoder);
        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}

    fn on_tick(&mut self, _dt: f32) -> UpdateResult {
        UpdateResult::Continue
    }
}

fn main() {
    main_loop_run::<HelloWorld>(1280, 720, 10f32);
}
