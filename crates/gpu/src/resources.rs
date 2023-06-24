use std::sync::Arc;
use parking_lot::{Mutex, MutexGuard};

use anyhow::{anyhow, Result};
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureUsages, TextureViewDescriptor, TextureViewDimension, util::DeviceExt};
use std::fmt::Debug;

pub enum ResourceHandle<'a>{
    T(&'a TextureHandle),
    B(&'a BufferHandle)
}

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

impl From<u64> for BufferSize{
    fn from(value: u64) -> Self {
        BufferSize::Custom(value)
    }
}
impl From<usize> for BufferSize{
    fn from(value: usize) -> Self {
        BufferSize::Custom(value as u64)
    }
}

fn match_buffer_size(
    config: &wgpu::SurfaceConfiguration,
    elements: &BufferSize,
    element_size: u32,
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
    pub element_size: u32,
    pub buffer: Option<wgpu::Buffer>,
}

impl Buffer {
    pub fn new(name: String, elements: BufferSize, element_size: u32) -> Self {
        Self {
            name,
            elements,
            element_size,
            buffer: None,
        }
    }
}

pub trait ResourceHandleConvertable<'a>: Debug{
    fn to_resource_handle(&'a self) -> ResourceHandle<'a>;
    fn get_index(&self) -> usize;
    fn clone(&self) -> Self;
    fn new(index: usize) -> Self;
    fn reference_count(&self) -> usize;
    fn get_mut(&mut self) -> MutexGuard<'_, usize>;
    fn ptr_eq(&self, other: &Self) -> bool;
}

#[derive(Debug)]
pub struct TextureHandle(Arc<Mutex<usize>>);
#[derive(Debug)]
pub struct BufferHandle(Arc<Mutex<usize>>);

impl<'a> ResourceHandleConvertable<'a> for TextureHandle{
    fn to_resource_handle(&'a self) -> ResourceHandle<'a> {
        ResourceHandle::T(self)
    }
    fn get_index(&self) -> usize {
        self.0.lock().clone()
    }
    fn clone(&self) -> Self {
        TextureHandle(self.0.clone())
    }
    fn new(index: usize) -> Self {
        TextureHandle(Arc::new(Mutex::new(index)))
    }
    fn reference_count(&self) -> usize {
        Arc::strong_count(&self.0) +  Arc::weak_count(&self.0)
    }
    fn get_mut(&mut self) -> MutexGuard<'_, usize>{
        self.0.lock()
    }
    fn ptr_eq(&self, other: &Self) -> bool{
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<'a> ResourceHandleConvertable<'a> for BufferHandle{
    fn to_resource_handle(&'a self) -> ResourceHandle<'a> {
        ResourceHandle::B(self)
    }
    fn get_index(&self) -> usize {
        self.0.lock().clone()
    }
    fn clone(&self) -> Self {
        BufferHandle(self.0.clone())
    }
    fn new(index: usize) -> Self {
        BufferHandle(Arc::new(Mutex::new(index)))
    }
    fn reference_count(&self) -> usize {
        Arc::strong_count(&self.0) +  Arc::weak_count(&self.0)
    }
    fn get_mut(&mut self) -> MutexGuard<'_, usize> {
        self.0.lock()
    }
    fn ptr_eq(&self, other: &Self) -> bool{
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub struct ResourcePool {
    pub recreate_resources: bool,
    pub buffers: Vec<Buffer>,
    pub textures: Vec<Texture>,
    pub buffer_handles: Vec<BufferHandle>,
    pub texture_handles: Vec<TextureHandle>,
}

impl ResourcePool {
    pub fn new() -> Self {
        ResourcePool {
            recreate_resources: false,
            buffers: Vec::new(),
            textures: Vec::new(),
            buffer_handles: Vec::new(),
            texture_handles: Vec::new(),
        }
    }

    pub fn grab_texture(&self, handle: &TextureHandle) -> &Texture{
        &self.textures[handle.get_index() as usize]
    }
    pub fn grab_buffer(&self, handle: &BufferHandle) -> &Buffer{
        &self.buffers[handle.get_index() as usize]
    }

    pub fn texture(
        &mut self,
        name: String,
        resolution: TextureRes,
        format: wgpu::TextureFormat,
    ) -> TextureHandle {
        let texture = Texture::new(name, resolution, format);
        let handle = TextureHandle::new(self.textures.len());
        self.textures.push(texture);
        self.texture_handles.push(handle.clone());
        handle
    }

    pub fn buffer(
        &mut self,
        name: String,
        elements: BufferSize,
        element_size: u32,
    ) -> BufferHandle {
        let buffer = Buffer::new(name, elements, element_size);
        let handle = BufferHandle::new(self.buffers.len());
        self.buffers.push(buffer);
        self.buffer_handles.push(handle.clone());
        handle
    }

    pub fn print_resources(&self) {
        self.buffers
            .iter()
            .enumerate()
            .for_each(|(index, buffer)| println!(
                    "Index {}: \n\tBuffer: {} \n\tElements: {:?} \n\tElement size: {} \n\tAllocated: {}",
                    index,
                    buffer.name,
                    buffer.elements,
                    buffer.element_size,
                    buffer.buffer.is_some()
                )
            
            );
            self.textures
            .iter()
            .enumerate().for_each(|(index, texture)|
                println!(
                    "Index {}: \n\tTexture: {} \n\tResolution: {:?} \n\tFormat: {:?} \n\tAllocated: {}",
                    index,
                    texture.name,
                    texture.resolution,
                    texture.format,
                    texture.texture.is_some())
                );
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
                self.buffer_handles.iter_mut().for_each(|handle|{
                    let mut lock = handle.get_mut();
                    if *lock > i{
                        *lock -= 1;
                    }
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
                self.buffer_handles.iter_mut().for_each(|handle|{
                    let mut lock = handle.get_mut();
                    if *lock > i{
                        *lock -= 1;
                    }
                });
                continue;
            }
            i += 1;
        }

        self.textures
            .iter_mut()
            .for_each(|texture|
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
                    

            });

        self.buffers
            .iter_mut()
            .for_each(|buffer| 
                    if buffer.buffer.is_none() {
                        buffer.buffer = Some(init_storage_buffer(
                            device,
                            &buffer.name,
                            match_buffer_size(config, &buffer.elements, buffer.element_size),
                        ));
                    
            });
    }

    
}

pub fn init_texture(
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

pub fn init_texture_with_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_name: &str,
    dims: (u32, u32, u32),
    format: wgpu::TextureFormat,
    data: &[u8]
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

    let texture = 
        device.create_texture_with_data(
            &queue,
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

pub fn init_storage_buffer(device: &wgpu::Device, buffer_name: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(buffer_name),
        size: size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}
