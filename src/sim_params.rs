use bytemuck::{NoUninit, Zeroable};
use crate::poly7::Poly7;
use crate::MassWrap;



#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, Zeroable)]
pub struct SimParams {
    pub attraction_force: [Poly7; 25],
    pub particle_type_masses: [MassWrap; 5],
    pub vector_field_dimensions: [u32; 3],
    pub delta_t: f32,
    pub max_velocity: f32,
    pub bounding_volume_radius: f32,
    pub cut_off_distance: f32,
    pub distance_exponent: f32,
}

impl SimParams {
    pub fn new() -> Self {
        SimParams {
            attraction_force: [Poly7::new(); 25],
            particle_type_masses: [MassWrap::new(1.0); 5],
            vector_field_dimensions: [10; 3],
            delta_t: 0.,
            max_velocity: 100.,
            bounding_volume_radius: 10.,
            cut_off_distance: 1.0,
            distance_exponent: 0.,
        }
    }
}

