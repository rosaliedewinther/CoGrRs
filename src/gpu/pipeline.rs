use rspirv_reflect::DescriptorType;
use wgpu::{ShaderModule, StorageTextureAccess, TextureFormat, TextureViewDimension};

use crate::gpu::shader::Shader;

use super::{CoGr, Resource, ResourceHandle};

#[derive(Debug)]
pub struct Pipeline {
    pub shader: Shader,
    pub shader_module: ShaderModule,
    pub last_pipeline: Option<wgpu::ComputePipeline>,
    pub last_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub last_bind_group_hash: Option<u64>,
    pub last_bind_group: Option<wgpu::BindGroup>,
}

pub struct CompiledPipeline {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
}

// fn map_texture_dimension(dimension: &ReflectDimension) -> wgpu::TextureViewDimension {
//     match dimension {
//         ReflectDimension::Undefined => unimplemented!(),
//         ReflectDimension::Type1d => wgpu::TextureViewDimension::D1,
//         ReflectDimension::Type2d => wgpu::TextureViewDimension::D2,
//         ReflectDimension::Type3d => wgpu::TextureViewDimension::D3,
//         ReflectDimension::Cube => wgpu::TextureViewDimension::Cube,
//         ReflectDimension::Rect => unimplemented!(),
//         ReflectDimension::Buffer => unimplemented!(),
//         ReflectDimension::SubPassData => unimplemented!(),
//     }
// }

// fn map_texture_format(format: &ReflectImageFormat) -> wgpu::TextureFormat {
//     match format {
//         ReflectImageFormat::Undefined => unimplemented!(),
//         ReflectImageFormat::RGBA32_FLOAT => wgpu::TextureFormat::Rgba32Float,
//         ReflectImageFormat::RGBA16_FLOAT => wgpu::TextureFormat::Rgba16Float,
//         ReflectImageFormat::R32_FLOAT => wgpu::TextureFormat::R32Float,
//         ReflectImageFormat::RGBA8 => wgpu::TextureFormat::Rgba8Unorm,
//         ReflectImageFormat::RGBA8_SNORM => wgpu::TextureFormat::Rgba8Snorm,
//         ReflectImageFormat::RG32_FLOAT => wgpu::TextureFormat::Rg32Float,
//         ReflectImageFormat::RG16_FLOAT => wgpu::TextureFormat::Rg16Float,
//         ReflectImageFormat::R11G11B10_FLOAT => wgpu::TextureFormat::Rg11b10Float,
//         ReflectImageFormat::R16_FLOAT => wgpu::TextureFormat::R16Float,
//         ReflectImageFormat::RGBA16 => wgpu::TextureFormat::Rgba16Unorm,
//         ReflectImageFormat::RGB10A2 => wgpu::TextureFormat::Rgb10a2Unorm,
//         ReflectImageFormat::RG16 => wgpu::TextureFormat::Rg16Snorm,
//         ReflectImageFormat::RG8 => wgpu::TextureFormat::Rg8Unorm,
//         ReflectImageFormat::R16 => wgpu::TextureFormat::R16Unorm,
//         ReflectImageFormat::R8 => wgpu::TextureFormat::R8Unorm,
//         ReflectImageFormat::RGBA16_SNORM => wgpu::TextureFormat::Rgba16Snorm,
//         ReflectImageFormat::RG16_SNORM => wgpu::TextureFormat::Rg16Snorm,
//         ReflectImageFormat::RG8_SNORM => wgpu::TextureFormat::Rg8Snorm,
//         ReflectImageFormat::R16_SNORM => wgpu::TextureFormat::R16Snorm,
//         ReflectImageFormat::R8_SNORM => wgpu::TextureFormat::R8Snorm,
//         ReflectImageFormat::RGBA32_INT => wgpu::TextureFormat::Rgba32Sint,
//         ReflectImageFormat::RGBA16_INT => wgpu::TextureFormat::Rgba16Sint,
//         ReflectImageFormat::RGBA8_INT => wgpu::TextureFormat::Rgba8Sint,
//         ReflectImageFormat::R32_INT => wgpu::TextureFormat::R32Sint,
//         ReflectImageFormat::RG32_INT => wgpu::TextureFormat::Rg32Sint,
//         ReflectImageFormat::RG16_INT => wgpu::TextureFormat::Rg16Sint,
//         ReflectImageFormat::RG8_INT => wgpu::TextureFormat::Rg8Sint,
//         ReflectImageFormat::R16_INT => wgpu::TextureFormat::R16Sint,
//         ReflectImageFormat::R8_INT => wgpu::TextureFormat::R8Sint,
//         ReflectImageFormat::RGBA32_UINT => wgpu::TextureFormat::Rgba32Uint,
//         ReflectImageFormat::RGBA16_UINT => wgpu::TextureFormat::Rgba16Uint,
//         ReflectImageFormat::RGBA8_UINT => wgpu::TextureFormat::Rgba8Uint,
//         ReflectImageFormat::R32_UINT => wgpu::TextureFormat::R32Uint,
//         ReflectImageFormat::RGB10A2_UINT => unimplemented!(),
//         ReflectImageFormat::RG32_UINT => wgpu::TextureFormat::Rg32Uint,
//         ReflectImageFormat::RG16_UINT => wgpu::TextureFormat::Rg16Uint,
//         ReflectImageFormat::RG8_UINT => wgpu::TextureFormat::Rg8Uint,
//         ReflectImageFormat::R16_UINT => wgpu::TextureFormat::R16Uint,
//         ReflectImageFormat::R8_UINT => wgpu::TextureFormat::R8Uint,
//     }
// }

