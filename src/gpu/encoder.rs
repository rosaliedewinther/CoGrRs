use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::size_of_val;
use std::ops::{Deref, DerefMut};

use anyhow::{Context, Result};
use egui::Ui;

use crate::gpu::Pipeline;
use bytemuck::{AnyBitPattern, NoUninit, Pod};
use egui_wgpu::renderer::ScreenDescriptor;
use tracing::info;
use wgpu::util::DeviceExt;
use wgpu::IndexFormat::Uint16;
use wgpu::{
    CommandEncoder, Extent3d, ImageCopyTexture, RenderPassDescriptor, SurfaceTexture, TextureView,
};
use wgpu_profiler::{wgpu_profiler, GpuTimerScopeResult};

use crate::gpu::ResourceHandle;
use crate::CoGr;

use super::to_screen_pipeline::ToScreenPipeline;

pub struct Encoder<'a> {
    pub(crate) command_encoder: Option<CommandEncoder>,
    pub(crate) gpu_context: &'a mut CoGr,
}

pub struct DrawEncoder<'a> {
    pub(crate) encoder: Option<Encoder<'a>>,
    pub(crate) surface_texture: Option<SurfaceTexture>,
    pub(crate) texture_view: TextureView,
}

impl<'a> Deref for DrawEncoder<'a> {
    type Target = Encoder<'a>;

    fn deref(&self) -> &Self::Target {
        self.encoder.as_ref().expect("There was no encoder")
    }
}

impl<'a> DerefMut for DrawEncoder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.encoder.as_mut().expect("There was no encoder")
    }
}

