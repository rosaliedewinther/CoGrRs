use std::f32::consts::PI;

use crate::bvh::{cross, BVHNode};
use anyhow::Result;
use bvh::{normalize, Bvh, Point};
use bytemuck::{Pod, Zeroable};
use cogrrs::{egui, main_loop_run, CoGr, Game, Input, Pipeline, ResourceHandle, TextureRes};

mod bvh;

struct RayTracer {
    pub time: f32,
    pub distance: f32,
    to_draw: ResourceHandle,
    triangles: ResourceHandle,
    bvh_nodes: ResourceHandle,
    trace_pipeline: Pipeline,
    timings: [f32; 1000],
    timings_ptr: usize,
    saved_timing: f32,
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
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        let mut bvh = Bvh::new("examples/ray_tracer/dragon.obj");
        bvh.build_bvh();

        let to_draw = gpu.texture("to_draw_texture", TextureRes::FullRes, gpu.config.format);
        let triangles = gpu.buffer(
            "triangles",
            bvh.triangles.len(),
            std::mem::size_of::<[Point; 4]>() as u32,
        );
        let bvh_nodes = gpu.buffer(
            "bvh_nodes",
            bvh.bvh_nodes.len(),
            std::mem::size_of::<BVHNode>() as u32,
        );

        {
            let mut encoder = gpu.get_encoder()?;
            encoder.set_buffer_data(&triangles, bvh.triangles)?;
            encoder.set_buffer_data(&bvh_nodes, bvh.bvh_nodes)?;
        }

        let trace_pipeline = gpu.pipeline("examples/ray_tracer/trace.hlsl")?;

        Ok(RayTracer {
            time: 0f32,
            distance: -1f32,
            to_draw,
            triangles,
            bvh_nodes,
            trace_pipeline,
            timings: [0f32; 1000],
            timings_ptr: 0,
            saved_timing: 0f32,
        })
    }

    fn on_render(&mut self, gpu: &mut CoGr, input: &mut Input, dt: f32) -> Result<()> {
        self.time += 0.001 * PI;
        if self.timings_ptr < self.timings.len() {
            self.timings[self.timings_ptr] = dt;
            self.timings_ptr += 1;
        } else {
            self.saved_timing = self.timings.iter().sum::<f32>() / self.timings.len() as f32;
            self.timings_ptr = 0;
        }
        self.distance += input.mouse_state.scroll_delta;

        let ray_origin = Point::new(
            self.time.sin() * self.distance,
            0f32,
            self.time.cos() * self.distance,
        );
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

        let mut encoder = gpu.get_encoder_for_draw()?;
        encoder.dispatch_pipeline(
            &mut self.trace_pipeline,
            (WIDTH / 32, HEIGHT / 32, 1),
            &camera_data,
            &[&self.to_draw, &self.triangles, &self.bvh_nodes],
        )?;

        encoder.to_screen(&self.to_draw)?;
        encoder.draw_ui(|ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label(format!("ms: {}", self.saved_timing * 1000f32));
            });
        })?;

        Ok(())
    }

    fn on_tick(&mut self, _gpu: &mut CoGr, _dt: f32) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    main_loop_run::<RayTracer>(WIDTH, HEIGHT, 10f32)?;
    Ok(())
}
