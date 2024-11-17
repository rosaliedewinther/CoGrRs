use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::size_of_val;
use std::ops::{Deref, DerefMut};

use anyhow::{anyhow, Context, Result};
use egui::Ui;
use egui_wgpu::ScreenDescriptor;
use wgpu_profiler::GpuTimerQueryResult;

use crate::gpu::resources::init_texture_with_data;
use crate::gpu::Pipeline;
use bytemuck::{AnyBitPattern, NoUninit, Pod};
use tracing::info;
use wgpu::util::DeviceExt;
use wgpu::IndexFormat::Uint16;
use wgpu::{
    CommandEncoder, Extent3d, ImageCopyTexture, RenderPassDescriptor, SurfaceTexture,
    TextureFormat, TextureView,
};

use crate::gpu::ResourceHandle;
use crate::{CoGr, Resource};

use super::to_screen_pipeline::ToScreenPipeline;

pub struct Encoder<'a> {
    pub(crate) command_encoder: Option<CommandEncoder>,
    pub(crate) gpu_context: &'a mut CoGr,
}

pub struct DrawEncoder<'a> {
    pub(crate) command_encoder: Option<CommandEncoder>,
    pub(crate) gpu_context: &'a mut CoGr,
    pub(crate) surface_texture: Option<SurfaceTexture>,
    pub(crate) texture_view: TextureView,
}

impl<'a> DrawEncoder<'a> {
    pub fn to_screen(
        &mut self,
        to_screen_texture: &ResourceHandle,
        texture_format: TextureFormat,
    ) -> Result<()> {
        puffin::profile_function!();
        let encoder = self.command_encoder.as_mut().expect("there was no encoder");
        let ctx = &mut self.gpu_context;

        let _ = ctx.profiler.scope("to_screen", encoder, &ctx.device);
        if let Resource::Texture(texture) = ctx.resource_pool.grab_resource(to_screen_texture) {
            let texture_view = texture.texture_view.as_ref().unwrap();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pipeline = ToScreenPipeline::new(&ctx.device, texture_view, texture_format);

            // run pipeline
            render_pass.set_pipeline(&pipeline.pipeline); // 2.
            render_pass.set_bind_group(0, &pipeline.bindgroup, &[]);
            render_pass.set_index_buffer(pipeline.index_buffer.slice(..), Uint16);
            render_pass.draw_indexed(0..pipeline.num_indices, 0, 0..1);

            Ok(())
        } else {
            Err(anyhow!(
                "Resource pool did not contain texture resource with this handle"
            ))
        }
    }

    fn draw_gpu_timings(egui_ctx: &egui::Context, frame_timings: &Vec<GpuTimerQueryResult>) {
        puffin::profile_function!();

        egui::Window::new("gpu_timings").show(egui_ctx, |ui: &mut Ui| {
            egui::Grid::new("gpu_timings_grid").show(ui, |ui| {
                let mut time_sum = 0.0;
                for timing in frame_timings {
                    if let Some(time) = &timing.time {
                        assert!(
                            timing.nested_queries.is_empty(),
                            "we don't ever want to capture nested scopes"
                        );
                        let time = time.end - time.start;
                        ui.label(format!("{}:", timing.label,));
                        ui.label(format!("{:.4}ms", time * 1000.0));
                        ui.end_row();
                        time_sum += time;
                    }
                }
                ui.separator();
                ui.separator();
                ui.end_row();
                ui.label("total gpu time:");
                ui.label(format!("{:.4}ms", time_sum * 1000.0));
                ui.end_row();
                ui.label("fps:");
                ui.label(format!("{:.4}fps", 1.0 / time_sum));
            });
        });
    }

    pub fn draw_ui(&mut self, ui_builder: impl FnOnce(&egui::Context)) -> Result<()> {
        puffin::profile_function!();
        {
            if let Some(command_encoder) = &mut self.command_encoder {
                //let encoder = &mut self.encoder.as_mut().expect("there was no encoder");
                let ctx = &mut self.gpu_context;

                // let command_encoder = encoder
                //     .command_encoder
                //     .as_mut()
                //     .context("encoder not available")?;

                let _ = ctx.profiler.scope("draw ui", command_encoder, &ctx.device);
                let state = &mut ctx.state;

                let screen_descriptor = ScreenDescriptor {
                    size_in_pixels: [ctx.config.width, ctx.config.height],
                    pixels_per_point: state.egui_ctx().pixels_per_point(),
                };

                let input = state.take_egui_input(ctx.window.as_ref());

                let egui_ctx = state.egui_ctx();

                state.egui_ctx().begin_pass(input);
                egui::TopBottomPanel::top("top_bar").show(egui_ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(ctx.draw_cpu_profiler, "cpu_profiler")
                            .clicked()
                        {
                            ctx.draw_cpu_profiler ^= true;
                        }
                        if ui
                            .selectable_label(ctx.draw_gpu_profiler, "gpu_profiler")
                            .clicked()
                        {
                            ctx.draw_gpu_profiler ^= true;
                        }
                        if ui.selectable_label(ctx.draw_user_ui, "user_ui").clicked() {
                            ctx.draw_user_ui ^= true;
                        }
                    });
                });

                if ctx.draw_gpu_profiler {
                    Self::draw_gpu_timings(egui_ctx, &ctx.frame_timings);
                }
                if ctx.draw_cpu_profiler {
                    puffin_egui::profiler_window(egui_ctx);
                }
                if ctx.draw_user_ui {
                    ui_builder(egui_ctx);
                }
                let full_output = state.egui_ctx().end_pass();

