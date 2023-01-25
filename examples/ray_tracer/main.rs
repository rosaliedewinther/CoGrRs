use crate::bvh::{cross, dot, BVHNode, Ray};
use bvh::{normalize, Bvh, Point};
use bytemuck::{Pod, Zeroable};
use gpu::wgpu::TextureFormat::Rgba8Uint;
use gpu::Context;
use gpu::Execution::PerPixel2D;
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ui::MainGui;
use window::{
    input::{button::ButtonState, Input},
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::window::Window,
};

mod bvh;

pub struct RayTracer {
    pub gpu_context: Context,
    pub ui: MainGui,
    pub bvh: Bvh,
    pub time: f32,
    pub distance: f32,
    pub screen_buffer: Vec<[u8; 4]>,
    pub frame_number: u32,
    pub render_on_gpu: bool,
}

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
struct CameraData {
    pub dir: Point,
    pub pos: Point,
    pub side: Point,
    pub up: Point,
    pub width: f32,
    pub half_width: f32,
    pub height: f32,
    pub half_height: f32,
    pub time: f32,
    padding1: u32,
    padding2: u32,
    padding3: u32,
}

const WIDTH: u32 = 1280;
const HALF_WIDTH: u32 = WIDTH / 2;
const HEIGHT: u32 = 720;
const HALF_HEIGHT: u32 = HEIGHT / 2;

impl Game for RayTracer {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = Context::new(window, "to_draw_texture", "examples/ray_tracer/");

        gpu_context.texture("to_draw_texture", (WIDTH, HEIGHT, 1), gpu_context.config.format);

        let screen_buffer = vec![[0; 4]; (WIDTH * HEIGHT) as usize];

        let mut bvh = Bvh::new("examples/ray_tracer/dragon.obj");
        bvh.build_bvh();

        let ui = MainGui::new(&gpu_context, window);

        gpu_context.texture("depth", (WIDTH, HEIGHT, 1), Rgba8Uint);
        gpu_context.buffer::<[Point; 4]>("triangles_block", bvh.triangles.len() as u32);
        gpu_context.buffer::<BVHNode>("bvh_nodes_block", bvh.bvh_nodes.len() as u32);

        gpu_context.set_buffer_data::<[Point; 4]>("triangles_block", bvh.triangles.as_slice(), bvh.triangles.len(), 0);
        gpu_context.set_buffer_data::<BVHNode>("bvh_nodes_block", bvh.bvh_nodes.as_slice(), bvh.bvh_nodes.len(), 0);

        RayTracer {
            gpu_context,
            ui,
            bvh,
            time: 0f32,
            distance: -1f32,
            screen_buffer,
            frame_number: 0,
            render_on_gpu: true,
        }
    }

    fn on_render(&mut self, input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        self.time += dt;

        self.frame_number += 1;

        self.distance += input.mouse_state.scroll_delta;
        let ray_origin = Point::new(self.time.sin() * self.distance, 0f32, self.time.cos() * self.distance);
        let ray_direction = normalize(Point::new(-ray_origin.pos[0], 0f32, -ray_origin.pos[2]));
        let ray_side = cross(ray_direction, normalize(Point::new(0f32, 1f32, 0f32)));
        let ray_up = cross(ray_direction, ray_side);

        let camera_data = CameraData {
            dir: ray_direction,
            pos: ray_origin,
            side: ray_side,
            up: ray_up,
            width: WIDTH as f32,
            half_width: HALF_WIDTH as f32,
            height: HEIGHT as f32,
            half_height: HALF_HEIGHT as f32,
            time: self.time,
            padding1: 0,
            padding2: 0,
            padding3: 0,
        };

        if self.render_on_gpu {
            self.render_gpu(&camera_data);
        } else {
            self.render_cpu(&camera_data)
        }

        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.gpu_context.image_buffer_to_screen(&mut encoder);

        self.ui.text("fps", &(1f32 / dt).to_string());

        self.render_on_gpu = self.ui.combobox("gpu_rendering", vec!["GPU", "CPU"]) == "GPU";

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

impl RayTracer {
    fn render_cpu(&mut self, camera_data: &CameraData) {
        (0..HEIGHT * WIDTH)
            .into_par_iter()
            .map(|index| {
                let x = index % WIDTH;
                let x = x as f32;
                let y = index / WIDTH;
                let y = y as f32;

                let screen_point = camera_data.pos
                    + camera_data.dir
                    + camera_data.side * (x - camera_data.half_width) / (camera_data.width / (camera_data.width / camera_data.height))
                    + camera_data.up * (y - camera_data.half_height) / camera_data.height;

                let ray_direction = normalize(screen_point - camera_data.pos);
                let ray_r_direction = Point::new(1f32 / ray_direction.pos[0], 1f32 / ray_direction.pos[1], 1f32 / ray_direction.pos[2]);
                let mut ray = Ray {
                    o: camera_data.pos,
                    d: ray_direction,
                    d_r: ray_r_direction,
                    t: f32::MAX,
                    prim: u32::MAX,
                    _padding1: 0,
                    _padding2: 0,
                };

                self.bvh.fast_intersect(&mut ray);

                if ray.t < 10000000f32 {
                    let normal = self.bvh.triangle_normal(ray.prim);
                    let intensity = (dot(normal, normalize(Point::new(1f32, -1f32, 1f32))) + 1f32) / 10f32;

                    [(intensity * 255f32) as u8, (intensity * 255f32) as u8, (intensity * 255f32) as u8, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect_into_vec(&mut self.screen_buffer);

        self.gpu_context.set_texture_data("depth", self.screen_buffer.as_slice());
        let mut encoder = self.gpu_context.get_encoder();

        self.gpu_context.dispatch_pipeline("draw", PerPixel2D, &mut encoder, &[0; 0]);
        self.gpu_context.execute_encoder(encoder);
    }
    fn render_gpu(&mut self, camera_data: &CameraData) {
        let mut encoder = self.gpu_context.get_encoder();
        self.gpu_context.dispatch_pipeline("trace", PerPixel2D, &mut encoder, camera_data);
        self.gpu_context.execute_encoder(encoder);
    }
}

fn main() {
    main_loop_run::<RayTracer>(WIDTH, HEIGHT, 10f32);
}
