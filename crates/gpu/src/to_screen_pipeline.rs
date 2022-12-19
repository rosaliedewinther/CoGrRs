use inline_spirv::include_spirv;
use wgpu::util::DeviceExt;

pub struct ToScreenPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bindgroup: wgpu::BindGroup,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl ToScreenPipeline {
    pub fn new(device: &wgpu::Device, screen_texture: &wgpu::TextureView) -> Self {
        let (index_buffer, num_indices) = ToScreenPipeline::init_primitives(device);

        let (bindgroup, bindgroup_layout) =
            ToScreenPipeline::init_bindgroup(device, screen_texture);
        let pipeline = ToScreenPipeline::init_pipeline(device, &bindgroup_layout);

        ToScreenPipeline {
            pipeline,
            bindgroup,
            index_buffer,
            num_indices,
        }
    }

    fn init_pipeline(
        device: &wgpu::Device,
        bindgroup_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let f_shader = unsafe {
            device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("../../shaders/to_screen.frag"),
                source: std::borrow::Cow::Borrowed(include_spirv!(
                    "shaders/to_screen.frag",
                    frag,
                    vulkan1_2
                )),
            })
        };

        let v_shader = unsafe {
            device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("../../shaders/to_screen.vert"),
                source: std::borrow::Cow::Borrowed(include_spirv!(
                    "shaders/to_screen.vert",
                    vert,
                    vulkan1_2
                )),
            })
        };
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[bindgroup_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &v_shader,
                entry_point: "main", // 1.
                buffers: &[],        // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &f_shader,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        });
        render_pipeline
    }

    fn init_bindgroup(
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout_to_screen"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::ReadOnly,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_group_to_screen"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(texture_view),
            }],
        });
        (bind_group, bind_group_layout)
    }
    fn init_primitives(device: &wgpu::Device) -> (wgpu::Buffer, u32) {
        let indices = vec![0, 1, 2];

        let indices: &[u16] = indices.as_slice();

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer_to_screen"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = indices.len() as u32;
        (index_buffer, num_indices)
    }
}
