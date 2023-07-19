use parking_lot::Mutex;
use std::{
    hash::{Hash, Hasher},
    ops::Sub,
    sync::Arc,
};

use anyhow::{anyhow, Result};
use std::fmt::Debug;
use wgpu::{
    util::DeviceExt, Extent3d, TextureDescriptor, TextureDimension, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

#[derive(Debug)]
pub enum TextureRes {
    FullRes,
    HalfRes,
    QuarterRes,
    EightRes,
    SixteenthRes,
    ThirtySecondRes,
    Custom(u32, u32, u32),
}
fn match_resolution(
    config: &wgpu::SurfaceConfiguration,
    texture_resolution: &TextureRes,
) -> (u32, u32, u32) {
    match texture_resolution {
        TextureRes::FullRes => (config.width, config.height, 1),
        TextureRes::HalfRes => (config.width / 2, config.height / 2, 1),
        TextureRes::QuarterRes => (config.width / 4, config.height / 4, 1),
        TextureRes::EightRes => (config.width / 8, config.height / 8, 1),
        TextureRes::SixteenthRes => (config.width / 16, config.height / 16, 1),
        TextureRes::ThirtySecondRes => (config.width / 32, config.height / 32, 1),
        TextureRes::Custom(x, y, z) => (*x, *y, *z),
    }
}

#[derive(Debug)]
pub enum BufferSize {
    FullRes,
    HalfRes,
    QuarterRes,
    EightRes,
    SixteenthRes,
    ThirtySecondRes,
    Custom(u64),
}

impl From<u64> for BufferSize {
    fn from(value: u64) -> Self {
        BufferSize::Custom(value)
    }
}
impl From<usize> for BufferSize {
    fn from(value: usize) -> Self {
        BufferSize::Custom(value as u64)
    }
}
impl From<i32> for BufferSize {
    fn from(value: i32) -> Self {
        BufferSize::Custom(value as u64)
    }
}

fn match_buffer_size(
    config: &wgpu::SurfaceConfiguration,
    elements: &BufferSize,
    element_size: usize,
) -> u64 {
    let width = config.width as u64;
    let height = config.height as u64;
    match elements {
        BufferSize::FullRes => width * height * element_size as u64,
        BufferSize::HalfRes => width * height * element_size as u64 / 2,
        BufferSize::QuarterRes => width * height * element_size as u64 / 4,
        BufferSize::EightRes => width * height * element_size as u64 / 8,
        BufferSize::SixteenthRes => width * height * element_size as u64 / 16,
        BufferSize::ThirtySecondRes => width * height * element_size as u64 / 32,
        BufferSize::Custom(x) => x * element_size as u64,
    }
}

#[derive(Debug)]
pub struct Texture {
    pub name: String,
    pub resolution: TextureRes,
    pub format: wgpu::TextureFormat,
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
}

impl Texture {
    fn new(name: String, resolution: TextureRes, format: wgpu::TextureFormat) -> Self {
        Self {
            name,
            resolution,
            format,
            texture: None,
            texture_view: None,
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    pub name: String,
    pub elements: BufferSize,
    pub element_size: usize,
    pub buffer: Option<wgpu::Buffer>,
}

impl Buffer {
    pub fn new(name: String, elements: BufferSize, element_size: usize) -> Self {
        Self {
            name,
            elements,
            element_size,
            buffer: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResourceHandle {
    Texture(Arc<Mutex<usize>>),
    Buffer(Arc<Mutex<usize>>),
}

impl ResourceHandle {
    pub fn get_index(&self) -> usize {
        match self {
            ResourceHandle::Texture(t) => *t.lock(),
            ResourceHandle::Buffer(b) => *b.lock(),
        }
    }
    pub fn new_t(index: usize) -> Self {
        ResourceHandle::Texture(Arc::new(Mutex::new(index)))
    }
    pub fn new_b(index: usize) -> Self {
        ResourceHandle::Buffer(Arc::new(Mutex::new(index)))
    }
    pub fn reference_count(&self) -> usize {
        match self {
            ResourceHandle::Texture(t) => Arc::strong_count(t) + Arc::weak_count(t),
            ResourceHandle::Buffer(b) => Arc::strong_count(b) + Arc::weak_count(b),
        }
    }
    pub fn decrement(&mut self) {
        match self {
            ResourceHandle::Texture(t) => t.lock().sub(1),
            ResourceHandle::Buffer(b) => b.lock().sub(1),
        };
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResourceHandle::Texture(h1), ResourceHandle::Texture(h2)) => Arc::ptr_eq(h1, h2),
            (ResourceHandle::Texture(h1), ResourceHandle::Buffer(h2)) => Arc::ptr_eq(h1, h2),
            (ResourceHandle::Buffer(h1), ResourceHandle::Texture(h2)) => Arc::ptr_eq(h1, h2),
            (ResourceHandle::Buffer(h1), ResourceHandle::Buffer(h2)) => Arc::ptr_eq(h1, h2),
        }
    }
}

impl Hash for ResourceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ResourceHandle::Texture(t) => t.data_ptr().hash(state),
            ResourceHandle::Buffer(b) => b.data_ptr().hash(state),
        }
    }
}

#[derive(Default)]
pub(crate) struct ResourcePool {
    pub recreate_resources: bool,
    pub buffers: Vec<Buffer>,
    pub textures: Vec<Texture>,
    pub buffer_handles: Vec<ResourceHandle>,
    pub texture_handles: Vec<ResourceHandle>,
}

impl ResourcePool {
    pub fn grab_texture(&self, handle: &ResourceHandle) -> &Texture {
        &self.textures[handle.get_index()]
    }
    pub fn grab_buffer(&self, handle: &ResourceHandle) -> &Buffer {
        &self.buffers[handle.get_index()]
    }

    pub fn texture(
        &mut self,
        name: String,
        resolution: TextureRes,
        format: wgpu::TextureFormat,
    ) -> ResourceHandle {
        let texture = Texture::new(name, resolution, format);
        let handle = ResourceHandle::new_t(self.textures.len());
        self.textures.push(texture);
        self.texture_handles.push(handle.clone());
        handle
    }

    pub fn buffer(
        &mut self,
        name: String,
        elements: BufferSize,
        element_size: usize,
    ) -> ResourceHandle {
        let buffer = Buffer::new(name, elements, element_size);
        let handle = ResourceHandle::new_b(self.buffers.len());
        self.buffers.push(buffer);
        self.buffer_handles.push(handle.clone());
        handle
    }

    pub fn print_resources(&self) {
        self.buffers.iter().enumerate().for_each(|(index, buffer)| {
            println!(
                "Index {}: \n\tBuffer: {} \n\tElements: {:?} \n\tElement size: {} \n\tAllocated: {}",
                index,
                buffer.name,
                buffer.elements,
                buffer.element_size,
                buffer.buffer.is_some()
            )
        });
        self.textures
            .iter()
            .enumerate()
            .for_each(|(index, texture)| {
                println!(
                "Index {}: \n\tTexture: {} \n\tResolution: {:?} \n\tFormat: {:?} \n\tAllocated: {}",
                index,
                texture.name,
                texture.resolution,
                texture.format,
                texture.texture.is_some()
            )
            });
    }

    pub fn prepare_resources(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) {
        // remove all resources which are only referenced by resource pool
        let mut i = 0;
        while i < self.buffer_handles.len() {
            let handle = &self.buffer_handles[i];
            if handle.reference_count() == 1 {
                self.buffers.remove(i);
                self.buffer_handles.remove(i);
                self.buffer_handles.iter_mut().for_each(|handle| {
                    handle.decrement();
                });
                continue;
            }
            i += 1;
        }
        let mut i = 0;
        while i < self.texture_handles.len() {
            let handle = &self.texture_handles[i];
            if handle.reference_count() == 1 {
                self.textures.remove(i);
                self.texture_handles.remove(i);
                self.buffer_handles.iter_mut().for_each(|handle| {
                    handle.decrement();
                });
                continue;
            }
            i += 1;
        }

        if self.recreate_resources {
            self.textures.iter_mut().for_each(|texture| {
                texture.texture = None;
                texture.texture_view = None;
            });
            self.recreate_resources = false;
        }

        self.textures.iter_mut().for_each(|texture| {
            if texture.texture.is_none() {
                let (new_texture, new_texture_view) = init_texture(
                    device,
                    &texture.name,
                    match_resolution(config, &texture.resolution),
                    texture.format,
                )
                .unwrap();
                texture.texture = Some(new_texture);
                texture.texture_view = Some(new_texture_view);
            }
        });

        self.buffers.iter_mut().for_each(|buffer| {
            if buffer.buffer.is_none() {
                buffer.buffer = Some(init_storage_buffer(
                    device,
                    &buffer.name,
                    match_buffer_size(config, &buffer.elements, buffer.element_size),
                ));
            }
        });
    }
}

pub(crate) fn init_texture(
    device: &wgpu::Device,
    texture_name: &str,
    dims: (u32, u32, u32),
    format: wgpu::TextureFormat,
) -> Result<(wgpu::Texture, wgpu::TextureView)> {
    if dims.0 == 0 || dims.1 == 0 || dims.2 == 0 {
        Err(anyhow!(
            "dim size of texture: {} was incorrect namely: {:?}, every dimension must be at least 1",
            texture_name,
            dims
        ))?
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

    let texture = device.create_texture(&TextureDescriptor {
        label: Some(texture_name),
        format,
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: texture_dimension,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
        view_formats: &[format],
    });

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
    Ok((texture, texture_view))
}

pub(crate) fn init_texture_with_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_name: &str,
    dims: (u32, u32, u32),
    format: wgpu::TextureFormat,
    data: &[u8],
) -> Result<(wgpu::Texture, wgpu::TextureView)> {
    if dims.0 == 0 || dims.1 == 0 || dims.2 == 0 {
        Err(anyhow!(
            "dim size of texture: {} was incorrect namely: {:?}, every dimension must be at least 1",
            texture_name,
            dims
        ))?
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

    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(texture_name),
            format,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: texture_dimension,
            usage: TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC,
            view_formats: &[format],
        },
        bytemuck::cast_slice(data),
    );

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
    Ok((texture, texture_view))
}

pub(crate) fn init_storage_buffer(
    device: &wgpu::Device,
    buffer_name: &str,
    size: u64,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(buffer_name),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}
