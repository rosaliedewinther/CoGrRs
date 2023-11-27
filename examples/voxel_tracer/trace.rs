use std::mem::size_of;

use bytemuck::{Pod, cast_mut};
use egui::{Slider, Ui, color_picker::color_edit_button_rgb};
use glam::{UVec2, Vec3, vec3, uvec2};
use bytemuck::Zeroable;
use cogrrs::{Encoder, ResourceHandle, Pipeline, CoGr, TextureRes, div_ceil};
use wgpu::TextureFormat;

use crate::camera::PrimaryRayGenResults;

pub struct Trace {
    trace_result: ResourceHandle,
    trace_data: ResourceHandle,
    trace_rays: Pipeline,

    pub skylight: Vec3,
    pub coeiff: f32,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct TraceGpu {
    skylight: Vec3,
    coeiff: f32,
    camera_pos: Vec3,
    time: f32,
    screen_dimensions: UVec2,
    _padding1: f32,
    _padding2: f32
}

pub struct TraceResults {
    pub trace_result: ResourceHandle,
}

impl Trace{
    pub fn new(gpu: &mut CoGr) -> Self {
        let trace_result = gpu.texture("trace_result", TextureRes::FullRes, TextureFormat::Rgba16Float);
        let trace_data = gpu.buffer("trace_data", 1, size_of::<TraceGpu>());
        let trace_rays = gpu.pipeline("examples/voxel_tracer/shaders/trace2.glsl").unwrap();
        Self {
            trace_result,
            trace_data,
            trace_rays,
            skylight: vec3(0.3, 0.5, 1.0),
            coeiff: 0.25,
        }
    }
    pub fn dispatch(&mut self, encoder: &mut Encoder, time: f32, ray_gen: &PrimaryRayGenResults, to_screen: &ResourceHandle, camera_position: Vec3) -> TraceResults {
        puffin::profile_scope!("Trace rays");

        let trace_data = TraceGpu {
            skylight: self.skylight,
            coeiff: self.coeiff,
            camera_pos: camera_position,
            time,
            screen_dimensions: uvec2(encoder.width(), encoder.height()),
            _padding1: 0.0,
            _padding2: 0.0
        };
        // upload latest camera data to gpu
        encoder.set_buffer_data(&self.trace_data, [trace_data]).unwrap();
        // use latest camera data to calculate new rays
        encoder
            .dispatch_pipeline(
                &mut self.trace_rays,
                (div_ceil(encoder.width(), 16), div_ceil(encoder.height(), 16), 1),
                &[&ray_gen.primary_ray_data, to_screen, &self.trace_data],
            )
            .unwrap();

            TraceResults {
                trace_result: self.trace_result.clone(),
        }
    }
    pub fn draw_ui(&mut self, ui: &mut Ui) {
        color_edit_button_rgb(ui, cast_mut(&mut self.skylight));
        ui.add(Slider::new(&mut self.coeiff, 0.0..=1.0).text("coeiff"));
    }
}