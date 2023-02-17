use wgpu;
use winit::window::Window;

use std::collections::HashMap;
use std::fmt::Debug;


use wgpu::TextureFormat::Bgra8Unorm;
use wgpu::TextureFormat::Rgba8Unorm;

use log::{info, warn};

use wgpu::{TextureViewDimension};



use crate::shader::Shader;
use crate::CoGr;


use self::auto_encoder::AutoEncoder;
use self::buffer::init_storage_buffer;
use self::compute_pipeline::ComputePipeline;
use self::compute_pipeline::TextureOrBuffer;
use self::texture::init_texture;
use self::to_screen_pipeline::ToScreenPipeline;

pub mod auto_encoder;
mod buffer;
mod compute_pipeline;
mod texture;
mod to_screen_pipeline;

enum GpuResource {
    Buffer(wgpu::Buffer),
    Texture(
        wgpu::TextureView,
        wgpu::TextureFormat,
        wgpu::TextureViewDimension,
        wgpu::Texture,
        (u32, u32, u32),
    ),
    Pipeline(ComputePipeline, Shader),
}

impl Debug for GpuResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buffer(arg0) => f.debug_tuple("Buffer").field(arg0).finish(),
            Self::Texture(arg0, arg1, arg2, arg3, arg4) => f.debug_tuple("Texture").field(arg0).field(arg1).field(arg2).field(arg3).field(arg4).finish(),
            Self::Pipeline(arg0, arg1) => f.debug_tuple("Pipeline").field(arg0).field(arg1).finish(),
        }
    }
}

pub enum Execution {
    PerPixel1D,
    PerPixel2D,
    N3D(u32),
    N1D(u32),
}

#[derive(Debug)]
enum PipelineCreationError {
    PipelineAlreadyExists(String),
    NameAlreadyUsedByTexture(String),
    NameAlreadyUsedByBuffer(String),
    ResourceDoesntExist(String),
    PipelineUsedAsShaderResource(String),
}

pub struct CoGrWGPU {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: (u32, u32),
    to_screen_texture_name: String,
    pub to_screen_pipeline: Option<ToScreenPipeline>,
    resources: HashMap<String, GpuResource>,
    shaders_folder: String,
}

impl CoGr for CoGrWGPU {
    type Encoder<'a> = AutoEncoder<'a>;

