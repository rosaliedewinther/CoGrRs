use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::Context;

pub fn init_texture<T>(
    gpu_context: &Context,
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

    let texture_size = wgpu::Extent3d {
        width: dims.0,
        height: dims.1,
        depth_or_array_layers: dims.2,
    };
    let texture_dimension = match dims.2 {
        1 => wgpu::TextureDimension::D2,
        _ => wgpu::TextureDimension::D3,
    };
    let texture_view_dimension = match dims.2 {
        1 => wgpu::TextureViewDimension::D2,
        _ => wgpu::TextureViewDimension::D3,
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
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            },
            bytemuck::cast_slice(data),
        ),
        None => gpu_context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(texture_name),
            format,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: texture_dimension,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
        }),
    };

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
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
