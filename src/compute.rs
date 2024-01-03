use std::{borrow::Cow, num::NonZeroU64, u64};

use crate::{Particle, SimParams};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

const PARTICLES_PER_GROUP: usize = 64;

pub struct Compute {
    sim_param_buffer: Buffer,
    pub particles_buffers: [Buffer; 2],
    force_grid_buffer: Buffer,
    bind_group_layout: BindGroupLayout,
    swap_bind_groups: [BindGroup; 2],
    // 0 or 1 depending on which BindGroup is used
    swap: usize,
    pub num_particles: usize,
    num_workgroups: usize,
    pipeline: ComputePipeline,
}

impl Compute {
    pub fn new(device: &Device, particles: &[Particle], force_grid: &[[f32; 4]]) -> Self {
        let num_particles = particles.len();
        let num_workgroups =
            ((num_particles as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as usize;
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("compute shader module"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("compute.wgsl"))),
        });
        let sim_params = SimParams::new();
        let sim_param_desc = BufferInitDescriptor {
            label: Some("SimParams buffer init descriptor"),
            contents: bytemuck::bytes_of(&sim_params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        };
        let sim_param_buffer = device.create_buffer_init(&sim_param_desc);
        let sim_param_entry = BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(
                    NonZeroU64::new(std::mem::size_of::<SimParams>() as u64).unwrap(),
                ),
            },
            count: None,
        };
        let particles_buffer1 = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("particles src buffer"),
            contents: bytemuck::cast_slice(particles),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let particles_buffer2 = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("particles dst buffer"),
            contents: bytemuck::cast_slice(particles),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let particles_buffers = [particles_buffer1, particles_buffer2];

        let particles_src_entry = BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        dbg!(particles_src_entry);
        dbg!(force_grid.len());
        let particles_dst_entry = BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let force_grid_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("force grid buffer"),
            contents: bytemuck::cast_slice(force_grid),
            usage: BufferUsages::STORAGE,
        });
        let force_grid_entry = BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let bind_group_layout_desc = BindGroupLayoutDescriptor {
            label: Some("compute shader bind group layout entry descriptor"),
            entries: &[
                sim_param_entry,
                particles_src_entry,
                particles_dst_entry,
                force_grid_entry,
            ],
        };
        let bind_group_layout = device.create_bind_group_layout(&bind_group_layout_desc);
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("compute pipeline layout descriptor"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline_descriptor = ComputePipelineDescriptor {
            label: Some("compute pipeline descriptor"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        };
        let pipeline = device.create_compute_pipeline(&pipeline_descriptor);
        let particles_buffer_refs = [&particles_buffers[0], &particles_buffers[1]];

        let particles_buffers_bind_groups = Compute::create_bind_groups(
            device,
            &bind_group_layout,
            &sim_param_buffer,
            &particles_buffer_refs,
            &force_grid_buffer,
        );

        Compute {
            sim_param_buffer,
            bind_group_layout,
            swap_bind_groups: particles_buffers_bind_groups,
            swap: 0,
            particles_buffers,
            force_grid_buffer,
            num_particles,
            num_workgroups,
            pipeline,
        }
    }

    fn create_bind_groups(
        device: &Device,
        layout: &BindGroupLayout,
        sim_param_buffer: &Buffer,
        particles_buffers: &[&Buffer; 2],
        force_grid_buffer: &Buffer,
    ) -> [BindGroup; 2] {
        // create two bind groups,
        // where the 2 particles buffers alternate between src and dst
        std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particles_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: force_grid_buffer.as_entire_binding(),
                    }
                ],
                label: None,
            })
        })
    }

    pub fn upload_particles(&mut self, device: &Device, particles: &[Particle]) {
        self.num_particles = particles.len();
        self.num_workgroups =
            ((self.num_particles as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as usize;
        self.particles_buffers[0] = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("particles src buffer"),
            contents: bytemuck::cast_slice(particles),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        self.particles_buffers[1] = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("particles src buffer"),
            contents: bytemuck::cast_slice(particles),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
    }

    pub fn update_force_grid(&mut self, device: &Device, force_grid: &[[f32; 4]]) {
        self.force_grid_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("force grid buffer"),
            contents: bytemuck::cast_slice(force_grid),
            usage: BufferUsages::STORAGE,
        });
    }

    pub fn update_sim_params(&mut self, device: &Device, sim_params: &SimParams) {
        self.sim_param_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("SimParams buffer init descriptor"),
            contents: bytemuck::bytes_of(sim_params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        self.swap_bind_groups = Self::create_bind_groups(
            device,
            &self.bind_group_layout,
            &self.sim_param_buffer,
            &[&self.particles_buffers[0], &self.particles_buffers[1]],
            &self.force_grid_buffer,
        );
    }

    pub fn compute<'a>(&'a mut self, cpass: &mut ComputePass<'a>) {
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &self.swap_bind_groups[self.swap], &[]);
        cpass.dispatch_workgroups(self.num_workgroups as u32, 1, 1);
        self.swap += 1;
        self.swap %= 2;
    }
}
