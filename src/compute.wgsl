struct Particle {
  pos: vec4<f32>,
  vel: vec4<f32>,
  ty: u32,
};

// Wrapper type for particle masses to satisfy array stride constraint of : 16 bytes per element
struct MassWrap {
  @size(16) mass: f32,
}

struct Polys {
  attraction_force: array<Poly7, 25>,
  
}

struct SimParams {
  attraction_force: array<Poly7, 25>,
  particle_type_masses: array<MassWrap, 5>,
  deltaT: f32,
  max_velocity: f32,
  // Bounding sphere for all the particles at coordinate system center
  bounding_sphere_radius: f32,
  // the maximum distance particles can influence each other
  cut_off_distance: f32,
  distance_exponent: f32,
};


// 3rd degree polynomials
struct Poly3 {
  a: f32, b: f32, c: f32, d: f32,
}

fn eval_poly3(x: f32, p: Poly3) -> f32 {
  return p.a * x * x * x + p.b * x * x + p.c * x + p.d;
}

struct Poly7 {
  h: f32,
  g: f32,
  f: f32,
  e: f32,
  d: f32,
  c: f32,
  b: f32,
  a: f32,
}

fn eval_poly7(x: f32, p: Poly7) -> f32 {
  let x2 = x * x;
  let x4 = x2 * x2;
  let x3 = x2 * x;
  return p.a * x4 * x3 + p.b * x3 * x3 + p.c * x2 * x3 + p.d * x4 + p.e * x3 + p.f * x2 + p.g * x + p.h;
}

fn wrap_symmetrically(val: f32, max: f32) -> f32 {
  if val > max {
    return val - 2.0 * max;
  }
  if val < -max {
    return val + 2.0 * max;
  }
  return val;
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

    var vPos: vec3<f32> = particlesSrc[index].pos.xyz;
    var vVel: vec3<f32> = particlesSrc[index].vel.xyz;
    var vParticleType: u32 = particlesSrc[index].ty;

    // accumulated acceleration vector
    var cAcc: vec3<f32> = vec3<f32>();
    var cForceCount: i32 = 0;

    var i: u32 = 0u;
    loop {
        if i >= total {
      break;
        }
        if i == index {
          continue;
        }
        
        let direction = particlesSrc[i].pos.xyz - vPos;
        let direction_length = length(direction);
        if direction_length < 0.001 {
          continue;
        }
        if direction_length > params.cut_off_distance {
          continue;
        }
        let distance_factor = pow(direction_length, params.distance_exponent); 
        let directionN: vec3<f32> = normalize(direction);
        let attractionForceIndex: u32 = particlesSrc[i].ty + vParticleType * 5u;
        let m2 = params.particle_type_masses[particlesSrc[i].ty].mass;

        // evaluate attraction force function and add it to the accumulative force
        // normaly to calculate the force you would multiply by both masses but to calculate the acceleration vector m1 would be devided out again
        cAcc += directionN * eval_poly7(direction_length,params.attraction_force[attractionForceIndex]) * m2 *  distance_factor;

        continuing {
            i = i + 1u;
        }
    }

    // let cAcc = cForce / params.particle_type_masses[vParticleType].mass;
    vVel += cAcc * params.deltaT;

    // clamp velocity for a more pleasing simulation
    vVel = normalize(vVel) * clamp(length(vVel), 0.0, params.max_velocity);

    // kinematic update
    vPos += vVel * params.deltaT;

    // if length(vPos) > params.bounding_sphere_radius {
        // TODO : mirror velocity on bounding sphere normal
    // }

    // Wrap around boundary
    vPos.x = wrap_symmetrically(vPos.x, 2.0);
    vPos.y = wrap_symmetrically(vPos.y, 2.0);
    vPos.z = wrap_symmetrically(vPos.z, 2.0);

    // Write back
    particlesDst[index] = Particle(vec4<f32>(vPos, 1.0), vec4<f32>(vVel, 1.0), particlesSrc[index].ty);
}