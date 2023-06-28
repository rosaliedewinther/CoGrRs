use std::mem::size_of_val;

use anyhow::{anyhow, Context, Result};

use crate::gpu::Pipeline;
use bytemuck::Pod;
use egui_wgpu::renderer::ScreenDescriptor;
use log::info;
use wgpu::util::DeviceExt;
use wgpu::IndexFormat::Uint16;
use wgpu::{
    CommandEncoder, Extent3d, ImageCopyTexture, RenderPassDescriptor, SurfaceTexture, TextureView,
};

use crate::gpu::ResourceHandle;
use crate::init_texture_with_data;
use crate::CoGr;

use super::to_screen_pipeline::ToScreenPipeline;

pub enum EncoderType {
    Draw(Option<SurfaceTexture>, TextureView),
    NonDraw,
}

pub struct Encoder<'a> {
    pub(crate) encoder: Option<CommandEncoder>,
    pub(crate) gpu_context: &'a mut CoGr,
    pub(crate) encoder_type: EncoderType,
}

impl<'a> Encoder<'a> {
    pub fn to_screen(&mut self, to_screen_texture: &ResourceHandle) -> Result<()> {
        let texture = self
            .gpu_context
            .resource_pool
            .grab_texture(to_screen_texture);
        let texture_view = texture.texture_view.as_ref().unwrap();

        let encoder = self.encoder.as_mut().context("encoder not available")?;
        let mut render_pass = match &self.encoder_type {
            EncoderType::NonDraw => {
                Err(anyhow!("non draw encoder was used for to_screen rendering"))?
            }
            EncoderType::Draw(_, texture_view) => {
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                })
            }
        };

        if self.gpu_context.last_to_screen_texture_handle.is_none()
            || !to_screen_texture.ptr_eq(
                self.gpu_context
                    .last_to_screen_texture_handle
                    .as_ref()
                    .unwrap(),
            )
        {
            self.gpu_context.last_to_screen_texture_handle = Some(to_screen_texture.clone());
            self.gpu_context.last_to_screen_pipeline = Some(ToScreenPipeline::new(
                &self.gpu_context.device,
                texture_view,
                self.gpu_context.config.format,
            ));
        }

        // run pipeline
        let pipeline = self.gpu_context.last_to_screen_pipeline.as_ref().unwrap();
        render_pass.set_pipeline(&pipeline.pipeline); // 2.
        render_pass.set_bind_group(0, &pipeline.bindgroup, &[]);
        render_pass.set_index_buffer(pipeline.index_buffer.slice(..), Uint16);
        render_pass.draw_indexed(0..pipeline.num_indices, 0, 0..1);

        Ok(())
    }

    // todo: change resources to accept either texture or buffer handle
    pub fn dispatch_pipeline<PushConstants: Pod>(
        &mut self,
        pipeline: &mut Pipeline,
        work_groups: (u32, u32, u32),
        push_constants: &PushConstants,
        resources: &[&ResourceHandle],
    ) -> Result<()> {
        let encoder = self.encoder.as_mut().context("encoder not available")?;

        let mut compute_pass =
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });

        let bind_group_entries = resources
            .iter()
            .enumerate()
            .map(|(i, val)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: match val {
                    ResourceHandle::Texture(_) => wgpu::BindingResource::TextureView(
                        self.gpu_context
                            .resource_pool
                            .grab_texture(val)
                            .texture_view
                            .as_ref()
                            .unwrap(),
                    ),
                    ResourceHandle::Buffer(_) => self
                        .gpu_context
                        .resource_pool
                        .grab_buffer(val)
                        .buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
            })
            .collect::<Vec<wgpu::BindGroupEntry>>();

        let bind_group = self
            .gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("resources bind group"),
                layout: &pipeline.bind_group_layout,
                entries: bind_group_entries.as_slice(),
            });

        pipeline.last_bind_group = Some(bind_group);

        compute_pass.set_pipeline(&pipeline.pipeline);
        compute_pass.set_bind_group(0, pipeline.last_bind_group.as_ref().unwrap(), &[]);
        compute_pass.set_push_constants(0, bytemuck::bytes_of(push_constants));
        compute_pass.dispatch_workgroups(work_groups.0, work_groups.1, work_groups.2);

        Ok(())
    }

    pub fn set_buffer_data<T: Pod, K: AsRef<[T]>>(
        &mut self,
        buffer: &ResourceHandle,
        data: K,
    ) -> Result<()> {
        let data = data.as_ref();
        info!(
            "writing buffer data to {:?}, from buffer with {} elements",
            buffer,
            data.len(),
        );
        let encoder = self.encoder.as_mut().context("encoder not available")?;
        let buffer = self.gpu_context.resource_pool.grab_buffer(buffer);
        let uploading_buffer =
            self.gpu_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uploading Buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });

        encoder.copy_buffer_to_buffer(
            &uploading_buffer,
            0,
            buffer.buffer.as_ref().unwrap(),
            0,
            size_of_val(data) as u64,
        );
        Ok(())
    }

    pub fn set_texture_data<T: Pod, K: AsRef<[T]>>(
        &mut self,
        texture: &ResourceHandle,
        data: K,
    ) -> Result<()> {
        let data = data.as_ref();
        info!(
            "writing texture data to {:?}, the data source has size {}",
            texture,
            size_of_val(data)
        );

        let encoder = self.encoder.as_mut().context("encoder not available")?;
        let texture = self.gpu_context.resource_pool.grab_texture(texture);

        match texture.resolution {
            crate::gpu::TextureRes::Custom(x, y, z) => {
                let bytes_per_pixel = texture
                    .format
                    .block_size(None)
                    .expect("could not get block size");

                if size_of_val(data) / bytes_per_pixel as usize != (x * y * z) as usize {
                    panic!(
                        "data had a size of {} while the texture had a size of {}",
                        size_of_val(data),
                        (x * y * z) as usize * bytes_per_pixel as usize
                    );
                }

                let (copy_texture, _) = init_texture_with_data(
                    &self.gpu_context.device,
                    &self.gpu_context.queue,
                    "copy_texture",
                    (x, y, z),
                    texture.format,
                    bytemuck::cast_slice(data),
                )?;
                encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &copy_texture,
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: Default::default(),
                    },
                    ImageCopyTexture {
                        texture: texture.texture.as_ref().unwrap(),
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: Default::default(),
                    },
                    Extent3d {
                        width: x,
                        height: y,
                        depth_or_array_layers: z,
                    },
                );
            }
            _ => unimplemented!(),
        }

        Ok(())
    }

    pub fn draw_ui(&mut self, ui_builder: impl FnOnce(&egui::Context)) -> Result<()> {
        let ctx = &mut self.gpu_context;
        let encoder = self.encoder.as_mut().context("encoder not available")?;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [ctx.config.width, ctx.config.height],
            pixels_per_point: 1f32,
        };
        let full_output = ctx
            .context
            .run(ctx.state.take_egui_input(ctx.window.as_ref()), |ctx| {
                ui_builder(ctx)
            });

        let paint_jobs = ctx.context.tessellate(full_output.shapes);
        let tdelta = full_output.textures_delta;

        {
            for d in tdelta.set {
                ctx.renderer
                    .update_texture(&ctx.device, &ctx.queue, d.0, &d.1);
            }
            ctx.renderer.update_buffers(
                &ctx.device,
                &ctx.queue,
                encoder,
                &paint_jobs,
                &screen_descriptor,
            );

            match &self.encoder_type {
                EncoderType::NonDraw => Err(anyhow!(
                    "Tried to draw without using get_encoder_for_draw()"
                ))?,
                EncoderType::Draw(_, texture_view) => {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        })],
                        ..Default::default()
                    });
                    ctx.renderer.render(
                        &mut render_pass,
                        paint_jobs.as_slice(),
                        &screen_descriptor,
                    );
                }
            }
        }
        Ok(())
    }
}

impl<'a> Drop for Encoder<'a> {
    fn drop(&mut self) {
        match &mut self.encoder_type {
            EncoderType::Draw(texture, _) => {
                self.encoder.as_mut().unwrap().pop_debug_group();
                self.gpu_context
                    .queue
                    .submit(std::iter::once(self.encoder.take().unwrap().finish()));
                let surface = texture.take().unwrap();
                surface.present();
            }
            EncoderType::NonDraw => {
                self.encoder.as_mut().unwrap().pop_debug_group();
                self.gpu_context
                    .queue
                    .submit(std::iter::once(self.encoder.take().unwrap().finish()));
            }
        }
    }
}
