use std::borrow::Cow;
use std::mem;

use crate::camera::Camera;
use crate::renderer::Vertex;
use crate::{Particle, V3};
use cgmath::Vector3;
use wgpu::util::DeviceExt;
use wgpu::*;

#[allow(dead_code)]
pub const INSTANCE_LAYOUT_POSITION: VertexBufferLayout = VertexBufferLayout {
    array_stride: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
    step_mode: VertexStepMode::Instance,
    attributes: &[VertexAttribute {
        format: VertexFormat::Float32x4,
        offset: 0,
        shader_location: 2,
    }],
};

#[allow(dead_code)]
pub const INSTANCE_LAYOUT_VECTOR_FIELD: VertexBufferLayout = VertexBufferLayout {
    array_stride: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
    step_mode: VertexStepMode::Instance,
    attributes: &[
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 16,
            shader_location: 3,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 32,
            shader_location: 4,
        }
    ],
};

pub const INSTANCE_LAYOUT_PARTICLE: wgpu::VertexBufferLayout = Particle::get_instance_layout();

pub struct DrawBuffer {
    pub vertex_buffer: Buffer,
    pub vertex_buffer_length: usize,
    pub index_buffer: Buffer,
    pub index_buffer_length: usize,
    pub instance_buffer: Buffer,
    pub instance_buffer_length: usize,
    pub texture: Texture,
    pub texture_bind_group: BindGroup,
    pub texture_bind_group_layout: BindGroupLayout,
}

impl DrawBuffer {
    pub fn new(device: &Device, queue: &Queue, texture_as_bytes: &[u8]) -> Self {
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
            DrawBuffer::create_texture(device, queue, texture_as_bytes);
        let instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("instance buffer"),
            size: 0,
            usage: BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        DrawBuffer {
            vertex_buffer,
            vertex_buffer_length: 0,
            index_buffer,
            index_buffer_length: 0,
            instance_buffer,
            instance_buffer_length: 0,
            texture,
            texture_bind_group,
            texture_bind_group_layout,
        }
    }

    pub fn create_texture(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
    ) -> (Texture, BindGroup, BindGroupLayout) {
        let image = image::load_from_memory(bytes).expect("could not load texture");
        let rgba = image.to_rgba8();
        let dimensions = rgba.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let tex = device.create_texture(&TextureDescriptor {
            label: Some("texture"),
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

pub struct ViewMatrix {
    pub matrix_bind_group: BindGroup,
    pub view_matrix_buffer: Buffer,
}

pub struct DrawPass {
    pub prefix: String,
    pub pipeline: RenderPipeline,
    pub draw_buffer: DrawBuffer,
    pub shader: ShaderModule,
    pub view_matrix: Option<ViewMatrix>,
    pub topology: PrimitiveTopology,
    pub instance_layout: VertexBufferLayout<'static>,
}

impl DrawPass {
    pub fn new(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        draw_buffer: DrawBuffer,
        shader: ShaderModule,
        camera: &mut Camera,
        topology: PrimitiveTopology,
        instance_layout: VertexBufferLayout<'static>,
        bcreate_viewmatrix: bool,
        prefix: &str,
    ) -> Self {
        let (pipeline, view_matrix) = DrawPass::create_pipeline(
            device,
            queue,
            surface_config,
            &shader,
            camera,
            topology,
            &draw_buffer.texture_bind_group_layout,
            &instance_layout,
            bcreate_viewmatrix,
            &prefix,
        );
        DrawPass {
            prefix: String::from(prefix),
            pipeline,
            draw_buffer,
            shader,
            view_matrix,
            topology,
            instance_layout,
        }
    }

    fn create_pipeline(
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
        shader: &ShaderModule,
        camera: &mut Camera,
        primitive_topology: PrimitiveTopology,
        texture_bind_group_layout: &BindGroupLayout,
        instance_layout: &VertexBufferLayout,
        bcreate_viewmatrix: bool,
        prefix: &str,
    ) -> (RenderPipeline, Option<ViewMatrix>) {
        let (view_matrix, pipeline_layout) = match bcreate_viewmatrix {
            true => {
                let (bind_group, transform_bind_group_layout, buffer) =
                    Self::create_view_matrix_bind_groups(device, queue, camera);
                let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some(&format!("{} pipeline layout", prefix)),
                    bind_group_layouts: &[&transform_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                });
                (
                    Some(ViewMatrix {
                        matrix_bind_group: bind_group,
                        view_matrix_buffer: buffer,
                    }),
                    pipeline_layout,
                )
            }
            false => {
                let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some(&format!("{} pipeline layout", prefix)),
                    bind_group_layouts: &[&texture_bind_group_layout],
                    push_constant_ranges: &[],
                });
                (None, pipeline_layout)
            }
        };

        let vertex_layout = VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x2],
        };

