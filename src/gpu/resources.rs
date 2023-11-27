use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    ops::SubAssign,
    rc::Rc,
};

use std::fmt::Debug;
use tracing::info;

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
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
}

impl Texture {
    fn new(name: String, texture: wgpu::Texture, texture_view: wgpu::TextureView) -> Self {
        Self {
            name,
            texture,
            texture_view,
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    pub name: String,
    pub buffer: wgpu::Buffer,
}

impl Buffer {
    pub fn new(name: String, buffer: wgpu::Buffer) -> Self {
        Self {
            name,
            buffer,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResourceHandle {
    Texture(Rc<RefCell<usize>>),
    Buffer(Rc<RefCell<usize>>),
}

impl ResourceHandle {
    pub fn get_index(&self) -> usize {
        match self {
            ResourceHandle::Texture(t) => *t.borrow(),
            ResourceHandle::Buffer(b) => *b.borrow(),
        }
    }
    pub fn new_t(index: usize) -> Self {
        ResourceHandle::Texture(Rc::new(RefCell::new(index)))
    }
    pub fn new_b(index: usize) -> Self {
        ResourceHandle::Buffer(Rc::new(RefCell::new(index)))
    }
    pub fn reference_count(&self) -> usize {
        match self {
            ResourceHandle::Texture(t) => Rc::strong_count(t) + Rc::weak_count(t),
            ResourceHandle::Buffer(b) => Rc::strong_count(b) + Rc::weak_count(b),
        }
    }
    pub fn decrement(&mut self) {
        match self {
            ResourceHandle::Texture(t) => t.borrow_mut().sub_assign(1),
            ResourceHandle::Buffer(b) => b.borrow_mut().sub_assign(1),
        };
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResourceHandle::Texture(h1), ResourceHandle::Texture(h2)) => Rc::ptr_eq(h1, h2),
            (ResourceHandle::Texture(h1), ResourceHandle::Buffer(h2)) => Rc::ptr_eq(h1, h2),
            (ResourceHandle::Buffer(h1), ResourceHandle::Texture(h2)) => Rc::ptr_eq(h1, h2),
            (ResourceHandle::Buffer(h1), ResourceHandle::Buffer(h2)) => Rc::ptr_eq(h1, h2),
        }
    }
}

impl Hash for ResourceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ResourceHandle::Texture(t) => t.as_ptr().hash(state),
            ResourceHandle::Buffer(b) => b.as_ptr().hash(state),
        }
    }
}

#[derive(Default, Debug)]
pub struct ResourcePool {
    pub(crate) recreate_resources: bool,
    pub(crate) buffers: Vec<Buffer>,
    pub(crate) textures: Vec<Texture>,
    pub(crate) buffer_handles: Vec<ResourceHandle>,
    pub(crate) texture_handles: Vec<ResourceHandle>,
}

impl ResourcePool {
    pub fn grab_texture(&self, handle: &ResourceHandle) -> &Texture {
        &self.textures[handle.get_index()]
    }
    pub fn grab_buffer(&self, handle: &ResourceHandle) -> &Buffer {
        &self.buffers[handle.get_index()]
    }

    pub(crate) fn texture(
        &mut self,
        name: String,
        texture: wgpu::Texture,
        texture_view: wgpu::TextureView,
    ) -> ResourceHandle {
        puffin::profile_function!();
        info!(
            "creating texture {} with {:?} and view {:?}",
            name, texture, texture_view
        );
        let texture = Texture::new(name, texture, texture_view);
        let handle = ResourceHandle::new_t(self.textures.len());
        self.textures.push(texture);
        self.texture_handles.push(handle.clone());
        handle
    }

    pub(crate) fn buffer(
        &mut self,
        name: String,
        buffer: wgpu::Buffer,
    ) -> ResourceHandle {
        puffin::profile_function!();
        info!(
            "creating buffer {} with {:?}",
            name, buffer
        );
        let buffer = Buffer::new(name, buffer);
        let handle = ResourceHandle::new_b(self.buffers.len());
        self.buffers.push(buffer);
        self.buffer_handles.push(handle.clone());
        handle
    }

    pub(crate) fn clean_up_resources(&mut self) {
        puffin::profile_function!();
        info!("{:?}", self.buffer_handles);
        // remove all resources which are only referenced by resource pool
        let mut i = 0;
        while i < self.buffer_handles.len() {
            let handle = &self.buffer_handles[i];
            if handle.reference_count() == 1 {
                info!(
                    "removing buffer at index {}, {} buffer(s) left",
                    i,
                    self.buffers.len() - 1
                );
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
                info!(
                    "removing texture at index {}, {} texture(s) left",
                    i,
                    self.textures.len() - 1
                );
                self.textures.remove(i);
                self.texture_handles.remove(i);
                self.texture_handles.iter_mut().for_each(|handle| {
                    handle.decrement();
                });
                continue;
            }
            i += 1;
        }
        info!("{:?}", self.buffer_handles);
    }

    pub(crate) fn prepare_resources(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) {
        puffin::profile_function!();
        self.clean_up_resources();
    }
}
/*
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
        usage: TextureUsages::STORAGE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::COPY_SRC
            | TextureUsages::TEXTURE_BINDING,
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
        usage: wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::UNIFORM
            | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    })
}*/
