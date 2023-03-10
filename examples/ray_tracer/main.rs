use std::fmt::Display;

use crate::bvh::{cross, dot, BVHNode, Ray};
use bvh::{normalize, Bvh, Point};
use bytemuck::{Pod, Zeroable};
use gpu::egui::ComboBox;
use gpu::wgpu::TextureFormat::Rgba8Uint;
use gpu::Execution::PerPixel2D;
use gpu::{CoGr, CoGrEncoder, Renderer, Ui, UI};
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use window::winit::event_loop::EventLoop;
use window::{
    input::Input,
    main_loop::{main_loop_run, Game, RenderResult},
    winit::window::Window,
};

mod bvh;

struct RayTracer {
    pub gpu_context: Renderer,
    pub ui: Ui,
    pub bvh: Bvh,
    pub time: f32,
    pub distance: f32,
    pub screen_buffer: Vec<[u8; 4]>,
    pub frame_number: u32,
    pub render_mode: RenderMode,
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
#[derive(Copy, Clone, PartialEq, Debug)]
enum RenderMode {
    Gpu,
    Cpu,
}

const WIDTH: u32 = 1280;
const HALF_WIDTH: u32 = WIDTH / 2;
const HEIGHT: u32 = 720;
const HALF_HEIGHT: u32 = HEIGHT / 2;

impl Game for RayTracer {
    fn on_init(window: &Window, event_loop: &EventLoop<()>) -> Self {
        let mut gpu_context = Renderer::new(window, "examples/ray_tracer/");

        gpu_context.texture("to_draw_texture", (WIDTH, HEIGHT, 1), gpu_context.config.format);

        let screen_buffer = vec![[0; 4]; (WIDTH * HEIGHT) as usize];

        let mut bvh = Bvh::new("examples/ray_tracer/dragon.obj");
        bvh.build_bvh();

        let ui = Ui::new(&gpu_context, window, event_loop);

        gpu_context.texture("depth", (WIDTH, HEIGHT, 1), Rgba8Uint);
        gpu_context.buffer::<[Point; 4]>("triangles_block", bvh.triangles.len() as u32);
        gpu_context.buffer::<BVHNode>("bvh_nodes_block", bvh.bvh_nodes.len() as u32);

        {
            let mut encoder = gpu_context.get_encoder();
            encoder.set_buffer_data::<[Point; 4]>("triangles_block", bvh.triangles.as_slice());
            encoder.set_buffer_data::<BVHNode>("bvh_nodes_block", bvh.bvh_nodes.as_slice());
        }

        gpu_context.log_state();

        RayTracer {
            gpu_context,
            ui,
            bvh,
            time: 0f32,
            distance: -1f32,
            screen_buffer,
            frame_number: 0,
            render_mode: RenderMode::Gpu,
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

        match self.render_mode {
            RenderMode::Gpu => self.render_gpu(&camera_data),
            RenderMode::Cpu => self.render_cpu(&camera_data),
        }
        {
            let mut encoder = self.gpu_context.get_encoder_for_draw();
            encoder.to_screen("to_draw_texture");
            self.ui.draw(&mut encoder, window, |ui| {
                ui.label(format!("ms: {}", dt * 1000f32));
                ComboBox::from_label("Render mode")
                    .selected_text(format!("{:?}", self.render_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.render_mode, RenderMode::Gpu, "GPU");
                        ui.selectable_value(&mut self.render_mode, RenderMode::Cpu, "CPU");
                    });
            });
        }

        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}
    fn on_tick(&mut self, _dt: f32) {}
    fn on_window_event(&mut self, event: &window::winit::event::WindowEvent) {
        self.ui.handle_window_event(event);
    }
}

impl RayTracer {
    fn render_cpu(&mut self, camera_data: &CameraData) {
        self.bvh.trace_rays(camera_data, &mut self.screen_buffer);
        let mut encoder = self.gpu_context.get_encoder();
        encoder.set_texture_data("depth", self.screen_buffer.as_slice());
        encoder.dispatch_pipeline("draw", PerPixel2D, &[0; 0]);
    }
    fn render_gpu(&mut self, camera_data: &CameraData) {
        let mut encoder = self.gpu_context.get_encoder();
        encoder.dispatch_pipeline("trace", PerPixel2D, camera_data);
    }
}

fn main() {
    main_loop_run::<RayTracer>(WIDTH, HEIGHT, 10f32);
}