impl Pipeline {
    pub(crate) fn new(gpu_context: &CoGr, shader_file: &str) -> Self {
        let shader = Shader::compile_shader(shader_file).unwrap();

        let shader_module = unsafe {
            gpu_context
                .device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label: Some(&shader.file),
                    source: std::borrow::Cow::Borrowed(&shader.shader),
                })
        };
        return Pipeline {
            shader,
            shader_module,
            last_pipeline: None,
            last_bind_group_layout: None,
            last_bind_group_hash: None,
            last_bind_group: None,
        };

        // println!("{:#?}", shader.bindings);

        // let mut bind_group_layout_entries = Vec::new();
        // for bind_group in shader.bindings {
        //     for binding in bind_group.1 {
        //         bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
        //             binding: binding.0,
        //             visibility: wgpu::ShaderStages::COMPUTE,
        //             ty: match binding.1.ty {
        //                 DescriptorType::SAMPLER => todo!(),
        //                 DescriptorType::COMBINED_IMAGE_SAMPLER => todo!(),
        //                 DescriptorType::SAMPLED_IMAGE => todo!(),
        //                 DescriptorType::STORAGE_IMAGE => wgpu::BindingType::StorageTexture {
        //                     access: StorageTextureAccess::ReadWrite,
        //                     format: TextureFormat::Bgra8UnormSrgb,
        //                     view_dimension: TextureViewDimension::D2,
        //                 },
        //                 DescriptorType::UNIFORM_TEXEL_BUFFER => todo!(),
        //                 DescriptorType::STORAGE_TEXEL_BUFFER => todo!(),
        //                 DescriptorType::UNIFORM_BUFFER => todo!(),
        //                 DescriptorType::STORAGE_BUFFER => wgpu::BindingType::Buffer {
        //                     ty: wgpu::BufferBindingType::Storage { read_only: false },
        //                     has_dynamic_offset: false,
        //                     min_binding_size: None,
        //                 },
        //                 DescriptorType::UNIFORM_BUFFER_DYNAMIC => todo!(),
        //                 DescriptorType::STORAGE_BUFFER_DYNAMIC => todo!(),
        //                 DescriptorType::INPUT_ATTACHMENT => todo!(),
        //                 DescriptorType::INLINE_UNIFORM_BLOCK_EXT => todo!(),
        //                 DescriptorType::ACCELERATION_STRUCTURE_KHR => todo!(),
        //                 DescriptorType::ACCELERATION_STRUCTURE_NV => todo!(),
        //                 _ => unreachable!(),
        //             },
        //             count: None,
        //         });
        //     }
        // }

        // let bind_group_layout =
        //     gpu_context
        //         .device
        //         .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //             label: Some(&(shader_file.to_owned() + "_bind_group_layout")),
        //             entries: bind_group_layout_entries.as_slice(),
        //         });

        // let push_constant_range_vec = match shader.push_constant_range {
        //     Some(info) => vec![wgpu::PushConstantRange {
        //         stages: wgpu::ShaderStages::COMPUTE,
        //         range: info.offset..info.offset + info.size,
        //     }],
        //     None => vec![],
        // };

        // let pipeline_layout =
        //     gpu_context
        //         .device
        //         .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //             label: Some(&(shader_file.to_owned() + "_layout")),
        //             bind_group_layouts: &[&bind_group_layout],
        //             push_constant_ranges: push_constant_range_vec.as_slice(),
        //         });

        // let pipeline =
        //     gpu_context
        //         .device
        //         .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        //             label: Some(shader_file),
        //             layout: Some(&pipeline_layout),
        //             module: &cs_module,
        //             entry_point: "main",
        //             compilation_options: Default::default(),
        //             cache: None,
        //         });

        // Pipeline {
        //     pipeline_name: shader_file.to_string(),
        //     pipeline,
        //     bind_group_layout,
        //     last_bind_group_hash: 0,
        //     last_bind_group: None,
        // }
    }
    pub(crate) fn get_compiled(
        &mut self,
        gpu_context: &CoGr,
        resources: &[Resource],
    ) -> CompiledPipeline {
        // hash resources to check if we can reuse the previous bind group of this pipeline

        let mut bind_group_layout_entries = Vec::new();
        let mut bind_group_entries = Vec::new();

        for (i, resource) in resources.iter().enumerate() {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: i as u32,
                resource: match resource {
                    Resource::Texture(t) => {
                        wgpu::BindingResource::TextureView(t.texture_view.as_ref().unwrap())
                    }
                    Resource::Buffer(b) => b.buffer.as_ref().unwrap().as_entire_binding(),
                },
            });
            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: match resource {
                    Resource::Texture(t) => wgpu::BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: t.format,
                        view_dimension: TextureViewDimension::D2,
                    },
                    Resource::Buffer(_) => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                count: None,
            })
        }

        let bind_group_layout =
            gpu_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&(self.shader.file.to_owned() + "_bind_group_layout")),
                    entries: bind_group_layout_entries.as_slice(),
                });

        let push_constant_range_vec = match self.shader.push_constant_range {
            Some(info) => vec![wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range: info.offset..info.offset + info.size,
            }],
            None => vec![],
        };

        let pipeline_layout =
            gpu_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&(self.shader.file.to_owned() + "_layout")),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: push_constant_range_vec.as_slice(),
                });

        let pipeline =
            gpu_context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(&self.shader.file),
                    layout: Some(&pipeline_layout),
                    module: &self.shader_module,
                    entry_point: "main",
                    compilation_options: Default::default(),
                    cache: None,
                });

        let bind_group = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("resources bind group"),
                layout: &bind_group_layout,
                entries: bind_group_entries.as_slice(),
            });

        CompiledPipeline {
            pipeline,
            bind_group,
        }
    }
}
