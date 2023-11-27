use std::borrow::Cow;

use anyhow::{anyhow, Result};
use naga::front::wgsl;
use wgpu::{BindingType, ShaderModule, ShaderModuleDescriptor};

use crate::CoGr;

pub struct Shader {
    pub file: String,
    pub shader: String,
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub shader_module: ShaderModule,
    pub bindings: Vec<BindingType>,
}

impl Shader {
    pub fn compile_shader(gpu_context: &CoGr, shader_file: &str) -> Result<Shader> {
        let code = std::fs::read_to_string(shader_file)?;

        let shader_module = wgsl::parse_str(&code)?;
        let bindings = shader_module.global_variables.iter().map(|var|var.)

        let shader_module = gpu_context
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some(shader_file),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&code)),
            });

        let bindings = reflector
            .enumerate_descriptor_bindings(None)
            .map_err(|val| anyhow!(val.to_string()))?;

        Ok(Shader {
            file: shader_file.to_string(),
            shader: code,
            cg_x: 0,
            cg_y: 0,
            cg_z: 0,
            shader_module,
            bindings,
        })
    }
}
