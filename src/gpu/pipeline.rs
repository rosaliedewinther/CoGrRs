use std::time::SystemTime;

use anyhow::Result;

use wgpu::{BindGroup, BindGroupLayout, BindGroupLayoutEntry, ComputePipeline, ShaderStages};

use crate::{gpu::shader::Shader, hash_handles, ResourceHandle};

use super::CoGr;

#[derive(Debug)]
pub struct Pipeline {
    pub pipeline_name: String,
    pub entry_point: String,
    pub source: String,
    pub last_update: SystemTime,
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
    pub last_bind_group_hash: u64,
    pub last_bind_group: Option<BindGroup>,
}

impl Pipeline {
    pub(crate) fn new(
        gpu_context: &CoGr,
        shader_file: &str,
        entry_point: &str,
        bindings: &[&ResourceHandle],
    ) -> Result<Self> {
        let shader = Shader::compile_shader(gpu_context, shader_file)?;
        let code = std::fs::read_to_string(shader_file)?;
        println!("compiled shader");

        let bind_group_layout_entries: Vec<BindGroupLayoutEntry> = bindings
            .iter()
            .enumerate()
            .map(|(index, val)| match val {
                ResourceHandle::Texture(_) => {
                    let texture = gpu_context.resource_pool.grab_texture(val);
                    BindGroupLayoutEntry {
                        visibility: ShaderStages::all(),
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: texture.format,
                            view_dimension: texture.view_dims,
                        },
                        count: None,
                        binding: index as u32,
                    }
                }
                ResourceHandle::Buffer(_) => {
                    let texture = gpu_context.resource_pool.grab_buffer(val);
                    BindGroupLayoutEntry {
                        visibility: ShaderStages::all(),
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                        binding: index as u32,
                    }
                }
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
                    module: &shader.shader_module,
                    entry_point,
                });

        Ok(Pipeline {
            pipeline_name: shader_file.to_string(),
            pipeline,
            source: shader_file.to_string(),
            entry_point: entry_point.to_string(),
            last_update: std::fs::metadata(shader_file).unwrap().modified().unwrap(),
            bind_group_layout,
            last_bind_group_hash: hash_handles(bindings),
            last_bind_group: None,
        })
    }

    pub fn check_hot_reload(&mut self, gpu_context: &CoGr, bindings: &[&ResourceHandle]) {
        if hash_handles(bindings) != self.last_bind_group_hash
            || self.last_update < std::fs::metadata(&self.source).unwrap().modified().unwrap()
        {
            match Pipeline::new(gpu_context, &self.source, &self.entry_point, bindings) {
                Ok(new_pipe) => *self = new_pipe,
                Err(err) => {
                    println!("{}", err);
                    self.last_update = std::fs::metadata(&self.source).unwrap().modified().unwrap();
                }
            }
        }
    }
}
