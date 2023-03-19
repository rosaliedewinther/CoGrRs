use crate::Execution;
use anyhow::{anyhow, Result};
use hassle_rs::{compile_hlsl, validate_dxil};
use rspirv_reflect::{DescriptorType, Reflection};

#[derive(Debug, Clone)]
pub(crate) struct Binding {
    pub name: String,
    pub binding_type: DescriptorType,
}

pub(crate) struct Shader {
    pub shader: Vec<u8>,
    pub push_constant_size: u32,
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<Binding>,
}

impl Shader {
    pub fn get_shader_properties(shader_name: &str, shaders_folder: &str) -> Result<Shader> {
        let shader_file = shaders_folder.to_string() + shader_name + ".hlsl";
        let code = std::fs::read_to_string(&shader_file)?;

        let dxil = compile_hlsl(&shader_file, &code, "main", "cs_6_5", &[], &[]).unwrap();
        let result = validate_dxil(&dxil);

        if let Some(err) = result.err() {
            println!("validation failed: {}", err);
        }

        let spirv = compile_hlsl(&shader_file, &code, "main", "cs_6_5", &["-spirv"], &[])?; //TODO add defines

        let reflector = Reflection::new_from_spirv(spirv.as_slice()).map_err(|val| anyhow!(val.to_string()))?;
        let push_constant_size = match reflector.get_push_constant_range().map_err(|val| anyhow!(val.to_string()))? {
            Some(p) => p.size,
            None => 0,
        };
        let compute_group_sizes = reflector.get_compute_group_size().unwrap_or((1, 1, 1));

        let bindings = reflector
            .get_descriptor_sets()
            .map_err(|val| anyhow!(val.to_string()))?
            .into_iter()
            .flat_map(|val| val.1)
            .map(|val| Binding {
                name: val.1.name,
                binding_type: val.1.ty,
            })
            .collect::<Vec<Binding>>();

        Ok(Shader {
            shader: spirv,
            cg_x: compute_group_sizes.0,
            cg_y: compute_group_sizes.1,
            cg_z: compute_group_sizes.2,
            bindings,
            push_constant_size,
        })
    }
}

pub fn get_execution_dims(workgroup_size: (u32, u32, u32), execution_mode: Execution, texture_size: (u32, u32)) -> (u32, u32, u32) {
    match execution_mode {
        Execution::PerPixel1D => ((texture_size.0 * texture_size.1 + workgroup_size.0 - 1) / workgroup_size.0, 1u32, 1u32),
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
