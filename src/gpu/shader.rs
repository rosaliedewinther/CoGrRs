use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use bytemuck::cast_slice;
use rspirv_reflect::{DescriptorInfo, PushConstantInfo};

#[derive(Debug)]
pub struct Shader {
    pub file: String,
    pub shader: Vec<u32>,
    pub push_constant_range: Option<PushConstantInfo>,
    pub compute_group_size: Option<(u32, u32, u32)>,
    pub bindings: BTreeMap<u32, BTreeMap<u32, DescriptorInfo>>,
}

impl Shader {
    pub fn compile_shader(shader_file: &str) -> Result<Shader> {
        let code = std::fs::read_to_string(shader_file)?;

        let compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.set_source_language(shaderc::SourceLanguage::HLSL);

        let spirv = compiler
            .compile_into_spirv(
                &code,
                shaderc::ShaderKind::Compute,
                shader_file,
                "main",
                Some(&options),
            )
            .unwrap()
            .as_binary()
            .to_vec();

        let info = rspirv_reflect::Reflection::new_from_spirv(&cast_slice(spirv.as_slice()))
            .expect("Invalid SPIRV");

        Ok(Shader {
            file: shader_file.to_string(),
            shader: spirv,
            bindings: info
                .get_descriptor_sets()
                .map_err(|err| anyhow::Error::msg(err.to_string()))?,
            push_constant_range: info
                .get_push_constant_range()
                .map_err(|err| anyhow::Error::msg(err.to_string()))?,
            compute_group_size: info.get_compute_group_size(),
        })
    }
}
