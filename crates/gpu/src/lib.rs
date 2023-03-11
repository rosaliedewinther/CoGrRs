use bytemuck::Pod;
pub use wgpu::TextureFormat;
use winit::{event::WindowEvent, event_loop::EventLoop, window::Window};

pub use egui;
mod shader;
mod wgpu_impl;

pub enum Execution {
    PerPixel1D,
    PerPixel2D,
    N3D(u32),
    N1D(u32),
}
pub trait CoGrReadHandle {
    fn wait_and_read<'a, T: Pod>(self, gpu_context: &Renderer) -> &'a [T];
}

pub trait CoGrEncoder {
    fn dispatch_pipeline<PushConstants: Pod>(&mut self, pipeline_name: &'static str, execution_mode: Execution, push_constants: &PushConstants);
    fn to_screen(&mut self, texture_name: &'static str);
    fn set_buffer_data<T: Pod>(&mut self, buffer_name: &'static str, data: &[T]);
    fn read_buffer<T: Pod>(&mut self, buffer_name: &'static str) -> ReadHandle;
    fn set_texture_data<T: Pod>(&mut self, texture_name: &'static str, data: &[T]);
    fn read_texture<T: Pod>(&mut self, texture_name: &'static str) -> ReadHandle;
}
pub trait CoGr {
    type Encoder<'a>
    where
        Self: 'a;
    fn new(window: &Window, shaders_folder: &str) -> Self;
    fn get_encoder_for_draw(&mut self) -> Self::Encoder<'_>;
    fn get_encoder(&mut self) -> Self::Encoder<'_>;
    fn buffer<Type>(&mut self, buffer_name: &'static str, number_of_elements: u32);
    fn texture(&mut self, texture_name: &'static str, number_of_elements: (u32, u32, u32), format: TextureFormat);
}
pub trait UI {
    fn new(gpu_context: &Renderer, window: &Window, event_loop: &EventLoop<()>) -> Self;
    fn draw(&mut self, gpu_context: &mut Encoder, window: &Window, ui_builder: impl FnOnce(&mut egui::Ui));
    fn handle_window_event(&mut self, event: &WindowEvent);
}

#[cfg(feature = "wgpu")]
pub type Renderer = wgpu_impl::CoGrWGPU;
#[cfg(feature = "wgpu")]
pub type Encoder<'a> = wgpu_impl::encoder::EncoderWGPU<'a>;
#[cfg(feature = "wgpu")]
pub type Ui = crate::wgpu_impl::ui::UiWGPU;
#[cfg(feature = "wgpu")]
pub type ReadHandle = crate::wgpu_impl::read_handle::WGPUReadhandle;
#[cfg(feature = "wgpu")]
pub use wgpu;
