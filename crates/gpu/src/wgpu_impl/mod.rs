use crate::wgpu_impl::compute_pipeline::TextureOrBuffer;
use anyhow::anyhow;
use anyhow::Result;
use wgpu::Backends;
use wgpu::Buffer;
use wgpu::InstanceDescriptor;
use wgpu::TextureFormat;
use wgpu::{Texture, TextureView};
use winit::window::Window;

use core::panic;
use std::collections::HashMap;
use std::fmt::Debug;

use wgpu::TextureFormat::{Bgra8Unorm, Rgba8Unorm};

use log::info;

use crate::shader::Shader;
use crate::CoGr;

use self::buffer::init_storage_buffer;
use self::compute_pipeline::ComputePipeline;
use self::encoder::EncoderWGPU;
use self::texture::init_texture;
use self::to_screen_pipeline::ToScreenPipeline;

mod buffer;
mod compute_pipeline;
pub(crate) mod encoder;
pub(crate) mod read_handle;
mod texture;
mod to_screen_pipeline;
pub(crate) mod ui;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct BufferDescriptor {
    name: &'static str,
    number_of_elements: u32,
    type_name: &'static str,
    buffer: Buffer,
}
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct TextureDescriptor {
    name: &'static str,
    size: (u32, u32, u32),
    format: TextureFormat,
    texture: Texture,
    texture_view: TextureView,
}
#[allow(dead_code)]
#[derive(Debug)]
struct PipelineDescriptor {
    name: &'static str,
    pipeline: ComputePipeline,
    workgroup_size: (u32, u32, u32),
}
#[allow(dead_code)]
#[derive(Debug)]
struct ToScreenPipelineDescriptor {
    texture_name: &'static str,
    pipeline: ToScreenPipeline,
}

#[derive(Debug)]
enum GpuResource {
    Buffer(BufferDescriptor),
    Texture(TextureDescriptor),
    Pipeline(PipelineDescriptor),
    ToScreenPipeline(ToScreenPipelineDescriptor),
}

pub struct CoGrWGPU {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    resources: HashMap<String, GpuResource>,
    shaders_folder: String,
}

impl CoGr for CoGrWGPU {
    type Encoder<'a> = EncoderWGPU<'a>;

