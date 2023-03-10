use bytemuck::Pod;
use wgpu::{util::DeviceExt, Extent3d, TextureDescriptor, TextureDimension, TextureUsages, TextureViewDescriptor, TextureViewDimension};

use super::CoGrWGPU;

pub fn init_texture<T>(
    gpu_context: &CoGrWGPU,
    texture_name: &str,
    dims: (u32, u32, u32),
    format: wgpu::TextureFormat,
    data: Option<&[T]>,
) -> (wgpu::Texture, wgpu::TextureView)
where
    T: Pod,
{
    if dims.0 == 0 || dims.1 == 0 || dims.2 == 0 {
        panic!(
            "dim size of texture: {} was incorrect namely: {:?}, every dimension must be at least 1",
            texture_name, dims
        )
    }

    let texture_size = Extent3d {
        width: dims.0,
        height: dims.1,
        depth_or_array_layers: dims.2,
    };
    let texture_dimension = match dims.2 {
        1 => TextureDimension::D2,
        _ => TextureDimension::D3,
    };
    let texture_view_dimension = match dims.2 {
        1 => TextureViewDimension::D2,
        _ => TextureViewDimension::D3,
    };

    let texture = match data {
        Some(data) => gpu_context.device.create_texture_with_data(
            &gpu_context.queue,
            &wgpu::TextureDescriptor {
                label: Some(texture_name),
                format,
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: texture_dimension,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
                view_formats: &[format],
            },
            bytemuck::cast_slice(data),
        ),
        None => gpu_context.device.create_texture(&TextureDescriptor {
            label: Some(texture_name),
            format,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: texture_dimension,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
            view_formats: &[format],
        }),
    };

    let texture_view = texture.create_view(&TextureViewDescriptor {
        label: Some(&(texture_name.to_string() + "_view")),
        format: Some(format),
        dimension: Some(texture_view_dimension),
        base_mip_level: 0,
        aspect: Default::default(),
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    });
    (texture, texture_view)
}
