use bytemuck::Pod;
use log::info;
use wgpu::util::DeviceExt;
use wgpu::IndexFormat::Uint16;
use wgpu::{CommandEncoder, Extent3d, ImageCopyTexture, SurfaceTexture, TextureView};

use crate::shader::get_execution_dims;
use crate::wgpu_impl::texture::init_texture;
use crate::CoGrEncoder;

use super::{CoGrWGPU, GpuResource};
use crate::shader::Execution;

pub struct EncoderWGPU<'a> {
    pub encoder: Option<CommandEncoder>,
    pub gpu_context: &'a mut CoGrWGPU,
    pub surface_texture: Option<SurfaceTexture>,
    pub surface_texture_view: Option<TextureView>,
}

impl<'a> CoGrEncoder for EncoderWGPU<'a> {
    fn image_buffer_to_screen(&mut self) {
        let mut render_pass = self.encoder.as_mut().unwrap().begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.surface_texture_view.as_ref().expect("there is no surface texture"),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        match &self.gpu_context.to_screen_pipeline {
            Some(pipeline) => {
                render_pass.set_pipeline(&pipeline.pipeline); // 2.
                render_pass.set_bind_group(0, &pipeline.bindgroup, &[]);
                render_pass.set_index_buffer(pipeline.index_buffer.slice(..), Uint16);
                render_pass.draw_indexed(0..pipeline.num_indices, 0, 0..1);
            }
            None => panic!("to_screen_pipeline is not available"),
        }
    }