        (
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(&format!("{} render pipeline", prefix)),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: "vs_main",
                    buffers: &[vertex_layout, instance_layout.clone()],
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
                primitive: PrimitiveState {
                    topology: primitive_topology,
                    front_face: FrontFace::Cw,
                    ..Default::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                // no multisample
                multisample: MultisampleState {
                    ..Default::default()
                },
                multiview: None,
            }),
            view_matrix,
        )
    }

    pub fn from_object_and_texture(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader_src: Cow<'static, str>,
        obj_path: &str,
        texture_bytes: &[u8],
        camera: &mut Camera,
        instance_layout: VertexBufferLayout<'static>,
        bcreate_viewmatrix: bool,
        prefix: &str,
    ) -> DrawPass {
        let cursor_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer: wgsl cursor shader module"),
            source: wgpu::ShaderSource::Wgsl(shader_src),
        });
        let draw_buffer_cursor = DrawBuffer::new(device, queue, texture_bytes);
        let mut res = DrawPass::new(
            surface_config,
            device,
            queue,
            draw_buffer_cursor,
            cursor_shader,
            camera,
            PrimitiveTopology::TriangleList,
            instance_layout,
            bcreate_viewmatrix,
            prefix,
        );
        let cursor_obj =
            tobj::load_obj(obj_path, &tobj::GPU_LOAD_OPTIONS).expect("could not load object");
        let cursor_vertices: Vec<V3> = cursor_obj.0[0]
            .mesh
            .positions
            .chunks(3)
            .map(|c| V3::new(c[0], c[1], c[2]))
            .collect();
        let cursor_texture_coordinates: Vec<[f32; 2]> = cursor_obj.0[0]
            .mesh
            .texcoords
            .chunks(2)
            .map(|tc| [tc[0], tc[1]])
            .collect();
        let cursor_indices: Vec<u16> = cursor_obj.0[0]
            .mesh
            .indices
            .iter()
            .map(|i| *i as u16)
            .collect();
        res.update_vertex_buffer(
            device,
            &cursor_vertices
                .iter()
                .copied()
                .zip(cursor_texture_coordinates)
                .collect::<Vec<_>>(),
        );
        res.update_index_buffer(device, &cursor_indices);
        // this puts up only a single instance at the origin
        res.update_instance_buffer(device, &[0., 0., 0., 1.], 1);
        res
    }

    pub fn recreate_pipeline(
        &mut self,
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        camera: &mut Camera,
    ) {
        let bcreate_viewmatrix = self.view_matrix.is_some();
        let (pipeline, view_matrix) = DrawPass::create_pipeline(
            device,
            queue,
            surface_config,
            &self.shader,
            camera,
            self.topology,
            &self.draw_buffer.texture_bind_group_layout,
            &self.instance_layout,
            bcreate_viewmatrix,
            &self.prefix,
        );
        self.pipeline = pipeline;
        self.view_matrix = view_matrix;
    }

    fn create_view_matrix_bind_groups(
        device: &Device,
        queue: &Queue,
        camera: &mut Camera,
    ) -> (BindGroup, BindGroupLayout, Buffer) {
        // create the projection matrix buffer
        let mx = camera.get_view_matrix();
        let mx_ref: &[f32; 16] = mx.as_ref();
        let mx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("u_Transform"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
        (bind_group, transform_bind_group_layout, mx_buf)
    }

    pub fn update_view_matrix(&mut self, queue: &Queue, camera: &mut Camera) {
        if let Some(ViewMatrix {
            matrix_bind_group: _,
            view_matrix_buffer,
        }) = self.view_matrix.as_ref()
        {
            let mx = camera.get_view_matrix();
            let mx_ref: &[f32; 16] = mx.as_ref();
            queue.write_buffer(view_matrix_buffer, 0, bytemuck::cast_slice(mx_ref));
        }
    }

    pub fn update_vertex_buffer(&mut self, device: &Device, vertices: &[(Vector3<f32>, [f32; 2])]) {
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

    pub fn update_instance_buffer(
        &mut self,
        device: &Device,
        instance_floats: &[f32],
        num_instances: usize,
    ) {
        self.draw_buffer.instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_floats),
                usage: BufferUsages::VERTEX,
            });
        self.draw_buffer.instance_buffer_length = num_instances;
    }

    pub fn render<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_pipeline(&self.pipeline);
        if let Some(ViewMatrix {
            matrix_bind_group,
            view_matrix_buffer: _,
        }) = self.view_matrix.as_ref()
        {
            rpass.set_bind_group(0, matrix_bind_group, &[]);
            rpass.set_bind_group(1, &self.draw_buffer.texture_bind_group, &[]);
        } else {
            rpass.set_bind_group(0, &self.draw_buffer.texture_bind_group, &[]);
        }
        rpass.set_vertex_buffer(0, self.draw_buffer.vertex_buffer.slice(..)); // slot 0
        rpass.set_index_buffer(self.draw_buffer.index_buffer.slice(..), IndexFormat::Uint16);
        rpass.set_vertex_buffer(1, self.draw_buffer.instance_buffer.slice(..));
        // rpass.draw(0..(self.vertex_buffer_length as u32), 0..1); // vertex range, instance range
        rpass.draw_indexed(
            0..(self.draw_buffer.index_buffer_length as u32),
            0,
            0..self.draw_buffer.instance_buffer_length as u32,
        );
    }

    pub fn render_with_instance_buffer<'a>(
        &'a self,
        rpass: &mut RenderPass<'a>,
        instance_buffer: &'a Buffer,
        instance_buffer_length: usize,
    ) {
        rpass.set_pipeline(&self.pipeline);
        if let Some(ViewMatrix {
            matrix_bind_group,
            view_matrix_buffer: _,
        }) = self.view_matrix.as_ref()
        {
            rpass.set_bind_group(0, &matrix_bind_group, &[]);
        }
        rpass.set_bind_group(1, &self.draw_buffer.texture_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.draw_buffer.vertex_buffer.slice(..)); // slot 0
        rpass.set_index_buffer(self.draw_buffer.index_buffer.slice(..), IndexFormat::Uint16);
        rpass.set_vertex_buffer(1, instance_buffer.slice(..));
        // rpass.draw(0..(self.vertex_buffer_length as u32), 0..1); // vertex range, instance range
        rpass.draw_indexed(
            0..(self.draw_buffer.index_buffer_length as u32),
            0,
            0..instance_buffer_length as u32,
        );
    }
}
