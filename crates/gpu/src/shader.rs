use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use hassle_rs::{compile_hlsl, validate_dxil};
use spirv_reflect::{types::ReflectDescriptorBinding, ShaderModule};

pub struct Shader {
    pub file: PathBuf,
    pub shader: Vec<u8>,
    pub push_constant_size: u32,
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<ReflectDescriptorBinding>,
}

pub enum Execution {
    PerPixel1D,
    PerPixel2D,
    N3D(u32),
    N1D(u32),
}

impl Shader {
    pub fn compile_shader(shader_file: &Path) -> Result<Shader> {
        let code = std::fs::read_to_string(&shader_file)?;

        let dxil = match compile_hlsl(
            &shader_file.to_string_lossy(),
            &code,
            "main",
            "cs_6_5",
            &[],
            &[],
        ) {
            Ok(data) => data,
            Err(err) => panic!("{}", err),
        };
        let result = validate_dxil(&dxil);

        if let Some(err) = result.err() {
            println!("validation failed: {}", err);
        }

        let spirv = compile_hlsl(
            &shader_file.to_string_lossy(),
            &code,
            "main",
            "cs_6_5",
            &["-spirv"],
            &[],
        )?; //TODO add defines

        let reflector =
            ShaderModule::load_u8_data(spirv.as_slice()).map_err(|val| anyhow!(val.to_string()))?;

        let push_constant_blocks = reflector
            .enumerate_push_constant_blocks(None)
            .map_err(|val| anyhow!(val.to_string()))?;

        let push_constant_size = match push_constant_blocks.len() {
            0 => 0,
            1 => push_constant_blocks[0].size,
            n => panic!("{} push constant blocks found, only 1 or 0 are allowed", n),
        };

        //let compute_group_sizes = dbg!(reflector.enumerate_input_variables(None));
        //dbg!(reflector.enumerate_descriptor_bindings(None));
        //dbg!(reflector.enumerate_descriptor_sets(None));
        //dbg!(reflector.enumerate_entry_points());
        //dbg!(reflector.enumerate_output_variables(None));
        //dbg!(reflector.enumerate_push_constant_blocks(None));

        let bindings = reflector
            .enumerate_descriptor_bindings(None)
            .map_err(|val| anyhow!(val.to_string()))?;

        Ok(Shader {
            file: shader_file.to_path_buf(),
            shader: spirv,
            cg_x: 0,
            cg_y: 0,
            cg_z: 0,
            bindings,
            push_constant_size,
        })
    }
}

pub fn get_execution_dims(
    workgroup_size: (u32, u32, u32),
    execution_mode: Execution,
    texture_size: (u32, u32),
) -> (u32, u32, u32) {
    match execution_mode {
        Execution::PerPixel1D => (
            (texture_size.0 * texture_size.1 + workgroup_size.0 - 1) / workgroup_size.0,
            1u32,
            1u32,
        ),
        Execution::PerPixel2D => (
            (texture_size.0 + workgroup_size.0 - 1) / workgroup_size.0,
            (texture_size.1 + workgroup_size.1 - 1) / workgroup_size.1,
            1,
        ),
        Execution::N3D(n) => (
            (n + workgroup_size.0 - 1) / workgroup_size.0,
            (n + workgroup_size.1 - 1) / workgroup_size.1,
            (n + workgroup_size.2 - 1) / workgroup_size.2,
        ),
        Execution::N1D(n) => ((n + workgroup_size.0 - 1) / workgroup_size.0, 1, 1),
    }
}
