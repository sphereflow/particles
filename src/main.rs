use cgmath::Vector3;
use instant::*;

mod framework;
mod gui;
mod renderer;
mod sub_render_pass;

type V3 = Vector3<f32>;

const fn zero_v3() -> V3 {
    V3::new(0., 0., 0.)
}

fn rand_v3() -> V3 {
    V3::new(rand::random(), rand::random(), rand::random())
}

fn main() {
    framework::wgpu_main();
}

#[repr(C)]
enum ParticleType {
    E = 0,
    W,
    F,
    A,
    N,
}

#[repr(C)]
struct Particle {
    ty: ParticleType,
    pos: V3,
    vel: V3,
}

struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    fn new(num_x: usize, num_y: usize, num_z: usize) -> Self {
        let mut particles = Vec::with_capacity(num_x * num_y * num_z);
        for ix in 0..num_x {
            for iy in 0..num_y {
                for iz in 0..num_z {
                    let index = ix * num_y * num_z + iy * num_z + iz;
                    let ty = match index % 5 {
                        0 => ParticleType::E,
                        1 => ParticleType::W,
                        2 => ParticleType::F,
                        3 => ParticleType::A,
                        _ => ParticleType::N,
                    };
                    particles.push(Particle {
                        ty,
                        pos: Vector3 {
                            x: ix as f32,
                            y: iy as f32,
                            z: iz as f32,
                        },
                        vel: zero_v3(),
                    });
                }
            }
        }

        ParticleSystem { particles }
    }

    fn render(&self) {}
}

#[repr(C)]
struct SimParams {
    bounding_sphere_radius: f32,
    delta_t: f32,
    attraction_force: [Poly3; 25],
    particle_type_masses: [f32; 5],
    max_velocity: f32,
}

impl SimParams {
    fn new() -> Self {
        SimParams {
            bounding_sphere_radius: 10.,
            delta_t: 0.,
            attraction_force: [Poly3::new(); 25],
            particle_type_masses: [1.; 5],
            max_velocity: 100.,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Poly3 {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

impl Poly3 {
    fn new() -> Self {
        Poly3 {
            a: 0.,
            b: 0.,
            c: 0.,
            d: 0.,
        }
    }

    fn eval(&self, x: f32) -> f32 {
        let x2 = x * x;
        let x3 = x2 * x;
        self.a * x3 + self.b * x2 * self.c * x + self.d
    }
}

struct App {
    time_step: Instant,
    pub psys: ParticleSystem,
    pub sim_params: SimParams,
}

impl App {
    fn new() -> Self {
        App {
            time_step: Instant::now(),
            psys: ParticleSystem::new(10, 10, 10),
            sim_params: SimParams::new(),
        }
    }

    fn update(&mut self) {
        // get time step
        self.sim_params.delta_t = self.time_step.elapsed().as_secs_f32();
        // upload SimParams
        // execute compute shader
    }

    fn render(&self) {
        self.psys.render();
    }
}
