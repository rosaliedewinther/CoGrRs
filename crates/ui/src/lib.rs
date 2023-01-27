use gpu::Context;
use std::fmt::Display;
use std::{cmp::max, time::Duration};
struct SliderData<T> {
    min: T,
    max: T,
    current: T,
}
struct MetricData {
    pub values: std::vec::Vec<f32>,
    pub current_index: usize,
    pub max_index: usize,
    pub min_index: usize,
    pub handled_indices: i32,
    pub rolling_average: f32,
}
impl MetricData {
    pub fn new(size: usize) -> Self {
        MetricData {
            values: vec![0f32; size],
            current_index: 0,
            max_index: 0,
            min_index: 0,
            handled_indices: 0,
            rolling_average: 0f32,
        }
    }
}

pub trait Comboboxable: Display + Copy {
    fn get_variants() -> Vec<Self>;
}

pub struct MainGui {
    imgui: imgui::Context,
    renderer: imgui_wgpu::Renderer,
    platform: imgui_winit_support::WinitPlatform,
    toggles: std::collections::HashMap<String, bool>,
    texts: std::collections::HashMap<String, String>,
    performance_metric: std::collections::HashMap<String, MetricData>,
    slider: std::collections::HashMap<String, SliderData<f32>>,
    combos: std::collections::HashMap<String, (usize, Vec<String>)>,
}

impl MainGui {
    pub fn new(gpu_context: &Context, window: &winit::window::Window) -> Self {
        let mut imgui = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), window, imgui_winit_support::HiDpiMode::Default);
        imgui.set_ini_filename(None);

        let font_size = (13.0 * window.scale_factor()) as f32;
        imgui.io_mut().font_global_scale = (1.0 / window.scale_factor()) as f32;

        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        //
        // Set up dear imgui wgpu renderer
        //

        let renderer_config = imgui_wgpu::RendererConfig {
            texture_format: gpu_context.config.format,
            ..Default::default()
        };

        let renderer = imgui_wgpu::Renderer::new(&mut imgui, &gpu_context.device, &gpu_context.queue, renderer_config);
        Self {
            imgui,
            renderer,
            platform,
            toggles: Default::default(),
            texts: Default::default(),
            performance_metric: Default::default(),
            slider: Default::default(),
            combos: Default::default(),
        }
    }

    pub fn draw(
        &mut self,
        gpu_context: &Context,
        encoder: &mut wgpu::CommandEncoder,
        window: &winit::window::Window,
        mouse_pos: [f32; 2],
        mouse_button_down: bool,
    ) {
        self.imgui.io_mut().mouse_pos = mouse_pos;
        self.imgui.io_mut().mouse_down[0] = mouse_button_down;
        self.imgui.io_mut().update_delta_time(Duration::from_secs_f32(0.01f32));

        let ui = self.imgui.frame();

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

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: gpu_context
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
            .render(imgui::Context::render(&mut self.imgui), &gpu_context.queue, &gpu_context.device, &mut rpass)
            .expect("Rendering failed");
    }
    pub fn slider(&mut self, name: &str, min_val: f32, max_val: f32, value: &mut f32) {
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
    pub fn set_toggle(&mut self, name: &str, state: &mut bool) -> bool {
        match self.toggles.get(name) {
            Some(toggle) => *state = *toggle,
            None => {
                self.toggles.insert(name.to_string(), *state);
            }
        };
        *state
    }
    pub fn remove_toggle(&mut self, name: &str) {
        self.toggles.remove(name);
    }
    pub fn text(&mut self, entry_name: &str, text: &str) {
        self.texts.insert(entry_name.to_string(), text.to_string());
    }
    pub fn combobox<Enum>(&mut self, combo_name: &str, item: &mut Enum)
    where
        Enum: Comboboxable + 'static,
    {
        match self.combos.get(combo_name) {
            Some(value) => {
                *item = Enum::get_variants()[value.0];
            }
            None => {
                self.combos
                    .insert(combo_name.to_string(), (0, Enum::get_variants().iter().map(|val| val.to_string()).collect()));
            }
        }
    }
    pub fn metric(&mut self, graph_name: &str, size: usize, val: f32) {
        match self.performance_metric.get_mut(graph_name) {
            None => {
                self.performance_metric.insert(graph_name.to_string(), MetricData::new(size));
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
}
