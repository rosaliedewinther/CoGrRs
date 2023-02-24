use bytemuck::Pod;
use shader::Execution;
use wgpu::TextureFormat;
use winit::{
    event::{WindowEvent},
    event_loop::{EventLoop},
    window::Window,
};

pub use wgpu;
pub mod shader;
mod ui;
mod wgpu_impl;

pub trait CoGrEncoder {
    fn image_buffer_to_screen(&mut self);
    fn dispatch_pipeline<PushConstants: Pod>(&mut self, pipeline_name: &str, execution_mode: Execution, push_constants: &PushConstants);
    fn set_buffer_data<T: Pod>(&self, buffer_name: &str, data: &[T], elements_to_copy: usize, element_copy_start: usize);
    fn read_buffer<T: Pod>(&self, buffer_name: &str, elements_to_copy: usize, data: &mut [T]);
    fn set_texture_data<T: Pod>(&mut self, texture_name: &str, data: &[T]);
    fn read_texture<T: Pod>(&self, texture_name: &str, data: &mut [T]);
}
pub trait CoGr {
    type Encoder<'a>
    where
        Self: 'a;
    fn new(window: &Window, to_screen_texture_name: &str, shaders_folder: &str) -> Self;
    fn get_encoder_for_draw<'a>(&'a mut self) -> Self::Encoder<'a>;
    fn get_encoder<'a>(&'a mut self) -> Self::Encoder<'a>;
    fn buffer<Type>(&mut self, buffer_name: &str, number_of_elements: u32);
    fn texture(&mut self, texture_name: &str, number_of_elements: (u32, u32, u32), format: TextureFormat);
    fn refresh_pipelines(&mut self);
}
pub trait ComboBoxable: Copy {
    fn get_names() -> &'static [&'static str];
    fn get_variant(index: usize) -> Self;
}
pub trait UI {
    fn new(gpu_context: &Renderer, window: &Window, event_loop: &EventLoop<()>) -> Self;
    fn draw(&mut self, gpu_context: &mut Encoder, window: &Window);
    fn handle_window_event(&mut self, event: &WindowEvent);
    fn slider(&mut self, name: &str, min_value: f32, max_val: f32, value: &mut f32);
    fn slideri(&mut self, name: &str, min_value: i32, max_val: i32, value: &mut i32);
    fn toggle(&mut self, name: &str, state: &mut bool);
    fn text(&mut self, name: &str, text: &str);
    fn combobox<Enum: ComboBoxable>(&mut self, name: &str, item: &mut Enum);
    fn metric(&mut self, name: &str, size: u32, value: f32);
}

#[cfg(feature = "wgpu")]
pub type Renderer = wgpu_impl::CoGrWGPU;
#[cfg(feature = "wgpu")]
pub type Encoder<'a> = wgpu_impl::encoder::EncoderWGPU<'a>;
#[cfg(feature = "wgpu")]
pub type Ui = crate::wgpu_impl::ui::UiWGPU;
