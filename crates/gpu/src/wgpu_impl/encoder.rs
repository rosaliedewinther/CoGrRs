use core::panic;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::shader::get_execution_dims;
use crate::{Execution, ReadHandle};
use bytemuck::Pod;
use log::info;
use wgpu::util::DeviceExt;
use wgpu::IndexFormat::Uint16;
use wgpu::{CommandEncoder, Extent3d, ImageCopyTexture, SurfaceTexture, TextureView};

use crate::wgpu_impl::texture::init_texture;
use crate::CoGrEncoder;

use super::read_handle::WGPUReadhandle;
use super::to_screen_pipeline::ToScreenPipeline;
use super::{CoGrWGPU, GpuResource, ToScreenPipelineDescriptor};

pub struct EncoderWGPU<'a> {
    pub encoder: Option<CommandEncoder>,
    pub gpu_context: &'a mut CoGrWGPU,
    pub surface_texture: Option<SurfaceTexture>,
    pub surface_texture_view: Option<TextureView>,
}

impl<'a> CoGrEncoder for EncoderWGPU<'a> {
    fn to_screen(&mut self, to_screen_texture_name: &'static str) {
        let surface_texture_view = self.surface_texture_view.as_ref().expect("there is no surface texture view");
        let encoder = self.encoder.as_mut().unwrap();

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        let mut hasher = DefaultHasher::new();
        to_screen_texture_name.as_bytes().hash(&mut hasher);
        "__to_screen_pipeline__".hash(&mut hasher);
        let hash = hasher.finish();
        let hash_str = hash.to_string();

        // create pipeline if it doesn't exist
        if !self.gpu_context.resources.contains_key(&hash_str) {
            self.gpu_context.resources.insert(
                hash_str.clone(),
                GpuResource::ToScreenPipeline(ToScreenPipelineDescriptor {
                    texture_name: to_screen_texture_name,
                    pipeline: ToScreenPipeline::new(
                        &self.gpu_context.device,
                        self.gpu_context.get_raw_texture(&to_screen_texture_name),
                        self.gpu_context.config.format,
                    ),
                }),
            );
        }

        // run pipeline
        match self.gpu_context.resources.get(&hash_str) {
            Some(GpuResource::ToScreenPipeline(desc)) => {
                render_pass.set_pipeline(&desc.pipeline.pipeline); // 2.
                render_pass.set_bind_group(0, &desc.pipeline.bindgroup, &[]);
                render_pass.set_index_buffer(desc.pipeline.index_buffer.slice(..), Uint16);
                render_pass.draw_indexed(0..desc.pipeline.num_indices, 0, 0..1);
            }
            val => panic!("{} was not a to screen pipeline but contained: {:?}", hash_str, val),
        }
    }

