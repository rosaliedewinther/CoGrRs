use std::sync::Arc;

use crate::bvh::{cross, BVHNode};
use bvh::{normalize, Bvh, Point};
use bytemuck::{Pod, Zeroable};
use gpu::wgpu::TextureFormat::Rgba8Uint;
use gpu::Execution::PerPixel2D;
use gpu::{egui, CoGr, CoGrEncoder, Renderer};
use window::winit::event::WindowEvent;
use window::winit::event_loop::EventLoop;
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult},
    winit::window::Window,
};

mod bvh;

struct RayTracer {
    pub gpu_context: Renderer,
    pub time: f32,
    pub distance: f32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct CameraData {
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
    fn on_init(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Self {
        let mut gpu_context = Renderer::new(window, "examples/ray_tracer/", event_loop);

        let mut bvh = Bvh::new("examples/ray_tracer/dragon.obj");
        bvh.build_bvh();

        gpu_context.texture("to_draw_texture", (WIDTH, HEIGHT, 1), gpu_context.config.format);
        gpu_context.texture("depth", (WIDTH, HEIGHT, 1), Rgba8Uint);
        gpu_context.buffer::<[Point; 4]>("triangles", bvh.triangles.len() as u32);
        gpu_context.buffer::<BVHNode>("bvh_nodes", bvh.bvh_nodes.len() as u32);

        {
            let mut encoder = gpu_context.get_encoder();
            encoder.set_buffer_data::<[Point; 4]>("triangles", bvh.triangles.as_slice());
            encoder.set_buffer_data::<BVHNode>("bvh_nodes", bvh.bvh_nodes.as_slice());
        }

        RayTracer {
            gpu_context,
            time: 0f32,
            distance: -1f32,
        }
    }

    fn on_render(&mut self, input: &mut Input, dt: f32) -> RenderResult {
        self.time += dt;
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

        let mut encoder = self.gpu_context.get_encoder_for_draw();
        encoder.dispatch_pipeline("trace", PerPixel2D, &camera_data);
        encoder.to_screen("to_draw_texture");
        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("ms: {}", dt * 1000f32));
            });
        });

        RenderResult::Continue
    }

    fn on_tick(&mut self, _dt: f32) {}
    fn on_window_event(&mut self, event: &WindowEvent) {
        self.gpu_context.handle_window_event(event);
    }
}

fn main() {
    main_loop_run::<RayTracer>(WIDTH, HEIGHT, 10f32);
}
