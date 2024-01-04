use crate::grid::{Bounds, Grid};
use crate::poly7::Poly7;
use crate::{zero_v3, MassWrap, V3};
use bytemuck::{NoUninit, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, NoUninit, Zeroable)]
pub struct SimParams {
    pub attraction_force: [Poly7; 25],
    pub particle_type_masses: [MassWrap; 5],
    pub force_grid_dimensions: [u32; 3],
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
            force_grid_dimensions: [10; 3],
            delta_t: 0.,
            max_velocity: 100.,
            bounding_volume_radius: 10.,
            cut_off_distance: 1.0,
            distance_exponent: 0.,
        }
    }

    pub fn new_force_grid_centered(&self) -> Grid<V3> {
        let bvr = self.bounding_volume_radius;
        let bvr_vec = V3::new(bvr, bvr, bvr);
        Grid::new_centered(
            self.force_grid_dimensions[0] as usize,
            self.force_grid_dimensions[1] as usize,
            self.force_grid_dimensions[2] as usize,
            Bounds {
                pos: -bvr_vec,
                dir: 2.0 * bvr_vec,
            },
        )
    }

    pub fn new_force_grid_zero(&self) -> Grid<V3> {
        let bvr = self.bounding_volume_radius;
        let bvr_vec = V3::new(bvr, bvr, bvr);
        Grid::new_uniform(
            self.force_grid_dimensions[0] as usize,
            self.force_grid_dimensions[1] as usize,
            self.force_grid_dimensions[2] as usize,
            Bounds {
                pos: -bvr_vec,
                dir: 2.0 * bvr_vec,
            },
            &zero_v3(),
        )
    }
}