    fn new(window: &Window, shaders_folder: &str) -> Self {
        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            ..Default::default()
        });
        let surface = unsafe { instance.create_surface(window).unwrap() };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("can't initialize gpu adapter");
        let limits = wgpu::Limits {
            max_push_constant_size: 128,
            max_storage_buffers_per_shader_stage: 32,
            max_storage_buffer_binding_size: 1073741824,
            max_storage_textures_per_shader_stage: 16,
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | wgpu::Features::SPIRV_SHADER_PASSTHROUGH | wgpu::Features::PUSH_CONSTANTS,
                limits,
                label: None,
            },
            None, // Trace path
        ))
        .expect("can't create device or command queue");
        let formats = surface.get_capabilities(&adapter).formats;
        info!("supported swapchain surface formats: {:?}", formats);
        let surface_format = match (formats.contains(&Rgba8Unorm), formats.contains(&Bgra8Unorm)) {
            (true, _) => Rgba8Unorm,
            (_, true) => Bgra8Unorm,
            _ => panic!("neither Rgba8Unorm nor Bgra8Unorm is supported"),
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![surface_format],
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            resources: Default::default(),
            shaders_folder: shaders_folder.to_string(),
        }
    }
    fn get_encoder_for_draw(&mut self) -> EncoderWGPU {
        let surface_texture = self.surface.get_current_texture().expect("can't get new surface texture");

        let texture_view_config = wgpu::TextureViewDescriptor {
            format: Some(self.config.format),
            ..Default::default()
        };

        let surface_texture_view = surface_texture.texture.create_view(&texture_view_config);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        encoder.push_debug_group("user_encoder_for_draw");
        EncoderWGPU {
            encoder: Some(encoder),
            gpu_context: self,
            surface_texture: Some(surface_texture),
            surface_texture_view: Some(surface_texture_view),
        }
    }
    fn get_encoder(&mut self) -> EncoderWGPU {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        encoder.push_debug_group("user_encoder");
        EncoderWGPU {
            encoder: Some(encoder),
            gpu_context: self,
            surface_texture: None,
            surface_texture_view: None,
        }
    }
    fn buffer<T>(&mut self, buffer_name: &'static str, number_of_elements: u32) {
        match self.resources.get(buffer_name) {
            Some(GpuResource::Buffer(_)) | None => {
                self.resources.insert(
                    buffer_name.to_string(),
                    GpuResource::Buffer(BufferDescriptor {
                        name: buffer_name,
                        number_of_elements,
                        type_name: std::any::type_name::<T>(),
                        buffer: init_storage_buffer(self, buffer_name, number_of_elements * std::mem::size_of::<T>() as u32),
                    }),
                );
            }
            val => {
                panic!("{} is not a buffer but contains: {:?}", buffer_name, val);
            }
        }
    }
    fn texture(&mut self, texture_name: &'static str, number_of_elements: (u32, u32, u32), format: wgpu::TextureFormat) {
        match self.resources.get(texture_name) {
            Some(GpuResource::Texture(_)) | None => {
                let (texture, texture_view) = init_texture::<()>(self, texture_name, number_of_elements, format, None);
                self.resources.insert(
                    texture_name.to_string(),
                    GpuResource::Texture(TextureDescriptor {
                        name: texture_name,
                        size: number_of_elements,
                        format,
                        texture,
                        texture_view,
                    }),
                );
            }
            val => {
                panic!("{} is not a texture but contains: {:?}", texture_name, val);
            }
        }
    }
}
impl CoGrWGPU {
    fn init_pipeline(&mut self, shader_name: &'static str) -> Result<()> {
        match self.resources.get(shader_name) {
            None => (),
            val => return Err(anyhow!("{} already exists and contains: {:?}", shader_name, val)),
        }

        let shader = Shader::get_shader_properties(shader_name, &self.shaders_folder);

        let mut errors = Vec::new();

        let bindings = shader
            .bindings
            .iter()
            .map(|resource| match self.resources.get(resource) {
                Some(GpuResource::Buffer(desc)) => Ok(TextureOrBuffer::Buffer(desc)),
                Some(GpuResource::Texture(desc)) => Ok(TextureOrBuffer::Texture(desc)),
                val => Err(anyhow!("{} is not a buffer or texture but contains: {:?}", resource, val)),
            })
            .filter_map(|r| r.map_err(|e| errors.push(e)).ok())
            .collect::<Vec<TextureOrBuffer>>();

        if !errors.is_empty() {
            return Err(anyhow!("{:?}", errors));
        }

        let push_constant_range = shader.push_constant_info.offset..shader.push_constant_info.offset + shader.push_constant_info.size;

        self.resources.insert(
            shader_name.to_string(),
            GpuResource::Pipeline(PipelineDescriptor {
                name: shader_name,
                workgroup_size: (shader.cg_x, shader.cg_y, shader.cg_z),
                pipeline: ComputePipeline::new(self, shader_name, shader.shader.as_slice(), bindings.as_slice(), Some(push_constant_range)),
            }),
        );
        Ok(())
    }
    fn get_raw_texture(&self, texture_name: &str) -> &wgpu::TextureView {
        match self.resources.get(texture_name) {
            Some(GpuResource::Texture(desc)) => &desc.texture_view,
            val => panic!("{} is not a texture but contained: {:?}", texture_name, val),
        }
    }
    pub fn log_state(&self) {
        println!("gpu resource state:");
        for (key, val) in &self.resources {
            println!("{}: {:#?}", key, val);
        }
    }
}
