use std::{f32::consts::PI, mem::size_of};

use bvh::{Bvh, BVHNode};
use cogrrs::{
    anyhow::Result, div_ceil, egui, main_loop_run, CoGr, Game, Input, Pipeline, ResourceHandle, TextureRes, glam::Vec3, glam::vec3, bytemuck::Zeroable, bytemuck::Pod, TextureFormat
};

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
    pub dir: Vec3,
    pub pos: Vec3,
    pub side: Vec3,
    pub up: Vec3,
    pub width: f32,
    pub half_width: f32,
    pub height: f32,
    pub half_height: f32,
    pub time: f32,
    padding1: u32,
    padding2: u32,
    padding3: u32,
}

impl Game for RayTracer {
    fn on_init(gpu: &mut CoGr) -> Result<Self> {
        let mut bvh = Bvh::new("examples/ray_tracer/dragon.obj");
        bvh.build_bvh();

        let to_draw = gpu.texture("to_draw_texture", TextureRes::FullRes, gpu.config.format);
        let triangles = gpu.buffer("triangles", bvh.triangles.len(), size_of::<[Vec3; 4]>());
        let bvh_nodes = gpu.buffer("bvh_nodes", bvh.bvh_nodes.len(), size_of::<BVHNode>());

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

    fn on_render(&mut self, gpu: &mut CoGr, input: &Input, dt: f32) -> Result<()> {
        self.time += 0.001 * PI;
        let width = gpu.config.width;
        let height = gpu.config.height;
        if self.timings_ptr < self.timings.len() {
            self.timings[self.timings_ptr] = dt;
            self.timings_ptr += 1;
        } else {
            self.saved_timing = self.timings.iter().sum::<f32>() / self.timings.len() as f32;
            self.timings_ptr = 0;
        }
        self.distance += input.mouse_state.scroll_delta;

        let ray_origin = vec3(
            self.time.sin() * self.distance,
            0f32,
            self.time.cos() * self.distance,
        );
        let ray_direction = vec3(-ray_origin.x, 0f32, -ray_origin.z).normalize();
        let ray_side = ray_direction.cross(vec3(0f32, 1f32, 0f32).normalize());
        let ray_up = ray_direction.cross(ray_side);

        let camera_data = CameraData {
            dir: ray_direction,
            pos: ray_origin,
            side: ray_side,
            up: ray_up,
            width: width as f32,
            half_width: width as f32 / 2.0,
            height: height as f32,
            half_height: height as f32 / 2.0,
            time: self.time,
            padding1: 0,
            padding2: 0,
            padding3: 0,
        };

        let mut encoder = gpu.get_encoder_for_draw()?;
        encoder.dispatch_pipeline(
            &mut self.trace_pipeline,
            (div_ceil(width, 32), div_ceil(height, 32), 1),
            &camera_data,
            &[&self.to_draw, &self.triangles, &self.bvh_nodes],
        )?;

        encoder.to_screen(&self.to_draw, TextureFormat::Rgba32Float)?;
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
    main_loop_run::<RayTracer>(10f32)?;
    Ok(())
}
