use anyhow::{anyhow, Result};
use hassle_rs::{compile_hlsl, validate_dxil};
use spirv_reflect::{types::ReflectDescriptorBinding, ShaderModule};

pub struct Shader {
    pub file: String,
    pub shader: Vec<u8>,
    pub push_constant_size: u32,
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<ReflectDescriptorBinding>,
}

impl Shader {
    pub fn compile_shader(shader_file: &str) -> Result<Shader> {
        let code = std::fs::read_to_string(&shader_file)?;

        let dxil = match compile_hlsl(&shader_file, &code, "main", "cs_6_5", &[], &[]) {
            Ok(data) => data,
            Err(err) => panic!("{}", err),
        };
        let result = validate_dxil(&dxil);

        if let Some(err) = result.err() {
            println!("validation failed: {}", err);
        }

        let spirv = compile_hlsl(&shader_file, &code, "main", "cs_6_5", &["-spirv"], &[])?; //TODO add defines

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
            file: shader_file.to_string(),
            shader: spirv,
            cg_x: 0,
            cg_y: 0,
            cg_z: 0,
            bindings,
            push_constant_size,
        })
    }
}
