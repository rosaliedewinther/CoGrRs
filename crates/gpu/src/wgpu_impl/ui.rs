use egui_wgpu::renderer::ScreenDescriptor;
use egui_winit::State;

use wgpu::RenderPassDescriptor;
use winit::{event_loop::EventLoop, window::Window};

use crate::{
    ui::{MetricData, SliderData},
    UI,
};

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

    fn draw(&mut self, encoder: &mut EncoderWGPU, window: &winit::window::Window, ui_builder: impl FnOnce(&mut egui::Ui)) {
        let ctx = &encoder.gpu_context;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [ctx.config.width, ctx.config.height],
            pixels_per_point: 1f32,
        };
        // let tdelta: egui::TexturesDelta = full_output.textures_delta;

        // Record all render passes.

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

        // Redraw egui
        // output_frame.present();

        //egui_rpass.remove_textures(tdelta).expect("remove texture ok");
        /*

        {
            let window = ui.window("debug window");
            window
                .size([200.0, 500.0], imgui::Condition::FirstUseEver)
                .position([0.0, 0.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    for value in &self.texts {
                        ui.text(format!("{}: {:?}", value.0, value.1));
                    }
                    for (toggle_name, toggle) in &mut self.toggles {
                        *toggle = ui.button(toggle_name);
                    }
                    for (slider_name, sliderdata) in &mut self.slider {
                        imgui::Ui::slider(ui, slider_name, sliderdata.min, sliderdata.max, &mut sliderdata.current);
                    }
                    for (metric_name, metric) in &self.performance_metric {
                        ui.text(format!(
                            "metric: {}\nnew: {}\nmin: {}\nmax: {}\naverage: {}",
                            metric_name,
                            metric.values[if metric.current_index == 0 {
                                metric.values.len() - 1
                            } else {
                                metric.current_index - 1
                            }],
                            metric.values[metric.min_index],
                            metric.values[metric.max_index],
                            metric.rolling_average
                        ));
                    }
                    for (combo_name, (current_item, items)) in &mut self.combos {
                        ui.combo_simple_string(combo_name, current_item, items);
                    }
                });

            //ui.show_demo_window(&mut true);
        }
        {
            let mut rpass = encoder
                .encoder
                .as_mut()
                .expect("somehow encoder was not made")
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ui_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: encoder
                            .surface_texture_view
                            .as_ref()
                            .expect("surface texture view is not available, be sure to call get_encoder_for_draw() before trying to render ui"),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

            self.platform.prepare_render(ui, window);
            self.renderer
                .render(
                    imgui::Context::render(&mut self.imgui),
                    &encoder.gpu_context.queue,
                    &encoder.gpu_context.device,
                    &mut rpass,
                )
                .expect("Rendering failed");
        }*/
    }

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.state.on_event(&self.context, event);
    }
}
