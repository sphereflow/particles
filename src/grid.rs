use crate::V3;
use cgmath::{MetricSpace, Vector3};
use egui::ahash::HashSet;

/// AABB
pub struct Bounds {
    /// bottom left front corner
    pub pos: Vector3<f32>,
    /// bottom left front to uper right back corner
    pub dir: Vector3<f32>,
}

impl Bounds {
    pub fn left(&self) -> f32 {
        self.pos.x
    }
    pub fn right(&self) -> f32 {
        self.pos.x + self.dir.x
    }
    pub fn bottom(&self) -> f32 {
        self.pos.y
    }
    pub fn top(&self) -> f32 {
        self.pos.y + self.dir.y
    }
    pub fn front(&self) -> f32 {
        self.pos.z
    }
    pub fn back(&self) -> f32 {
        self.pos.z + self.dir.z
    }
    pub fn set_centered(&mut self, bounding_box_side_length: f32) {
        let bbs = bounding_box_side_length;
        self.pos = V3::new(-bbs, -bbs, -bbs) * 0.5;
        self.dir = V3::new(bbs, bbs, bbs);
    }
    pub fn center(&self) -> V3 {
        self.pos + 0.5 * self.dir
    }
}

pub struct Grid<T> {
    pub grid: Vec<T>,
    size: Vector3<u32>,
    pub bounds: Bounds,
}

impl<T: Clone> Grid<T> {
    pub fn new_uniform(n_x: usize, n_y: usize, n_z: usize, bounds: Bounds, t: &T) -> Self {
        let cap = n_x * n_y * n_z;
        let mut grid = Vec::with_capacity(cap);
        for _ in 0..cap {
            grid.push(t.clone());
        }
        Grid {
            grid,
            size: Vector3 {
                x: n_x as u32,
                y: n_y as u32,
                z: n_z as u32,
            },
            bounds,
        }
    }
}

impl Grid<Vector3<f32>> {
    pub fn new_centered(n_x: usize, n_y: usize, n_z: usize, bounds: Bounds) -> Self {
        let cap = n_x * n_y * n_z;
        let mut v = Vec::with_capacity(cap);
        let center = V3::new(n_x as f32, n_y as f32, n_z as f32) * 0.5;
        for i_x in 0..n_x {
            for i_y in 0..n_y {
                for i_z in 0..n_z {
                    v.push(
                        center
                            - Vector3 {
                                x: i_x as f32,
                                y: i_y as f32,
                                z: i_z as f32,
                            },
                    );
                }
            }
        }
        Grid {
            grid: v,
            size: Vector3 {
                x: n_x as u32,
                y: n_y as u32,
                z: n_z as u32,
            },
            bounds,
        }
    }

    pub fn get_force_vectors(&self) -> Vec<[f32; 4]> {
        self.grid
            .iter()
            .copied()
            .map(|v| [v.x, v.y, v.z, 1.0])
            .collect()
    }

    pub fn num_instances(&self) -> usize {
        (self.size.x * self.size.y * self.size.z) as usize
    }

    pub fn get_indices(&self, center: V3, radius: f32) -> Vec<usize> {
        let mut res = Vec::new();
        for (ix, (pos, _dir)) in self.get_instances().iter().enumerate() {
            if pos.distance(center) < radius {
                res.push(ix);
            }
        }
        res
    }

    pub fn get_positions(&self) -> Vec<[f32; 4]> {
        let mut res = Vec::new();
        for i_x in 0..self.size.x {
            for i_y in 0..self.size.y {
                for i_z in 0..self.size.z {
                    let p = [
                        self.bounds.left()
                            + self.bounds.dir.x * (((i_x as f32) + 0.5) / (self.size.x as f32)),
                        self.bounds.bottom()
                            + self.bounds.dir.y * (((i_y as f32) + 0.5) / (self.size.y as f32)),
                        self.bounds.front()
                            + self.bounds.dir.z * (((i_z as f32) + 0.5) / (self.size.z as f32)),
                        1.0,
                    ];
                    res.push(p);
                }
            }
        }
        res
    }

    pub fn get_instances(&self) -> Vec<(V3, V3)> {
        let positions = self.get_positions();
        positions
            .iter()
            .zip(&self.grid)
            .map(|(pos, dir)| (V3::new(pos[0], pos[1], pos[2]), *dir))
            .collect()
    }

    pub fn get_instances_raw(&self, selected_indices: &[usize]) -> Vec<f32> {
        let positions = self.get_positions();
        let index_set = HashSet::from_iter(selected_indices.iter());
        positions
            .iter()
            .zip(&self.grid)
            .enumerate()
            .flat_map(|(ix, (pos, dir))| {
                if index_set.contains(&ix) {
                    [
                        pos[0], pos[1], pos[2], pos[3], dir.x, dir.y, dir.z, 1.0, 1.0, 1.0, 1.0,
                        1.0,
                    ]
                } else {
                    [
                        pos[0], pos[1], pos[2], pos[3], dir.x, dir.y, dir.z, 1.0, 0.5, 0.5, 0.5,
                        0.2,
                    ]
                }
            })
            .collect()
    }
}
