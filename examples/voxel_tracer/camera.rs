use std::mem::size_of;

use bytemuck::Pod;
use egui::{Slider, Ui};
use glam::{UVec2, Vec3, Vec2};
use bytemuck::Zeroable;
use cogrrs::{Encoder, ResourceHandle, Pipeline, CoGr, TextureRes, div_ceil, Input};
use wgpu::TextureFormat;
use dolly::{rig::CameraRig, drivers::{YawPitch, Position, Smooth}};

use crate::key_mapping::{MOVE_RIGHT, MOVE_LEFT, MOVE_DOWN, MOVE_BACKWARD, MOVE_FORWARD, MOVE_UP, ENABLE_MOVEMENT};

pub struct Camera {
    camera: CameraRig,
    random_seed: u32,
    primary_ray_data: ResourceHandle,
    camera_data: ResourceHandle,
    generate_rays: Pipeline,
    debug_ray_direction: Pipeline,

    pub aperture: f32,
    pub focal_length: f32,
    pub sensor_height: f32,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
pub struct CameraGpu {
    pub position: Vec3,
    pub aperture: f32,
    pub direction: Vec3,
    pub focal_length: f32,
    pub direction_side: Vec3,
    pub sensor_height: f32,
    pub direction_up: Vec3,
    pub random_seed: u32,
    pub screen_dimensions: UVec2,
    _padding: Vec2
}

pub struct PrimaryRayGenResults {
    pub primary_ray_data: ResourceHandle,
    pub camera_gpu: ResourceHandle,
    pub camera: CameraGpu,
}

impl Camera{
    pub fn new(gpu: &mut CoGr) -> Self {
        let camera: CameraRig = CameraRig::builder()
            .with(YawPitch::new().yaw_degrees(225.0).pitch_degrees(30.0))
            .with(Position::new(Vec3::ZERO))
            .with(Smooth::new_position_rotation(0.5, 0.5))
            .build();
        let primary_ray_data = gpu.texture("primary_ray_direction", TextureRes::FullRes, TextureFormat::Rgba32Float);
        let camera_data = gpu.buffer("camera_data", 1, size_of::<CameraGpu>());
        let generate_rays = gpu.pipeline("examples/voxel_tracer/shaders/generate_rays.glsl").unwrap();
        let debug_ray_direction = gpu.pipeline("examples/voxel_tracer/shaders/ray_direction.glsl").unwrap();
        Self {
            camera,
            random_seed: 1,
            primary_ray_data,
            camera_data,
            generate_rays,
            debug_ray_direction,
            aperture: 1000f32,
            focal_length: 1.7,
            sensor_height: 1.57f32,
        }
    }
    pub fn dispatch(&mut self, encoder: &mut Encoder) -> PrimaryRayGenResults {
        puffin::profile_scope!("Generate rays");

        self.random_seed += 1;
        let camera_data = CameraGpu {
            position: self.camera.final_transform.position,
            aperture: self.aperture,
            direction: self.camera.final_transform.forward(),
            focal_length: self.focal_length,
            direction_side: self.camera.final_transform.right(),
            sensor_height: self.sensor_height,
            direction_up: self.camera.final_transform.up(),
            random_seed: self.random_seed,
            screen_dimensions: UVec2::new(encoder.width(), encoder.height()),
            _padding: Vec2::ZERO
        };
        // upload latest camera data to gpu
        encoder.set_buffer_data(&self.camera_data, [camera_data]).unwrap();
        // use latest camera data to calculate new rays
        encoder
            .dispatch_pipeline(
                &mut self.generate_rays,
                (div_ceil(encoder.width(), 16), div_ceil(encoder.height(), 16), 1),
                &[&self.primary_ray_data, &self.camera_data],
            )
            .unwrap();

        PrimaryRayGenResults {
            primary_ray_data: self.primary_ray_data.clone(),
            camera_gpu: self.camera_data.clone(),
            camera: camera_data
        }
    }

    pub fn update(&mut self, input: &Input, dt: f32) {
        if input.key_pressed(ENABLE_MOVEMENT){

                let move_right = bool_to_f32(input.key_pressed(MOVE_RIGHT)) - bool_to_f32(input.key_pressed(MOVE_LEFT));
                let move_up = bool_to_f32(input.key_pressed(MOVE_UP)) - bool_to_f32(input.key_pressed(MOVE_DOWN));
                let move_forward = bool_to_f32(input.key_pressed(MOVE_FORWARD)) - bool_to_f32(input.key_pressed(MOVE_BACKWARD));
                
                let move_vec = self.camera.final_transform.rotation * Vec3::new(-move_right, move_up, -move_forward).clamp_length_max(1.0);
                
                self.camera
                .driver_mut::<YawPitch>()
                .rotate_yaw_pitch(input.mouse_change()[0], -input.mouse_change()[1]);
            self.camera.driver_mut::<Position>().translate(move_vec * dt * 10.0);
        }
        self.camera.update(dt);
    }
    pub fn draw_ui(&mut self, ui: &mut Ui) {
        ui.add(Slider::new(&mut self.aperture, 0.0..=1.0).text("Aperture"));
        ui.add(Slider::new(&mut self.focal_length, 1.7..=5.0).text("Focal length"));
        ui.add(Slider::new(&mut self.sensor_height, 0.0..=10.0).text("Sensor height"));
    }
    pub fn debug_ray_direction(&mut self, encoder: &mut Encoder, to_screen: &ResourceHandle) {
        encoder
            .dispatch_pipeline(
                &mut self.debug_ray_direction,
                (div_ceil(encoder.width(), 16), div_ceil(encoder.height(), 16), 1),
                &[&self.primary_ray_data, to_screen],
            )
            .unwrap();
    }
}

pub fn bool_to_f32(x: bool) -> f32 {
    x as u8 as f32
}
