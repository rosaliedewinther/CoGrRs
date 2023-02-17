use std::fmt::Debug;

use inline_spirv_runtime::{ShaderCompilationConfig, ShaderKind};
use regex::Regex;
use rspirv_reflect::PushConstantInfo;

use crate::wgpu_impl::Execution;

pub struct Shader {
    pub config: ShaderCompilationConfig,
    pub shader: Vec<u32>,
    pub push_constant_info: PushConstantInfo,
    //pub shader_bytes: &'a [u8],
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<String>,
}

impl Debug for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shader")
            .field("shader", &self.shader)
            .field("push_constant_offset", &self.push_constant_info.offset)
            .field("push_constant_size", &self.push_constant_info.size)
            .field("cg_x", &self.cg_x)
            .field("cg_y", &self.cg_y)
            .field("cg_z", &self.cg_z)
            .field("bindings", &self.bindings)
            .finish()
    }
}

impl Shader {
    pub fn get_shader_properties(shader_name: &str, shaders_folder: &str) -> Shader {
        let mut config = inline_spirv_runtime::ShaderCompilationConfig::default();
        config.debug = true;
        config.kind = ShaderKind::Compute;
        let shader_file = shaders_folder.to_string() + shader_name + ".comp";

        let shader_vec: Vec<u32> = inline_spirv_runtime::runtime_compile(
            &std::fs::read_to_string(&shader_file).unwrap_or_else(|_| panic!("Could not find {}", shader_file)),
            Some(&(shader_file)),
            &config,
        )
        .map_err(|e| println!("{}", e))
        .unwrap_or_else(|_| panic!("could not compile shader: {}", shader_file));

        let shader: &[u8] = unsafe { std::slice::from_raw_parts(shader_vec.as_ptr() as *const u8, shader_vec.len() * 4) };
        let reflector = rspirv_reflect::Reflection::new_from_spirv(shader).unwrap_or_else(|_| panic!("could not reflect shader: {}", shader_file));
        let push_constant_info = match reflector
            .get_push_constant_range()
            .unwrap_or_else(|_| panic!("could not get push constant range from shader: {}", shader_file))
        {
            Some(p) => p,
            None => PushConstantInfo { offset: 0, size: 0 },
        };
        let compute_group_sizes = reflector
            .get_compute_group_size()
            .unwrap_or_else(|| panic!("could not get compute group size from shader: {}", shader_file));

        let text = reflector.disassemble();

        let re = Regex::new(r"buffer [^\s\\]*_block|(([ui]*image3D|[ui]*image2D|[ui]*image1D) [a-z_A-Z]*)").expect("somehow couldnt compile regex");
        let bindings: Vec<String> = re
            .find_iter(&text)
            .map(|val| val.as_str().split(' ').collect::<Vec<&str>>()[1].to_string())
            .collect::<Vec<String>>();

        Shader {
            config,
            shader: shader_vec,
            cg_x: compute_group_sizes.0,
            cg_y: compute_group_sizes.1,
            cg_z: compute_group_sizes.2,
            bindings,
            push_constant_info,
        }
    }
}

pub fn get_execution_dims(shader: &Shader, execution_mode: Execution, texture_size: (u32, u32)) -> (u32, u32, u32) {
    match execution_mode {
        Execution::PerPixel1D => ((texture_size.0 * texture_size.1 + shader.cg_x - 1) / shader.cg_x, 1u32, 1u32),
        Execution::PerPixel2D => (
            (texture_size.0 + shader.cg_x - 1) / shader.cg_x,
            (texture_size.1 + shader.cg_y - 1) / shader.cg_y,
            1,
        ),
        Execution::N3D(n) => (
            (n + shader.cg_x - 1) / shader.cg_x,
            (n + shader.cg_y - 1) / shader.cg_y,
            (n + shader.cg_z - 1) / shader.cg_z,
        ),
        Execution::N1D(n) => ((n + shader.cg_x - 1) / shader.cg_x, 1, 1),
    }
}
