use spirv_reflect::types::{ReflectDescriptorType, ReflectDimension, ReflectImageFormat};
use wgpu::{
    util::make_spirv, BindGroup, BindGroupLayout, ComputePipeline, ShaderModuleDescriptor,
    TextureFormat, TextureViewDimension,
};

use crate::gpu::shader::Shader;

use super::CoGr;

#[derive(Debug)]
pub struct Pipeline {
    pub pipeline_name: String,
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
    pub last_bind_group_hash: u64,
    pub last_bind_group: Option<BindGroup>,
}

fn map_texture_dimension(dimension: &ReflectDimension) -> TextureViewDimension {
    match dimension {
        ReflectDimension::Undefined => unimplemented!(),
        ReflectDimension::Type1d => TextureViewDimension::D1,
        ReflectDimension::Type2d => TextureViewDimension::D2,
        ReflectDimension::Type3d => TextureViewDimension::D3,
        ReflectDimension::Cube => TextureViewDimension::Cube,
        ReflectDimension::Rect => unimplemented!(),
        ReflectDimension::Buffer => unimplemented!(),
        ReflectDimension::SubPassData => unimplemented!(),
    }
}

fn map_texture_format(format: &ReflectImageFormat) -> wgpu::TextureFormat {
    match format {
        ReflectImageFormat::Undefined => unimplemented!(),
        ReflectImageFormat::RGBA32_FLOAT => TextureFormat::Rgba32Float,
        ReflectImageFormat::RGBA16_FLOAT => TextureFormat::Rgba16Float,
        ReflectImageFormat::R32_FLOAT => TextureFormat::R32Float,
        ReflectImageFormat::RGBA8 => TextureFormat::Rgba8Unorm,
        ReflectImageFormat::RGBA8_SNORM => TextureFormat::Rgba8Snorm,
        ReflectImageFormat::RG32_FLOAT => TextureFormat::Rg32Float,
        ReflectImageFormat::RG16_FLOAT => TextureFormat::Rg16Float,
        ReflectImageFormat::R11G11B10_FLOAT => TextureFormat::Rg11b10Float,
        ReflectImageFormat::R16_FLOAT => TextureFormat::R16Float,
        ReflectImageFormat::RGBA16 => TextureFormat::Rgba16Unorm,
        ReflectImageFormat::RGB10A2 => TextureFormat::Rgb10a2Unorm,
        ReflectImageFormat::RG16 => TextureFormat::Rg16Snorm,
        ReflectImageFormat::RG8 => TextureFormat::Rg8Unorm,
        ReflectImageFormat::R16 => TextureFormat::R16Unorm,
        ReflectImageFormat::R8 => TextureFormat::R8Unorm,
        ReflectImageFormat::RGBA16_SNORM => TextureFormat::Rgba16Snorm,
        ReflectImageFormat::RG16_SNORM => TextureFormat::Rg16Snorm,
        ReflectImageFormat::RG8_SNORM => TextureFormat::Rg8Snorm,
        ReflectImageFormat::R16_SNORM => TextureFormat::R16Snorm,
        ReflectImageFormat::R8_SNORM => TextureFormat::R8Snorm,
        ReflectImageFormat::RGBA32_INT => TextureFormat::Rgba32Sint,
        ReflectImageFormat::RGBA16_INT => TextureFormat::Rgba16Sint,
        ReflectImageFormat::RGBA8_INT => TextureFormat::Rgba8Sint,
        ReflectImageFormat::R32_INT => TextureFormat::R32Sint,
        ReflectImageFormat::RG32_INT => TextureFormat::Rg32Sint,
        ReflectImageFormat::RG16_INT => TextureFormat::Rg16Sint,
        ReflectImageFormat::RG8_INT => TextureFormat::Rg8Sint,
        ReflectImageFormat::R16_INT => TextureFormat::R16Sint,
        ReflectImageFormat::R8_INT => TextureFormat::R8Sint,
        ReflectImageFormat::RGBA32_UINT => TextureFormat::Rgba32Uint,
        ReflectImageFormat::RGBA16_UINT => TextureFormat::Rgba16Uint,
        ReflectImageFormat::RGBA8_UINT => TextureFormat::Rgba8Uint,
        ReflectImageFormat::R32_UINT => TextureFormat::R32Uint,
        ReflectImageFormat::RG32_UINT => TextureFormat::Rg32Uint,
        ReflectImageFormat::RG16_UINT => TextureFormat::Rg16Uint,
        ReflectImageFormat::RG8_UINT => TextureFormat::Rg8Uint,
        ReflectImageFormat::R16_UINT => TextureFormat::R16Uint,
        ReflectImageFormat::R8_UINT => TextureFormat::R8Uint,
        ReflectImageFormat::RGB10A2_UINT => unimplemented!(),
    }
}

impl Pipeline {
    pub(crate) fn new(gpu_context: &CoGr, shader_file: &str) -> Self {
        let shader = Shader::compile_shader(gpu_context, shader_file).unwrap();

        let cs_module = gpu_context
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some(shader_file),
                source: make_spirv(&shader.shader),
            });

        let bind_group_layout_entries = shader
            .bindings
            .iter()
            .enumerate()
            .map(|(i, binding)| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: match binding.descriptor_type {
                    ReflectDescriptorType::StorageImage => wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: map_texture_format(&binding.image.image_format),
                        view_dimension: map_texture_dimension(&binding.image.dim),
                    },
                    ReflectDescriptorType::StorageBuffer => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    binding => panic!("impossible binding type: {:#?}", binding),
                },
                count: None,
            })
            .collect::<Vec<_>>();

        let bind_group_layout =
            gpu_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&(shader_file.to_owned() + "_bindgroup_layout")),
                    entries: bind_group_layout_entries.as_slice(),
                });

        let pipeline_layout =
            gpu_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&(shader_file.to_owned() + "_layout")),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline =
            gpu_context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(shader_file),
                    layout: Some(&pipeline_layout),
                    module: &cs_module,
                    entry_point: "main",
                });

        Pipeline {
            pipeline_name: shader_file.to_string(),
            pipeline,
            bind_group_layout,
            last_bind_group_hash: 0,
            last_bind_group: None,
        }
    }
}