    fn new(window: &Window, to_screen_texture_name: &str, shaders_folder: &str) -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);
        let surface = unsafe { instance.create_surface(window) };
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
        info!("supported swapchain surface formats: {:?}", surface.get_supported_formats(&adapter));

        let surface_format = match surface.get_supported_formats(&adapter).contains(&Rgba8Unorm) {
            true => Rgba8Unorm,
            false => match surface.get_supported_formats(&adapter).contains(&Bgra8Unorm) {
                true => Bgra8Unorm,
                false => panic!("neither Rgba8Unorm nor Brga8Unorm is supported"),
            },
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size: (window.inner_size().width, window.inner_size().height),
            to_screen_texture_name: to_screen_texture_name.to_string(),
            to_screen_pipeline: None,
            resources: Default::default(),
            shaders_folder: shaders_folder.to_string(),
        }
    }
    fn get_encoder_for_draw<'a>(&'a mut self) -> AutoEncoder<'a> {
        let surface_texture = self.surface.get_current_texture().expect("can't get new surface texture");

        let texture_view_config = wgpu::TextureViewDescriptor {
            format: Some(self.config.format),
            ..Default::default()
        };

        let surface_texture_view = surface_texture.texture.create_view(&texture_view_config);

        if self.to_screen_pipeline.is_none() {
            self.to_screen_pipeline = Some(ToScreenPipeline::new(
                &self.device,
                self.get_raw_texture(&self.to_screen_texture_name),
                self.config.format,
            ));
        }
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        AutoEncoder {
            encoder: Some(encoder),
            gpu_context: self,
            surface_texture: Some(surface_texture),
            surface_texture_view: Some(surface_texture_view),
        }
    }
    fn get_encoder<'a>(&'a mut self) -> AutoEncoder<'a> {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        AutoEncoder {
            encoder: Some(encoder),
            gpu_context: self,
            surface_texture: None,
            surface_texture_view: None,
        }
    }

    fn buffer<Type>(&mut self, buffer_name: &str, number_of_elements: u32) {
        match self.resources.get(buffer_name) {
            Some(GpuResource::Texture(_, _, _, _, _)) => panic!("{} is not a buffer but a texture", buffer_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a buffer but a pipeline", buffer_name),
            Some(GpuResource::Buffer(_)) => warn!("buffer {} already exists", buffer_name),
            None => {
                self.resources.insert(
                    buffer_name.to_string(),
                    GpuResource::Buffer(init_storage_buffer(self, buffer_name, number_of_elements * std::mem::size_of::<Type>() as u32)),
                );
            }
        }
    }
    fn texture(&mut self, texture_name: &str, number_of_elements: (u32, u32, u32), format: wgpu::TextureFormat) {
        match self.resources.get(texture_name) {
            Some(GpuResource::Buffer(_)) => panic!("{} is not a texture but a buffer", texture_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a texture but a buffer", texture_name),
            Some(GpuResource::Texture(_, _, _, _, _)) => warn!("texture {} already exists", texture_name),
            None => {
                let (texture, texture_view) = init_texture::<()>(self, texture_name, number_of_elements, format, None);

                self.resources.insert(
                    texture_name.to_string(),
                    GpuResource::Texture(
                        texture_view,
                        format,
                        match number_of_elements.2 {
                            0 => TextureViewDimension::D2,
                            1 => TextureViewDimension::D2,
                            _ => TextureViewDimension::D3,
                        },
                        texture,
                        number_of_elements,
                    ),
                );
            }
        }
    }

    fn refresh_pipelines(&mut self) {
        todo!()
    }
}
impl CoGrWGPU {
    fn init_pipeline(&mut self, shader_name: &str) -> Result<(), Vec<PipelineCreationError>> {
        match self.resources.get(shader_name) {
            Some(GpuResource::Texture(_, _, _, _, _)) => {
                return Err(vec![PipelineCreationError::NameAlreadyUsedByTexture(shader_name.to_string())]);
            }
            Some(GpuResource::Buffer(_)) => {
                return Err(vec![PipelineCreationError::NameAlreadyUsedByBuffer(shader_name.to_string())]);
            }
            Some(GpuResource::Pipeline(_, _)) => return Err(vec![PipelineCreationError::PipelineAlreadyExists(shader_name.to_string())]),
            None => (),
        }

        let shader = Shader::get_shader_properties(shader_name, &self.shaders_folder);

        let mut errors = Vec::new();

        let bindings = shader
            .bindings
            .iter()
            .map(|resource| match self.resources.get(resource) {
                Some(GpuResource::Buffer(buffer)) => Ok(TextureOrBuffer::Buffer(buffer, false)),
                Some(GpuResource::Texture(texture, format, dimension, _, _)) => {
                    Ok(TextureOrBuffer::Texture(texture, wgpu::StorageTextureAccess::ReadWrite, *format, *dimension))
                }
                Some(GpuResource::Pipeline(_, _)) => Err(PipelineCreationError::PipelineUsedAsShaderResource(resource.to_string())),
                None => Err(PipelineCreationError::ResourceDoesntExist(resource.to_string())),
            })
            .filter_map(|r| r.map_err(|e| errors.push(e)).ok())
            .collect::<Vec<TextureOrBuffer>>();

        if !errors.is_empty() {
            return Err(errors);
        }

        let push_constant_range = shader.push_constant_info.offset..shader.push_constant_info.offset + shader.push_constant_info.size;

        self.resources.insert(
            shader_name.to_string(),
            GpuResource::Pipeline(
                ComputePipeline::new(self, shader_name, shader.shader.as_slice(), bindings.as_slice(), Some(push_constant_range)),
                shader,
            ),
        );
        Ok(())
    }
    fn get_raw_texture(&self, texture_name: &str) -> &wgpu::TextureView {
        match self.resources.get(texture_name) {
            Some(GpuResource::Buffer(_)) => panic!("{} is not a texture but a buffer", texture_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a texture but a buffer", texture_name),
            Some(GpuResource::Texture(t, _, _, _, _)) => t,
            None => panic!("resource does not exist: {}", texture_name),
        }
    }
    fn get_raw_buffer(&self, buffer_name: &str) -> &wgpu::Buffer {
        match self.resources.get(buffer_name) {
            Some(GpuResource::Texture(_, _, _, _, _)) => panic!("{} is not a buffer but a texture", buffer_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a buffer but a pipeline", buffer_name),
            Some(GpuResource::Buffer(b)) => b,
            None => panic!("resource does not exist: {}", buffer_name),
        }
    }
    pub fn delete_all_pipelines(&mut self) {
        self.resources
            .retain(|_, elem| if let GpuResource::Pipeline(_, _) = elem { false } else { true });
    }
    pub fn log_state(&self) {
        info!("gpu resource state:");
        for (key, val) in &self.resources {
            info!("{}: {:?}", key, val);
        }
    }
}
