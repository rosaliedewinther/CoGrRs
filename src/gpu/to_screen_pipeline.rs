use inline_spirv::inline_spirv;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferUsages,
    ColorTargetState, ColorWrites, Device, FragmentState, FrontFace, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptorSpirV, ShaderStages, StorageTextureAccess,
    TextureFormat, TextureView, TextureViewDimension, VertexState,
};

#[derive(Debug)]
pub struct ToScreenPipeline {
    pub pipeline: RenderPipeline,
    pub bind_group: BindGroup,
    pub index_buffer: Buffer,
    pub num_indices: u32,
}

impl ToScreenPipeline {
    pub fn new(
        device: &Device,
        screen_texture: &TextureView,
        texture_format: TextureFormat,
    ) -> Self {
        // init primitives
        let indices = vec![0, 1, 2];

        let indices: &[u16] = indices.as_slice();

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index_buffer_to_screen"),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX,
        });
        let num_indices = indices.len() as u32;

        // init bind group
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
                resource: BindingResource::TextureView(screen_texture),
            }],
        });

        // init compute pass
        let f_shader = unsafe {
            device.create_shader_module_spirv(&ShaderModuleDescriptorSpirV {
                label: Some("../../shaders/to_screen.frag"),
                source: std::borrow::Cow::Borrowed(inline_spirv!(
                    "#version 460

                    layout(location=0) in vec2 v_tex_coords;
                    layout(location=0) out vec4 f_color;
                    
                    layout(rgba8, binding = 0) readonly uniform image2D to_draw;
                    
                    void main() {
                        vec2 size = imageSize(to_draw);
                        f_color = vec4(imageLoad(to_draw, ivec2(v_tex_coords*size)));
                    }",
                    frag,
                    vulkan1_2
                )),
            })
        };

        let v_shader = unsafe {
            device.create_shader_module_spirv(&ShaderModuleDescriptorSpirV {
                label: Some("to_screen_vert"),
                source: std::borrow::Cow::Borrowed(inline_spirv!(
                    "#version 460
                    layout(location=0) out vec2 v_tex_coords;
                    void main() {
                        vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
                        gl_Position = vec4(uv * vec2(2, -2) + vec2(-1, 1), 0, 1);
                        v_tex_coords = uv;
                    }
                    ",
                    vert,
                    vulkan1_2
                )),
            })
        };
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
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
        });

        ToScreenPipeline {
            pipeline,
            bind_group,
            index_buffer,
            num_indices,
        }
    }
}