    fn dispatch_pipeline<PushConstants: Pod>(&mut self, pipeline_name: &str, execution_mode: Execution, push_constants: &PushConstants) {
        loop {
            match self.gpu_context.resources.get(pipeline_name) {
                Some(GpuResource::Buffer(_)) => panic!("{} is not a compute pipeline but a buffer", pipeline_name),
                Some(GpuResource::Texture(_, _, _, _, _)) => panic!("{} is not a compute pipeline but a texture", pipeline_name),
                Some(GpuResource::Pipeline(pipeline, shader)) => {
                    let mut cpass = self
                        .encoder
                        .as_mut()
                        .unwrap()
                        .begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(pipeline_name) });
                    let exec_dims = get_execution_dims(shader, execution_mode, self.gpu_context.size);
                    cpass.insert_debug_marker(pipeline_name);
                    cpass.set_pipeline(&pipeline.pipeline);
                    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
                    cpass.set_push_constants(0, bytemuck::bytes_of(push_constants));
                    cpass.dispatch_workgroups(exec_dims.0, exec_dims.1, exec_dims.2);
                    break;
                }
                None => match self.gpu_context.init_pipeline(pipeline_name) {
                    Ok(_) => continue,
                    Err(error) => panic!("{:?}", error),
                },
            }
        }
    }

    fn set_buffer_data<T: Pod>(&self, buffer_name: &str, data: &[T], elements_to_copy: usize, element_copy_start: usize) {
        info!(
            "writing buffer data to {}, from buffer with {} elements, writing {} bytes starting at {}",
            buffer_name,
            data.len(),
            elements_to_copy * std::mem::size_of::<T>(),
            element_copy_start * std::mem::size_of::<T>()
        );
        match self.gpu_context.resources.get(buffer_name) {
            Some(GpuResource::Texture(_, _, _, _, _)) => panic!("{} is not a buffer but a texture", buffer_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a buffer but a pipeline", buffer_name),
            None => panic!("resource does not exist: {}", buffer_name),
            Some(GpuResource::Buffer(b)) => {
                let uploading_buffer = self.gpu_context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uploading Buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });
                let mut encoder = self.gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.copy_buffer_to_buffer(
                    &uploading_buffer,
                    0,
                    b,
                    (element_copy_start * std::mem::size_of::<T>()) as u64,
                    (elements_to_copy * std::mem::size_of::<T>()) as u64,
                );
                self.gpu_context.queue.submit(std::iter::once(encoder.finish()));
            }
        };
    }

    fn read_buffer<T: Pod>(&self, buffer_name: &str, elements_to_copy: usize, _data: &mut [T]) {
        info!(
            "reading buffer data from {}, with {} elements with a size of {} bytes",
            buffer_name,
            elements_to_copy,
            std::mem::size_of::<T>()
        );
        match self.gpu_context.resources.get(buffer_name) {
            Some(GpuResource::Texture(_, _, _, _, _)) => panic!("{} is not a buffer but a texture", buffer_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a buffer but a pipeline", buffer_name),
            None => panic!("resource does not exist: {}", buffer_name),
            Some(GpuResource::Buffer(b)) => {
                let staging_buffer = self.gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: std::mem::size_of::<T>() as u64 * elements_to_copy as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let mut encoder = self.gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.copy_buffer_to_buffer(b, 0, &staging_buffer, 0, std::mem::size_of::<T>() as u64 * elements_to_copy as u64);
                self.gpu_context.queue.submit(Some(encoder.finish()));

                let buffer_slice = staging_buffer.slice(..);
                let (sender, _receiver) = futures_intrusive::channel::shared::oneshot_channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
                    sender.send(v).expect("could not send received data from gpu back to caller")
                });

                self.gpu_context.device.poll(wgpu::Maintain::Wait);

                todo!();
                //let _ = receiver.receive().await.expect("never received buffer data");
                let data = buffer_slice.get_mapped_range();
                //let result = bytemuck::cast_slice(&data).to_vec();
                drop(data);
                staging_buffer.unmap();
                //result
            }
        }
    }

    fn set_texture_data<T: Pod>(&mut self, texture_name: &str, data: &[T]) {
        info!(
            "writing texture data to {}, the data source has size {}",
            texture_name,
            data.len() * std::mem::size_of::<T>()
        );
        match self.gpu_context.resources.get(texture_name) {
            Some(GpuResource::Buffer(_)) => panic!("{} is not a texture but a buffer", texture_name),
            Some(GpuResource::Pipeline(_, _)) => panic!("{} is not a buffer but a pipeline", texture_name),
            None => panic!("resource does not exist: {}", texture_name),
            Some(GpuResource::Texture(_, format, _, tex, size)) => {
                let mut encoder = self.gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let bytes_per_pixel = format.describe().block_size;

                if data.len() * std::mem::size_of::<T>() / bytes_per_pixel as usize != (size.0 * size.1 * size.2) as usize {
                    panic!(
                        "data had a size of {} while the texture had a size of {}",
                        data.len() * std::mem::size_of::<T>(),
                        (size.0 * size.1 * size.2) as usize * bytes_per_pixel as usize
                    );
                }

                let (texture, _) = init_texture(self.gpu_context, "copy_texture", *size, *format, Some(data));
                encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &texture,
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: Default::default(),
                    },
                    ImageCopyTexture {
                        texture: tex,
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: Default::default(),
                    },
                    Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: size.2,
                    },
                );

                self.gpu_context.queue.submit(std::iter::once(encoder.finish()));
            }
        };
    }

    fn read_texture<T: Pod>(&self, _texture_name: &str, _data: &mut [T]) {
        todo!()
    }
}

impl<'a> Drop for EncoderWGPU<'a> {
    fn drop(&mut self) {
        match (&mut self.encoder, &mut self.surface_texture, &self.surface_texture_view) {
            (encoder, surface_texture, surface_texture_view) if encoder.is_some() && surface_texture.is_some() && surface_texture_view.is_some() => {
                self.gpu_context.queue.submit(std::iter::once(encoder.take().unwrap().finish()));
                let surface = surface_texture.take().unwrap();
                surface.present();
            }
            (encoder, surface_texture, _) if encoder.is_some() && surface_texture.is_none() => {
                self.gpu_context.queue.submit(std::iter::once(encoder.take().unwrap().finish()));
            }
            (encoder, _, _) if encoder.is_none() => {
                panic!("encoder is none while that should be impossbile");
            }
            (_, _, _) => panic!("impossible state"),
        }
    }
}
