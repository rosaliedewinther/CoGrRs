use anyhow::anyhow;
use anyhow::Result;
use egui_winit::State;
use resources::BufferHandle;
use resources::BufferSize;
use resources::ResourcePool;
use resources::TextureHandle;
use resources::TextureRes;
use wgpu::Backends;
use wgpu::Buffer;
use wgpu::InstanceDescriptor;
use wgpu::TextureFormat;
use wgpu::{Texture, TextureView};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub use egui;
use std::fmt::Debug;
use std::sync::Arc;
pub use wgpu;

use wgpu::TextureFormat::{Bgra8Unorm, Rgba8Unorm};

use log::info;

use self::compute_pipeline::ComputePipeline;
use self::encoder::Encoder;
use self::encoder::EncoderType;
use self::to_screen_pipeline::ToScreenPipeline;

pub mod compute_pipeline;
pub(crate) mod encoder;
pub mod resources;
pub mod shader;
mod to_screen_pipeline;

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

pub struct CoGr {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,

    resource_pool: ResourcePool,

    last_to_screen_texture_handle: Option<TextureHandle>,
    last_to_screen_pipeline: Option<ToScreenPipeline>,

    // ui
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    state: State,
}

impl CoGr {
    pub fn new(window: &Arc<Window>, event_loop: &EventLoop<()>) -> Result<Self> {
        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            ..Default::default()
        });
        let surface = unsafe { instance.create_surface(window.as_ref())? };
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
                features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                    | wgpu::Features::PUSH_CONSTANTS,
                limits,
                label: None,
            },
            None, // Trace path
        ))?;
        let formats = surface.get_capabilities(&adapter).formats;
        info!("supported swapchain surface formats: {:?}", formats);
        let surface_format = match (formats.contains(&Rgba8Unorm), formats.contains(&Bgra8Unorm)) {
            (true, _) => Rgba8Unorm,
            (_, true) => Bgra8Unorm,
            _ => Err(anyhow!("neither Rgba8Unorm nor Bgra8Unorm is supported"))?,
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

        let renderer = egui_wgpu::renderer::Renderer::new(&device, config.format, None, 1);
        let context = egui::Context::default();
        let state = egui_winit::State::new(event_loop);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            window: window.clone(),
            resource_pool: ResourcePool::new(),

            renderer,
            context,
            state,
            last_to_screen_texture_handle: None,
            last_to_screen_pipeline: None,
        })
    }
    pub fn get_encoder_for_draw(&mut self) -> Result<Encoder> {
        self.resource_pool
            .prepare_resources(&self.device, &self.config);
        let surface_texture = self.surface.get_current_texture()?;

        let texture_view_config = wgpu::TextureViewDescriptor {
            format: Some(self.config.format),
            ..Default::default()
        };

        let surface_texture_view = surface_texture.texture.create_view(&texture_view_config);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        encoder.push_debug_group("user_encoder_for_draw");
        Ok(Encoder {
            encoder: Some(encoder),
            gpu_context: self,
            encoder_type: EncoderType::Draw(Some(surface_texture), surface_texture_view),
        })
    }
    pub fn get_encoder(&mut self) -> Result<Encoder> {
        self.resource_pool
            .prepare_resources(&self.device, &self.config);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        encoder.push_debug_group("user_encoder");
        Ok(Encoder {
            encoder: Some(encoder),
            gpu_context: self,
            encoder_type: EncoderType::NonDraw,
        })
    }
    pub fn buffer<S: Into<BufferSize>>(
        &mut self,
        name: &str,
        elements: S,
        element_size: u32,
    ) -> BufferHandle {
        let elements = elements.into();
        self.resource_pool
            .buffer(name.to_string(), elements, element_size)
    }
    pub fn texture(
        &mut self,
        name: &str,
        elements: TextureRes,
        format: wgpu::TextureFormat,
    ) -> TextureHandle {
        self.resource_pool
            .texture(name.to_string(), elements, format)
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        let _ = self.state.on_event(&self.context, event);
    }
}
impl CoGr {
    pub fn init_pipeline(&mut self, shader_file: &str) -> Result<ComputePipeline> {
        Ok(ComputePipeline::new(self, shader_file))
    }
    pub fn log_state(&self) {
        println!("Textures:");
        for handle in &self.resource_pool.texture_handles {
            println!(
                "{:?}: {:#?}",
                handle,
                self.resource_pool.grab_texture(handle)
            );
        }
        println!("Buffers:");
        for handle in &self.resource_pool.buffer_handles {
            println!(
                "{:?}: {:#?}",
                handle,
                self.resource_pool.grab_buffer(handle)
            );
        }
    }
}
