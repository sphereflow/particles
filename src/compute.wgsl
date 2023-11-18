struct Particle {
  ty: u32,
  pos: vec3<f32>,
  vel: vec3<f32>,
};

struct SimParams {
  // Bounding sphere for all the particles at coordinate system center
  bounding_sphere_radius: f32,
  deltaT: f32,
  attraction_force: array<Poly3, 25>,
  particle_type_masses: array<f32, 5>,
  max_velocity: f32,
};

// 3rd degree polynomials
struct Poly3 {
  a: f32, b: f32, c: f32, d: f32,
}

fn eval_poly3(x: f32, p: Poly3) -> f32 {
  return p.a * x * x * x + p.b * x * x + p.c * x + p.d;
}

@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;

// https://github.com/austinEng/Project6-Vulkan-Flocking/blob/master/data/shaders/computeparticles/particle.comp
@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = arrayLength(&particlesSrc);
    let index = global_invocation_id.x;
    if index >= total {
        return;
    }

    var vPos: vec3<f32> = particlesSrc[index].pos;
    var vVel: vec3<f32> = particlesSrc[index].vel;
    var vParticleType: u32 = particlesSrc[index].ty;

    // accumulated force vector
    var cForce: vec3<f32> = vec3<f32>();
    var cForceCount: i32 = 0;

    var i: u32 = 0u;
    loop {
        if i >= total {
      break;
        }
        if i == index {
      continue;
        }
        
        let direction = particlesSrc[i].pos - vPos;
        let direction_length = length(direction);
        let directionN: vec3<f32> = normalize(direction);
        let attractionForceIndex: u32 = particlesSrc[i].ty + vParticleType * 5u32;

        // evaluate attraction force function and add it to the accumulative force
        cForce += eval_poly3(direction_length,params.attraction_force[attractionForceIndex]);

        if length(vPos) > params.bounding_sphere_radius {
            // TODO : mirror velocity on bounding sphere normal
        }

        continuing {
            i = i + 1u;
        }
    }

    let cAcc = cForce / params.particle_type_masses[vParticleType];
    vVel += vVel + cAcc * params.deltaT;

  // clamp velocity for a more pleasing simulation
    vVel = normalize(vVel) * clamp(length(vVel), 0.0, params.max_velocity);

  // kinematic update
    vPos += vVel * params.deltaT;

  // Wrap around boundary
    if vPos.x < -1.0 {
        vPos.x = 1.0;
    }
    if vPos.x > 1.0 {
        vPos.x = -1.0;
    }
    if vPos.y < -1.0 {
        vPos.y = 1.0;
    }
    if vPos.y > 1.0 {
        vPos.y = -1.0;
    }

  // Write back
    particlesDst[index] = Particle(particlesSrc[i].ty, vPos, vVel);
}