impl<'a> DrawEncoder<'a> {
    pub fn to_screen(&mut self, to_screen_texture: &ResourceHandle) -> Result<()> {
        puffin::profile_function!();
        let encoder = &mut self.encoder.as_mut().expect("there was no encoder");
        let ctx = &mut encoder.gpu_context;
        let command_encoder = encoder
            .command_encoder
            .as_mut()
            .context("encoder not available")?;

        wgpu_profiler!(
            "to_screen",
            &mut ctx.profiler,
            command_encoder,
            &ctx.device,
            {
                let texture = ctx.resource_pool.grab_texture(to_screen_texture);
                let texture_view = texture.texture_view.as_ref().unwrap();

                let mut render_pass =
                    command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("To screen render pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                if ctx.last_to_screen_texture_handle.is_none()
                    || !to_screen_texture
                        .ptr_eq(ctx.last_to_screen_texture_handle.as_ref().unwrap())
                {
                    ctx.last_to_screen_texture_handle = Some(to_screen_texture.clone());
                    ctx.last_to_screen_pipeline = Some(ToScreenPipeline::new(
                        &ctx.device,
                        &texture.texture_view,
                        texture.format,
                    ));
                }

                // run pipeline
                let pipeline = ctx.last_to_screen_pipeline.as_ref().unwrap();
                render_pass.set_pipeline(&pipeline.pipeline); // 2.
                render_pass.set_bind_group(0, &pipeline.bind_group, &[]);
                render_pass.set_index_buffer(pipeline.index_buffer.slice(..), Uint16);
                render_pass.draw_indexed(0..pipeline.num_indices, 0, 0..1);
            }
        );
        Ok(())
    }

    fn draw_gpu_timings(egui_ctx: &egui::Context, frame_timings: &Vec<GpuTimerScopeResult>) {
        puffin::profile_function!();

        egui::Window::new("gpu_timings").show(egui_ctx, |ui: &mut Ui| {
            egui::Grid::new("gpu_timings_grid").show(ui, |ui| {
                let mut time_sum = 0.0;
                for timing in frame_timings {
                    assert!(
                        timing.nested_scopes.is_empty(),
                        "we dont ever wanna capture nested scopes"
                    );
                    let time = timing.time.end - timing.time.start;
                    ui.label(format!("{}:", timing.label,));
                    ui.label(format!("{:.4}ms", time * 1000.0));
                    ui.end_row();
                    time_sum += time;
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
        let encoder = &mut self.encoder.as_mut().expect("there was no encoder");
        let ctx = &mut encoder.gpu_context;
        let command_encoder = encoder
            .command_encoder
            .as_mut()
            .context("encoder not available")?;

        wgpu_profiler!(
            "draw_ui",
            &mut ctx.profiler,
            command_encoder,
            &ctx.device,
            {
                let screen_descriptor = ScreenDescriptor {
                    size_in_pixels: [ctx.config.width, ctx.config.height],
                    pixels_per_point: 1f32,
                };
                let full_output =
                    ctx.context
                        .run(ctx.state.take_egui_input(ctx.window.as_ref()), |egui_ctx| {
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
                        command_encoder,
                        &paint_jobs,
                        &screen_descriptor,
                    );

                    let mut render_pass =
                        command_encoder.begin_render_pass(&RenderPassDescriptor {
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &self.texture_view,
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
        );
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
    // todo: change resources to accept either texture or buffer handle
    pub fn dispatch_pipeline(
        &mut self,
        pipeline: &mut Pipeline,
        work_groups: (u32, u32, u32),
        resources: &[&ResourceHandle],
    ) -> Result<()> {
        puffin::profile_function!();
        pipeline.check_hot_reload(&self.gpu_context, resources);
        let encoder = self
            .command_encoder
            .as_mut()
            .context("encoder not available")?;

        wgpu_profiler!(
            &pipeline.pipeline_name,
            &mut self.gpu_context.profiler,
            encoder,
            &self.gpu_context.device,
            {
                let mut compute_pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
                // hash resources to check if we can reuse the previous bind group of this pipeline
                let mut hasher = DefaultHasher::new();
                resources.hash(&mut hasher);
                let last_bind_group_hash = hasher.finish();
                if last_bind_group_hash != pipeline.last_bind_group_hash {
                    let bind_group_entries = resources
                        .iter()
                        .enumerate()
                        .map(|(i, val)| wgpu::BindGroupEntry {
                            binding: i as u32,
                            resource: match val {
                                ResourceHandle::Texture(_) => wgpu::BindingResource::TextureView(
                                    &self
                                        .gpu_context
                                        .resource_pool
                                        .grab_texture(val)
                                        .texture_view,
                                ),
                                ResourceHandle::Buffer(_) => self
                                    .gpu_context
                                    .resource_pool
                                    .grab_buffer(val)
                                    .buffer
                                    .as_entire_binding(),
                            },
                        })
                        .collect::<Vec<wgpu::BindGroupEntry>>();

                    let bind_group =
                        self.gpu_context
                            .device
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("resources bind group"),
                                layout: &pipeline.bind_group_layout,
                                entries: bind_group_entries.as_slice(),
                            });

                    pipeline.last_bind_group = Some(bind_group);
                }

                compute_pass.set_pipeline(&pipeline.pipeline);
                compute_pass.set_bind_group(0, pipeline.last_bind_group.as_ref().unwrap(), &[]);
                compute_pass.dispatch_workgroups(work_groups.0, work_groups.1, work_groups.2);
            }
        );

        Ok(())
    }
    /*
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
        wgpu_profiler!(
            "to_screen",
            &mut self.gpu_context.profiler,
            encoder,
            &self.gpu_context.device,
            {
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
                    &buffer.buffer,
                    0,
                    size_of_val(data) as u64,
                );
            }
        );
        Ok(())
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
        wgpu_profiler!(
            "to_screen",
            &mut self.gpu_context.profiler,
            encoder,
            &self.gpu_context.device,
            {
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

                        let (copy_texture, _) = self.gpu_context.device.init_texture_with_data(
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
                                texture: &texture.texture,
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
            }
        );

        Ok(())
    }*/
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

        self.gpu_context.profiler.end_frame().unwrap();
        if let Some(timings) = self.gpu_context.profiler.process_finished_frame() {
            self.gpu_context.frame_timings = timings;
        }
    }
}

impl<'a> Drop for DrawEncoder<'a> {
    fn drop(&mut self) {
        puffin::profile_function!();
        drop(self.encoder.take());
        let surface = self.surface_texture.take().unwrap();
        surface.present();
    }
}

pub fn div_ceil(val: u32, div: u32) -> u32 {
    (val / div) + (val % div)
}
