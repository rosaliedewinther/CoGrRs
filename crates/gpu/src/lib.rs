use shader::Shader;
pub use wgpu;

mod buffer;
mod compute_pipeline;
mod shader;
mod texture;
mod to_screen_pipeline;

use crate::compute_pipeline::ComputePipeline;
use std::{cmp::max, collections::HashMap, num::NonZeroU32};

use bytemuck::{Pod, Zeroable};
use wgpu::TextureFormat::Bgra8Unorm;
use wgpu::TextureFormat::Rgba8Unorm;

use log::info;

use wgpu::{
    util::DeviceExt, CommandEncoder, Extent3d, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout,
    TextureViewDimension,
};

use wgpu::IndexFormat::Uint16;

use crate::{
    buffer::init_storage_buffer, compute_pipeline::TextureOrBuffer, texture::init_texture,
    to_screen_pipeline::ToScreenPipeline,
};

#[derive(Debug)]
enum GpuResource {
    Buffer(wgpu::Buffer),
    Texture(
        wgpu::TextureView,
        wgpu::TextureFormat,
        wgpu::TextureViewDimension,
        wgpu::Texture,
    ),
    Pipeline(ComputePipeline),
}
pub enum Execution {
    PerPixel1D,
    PerPixel2D,
    N3D(u32),
    N1D(u32),
}

pub struct Context {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface_texture: Option<wgpu::SurfaceTexture>,
    pub surface_texture_view: Option<wgpu::TextureView>,
    to_screen_texture_name: String,
    pub to_screen_pipeline: Option<ToScreenPipeline>,
    resources: HashMap<String, GpuResource>,
    shaders_folder: String,
}

