use crate::renderer::{Renderer, Vertex};
use cgmath::{Vector2, Vector3};
use wgpu::util::DeviceExt;
use wgpu::*;

pub struct DrawPass {
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub vertex_buffer_length: usize,
    pub index_buffer: Buffer,
    pub index_buffer_length: usize,
    pub matrix_bind_group: BindGroup,
    pub topology: PrimitiveTopology,
}

impl DrawPass {
    pub fn new(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        topology: PrimitiveTopology,
    ) -> Self {
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
        let (pipeline, matrix_bind_group) =
            DrawPass::create_pipeline(surface_config, device, queue, shader, topology);
        DrawPass {
            pipeline,
            vertex_buffer,
            vertex_buffer_length: 0,
            index_buffer,
            index_buffer_length: 0,
            matrix_bind_group,
            topology,
        }
    }

    fn create_pipeline(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
        shader: &ShaderModule,
        primitive_topology: PrimitiveTopology,
    ) -> (RenderPipeline, BindGroup) {
        // layout for the projection matrix
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            layout: &bind_group_layout,
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
            bind_group_layouts: &[&bind_group_layout],
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
                        attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2],
                    }],
                },
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: "fs_main",
                    targets: &[Some(surface_config.format.into())],
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
        let (pipeline, bind_group) =
            DrawPass::create_pipeline(surface_config, device, queue, shader, self.topology);
        self.pipeline = pipeline;
        self.matrix_bind_group = bind_group;
    }

    pub fn update_vertex_buffer(&mut self, device: &Device, vertices: &[(Vector3<f32>, Color)]) {
        let vertex_data: Vec<Vertex> = vertices
            .iter()
            .map(|(p, color)| Vertex {
                _pos: [p.x as f32, p.y as f32, p.z as f32],
                _color: [color.r as f32, color.g as f32, color.b as f32, color.a as f32],
                _tex_coord: [0., 0.],
            })
            .collect();
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: BufferUsages::VERTEX,
        });
        self.vertex_buffer_length = vertex_data.len();
    }

    pub fn update_index_buffer(&mut self, device: &Device, indices: &[u16]) {
        self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });
        self.index_buffer_length = indices.len();
    }

    pub fn render<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_bind_group(0, &self.matrix_bind_group, &[]);
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // slot 0
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        // rpass.draw(0..(self.vertex_buffer_length as u32), 0..1); // vertex range, instance range
        rpass.draw_indexed(0..(self.index_buffer_length as u32), 0, 0..1);
    }
}
