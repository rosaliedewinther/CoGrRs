use crate::Context;

pub fn init_texture(
    gpu_context: &Context,
    texture_name: &str,
    width: u32,
    height: u32,
    depth: Option<u32>,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture_size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: depth.unwrap_or(1),
    };
    let texture_dimension = match depth {
        Some(n) => {
            if n <= 1 {
                wgpu::TextureDimension::D2
            } else {
                wgpu::TextureDimension::D3
            }
        }
        None => wgpu::TextureDimension::D2,
    };
    let texture_view_dimension = match depth {
        Some(n) => {
            if n <= 1 {
                wgpu::TextureViewDimension::D2
            } else {
                wgpu::TextureViewDimension::D3
            }
        }
        None => wgpu::TextureViewDimension::D2,
    };
    let texture = gpu_context.device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: texture_dimension,
        format,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some(texture_name),
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some(&(texture_name.to_string() + "_view")),
        format: Some(format),
        dimension: Some(texture_view_dimension),
        aspect: Default::default(),
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    });
    (texture, texture_view)
}