                let paint_jobs = ctx
                    .state
                    .egui_ctx()
                    .tessellate(full_output.shapes, ctx.state.egui_ctx().pixels_per_point());
                let t_delta = full_output.textures_delta;

                {
                    for d in t_delta.set {
                        ctx.renderer
                            .update_texture(&ctx.device, &ctx.queue, d.0, &d.1);
                    }
                    {
                        ctx.renderer.update_buffers(
                            &ctx.device,
                            &ctx.queue,
                            command_encoder,
                            &paint_jobs,
                            &screen_descriptor,
                        );
                    }

                    let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        label: Some("Ui render command encoder"),
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    ctx.renderer.render(
                        &mut render_pass.forget_lifetime(),
                        paint_jobs.as_slice(),
                        &screen_descriptor,
                    );
                }
            }
        }

        Ok(())
    }
}

impl Encoder<'_> {
    pub fn width(&self) -> u32 {
        self.gpu_context.config.width
    }
    pub fn height(&self) -> u32 {
        self.gpu_context.config.height
    }

    pub fn dispatch_pipeline(
        &mut self,
        pipeline: &mut Pipeline,
        work_groups: (u32, u32, u32),
        resources: &[&ResourceHandle],
    ) -> Result<()> {
        puffin::profile_function!();

        let mut loaded_resources = Vec::new();

        for handle in resources {
            loaded_resources.push(self.gpu_context.resource_pool.grab_resource(handle));
        }

        let encoder = self
            .command_encoder
            .as_mut()
            .context("encoder not available")?;

        let _ = self.gpu_context.profiler.scope(
            &pipeline.shader.file,
            encoder,
            &self.gpu_context.device,
        );

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        let compiled_pipeline =
            pipeline.get_compiled(self.gpu_context, loaded_resources.as_slice());

        compute_pass.set_pipeline(&compiled_pipeline.pipeline);
        compute_pass.set_bind_group(0, &compiled_pipeline.bind_group, &[]);
        compute_pass.dispatch_workgroups(work_groups.0, work_groups.1, work_groups.2);

        Ok(())
    }

    pub fn set_buffer_data<T: AnyBitPattern + NoUninit, K: AsRef<[T]>>(
        &mut self,
        buffer: &ResourceHandle,
        data: K,
    ) -> Result<()> {
        puffin::profile_function!();
        let data = data.as_ref();
        info!(
            "writing buffer data to {:?}, from buffer with {} elements",
            buffer,
            data.len(),
        );
        let encoder = self
            .command_encoder
            .as_mut()
            .context("encoder not available")?;

        if let Resource::Buffer(buffer) = self.gpu_context.resource_pool.grab_resource(buffer) {
            let _ = self.gpu_context.profiler.scope(
                format!("Set buffer data: {}", buffer.name),
                encoder,
                &self.gpu_context.device,
            );

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
        } else {
            Err(anyhow!(
                "Resource pool did not contain buffer resource with this handle"
            ))
        }
    }

    pub fn set_texture_data<T: Pod, K: AsRef<[T]>>(
        &mut self,
        texture: &ResourceHandle,
        data: K,
    ) -> Result<()> {
        puffin::profile_function!();
        let data = data.as_ref();
        info!(
            "writing texture data to {:?}, the data source has size {}",
            texture,
            size_of_val(data)
        );

        let encoder = self
            .command_encoder
            .as_mut()
            .context("encoder not available")?;

        if let Resource::Texture(texture) = self.gpu_context.resource_pool.grab_resource(texture) {
            let _ = self.gpu_context.profiler.scope(
                format!("set texture data: {}", texture.name),
                encoder,
                &self.gpu_context.device,
            );

            match texture.resolution {
                crate::gpu::TextureRes::Custom(x, y, z) => {
                    todo!("grab correct bytes per pixel");
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
        } else {
            Err(anyhow!(
                "Resource pool did not contain texture resource with this handle"
            ))
        }
    }
}

impl<'a> Drop for Encoder<'a> {
    fn drop(&mut self) {
        puffin::profile_function!();
        self.command_encoder.as_mut().unwrap().pop_debug_group();
        self.gpu_context
            .profiler
            .resolve_queries(self.command_encoder.as_mut().unwrap());
        self.gpu_context.queue.submit(std::iter::once(
            self.command_encoder.take().unwrap().finish(),
        ));
        let timestamp_period = self.gpu_context.queue.get_timestamp_period();

        self.gpu_context.profiler.end_frame().unwrap();
        if let Some(timings) = self
            .gpu_context
            .profiler
            .process_finished_frame(timestamp_period)
        {
            self.gpu_context.frame_timings = timings;
        }
    }
}

impl<'a> Drop for DrawEncoder<'a> {
    fn drop(&mut self) {
        puffin::profile_function!();
        self.command_encoder.as_mut().unwrap().pop_debug_group();
        self.gpu_context
            .profiler
            .resolve_queries(self.command_encoder.as_mut().unwrap());
        self.gpu_context.queue.submit(std::iter::once(
            self.command_encoder.take().unwrap().finish(),
        ));
        let timestamp_period = self.gpu_context.queue.get_timestamp_period();

        self.gpu_context.profiler.end_frame().unwrap();
        if let Some(timings) = self
            .gpu_context
            .profiler
            .process_finished_frame(timestamp_period)
        {
            self.gpu_context.frame_timings = timings;
        }
        // make sure to present the frame after all
        let surface = self.surface_texture.take().unwrap();
        surface.present();
    }
}

pub fn div_ceil(val: u32, div: u32) -> u32 {
    (val / div) + (val % div)
}
