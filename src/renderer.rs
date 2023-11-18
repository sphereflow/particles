use crate::gui::Gui;
use crate::draw_pass::DrawPass;
use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use egui::FullOutput;
use egui_wgpu::renderer::ScreenDescriptor;
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub _pos: [f32; 3],
    pub _color: [f32; 4],
    pub _tex_coord: [f32; 2],
}
unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

pub struct Renderer {
    shader: ShaderModule,
    sub_rpass_triangles: DrawPass,
    egui_rpass: egui_wgpu::renderer::Renderer,
    surface_config: SurfaceConfiguration,
    pub make_screenshot: bool,
    pub recreate_pipelines: bool,
}

impl Renderer {
    pub fn init(
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue, // we might need to meddle with the command queue
    ) -> Self {
        use std::borrow::Cow;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer: wgsl shader module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let mut sub_rpass_triangles = DrawPass::new(
            surface_config,
            device,
            queue,
            &shader,
            PrimitiveTopology::TriangleList,
        );
        sub_rpass_triangles.update_vertex_buffer(
            device,
            &[
                (Vector3::new(-0.5, 0.5, 0.5), Color::WHITE),
                (Vector3::new(0.5, 0.5, 0.5), Color::WHITE),
                (Vector3::new(-0.5, -0.5, 0.5), Color::WHITE),
                (Vector3::new(0.5, -0.5, 0.5), Color::WHITE),
            ],
        );
        sub_rpass_triangles.update_index_buffer(device, &[0, 1, 2, 1, 2, 3]);

        let egui_rpass = egui_wgpu::renderer::Renderer::new(device, surface_config.format, None, 1);

        Renderer {
            shader,
            sub_rpass_triangles,
            egui_rpass,
            surface_config: surface_config.clone(),
            make_screenshot: false,
            recreate_pipelines: false,
        }
    }

    fn recreate_pipelines(&mut self, device: &Device, queue: &Queue) {
        self.recreate_pipelines = false;
        self.sub_rpass_triangles.recreate_pipeline(
            &self.surface_config,
            device,
            queue,
            &self.shader,
        );
    }

    pub fn generate_matrix(aspect_ratio: f32) -> cgmath::Matrix4<f32> {
        let mx_projection = cgmath::ortho(-aspect_ratio, aspect_ratio, -1.0, 1.0, 0., 1.);
        let mx_correction = crate::framework::OPENGL_TO_WGPU_MATRIX;
        mx_correction * mx_projection //* mx_view
    }

    pub fn resize(
        &mut self,
        surface_config: &SurfaceConfiguration,
        device: &Device,
        queue: &Queue,
    ) {
        self.surface_config = surface_config.clone();
        self.recreate_pipelines(device, queue);
    }

    pub fn render(
        &mut self,
        frame: &SurfaceTexture,
        device: &Device,
        queue: &Queue,
        output: FullOutput,
        context: &egui::Context,
        scale_factor: f32,
    ) {
        //self.sub_rpass_triangles
        //    .update_vertex_buffer(device, &render_result.triangles);
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });
        let clipped_primitives = context.tessellate(output.shapes);

        {
            let view = frame.texture.create_view(&TextureViewDescriptor::default());

            // Upload all resources for the GPU.
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.surface_config.width, self.surface_config.height],
                pixels_per_point: scale_factor,
            };
            for (id, image_delta) in &output.textures_delta.set {
                self.egui_rpass
                    .update_texture(&device, &queue, *id, image_delta);
            }
            for id in &output.textures_delta.free {
                self.egui_rpass.free_texture(id);
            }

            self.egui_rpass.update_buffers(
                device,
                queue,
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
                        load: LoadOp::Clear(Color::BLACK),
                        store: false,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.sub_rpass_triangles.render(&mut rpass);

            self.egui_rpass
                .render(&mut rpass, &clipped_primitives, &screen_descriptor);
        }

        queue.submit(Some(encoder.finish()));
    }
}
