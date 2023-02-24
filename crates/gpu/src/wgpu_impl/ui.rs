
use egui_wgpu::renderer::ScreenDescriptor;
use egui_winit::State;
use std::cmp::max;
use wgpu::RenderPassDescriptor;
use winit::{
    event_loop::{EventLoop},
    window::Window,
};

use crate::{
    ui::{MetricData, SliderData},
    ComboBoxable, UI,
};

use super::{encoder::EncoderWGPU, CoGrWGPU};

pub struct UiWGPU {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    state: State,
    toggles: std::collections::HashMap<String, bool>,
    texts: std::collections::HashMap<String, String>,
    performance_metric: std::collections::HashMap<String, MetricData>,
    slider: std::collections::HashMap<String, SliderData<f32>>,
    combos: std::collections::HashMap<String, (usize, &'static [&'static str])>,
}

impl UI for UiWGPU {
    fn new(gpu_context: &CoGrWGPU, _window: &Window, event_loop: &EventLoop<()>) -> Self {
        let renderer = egui_wgpu::renderer::Renderer::new(&gpu_context.device, gpu_context.config.format, None, 1);
        let context = egui::Context::default();
        let state = egui_winit::State::new(event_loop);

        Self {
            context,
            renderer,
            state,
            toggles: Default::default(),
            texts: Default::default(),
            performance_metric: Default::default(),
            slider: Default::default(),
            combos: Default::default(),
        }
    }

    fn draw(&mut self, encoder: &mut EncoderWGPU, window: &winit::window::Window) {
        let ctx = &encoder.gpu_context;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [ctx.config.width, ctx.config.height],
            pixels_per_point: 1f32,
        };
        // let tdelta: egui::TexturesDelta = full_output.textures_delta;

        // Record all render passes.

        let full_output = self.context.run(self.state.take_egui_input(window), |ctx| {
            egui::Window::new("debug").show(ctx, |ui| {
                ui.label("Hello world!");
                if ui.button("Click me").clicked() {
                    // take some action here
                }
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
    fn slider(&mut self, name: &str, min_val: f32, max_val: f32, value: &mut f32) {
        debug_assert!(min_val <= max_val);
        debug_assert!(*value >= min_val && *value <= max_val);

        if let Some(slider_value) = self.slider.get(name) {
            debug_assert!(slider_value.min == min_val);
            debug_assert!(slider_value.max == max_val);
            debug_assert!(slider_value.current >= min_val && slider_value.current <= max_val);
            *value = slider_value.current;
            //.expect(&format!("slider value {:?} was not convertible to i32", value));
            return;
        }

        self.slider.insert(
            name.to_string(),
            SliderData {
                min: min_val,
                max: max_val,
                current: *value,
            },
        );
    }
    // returns the new value
    fn toggle(&mut self, name: &str, state: &mut bool) {
        match self.toggles.get(name) {
            Some(toggle) => *state = *toggle,
            None => {
                self.toggles.insert(name.to_string(), *state);
            }
        };
    }
    fn text(&mut self, entry_name: &str, text: &str) {
        self.texts.insert(entry_name.to_string(), text.to_string());
    }
    fn combobox<Enum: ComboBoxable>(&mut self, combo_name: &str, item: &mut Enum) {
        match self.combos.get(combo_name) {
            Some(value) => {
                *item = Enum::get_variant(value.0);
            }
            None => {
                self.combos.insert(combo_name.to_string(), (0, (Enum::get_names())));
            }
        }
    }
    fn metric(&mut self, graph_name: &str, size: u32, val: f32) {
        match self.performance_metric.get_mut(graph_name) {
            None => {
                self.performance_metric.insert(graph_name.to_string(), MetricData::new(size as usize));
            }
            Some(metric_data) => {
                if metric_data.handled_indices == 0 {
                    metric_data.rolling_average = val;
                } else {
                    metric_data.rolling_average =
                        (1f32 / metric_data.values.len() as f32) * val + (1f32 - 1f32 / metric_data.values.len() as f32) * metric_data.rolling_average;
                }

                // set min/max indices on new val
                if val <= metric_data.values[metric_data.min_index] {
                    metric_data.min_index = metric_data.current_index;
                }
                if val >= metric_data.values[metric_data.max_index] {
                    metric_data.max_index = metric_data.current_index;
                }
                // set val
                metric_data.values[metric_data.current_index] = val;
                //update min/max index when overwriting
                if metric_data.min_index == metric_data.current_index {
                    let mut min_i = 0;
                    for i in 0..metric_data.handled_indices as usize {
                        if metric_data.values[i] < metric_data.values[min_i] {
                            min_i = i;
                        }
                    }
                    metric_data.min_index = min_i;
                }
                if metric_data.max_index == metric_data.current_index {
                    let mut max_i = 0;
                    for i in 0..metric_data.handled_indices as usize {
                        if metric_data.values[i] > metric_data.values[max_i] {
                            max_i = i;
                        }
                    }
                    metric_data.max_index = max_i;
                }

                metric_data.current_index += 1;
                metric_data.handled_indices = max(metric_data.handled_indices, metric_data.current_index as i32);
                // make sure to wrap around when needed
                if metric_data.current_index == metric_data.values.len() {
                    metric_data.current_index = 0;
                }
            }
        };
    }

    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.state.on_event(&self.context, event);
    }

    fn slideri(&mut self, _name: &str, _min_value: i32, _max_val: i32, _value: &mut i32) {
        todo!()
    }
}
