use bytemuck::{Pod};
use wgpu::TextureFormat;
use wgpu_impl::Execution;
use winit::window::Window;

pub use wgpu;
mod shader;
pub mod wgpu_impl;

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
