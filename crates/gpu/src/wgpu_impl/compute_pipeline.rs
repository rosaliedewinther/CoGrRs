use std::ops::Range;

use wgpu::StorageTextureAccess;

use super::{BufferDescriptor, CoGrWGPU, TextureDescriptor};

#[derive(Debug)]
pub struct ComputePipeline {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub(crate) enum TextureOrBuffer<'a> {
    Texture(&'a TextureDescriptor),
    Buffer(&'a BufferDescriptor), //buffer and boolean which is true if readonly
}

impl ComputePipeline {
    pub(crate) fn new(
        gpu_context: &CoGrWGPU,
        pipeline_name: &str,
        spirv: &[u32],
        buffers: &[TextureOrBuffer], // buffer and read only flag
        push_constant_range: Option<Range<u32>>,
    ) -> Self {
        let cs_module = unsafe {
            gpu_context.device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some(pipeline_name),
                source: std::borrow::Cow::Borrowed(spirv),
            })
        };

        let mut bind_group_entries = Vec::new();
        let mut bind_group_layout_entries = Vec::new();

        for (buffer_index, _) in buffers.iter().enumerate() {
            let resource = match buffers[buffer_index] {
                TextureOrBuffer::Texture(desc) => wgpu::BindingResource::TextureView(&desc.texture_view),
                TextureOrBuffer::Buffer(desc) => desc.buffer.as_entire_binding(),
            };

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: buffer_index as u32,
                resource,
            });
            let bindingtype = match buffers[buffer_index] {
                TextureOrBuffer::Texture(desc) => wgpu::BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadWrite,
                    format: desc.format,
                    view_dimension: match desc.size.2 {
                        1 => wgpu::TextureViewDimension::D2,
                        _ => wgpu::TextureViewDimension::D3,
                    },
                },
                TextureOrBuffer::Buffer(_) => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None, //TODO set this to correct value
                },
            };

            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: buffer_index as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: bindingtype,
                count: None,
            });
        }

        let bind_group_layout = gpu_context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&(pipeline_name.to_owned() + "_bindgroup_layout")),
            entries: bind_group_layout_entries.as_slice(),
        });

        let bind_group = gpu_context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&(pipeline_name.to_owned() + "_bindgroup")),
            layout: &bind_group_layout,
            entries: &bind_group_entries,
        });

        let mut push_constant_range_vec = Vec::new();

        if let Some(range) = push_constant_range {
            push_constant_range_vec.push(wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range,
            })
        };

        let pipeline_layout = gpu_context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&(pipeline_name.to_owned() + "_layout")),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: push_constant_range_vec.as_slice(),
        });

        let pipeline = gpu_context.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(pipeline_name),
            layout: Some(&pipeline_layout),
            module: &cs_module,
            entry_point: "main",
        });
        ComputePipeline { pipeline, bind_group }
    }
}