impl Context {
    pub fn new(
        window: &winit::window::Window,
        to_screen_texture_name: &str,
        shaders_folder: &str,
    ) -> Context {
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
                features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                    | wgpu::Features::PUSH_CONSTANTS,
                limits,
                label: None,
            },
            None, // Trace path
        ))
        .expect("can't create device or command queue");
        info!(
            "supported swapchain surface formats: {:?}",
            surface.get_supported_formats(&adapter)
        );

        let surface_format = match surface
            .get_supported_formats(&adapter)
            .contains(&Rgba8Unorm)
        {
            true => Rgba8Unorm,
            false => match surface
                .get_supported_formats(&adapter)
                .contains(&Bgra8Unorm)
            {
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
        let size = winit::dpi::PhysicalSize {
            width: window.inner_size().width,
            height: window.inner_size().height,
        };

        Context {
            surface,
            device,
            queue,
            config,
            size,
            surface_texture: None,
            surface_texture_view: None,
            to_screen_texture_name: to_screen_texture_name.to_string(),
            to_screen_pipeline: None,
            resources: Default::default(),
            shaders_folder: shaders_folder.to_string(),
        }
    }
    pub fn get_encoder_for_draw(&mut self) -> wgpu::CommandEncoder {
        self.surface_texture = Some(
            self.surface
                .get_current_texture()
                .expect("can't get new surface texture"),
        );

        let mut texture_view_config = wgpu::TextureViewDescriptor::default();
        texture_view_config.format = Some(self.config.format);

        self.surface_texture_view = Some(
            self.surface_texture
                .as_ref()
                .expect("surface texture is not stored properly")
                .texture
                .create_view(&texture_view_config),
        );

        if self.to_screen_pipeline.is_none() {
            self.to_screen_pipeline = Some(ToScreenPipeline::new(
                &self.device,
                &self.get_raw_texture(&self.to_screen_texture_name),
                self.config.format,
            ));
        }
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }
    pub fn get_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }

    pub fn image_buffer_to_screen(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self
                    .surface_texture_view
                    .as_ref()
                    .expect("there is no surface texture"),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        match &self.to_screen_pipeline {
            Some(pipeline) => {
                render_pass.set_pipeline(&pipeline.pipeline); // 2.
                render_pass.set_bind_group(0, &pipeline.bindgroup, &[]);
                render_pass.set_index_buffer(pipeline.index_buffer.slice(..), Uint16);
                render_pass.draw_indexed(0..pipeline.num_indices, 0, 0..1);
            }
            None => panic!("to_screen_pipeline is not available"),
        }
    }

    pub fn execute_encoder(&mut self, encoder: wgpu::CommandEncoder) {
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        if self.surface_texture.is_some() {
            let surface = std::mem::replace(&mut self.surface_texture, None);
            surface.expect("unable to present surface").present();
        }
    }
    pub fn dispatch_pipeline<PushConstants>(
        &self,
        pipeline_name: &str,
        encoder: &mut CommandEncoder,
        push_constants: &PushConstants,
    ) where
        PushConstants: bytemuck::Pod,
    {
        let pipeline = self
            .resources
            .get(pipeline_name)
            .unwrap_or_else(|| panic!("resource does not exist: {}", pipeline_name));
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(pipeline_name),
            });

            match pipeline {
                GpuResource::Buffer(_) => {
                    panic!("{} is not a compute pipeline but a buffer", pipeline_name)
                }
                GpuResource::Texture(_, _, _, _) => {
                    panic!("{} is not a compute pipeline but a texture", pipeline_name)
                }
                GpuResource::Pipeline(pipeline) => {
                    cpass.insert_debug_marker(pipeline_name);
                    cpass.set_pipeline(&pipeline.pipeline);
                    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
                    cpass.set_push_constants(0, bytemuck::bytes_of(push_constants));
                    cpass.dispatch_workgroups(
                        pipeline.work_group_dims.0,
                        pipeline.work_group_dims.1,
                        pipeline.work_group_dims.2,
                    );
                }
            }
        }
    }

    pub fn pipeline<const M: usize>(
        &mut self,
        shader_name: &str,
        flags: [&str; M],
        execution_mode: Execution,
    ) {
        if self.resources.contains_key(shader_name) {
            if cfg!(debug_assertions) {
                let shader = self
                    .resources
                    .get(shader_name)
                    .unwrap_or_else(|| panic!("resource does not exist: {}", shader_name));
                match shader {
                    GpuResource::Texture(_, _, _, _) => {
                        panic!("{} is not a shader but a texture", shader_name)
                    }
                    GpuResource::Buffer(_) => {
                        panic!("{} is not a shader but a buffer", shader_name)
                    }
                    _ => (),
                }
            }
            return;
        }

        let shader = Shader::get_shader_properties(shader_name, &self.shaders_folder, flags);

        let execution_mode = match execution_mode {
            Execution::PerPixel1D => (
                (self.size.width * self.size.height + shader.cg_x - 1) / shader.cg_x,
                1u32,
                1u32,
            ),
            Execution::PerPixel2D => (
                (self.size.width + shader.cg_x - 1) / shader.cg_x,
                (self.size.height + shader.cg_y - 1) / shader.cg_y,
                1,
            ),
            Execution::N3D(n) => (
                (n + shader.cg_x - 1) / shader.cg_x,
                (n + shader.cg_y - 1) / shader.cg_y,
                (n + shader.cg_z - 1) / shader.cg_z,
            ),
            Execution::N1D(n) => ((n + shader.cg_x - 1) / shader.cg_x, 1, 1),
        };

        let bindings = shader
            .bindings
            .iter()
            .map(|resource| {
                match self
                    .resources
                    .get(resource)
                    .unwrap_or_else(|| panic!("resource does not exist: {}", resource))
                {
                    GpuResource::Buffer(buffer) => TextureOrBuffer::Buffer(buffer, false),
                    GpuResource::Texture(texture, format, dimension, _) => {
                        TextureOrBuffer::Texture(
                            texture,
                            wgpu::StorageTextureAccess::ReadWrite,
                            *format,
                            *dimension,
                        )
                    }
                    GpuResource::Pipeline(_) => panic!(
                        "{} is a pipeline and can not be used as a resource",
                        resource
                    ),
                }
            })
            .collect::<Vec<TextureOrBuffer>>();
        let push_constant_range = shader.push_constant_info.offset
            ..shader.push_constant_info.offset + shader.push_constant_info.size;

        self.resources.insert(
            shader_name.to_string(),
            GpuResource::Pipeline(ComputePipeline::new(
                self,
                shader_name,
                shader.shader.as_slice(),
                bindings.as_slice(),
                execution_mode,
                Some(push_constant_range),
            )),
        );
    }

    pub fn buffer<Type>(&mut self, buffer_name: &str, number_of_elements: u32) {
        if self.resources.contains_key(buffer_name) {
            if cfg!(debug_assertions) {
                let buffer = self
                    .resources
                    .get(buffer_name)
                    .unwrap_or_else(|| panic!("resource does not exist: {}", buffer_name));
                match buffer {
                    GpuResource::Texture(_, _, _, _) => {
                        panic!("{} is not a buffer but a texture", buffer_name)
                    }
                    GpuResource::Pipeline(_) => {
                        panic!("{} is not a buffer but a pipeline", buffer_name)
                    }
                    _ => (),
                }
            }
            return;
        }
        self.resources.insert(
            buffer_name.to_string(),
            GpuResource::Buffer(init_storage_buffer(
                self,
                buffer_name,
                number_of_elements * std::mem::size_of::<Type>() as u32,
                true,
            )),
        );
    }
    pub fn texture(
        &mut self,
        texture_name: &str,
        number_of_elements: (u32, u32, u32),
        format: wgpu::TextureFormat,
    ) {
        if self.resources.contains_key(texture_name) {
            if cfg!(debug_assertions) {
                let texture = self
                    .resources
                    .get(texture_name)
                    .unwrap_or_else(|| panic!("resource does not exist: {}", texture_name));
                match texture {
                    GpuResource::Buffer(_) => {
                        panic!("{} is not a texture but a buffer", texture_name)
                    }
                    GpuResource::Pipeline(_) => {
                        panic!("{} is not a texture but a buffer", texture_name)
                    }
                    _ => (),
                }
            }
            return;
        }

        let (texture, texture_view) = init_texture(
            self,
            texture_name,
            number_of_elements.0,
            number_of_elements.1,
            match number_of_elements.2 {
                0 => None,
                1 => None,
                _ => Some(number_of_elements.2),
            },
            format,
        );

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
            ),
        );
    }

    pub fn get_raw_texture(&self, texture_name: &str) -> &wgpu::TextureView {
        match self
            .resources
            .get(texture_name)
            .unwrap_or_else(|| panic!("resource does not exist: {}", texture_name))
        {
            GpuResource::Buffer(_) => {
                panic!("{} is not a texture but a buffer", texture_name)
            }
            GpuResource::Pipeline(_) => {
                panic!("{} is not a texture but a buffer", texture_name)
            }
            GpuResource::Texture(t, _, _, _) => t,
        }
    }

    pub fn get_raw_buffer(&self, buffer_name: &str) -> &wgpu::Buffer {
        match self
            .resources
            .get(buffer_name)
            .unwrap_or_else(|| panic!("resource does not exist: {}", buffer_name))
        {
            GpuResource::Texture(_, _, _, _) => {
                panic!("{} is not a buffer but a texture", buffer_name)
            }
            GpuResource::Pipeline(_) => {
                panic!("{} is not a buffer but a pipeline", buffer_name)
            }
            GpuResource::Buffer(b) => b,
        }
    }

    pub fn set_buffer_data<T: Pod>(
        &self,
        buffer_name: &str,
        data: &[T],
        data_size: usize,
        location: usize,
    ) {
        info!("writing buffer data to {}, from buffer with {} elements, writing {} bytes starting at {}", buffer_name, data.len(), data_size, location);
        let buffer = self
            .resources
            .get(buffer_name)
            .unwrap_or_else(|| panic!("resource does not exist: {}", buffer_name));
        let buffer = match buffer {
            GpuResource::Texture(_, _, _, _) => {
                panic!("{} is not a buffer but a texture", buffer_name)
            }
            GpuResource::Pipeline(_) => {
                panic!("{} is not a buffer but a pipeline", buffer_name)
            }
            GpuResource::Buffer(b) => b,
        };

        let uploading_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uploading Buffer"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(
            &uploading_buffer,
            0,
            buffer,
            location as u64,
            data_size as u64,
        );
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub async fn read_buffer<T: Pod + Sized + Zeroable>(
        &self,
        buffer_name: &str,
        count: u32,
    ) -> Vec<T> {
        info!(
            "reading buffer data from {}, with {} elements of size {}",
            buffer_name,
            count,
            std::mem::size_of::<T>()
        );
        let buffer = self
            .resources
            .get(buffer_name)
            .unwrap_or_else(|| panic!("resource does not exist: {}", buffer_name));
        let buffer = match buffer {
            GpuResource::Texture(_, _, _, _) => {
                panic!("{} is not a buffer but a texture", buffer_name)
            }
            GpuResource::Pipeline(_) => {
                panic!("{} is not a buffer but a pipeline", buffer_name)
            }
            GpuResource::Buffer(b) => b,
        };

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<T>() as u64 * count as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            std::mem::size_of::<T>() as u64 * count as u64,
        );
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            sender
                .send(v)
                .expect("could not send received data from gpu back to caller")
        });

        self.device.poll(wgpu::Maintain::Wait);

        let _ = receiver
            .receive()
            .await
            .expect("never received buffer data");
        let data = buffer_slice.get_mapped_range();
        let result = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();
        result
    }

    pub fn set_texture_data<T: Pod + Sized + Zeroable>(
        &mut self,
        texture_name: &str,
        data: &[T],
        image_size: (u32, u32, u32),
    ) {
        info!(
            "writing texture data to {}, with size {:?}, the data source has size {}",
            texture_name,
            image_size,
            data.len() * std::mem::size_of::<T>()
        );
        debug_assert!(
            (image_size.0 * std::mem::size_of::<T>() as u32 % 256) == 0,
            "bytes per row must be multiple of 256"
        );
        let uploading_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uploading Buffer"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        let resource = self
            .resources
            .get(texture_name)
            .unwrap_or_else(|| panic!("resource does not exist: {}", texture_name));
        let texture = match resource {
            GpuResource::Buffer(_) => panic!("{} is not a texture but a buffer", texture_name),
            GpuResource::Texture(_, _, _, tex) => tex,
            GpuResource::Pipeline(_) => panic!("{} is not a buffer but a pipeline", texture_name),
        };

        let mut encoder = self.get_encoder();

        let bytes_per_row = max(1, (image_size.0 * std::mem::size_of::<T>() as u32) / 256) * 256;
        let rows_per_image = image_size.1;

        encoder.copy_buffer_to_texture(
            ImageCopyBuffer {
                buffer: &uploading_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        NonZeroU32::new(bytes_per_row)
                            .unwrap_or_else(|| panic!("impossible image width: {}", image_size.0)),
                    ),
                    rows_per_image: Some(
                        NonZeroU32::new(rows_per_image)
                            .unwrap_or_else(|| panic!("impossible image height: {}", image_size.1)),
                    ),
                },
            },
            ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            },
            Extent3d {
                width: image_size.0,
                height: image_size.1,
                depth_or_array_layers: image_size.2,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn log_state(&self) {
        info!("gpu resource state:");
        for (key, val) in &self.resources {
            info!("{}: {:?}", key, val);
        }
    }
}
