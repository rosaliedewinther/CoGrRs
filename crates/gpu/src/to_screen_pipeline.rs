use inline_spirv::include_spirv;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferUsages,
    ColorTargetState, ColorWrites, Device, FragmentState, FrontFace, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptorSpirV, ShaderStages, StorageTextureAccess,
    TextureFormat, TextureView, TextureViewDimension, VertexState,
};

#[derive(Debug)]
pub struct ToScreenPipeline {
    pub pipeline: RenderPipeline,
    pub bindgroup: BindGroup,
    pub index_buffer: Buffer,
    pub num_indices: u32,
}

impl ToScreenPipeline {
    pub fn new(
        device: &Device,
        screen_texture: &TextureView,
        texture_format: TextureFormat,
    ) -> Self {
        let (index_buffer, num_indices) = ToScreenPipeline::init_primitives(device);

        let (bindgroup, bindgroup_layout) =
            ToScreenPipeline::init_bindgroup(device, screen_texture, texture_format);
        let pipeline = ToScreenPipeline::init_pipeline(device, &bindgroup_layout, texture_format);

        ToScreenPipeline {
            pipeline,
            bindgroup,
            index_buffer,
            num_indices,
        }
    }

    fn init_pipeline(
        device: &Device,
        bindgroup_layout: &BindGroupLayout,
        texture_format: TextureFormat,
    ) -> RenderPipeline {
        let f_shader = unsafe {
            device.create_shader_module_spirv(&ShaderModuleDescriptorSpirV {
                label: Some("../../shaders/to_screen.frag"),
                source: std::borrow::Cow::Borrowed(include_spirv!(
                    "shaders/to_screen.frag",
                    frag,
                    vulkan1_2
                )),
            })
        };

        let v_shader = unsafe {
            device.create_shader_module_spirv(&ShaderModuleDescriptorSpirV {
                label: Some("../../shaders/to_screen.vert"),
                source: std::borrow::Cow::Borrowed(include_spirv!(
                    "shaders/to_screen.vert",
                    vert,
                    vulkan1_2
                )),
            })
        };
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[bindgroup_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &v_shader,
                entry_point: "main", // 1.
                buffers: &[],        // 2.
            },
            fragment: Some(FragmentState {
                // 3.
                module: &f_shader,
                entry_point: "main",
                targets: &[Some(ColorTargetState {
                    // 4.
                    format: texture_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: FrontFace::Ccw, // 2.
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        })
    }

    fn init_bindgroup(
        device: &Device,
        texture_view: &TextureView,
        texture_format: TextureFormat,
    ) -> (BindGroup, BindGroupLayout) {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout_to_screen"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadOnly,
                    view_dimension: TextureViewDimension::D2,
                    format: texture_format,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bind_group_to_screen"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(texture_view),
            }],
        });
        (bind_group, bind_group_layout)
    }
    fn init_primitives(device: &Device) -> (Buffer, u32) {
        let indices = vec![0, 1, 2];

        let indices: &[u16] = indices.as_slice();

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index_buffer_to_screen"),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX,
        });
        let num_indices = indices.len() as u32;
        (index_buffer, num_indices)
    }
}
