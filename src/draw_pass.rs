use crate::renderer::{Renderer, Vertex};
use cgmath::Vector3;
use wgpu::util::DeviceExt;
use wgpu::*;

pub struct DrawBuffer {
    pub vertex_buffer: Buffer,
    pub vertex_buffer_length: usize,
    pub index_buffer: Buffer,
    pub index_buffer_length: usize,
    pub texture: Texture,
    pub texture_bind_group: BindGroup,
    pub texture_bind_group_layout: BindGroupLayout,
}

impl DrawBuffer {
    pub fn new(device: &Device, queue: &Queue, texture_filename: &str) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 0,
            mapped_at_creation: false,
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 0,
            mapped_at_creation: false,
            usage: BufferUsages::INDEX,
        });
        let (texture, texture_bind_group, texture_bind_group_layout) =
            DrawBuffer::create_texture(device, queue, texture_filename);
        DrawBuffer {
            vertex_buffer,
            vertex_buffer_length: 0,
            index_buffer,
            index_buffer_length: 0,
            texture,
            texture_bind_group,
            texture_bind_group_layout,
        }
    }

    pub fn create_texture(
        device: &Device,
        queue: &Queue,
        file_name: &str,
    ) -> (Texture, BindGroup, BindGroupLayout) {
        let image = image::open(file_name).expect("could not load texture");
        let rgba = image.to_rgba8();
        let dimensions = rgba.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let tex = device.create_texture(&TextureDescriptor {
            label: Some(&format!("Texture: {}", file_name)),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            tex.as_image_copy(),
            &rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );
        let tex_view = tex.create_view(&TextureViewDescriptor::default());
        let (bind_group, bind_group_layout) =
            DrawBuffer::create_texture_bind_group(&device, &tex_view);
        (tex, bind_group, bind_group_layout)
    }

    pub fn create_texture_bind_group(
        device: &Device,
        view: &TextureView,
    ) -> (BindGroup, BindGroupLayout) {
        let sampler = device.create_sampler(&SamplerDescriptor::default());
        let entries = &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ];
        let layout_desc = BindGroupLayoutDescriptor {
            label: Some("texture bind group layout descriptor"),
            entries,
        };
        let layout = device.create_bind_group_layout(&layout_desc);
        let desc = BindGroupDescriptor {
            label: Some("texture bind group layout"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        };
        (device.create_bind_group(&desc), layout)
    }
}

pub struct DrawPass {
    pub pipeline: RenderPipeline,
    pub draw_buffer: DrawBuffer,
    pub matrix_bind_group: BindGroup,
    pub topology: PrimitiveTopology,
}

impl DrawPass {
    pub fn new(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        draw_buffer: DrawBuffer,
        shader: &ShaderModule,
        topology: PrimitiveTopology,
    ) -> Self {
        let (pipeline, matrix_bind_group) = DrawPass::create_pipeline(
            device,
            queue,
            surface_config,
            shader,
            topology,
            &draw_buffer.texture_bind_group_layout,
        );
        DrawPass {
            pipeline,
            draw_buffer,
            matrix_bind_group,
            topology,
        }
    }

    fn create_pipeline(
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
        shader: &ShaderModule,
        primitive_topology: PrimitiveTopology,
        texture_bind_group_layout: &BindGroupLayout,
    ) -> (RenderPipeline, BindGroup) {
        // layout for the projection matrix
        let transform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Renderer: bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
            });

        // create the projection matrix
        let aspect = surface_config.width as f32 / surface_config.height as f32;
        let mx = Renderer::generate_matrix(aspect);
        let mx_ref: &[f32; 16] = mx.as_ref();
        let mx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("u_Transform"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // write to the projection matix buffer
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("u_Transform"),
            layout: &transform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &mx_buf,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        queue.write_buffer(&mx_buf, 0, bytemuck::cast_slice(mx_ref));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&transform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        (
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("render pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                    }],
                },
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: surface_config.format,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Max,
                            },
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                // render lines
                primitive: PrimitiveState {
                    topology: primitive_topology,
                    front_face: FrontFace::Cw,
                    ..Default::default()
                },
                depth_stencil: None,
                // no multisample
                multisample: MultisampleState {
                    ..Default::default()
                },
                multiview: None,
            }),
            bind_group,
        )
    }

    pub fn recreate_pipeline(
        &mut self,
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
    ) {
        let (pipeline, bind_group) = DrawPass::create_pipeline(
            device,
            queue,
            surface_config,
            shader,
            self.topology,
            &self.draw_buffer.texture_bind_group_layout,
        );
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
    }

    pub fn update_vertex_buffer(
        &mut self,
        device: &Device,
        draw_buffer_index: usize,
        vertices: &[(Vector3<f32>, [f32; 2])],
    ) {
        let vertex_data: Vec<Vertex> = vertices
            .iter()
            .map(|(p, tex_coord)| Vertex {
                _pos: [p.x as f32, p.y as f32, p.z as f32],
                _tex_coord: *tex_coord,
            })
            .collect();
        self.draw_buffer.vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: BufferUsages::VERTEX,
            });
        self.draw_buffer.vertex_buffer_length = vertex_data.len();
    }

    pub fn update_index_buffer(&mut self, device: &Device, indices: &[u16]) {
        self.draw_buffer.index_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            });
        self.draw_buffer.index_buffer_length = indices.len();
    }

    pub fn render<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_bind_group(0, &self.matrix_bind_group, &[]);
        rpass.set_bind_group(1, &self.draw_buffer.texture_bind_group, &[]);
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.draw_buffer.vertex_buffer.slice(..)); // slot 0
        rpass.set_index_buffer(self.draw_buffer.index_buffer.slice(..), IndexFormat::Uint16);
        // rpass.draw(0..(self.vertex_buffer_length as u32), 0..1); // vertex range, instance range
        rpass.draw_indexed(0..(self.draw_buffer.index_buffer_length as u32), 0, 0..1);
    }
}