    fn dispatch_pipeline<PushConstants: Pod>(&mut self, pipeline_name: &'static str, execution_mode: Execution, push_constants: &PushConstants) {
        if !self.gpu_context.resources.contains_key(pipeline_name) {
            self.gpu_context.init_pipeline(pipeline_name);
        }

        match self.gpu_context.resources.get(pipeline_name) {
            Some(GpuResource::Pipeline(desc)) => {
                let mut compute_pass = self
                    .encoder
                    .as_mut()
                    .unwrap()
                    .begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(pipeline_name) });
                let exec_dims = get_execution_dims(
                    desc.workgroup_size,
                    execution_mode,
                    (self.gpu_context.config.width, self.gpu_context.config.height),
                );
                compute_pass.set_pipeline(&desc.pipeline.pipeline);
                compute_pass.set_bind_group(0, &desc.pipeline.bind_group, &[]);
                compute_pass.set_push_constants(0, bytemuck::bytes_of(push_constants));
                compute_pass.dispatch_workgroups(exec_dims.0, exec_dims.1, exec_dims.2);
            }
            val => panic!("{} was not a pipeline but contained: {:?}", pipeline_name, val),
        }
    }

    fn set_buffer_data<T: Pod>(&mut self, buffer_name: &'static str, data: &[T]) {
        info!("writing buffer data to {}, from buffer with {} elements", buffer_name, data.len(),);
        match self.gpu_context.resources.get(buffer_name) {
            Some(GpuResource::Buffer(desc)) => {
                let uploading_buffer = self.gpu_context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uploading Buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });

                self.encoder
                    .as_mut()
                    .unwrap()
                    .copy_buffer_to_buffer(&uploading_buffer, 0, &desc.buffer, 0, (data.len() * std::mem::size_of::<T>()) as u64);
            }
            val => panic!("{} was not a buffer but contained: {:?}", buffer_name, val),
        };
    }

    fn read_buffer<T: Pod>(&mut self, buffer_name: &'static str) -> ReadHandle {
        /*info!("reading buffer data from {}, with size of {} bytes", buffer_name, std::mem::size_of::<T>());
        match self.gpu_context.resources.get(buffer_name) {
            Some(GpuResource::Texture(_, _, _, _, _)) => panic!("{} is not a buffer but a texture", buffer_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a buffer but a pipeline", buffer_name),
            None => panic!("resource does not exist: {}", buffer_name),
            Some(GpuResource::Buffer(b)) => {
                let staging_buffer = self.gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ReadBuffer"),
                    size: std::mem::size_of::<T>() as u64 * elements_to_copy as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.encoder
                    .as_mut()
                    .unwrap()
                    .copy_buffer_to_buffer(b, 0, &staging_buffer, 0, std::mem::size_of::<T>() as u64 * elements_to_copy as u64);

                thread::spawn(move || {
                    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();

                    {
                        let buffer_slice = staging_buffer.slice(..);
                        println!("before send: {:?}", buffer_slice);
                        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
                            sender.send(v).expect("could not send received data from gpu back to caller")
                        });
                    }
                    self.gpu_context.device.poll(wgpu::Maintain::Wait);
                    let _ = pollster::block_on(receiver.receive()).expect("never received buffer data");
                    let buffer_slice = staging_buffer.slice(..);
                    println!("after send: {:?}", buffer_slice);
                    let data = buffer_slice.get_mapped_range();
                    to_write_buffer = bytemuck::cast_slice(&data);
                    drop(data);
                    staging_buffer.unmap();
                });
            }
        }*/
        todo!()
    }

    fn set_texture_data<T: Pod>(&mut self, texture_name: &'static str, data: &[T]) {
        info!(
            "writing texture data to {}, the data source has size {}",
            texture_name,
            data.len() * std::mem::size_of::<T>()
        );
        match self.gpu_context.resources.get(texture_name) {
            Some(GpuResource::Texture(desc)) => {
                let bytes_per_pixel = desc.format.describe().block_size;

                if data.len() * std::mem::size_of::<T>() / bytes_per_pixel as usize != (desc.size.0 * desc.size.1 * desc.size.2) as usize {
                    panic!(
                        "data had a size of {} while the texture had a size of {}",
                        data.len() * std::mem::size_of::<T>(),
                        (desc.size.0 * desc.size.1 * desc.size.2) as usize * bytes_per_pixel as usize
                    );
                }

                let (texture, _) = init_texture(self.gpu_context, "copy_texture", desc.size, desc.format, Some(data));
                self.encoder.as_mut().unwrap().copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &texture,
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: Default::default(),
                    },
                    ImageCopyTexture {
                        texture: &desc.texture,
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: Default::default(),
                    },
                    Extent3d {
                        width: desc.size.0,
                        height: desc.size.1,
                        depth_or_array_layers: desc.size.2,
                    },
                );
            }
            val => panic!("{} was not a texture but contained: {:?}", texture_name, val),
        };
    }

    fn read_texture<T: Pod>(&mut self, _texture_name: &'static str) -> WGPUReadhandle {
        todo!()
    }
}

impl<'a> Drop for EncoderWGPU<'a> {
    fn drop(&mut self) {
        match (&mut self.encoder, &mut self.surface_texture, &self.surface_texture_view) {
            (encoder, surface_texture, surface_texture_view) if encoder.is_some() && surface_texture.is_some() && surface_texture_view.is_some() => {
                encoder.as_mut().unwrap().pop_debug_group();
                self.gpu_context.queue.submit(std::iter::once(encoder.take().unwrap().finish()));
                let surface = surface_texture.take().unwrap();
                surface.present();
            }
            (encoder, surface_texture, _) if encoder.is_some() && surface_texture.is_none() => {
                encoder.as_mut().unwrap().pop_debug_group();
                self.gpu_context.queue.submit(std::iter::once(encoder.take().unwrap().finish()));
            }
            (encoder, _, _) if encoder.is_none() => {
                panic!("encoder is none while that should be impossbile");
            }
            (_, _, _) => panic!("impossible state"),
        }
    }
}
