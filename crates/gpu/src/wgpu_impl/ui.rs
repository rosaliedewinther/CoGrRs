use egui_wgpu::renderer::ScreenDescriptor;
use egui_winit::State;

use wgpu::RenderPassDescriptor;
use winit::{event::WindowEvent, event_loop::EventLoop, window::Window};

use crate::UI;

use super::{encoder::EncoderWGPU, CoGrWGPU};

pub struct UiWGPU {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    state: State,
}

impl UI for UiWGPU {
    fn new(gpu_context: &CoGrWGPU, _window: &Window, event_loop: &EventLoop<()>) -> Self {
        let renderer = egui_wgpu::renderer::Renderer::new(&gpu_context.device, gpu_context.config.format, None, 1);
        let context = egui::Context::default();
        let state = egui_winit::State::new(event_loop);

        Self { context, renderer, state }
    }

    fn draw(&mut self, encoder: &mut EncoderWGPU, window: &Window, ui_builder: impl FnOnce(&mut egui::Ui)) {
        let ctx = &encoder.gpu_context;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [ctx.config.width, ctx.config.height],
            pixels_per_point: 1f32,
        };
        let full_output = self.context.run(self.state.take_egui_input(window), |ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui_builder(ui);
            });
        });

        let paint_jobs = self.context.tessellate(full_output.shapes);
        let tdelta = full_output.textures_delta;

        {
            for d in tdelta.set {
                self.renderer.update_texture(&ctx.device, &ctx.queue, d.0, &d.1);
            }
            self.renderer
                .update_buffers(&ctx.device, &ctx.queue, encoder.encoder.as_mut().unwrap(), &paint_jobs, &screen_descriptor);
            let mut render_pass = encoder.encoder.as_mut().unwrap().begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: encoder.surface_texture_view.as_ref().expect("there is no surface texture"),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                ..Default::default()
            });
            self.renderer.render(&mut render_pass, paint_jobs.as_slice(), &screen_descriptor);
        }
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        let _ = self.state.on_event(&self.context, event);
    }
}
