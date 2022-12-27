use inline_spirv_runtime::{ShaderCompilationConfig, ShaderKind};
use regex::Regex;
use rspirv_reflect::PushConstantInfo;

pub struct Shader<'a> {
    pub config: ShaderCompilationConfig,
    pub shader: Vec<u32>,
    pub push_constant_info: PushConstantInfo,
    pub shader_bytes: &'a [u8],
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<String>,
}

impl<'a> Shader<'a> {
    pub fn get_shader_properties<const M: usize>(
        shader_name: &str,
        shaders_folder: &str,
        flags: [&str; M],
    ) -> Shader<'a> {
        let mut config = inline_spirv_runtime::ShaderCompilationConfig::default();
        config.debug = true;
        config.kind = ShaderKind::Compute;
        let shader_file = shaders_folder.to_string() + shader_name + ".comp";
        flags
            .iter()
            .for_each(|flag| config.defs.push((flag.to_string(), None)));

        let shader_vec: Vec<u32> = inline_spirv_runtime::runtime_compile(
            &std::fs::read_to_string(&shader_file)
                .unwrap_or_else(|_| panic!("Could not find {}", shader_name)),
            Some(&(shader_file)),
            &config,
        )
        .map_err(|e| println!("{}", e))
        .unwrap_or_else(|_| panic!("could not compile shader: {}", shader_name));

        let shader: &[u8] = unsafe {
            std::slice::from_raw_parts(shader_vec.as_ptr() as *const u8, shader_vec.len() * 4)
        };
        let reflector = rspirv_reflect::Reflection::new_from_spirv(shader).unwrap();
        let push_constant_info = match reflector.get_push_constant_range().unwrap() {
            Some(p) => p,
            None => PushConstantInfo { offset: 0, size: 0 },
        };
        let compute_group_sizes = reflector.get_compute_group_size().unwrap();

        let text = reflector.disassemble();

        let re = Regex::new(
            r"buffer [^\s\\]*_block|(([ui]*image3D|[ui]*image2D|[ui]*image1D) [a-z_A-Z]*)",
        )
        .unwrap();
        let bindings: Vec<String> = re
            .find_iter(&text)
            .map(|val| val.as_str().split(' ').collect::<Vec<&str>>()[1].to_string())
            .collect::<Vec<String>>();

        Shader {
            config,
            shader: shader_vec,
            shader_bytes: shader,
            cg_x: compute_group_sizes.0,
            cg_y: compute_group_sizes.1,
            cg_z: compute_group_sizes.2,
            bindings,
            push_constant_info,
        }
    }
}
