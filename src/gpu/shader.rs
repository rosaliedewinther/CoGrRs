use std::borrow::Cow;

use anyhow::Result;
use wgpu::{ShaderModule, ShaderModuleDescriptor};

use crate::CoGr;

pub struct Shader {
    pub file: String,
    pub shader: String,
    pub shader_module: ShaderModule,
}

impl Shader {
    pub fn compile_shader(gpu_context: &CoGr, shader_file: &str) -> Result<Shader> {
        let code = std::fs::read_to_string(shader_file)?;

        let shader_module = gpu_context
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some(shader_file),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&code)),
            });

        Ok(Shader {
            file: shader_file.to_string(),
            shader: code,
            shader_module,
        })
    }
}
