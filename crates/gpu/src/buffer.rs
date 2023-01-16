use crate::Context;

pub fn init_storage_buffer(gpu_context: &Context, buffer_name: &str, size: u32) -> wgpu::Buffer {
    gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(buffer_name),
        size: size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}
