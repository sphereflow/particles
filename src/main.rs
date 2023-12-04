use crate::camera::Direction;
use bytemuck::{NoUninit, Pod, Zeroable};
use cgmath::{Vector2, Vector3};
use compute::Compute;
use poly3::*;
use renderer::Renderer;
use std::time::Instant;
use wgpu::{Device, Queue, VertexAttribute, VertexBufferLayout, VertexStepMode};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

mod camera;
mod compute;
mod draw_pass;
mod framework;
mod gui;
mod poly3;
mod renderer;

type V3 = Vector3<f32>;
type Key = winit::event::VirtualKeyCode;

const fn zero_v3() -> V3 {
    V3::new(0., 0., 0.)
}

fn rand_v3(max: f32) -> V3 {
    let res = V3::new(
        rand::random::<f32>() - 0.5,
        rand::random::<f32>() - 0.5,
        rand::random::<f32>() - 0.5,
    );
    res * max
}

fn rand_v4(max: f32) -> [f32; 4] {
    [
        max * rand::random::<f32>() - 0.5,
        max * rand::random::<f32>() - 0.5,
        max * rand::random::<f32>() - 0.5,
        1.0,
    ]
}

fn main() {
    framework::wgpu_main();
}

#[repr(C)]
#[derive(Clone, Copy)]
enum ParticleType {
    E = 0,
    W,
    F,
    A,
    N,
}

impl From<u32> for ParticleType {
    fn from(value: u32) -> Self {
        match value % 5 {
            0 => ParticleType::E,
            1 => ParticleType::W,
            2 => ParticleType::F,
            3 => ParticleType::A,
            _ => ParticleType::N,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Particle {
    pos: [f32; 4],
    vel: [f32; 4],
    ty: u32,
    _padd: [u32; 3],
}

impl Particle {
    const fn get_instance_layout() -> VertexBufferLayout<'static> {
        let array_stride = std::mem::size_of::<Particle>() as u64;
        VertexBufferLayout {
            // particle_type : 4, position : 4 * 3, velocity: 4 * 3
            array_stride,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                // particle position
                VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 2,
                },
                // particle type
                VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 4 * 4 * 2,
                    shader_location: 3,
                },
            ],
        }
    }
}

struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    fn new(max: V3, num_x: usize, num_y: usize, num_z: usize) -> Self {
        let mut particles = Vec::with_capacity(num_x * num_y * num_z);
        for ix in 0..num_x {
            for iy in 0..num_y {
                for iz in 0..num_z {
                    let index = ix * num_y * num_z + iy * num_z + iz;
                    particles.push(Particle {
                        pos: [
                            (ix as f32 / num_x as f32) * max.x,
                            (iy as f32 / num_y as f32) * max.y,
                            (iz as f32 / num_z as f32) * max.z,
                            1.0,
                        ],
                        vel: rand_v4(10.0).into(),
                        ty: (index % 5) as u32,
                        _padd: [0; 3],
                    });
                }
            }
        }

        ParticleSystem { particles }
    }

    fn set_num_particles(&mut self, num_particles: usize) {
        while self.particles.len() < num_particles {
            let plen = self.particles.len();
            self.particles.push(Particle {
                pos: rand_v4(2.0),
                vel: rand_v4(10.0),
                ty: (plen % 5) as u32,
                _padd: [0; 3],
            })
        }
        while self.particles.len() > num_particles {
            self.particles.pop();
        }
    }

    fn get_instances(&self) -> Vec<V3> {
        self.particles
            .iter()
            .map(|p| [p.pos[0], p.pos[1], p.pos[2]].into())
            .collect()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
struct MassWrap {
    mass: f32,
    _pad: [f32; 3],
}

impl MassWrap {
    fn new(mass: f32) -> MassWrap {
        MassWrap {
            mass,
            _pad: [0.; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, Zeroable)]
struct SimParams {
    attraction_force: [Poly7; 25],
    particle_type_masses: [MassWrap; 5],
    delta_t: f32,
    max_velocity: f32,
    bounding_sphere_radius: f32,
    cut_off_distance: f32,
    distance_exponent: f32,
}

impl SimParams {
    fn new() -> Self {
        SimParams {
            attraction_force: [Poly7::new(); 25],
            particle_type_masses: [MassWrap::new(1.0); 5],
            delta_t: 0.,
            max_velocity: 100.,
            bounding_sphere_radius: 10.,
            cut_off_distance: 1.0,
            distance_exponent: 0.,
        }
    }
}

struct App {
    time_step: Instant,
    pub psys: ParticleSystem,
    pub sim_params: SimParams,
    pub renderer: Renderer,
    pub compute: Compute,
    pub speed: Option<f32>,
    pressed_keys: Vec<VirtualKeyCode>,
}

impl App {
    fn new(device: &Device, queue: &Queue, mut renderer: Renderer) -> Self {
        let psys = ParticleSystem::new(V3::new(5.0, 2.0, 2.0), 10, 10, 10);
        let compute = Compute::new(device, &psys.particles);
        renderer.recreate_pipelines(device, queue);
        App {
            time_step: Instant::now(),
            psys,
            sim_params: SimParams::new(),
            renderer,
            compute,
            speed: Some(1.0),
            pressed_keys: Vec::new(),
        }
    }

    pub fn winit_update(&mut self, event: &winit::event::WindowEvent) {
        use winit::event;
        use winit::event::WindowEvent;
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if let Some(code) = virtual_keycode {
                    if !self.pressed_keys.contains(code) {
                        self.pressed_keys.push(*code);
                    }
                }
            }

            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode,
                        state: ElementState::Released,
                        ..
                    },
                ..
            } => {
                if let Some(code) = virtual_keycode {
                    self.pressed_keys.retain(|key| key != code);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {}
            WindowEvent::MouseInput {
                state: event::ElementState::Pressed,
                button: event::MouseButton::Left,
                ..
            } => {}
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Left,
                ..
            } => {}
            WindowEvent::MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Right,
                ..
            } => {}
            _ => {}
        }
    }

    fn update(&mut self, device: &Device, queue: &Queue) {
        // get time step
        let elapsed = self.time_step.elapsed().as_secs_f32();
        self.time_step = Instant::now();
        // adjust simulation speed
        if let Some(speed) = self.speed {
            self.sim_params.delta_t = speed * elapsed;
        } else {
            self.sim_params.delta_t = 0.0;
        }

        self.renderer
            .sub_rpass_triangles
            .update_view_matrix(queue, &mut self.renderer.camera);
        self.compute.update_sim_params(device, &self.sim_params);
        for code in &self.pressed_keys {
            match code {
                Key::W => {
                    self.renderer.camera.motion(Direction::Up, elapsed);
                }
                Key::S => {
                    self.renderer.camera.motion(Direction::Down, elapsed);
                }
                Key::A => {
                    self.renderer.camera.motion(Direction::Left, elapsed);
                }
                Key::D => {
                    self.renderer.camera.motion(Direction::Right, elapsed);
                }
                Key::E => {
                    self.renderer.camera.motion(Direction::RotateRight, elapsed);
                }
                Key::U => {
                    self.renderer.camera.motion(Direction::RotateLeft, elapsed);
                }
                _ => {}
            }
        }
    }
}
