use anyhow::{anyhow, Result};
use spirv_reflect::{types::ReflectDescriptorBinding, ShaderModule};

pub struct Shader {
    pub file: String,
    pub shader: Vec<u8>,
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<ReflectDescriptorBinding>,
}

impl Shader {
    pub fn compile_shader(shader_file: &str) -> Result<Shader> {
        let code = std::fs::read_to_string(shader_file)?;

        let compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.set_forced_version_profile(460, shaderc::GlslProfile::None);
        options.set_auto_bind_uniforms(true);
        //options.add_macro_definition("EP", Some("main"));
        let spirv = match compiler.compile_into_spirv(
            &code,
            shaderc::ShaderKind::Compute,
            shader_file,
            "main",
            Some(&options),
        ) {
            Ok(result) => result.as_binary_u8().to_vec(),
            Err(error) => {
                println!("{}", error);
                panic!("compilation error");
            }
        };

        let reflector =
            ShaderModule::load_u8_data(spirv.as_slice()).map_err(|val| anyhow!(val.to_string()))?;

        //let compute_group_sizes = dbg!(reflector.enumerate_input_variables(None));
        //dbg!(reflector.enumerate_descriptor_bindings(None));
        //dbg!(reflector.enumerate_descriptor_sets(None));
        //dbg!(reflector.enumerate_entry_points());
        //dbg!(reflector.enumerate_output_variables(None));

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
        })
    }
}
