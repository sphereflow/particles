use crate::camera::Camera;
use crate::compute::Compute;
use crate::draw_pass::DrawBuffer;
use crate::draw_pass::DrawPass;
use crate::draw_pass::INSTANCE_LAYOUT_POSITION;
use crate::draw_pass::INSTANCE_LAYOUT_VECTOR_FIELD;
use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use egui::FullOutput;
use egui_wgpu::renderer::ScreenDescriptor;
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub _pos: [f32; 3],
    pub _tex_coord: [f32; 2],
}
unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

pub struct Renderer {
    pub sub_rpass_triangles: DrawPass,
    pub sub_rpass_cursor: DrawPass,
    pub sub_rpass_vector_field: DrawPass,
    pub device: Device,
    pub queue: Queue,
    egui_rpass: egui_wgpu::renderer::Renderer,
    surface_config: SurfaceConfiguration,
    pub camera: Camera,
    depth_texture: Texture,
    depth_view: TextureView,
    depth_sampler: Sampler,
    pub recreate_pipelines: bool,
}

impl Renderer {
    pub fn init(
        surface_config: &SurfaceConfiguration,
        device: Device,
        queue: Queue, // we might need to meddle with the command queue
    ) -> Self {
        use std::borrow::Cow;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer: wgsl shader module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let mut camera: Camera = Camera::new(
            surface_config.width as f32,
            surface_config.height as f32,
            90.0,
        );

        let texture_as_bytes = include_bytes!("../assets/all_textures.png");

        let draw_buffer = DrawBuffer::new(&device, &queue, texture_as_bytes);

        let mut sub_rpass_particles = DrawPass::new(
            surface_config,
            &device,
            &queue,
            draw_buffer,
            shader,
            &mut camera,
            PrimitiveTopology::TriangleList,
            crate::draw_pass::INSTANCE_LAYOUT_PARTICLE,
            true,
            "particles",
        );
        dbg!(crate::draw_pass::INSTANCE_LAYOUT_PARTICLE);
        dbg!(crate::draw_pass::INSTANCE_LAYOUT_VECTOR_FIELD);
        dbg!(crate::draw_pass::INSTANCE_LAYOUT_POSITION);
        let d = 0.01;
        let md = -0.01;
        sub_rpass_particles.update_vertex_buffer(
            &device,
            &[
                (Vector3::new(md, d, d), [0.0, 1.0]),
                (Vector3::new(d, d, d), [1.0, 1.0]),
                (Vector3::new(md, md, d), [0.0, 0.0]),
                (Vector3::new(d, md, d), [1.0, 0.0]),
            ],
        );
        sub_rpass_particles.update_index_buffer(&device, &[0, 1, 2, 1, 2, 3]);

        let cursor_texture_bytes = include_bytes!("../assets/cursor.png");
        let sub_rpass_cursor = DrawPass::from_object_and_texture(
            surface_config,
            &device,
            &queue,
            Cow::Borrowed(include_str!("cursor_shader.wgsl")),
            "./assets/cursor.obj",
            cursor_texture_bytes,
            &mut camera,
            INSTANCE_LAYOUT_POSITION,
            true,
            "cursor",
        );

        let vector_texture_bytes = include_bytes!("../assets/vector.png");
        let sub_rpass_vector_field = DrawPass::from_object_and_texture(
            surface_config,
            &device,
            &queue,
            Cow::Borrowed(include_str!("vector_field_shader.wgsl")),
            "./assets/vector.obj",
            vector_texture_bytes,
            &mut camera,
            INSTANCE_LAYOUT_VECTOR_FIELD,
            true,
            "vector field",
        );

        let egui_rpass = egui_wgpu::renderer::Renderer::new(&device, surface_config.format, None, 1);

        let (depth_texture, depth_view, depth_sampler) =
            Self::create_depth_texture(&device, surface_config);

        Renderer {
            sub_rpass_triangles: sub_rpass_particles,
            sub_rpass_cursor,
            sub_rpass_vector_field,
            egui_rpass,
            device,
            queue,
            surface_config: surface_config.clone(),
            camera,
            depth_texture,
            depth_view,
            depth_sampler,
            recreate_pipelines: false,
        }
    }

    pub fn recreate_pipelines(&mut self) {
        self.recreate_pipelines = false;
        self.sub_rpass_triangles.recreate_pipeline(
            &self.surface_config,
            &self.device,
            &self.queue,
            &mut self.camera,
        );
        self.sub_rpass_cursor.recreate_pipeline(
            &self.surface_config,
            &self.device,
            &self.queue,
            &mut self.camera,
        );
    }

    pub fn create_depth_texture(
        device: &Device,
        surface_config: &SurfaceConfiguration,
    ) -> (Texture, TextureView, Sampler) {
        let size = Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };
        let tex_desc = TextureDescriptor {
            label: Some("depth texture descriptor"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let texture = device.create_texture(&tex_desc);
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("depth texture sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1000.0,
            compare: Some(CompareFunction::LessEqual),
            ..Default::default()
        });
        (texture, view, sampler)
    }

    pub fn resize(
        &mut self,
        surface_config: &SurfaceConfiguration,
    ) {
        self.surface_config = surface_config.clone();
        let (depth_texture, depth_view, depth_sampler) =
            Self::create_depth_texture(&self.device, surface_config);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.depth_sampler = depth_sampler;
        self.camera
            .resize(surface_config.width as f32, surface_config.height as f32);
        self.recreate_pipelines();
    }

    pub fn render(
        &mut self,
        frame: &SurfaceTexture,
        output: FullOutput,
        compute: &mut Compute,
        context: &egui::Context,
        scale_factor: f32,
    ) {
        //self.sub_rpass_triangles
        //    .update_vertex_buffer(device, &render_result.triangles);
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });
        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute pass"),
                timestamp_writes: None,
            });
            compute.compute(&mut cpass);
        }
        let clipped_primitives = context.tessellate(output.shapes, 1.0);
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass: RenderPassDescriptor"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.sub_rpass_triangles.render_with_instance_buffer(
                &mut rpass,
                &compute.particles_buffers[0],
                compute.num_particles,
            );
            self.sub_rpass_vector_field.render(&mut rpass);
            self.sub_rpass_cursor.render(&mut rpass);
        }
        {
            // Upload all resources for the GPU.
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.surface_config.width, self.surface_config.height],
                pixels_per_point: scale_factor,
            };
            for (id, image_delta) in &output.textures_delta.set {
                self.egui_rpass
                    .update_texture(&self.device, &self.queue, *id, image_delta);
            }
            for id in &output.textures_delta.free {
                self.egui_rpass.free_texture(id);
            }

            self.egui_rpass.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &clipped_primitives,
                &screen_descriptor,
            );

            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("rpass: RenderPassDescriptor"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_rpass
                .render(&mut rpass, &clipped_primitives, &screen_descriptor);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}
