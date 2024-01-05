use crate::{grid::Grid, zero_v3, V3};
use cgmath::{InnerSpace, Matrix, Matrix3, MetricSpace, Quaternion, SquareMatrix};
use winit::event::VirtualKeyCode;

pub struct Cursor {
    pub pos: V3,
    pub distance_from_camera: f32,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub mouse_pos_x: f32,
    pub mouse_pos_y: f32,
    pub modify_vector_indices: Vec<usize>,
    pub mouse_down_vectors: Vec<V3>,
    pub mouse_down_on: Option<(V3, Matrix3<f32>)>,
    pub rot: Matrix3<f32>,
    pub edit_mode: EditMode,
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor {
            pos: zero_v3(),
            rot: Matrix3::identity(),
            distance_from_camera: 5.0,
            outer_radius: 3.0,
            inner_radius: 0.1,
            modify_vector_indices: Vec::new(),
            mouse_down_vectors: Vec::new(),
            mouse_down_on: None,
            mouse_pos_x: 0.,
            mouse_pos_y: 0.,
            edit_mode: EditMode::default(),
        }
    }

    pub fn update(
        &mut self,
        screen_width: f32,
        screen_height: f32,
        camera_position: V3,
        rot: Quaternion<f32>,
    ) {
        let swh = screen_width * 0.5;
        let shh = screen_height * 0.5;
        let aspect = screen_width / screen_height;
        let mouse_x = self.mouse_pos_x;
        let mouse_y = self.mouse_pos_y;
        let rotm: Matrix3<f32> = rot.into();
        let rotm = rotm.transpose();
        let right = rotm.x;
        let up = rotm.y;
        let dir = -rotm.z;
        let offset = dir + right * aspect * ((mouse_x - swh) / swh) + up * ((-mouse_y + shh) / shh);
        let res = -camera_position + offset * self.distance_from_camera;
        self.pos = res;
        self.rot = rotm;
    }

    pub fn process_input(&mut self, keys: &[VirtualKeyCode]) {
        self.edit_mode.mode = EditModeE::Centered;
        if keys.contains(&VirtualKeyCode::Space) {
            self.edit_mode.ra = RelAbE::Absolute;
        } else {
            self.edit_mode.ra = RelAbE::Relative;
        }
        for key in keys {
            match key {
                VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                    self.edit_mode.mode = EditModeE::Rotate
                }
                VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                    self.edit_mode.mode = EditModeE::Shift
                }
                _ => {}
            }
        }
    }

    pub fn mouse_moved(&mut self, mouse_x: f32, mouse_y: f32, grid: &mut Grid<V3>) {
        self.mouse_pos_x = mouse_x;
        self.mouse_pos_y = mouse_y;
        if let Some((md_pos, _mdrot)) = self.mouse_down_on {
            let v_pos_dir = grid.get_instances();
            for (ix, md_v) in self
                .modify_vector_indices
                .iter()
                .zip(&self.mouse_down_vectors)
            {
                let displacement = self.edit_mode.get_vector(
                    v_pos_dir[*ix].1,
                    v_pos_dir[*ix].0,
                    md_pos,
                    self.pos,
                    self.rot,
                );
                // grid.grid[*ix] = self.pos - md_pos;
                match self.edit_mode.ra {
                    RelAbE::Relative => grid.grid[*ix] = displacement + md_v,
                    RelAbE::Absolute => grid.grid[*ix] = displacement,
                }
            }
        }
    }

    pub fn mouse_down(&mut self, grid: &Grid<V3>) {
        self.mouse_down_on = Some((self.pos, self.rot));
        self.modify_vector_indices.clear();
        self.mouse_down_vectors.clear();
        for (ix, (vpos, _)) in grid.get_instances().iter().enumerate() {
            if self.pos.distance(*vpos) < self.edit_mode.falloff_dist {
                self.modify_vector_indices.push(ix);
                self.mouse_down_vectors.push(grid.grid[ix]);
            }
        }
        dbg!(&self.modify_vector_indices);
    }

    pub fn mouse_up(&mut self) {
        self.mouse_down_on = None;
        self.modify_vector_indices.clear();
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum EditModeE {
    Centered,
    Shift,
    Rotate,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum RelAbE {
    Relative,
    Absolute,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Falloff {
    Abrupt,
    Linear,
    InverseDistance,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct EditMode {
    pub mode: EditModeE,
    pub ra: RelAbE,
    pub falloff: Falloff,
    pub falloff_dist: f32,
    pub strength: f32,
}

impl EditMode {
    /// v: the vector to be editted
    /// v_pos: the position of the vector to be editted
    /// md_pos: the 3d coordinates of the mouse when it was clicked
    /// cursor_pos: the current location of the cursor
    fn get_vector(
        &self,
        v: V3,
        v_pos: V3,
        md_pos: V3,
        cursor_pos: V3,
        cam_rot: Matrix3<f32>,
    ) -> V3 {
        let mut res;
        let cam_dir = cam_rot.z;
        let cam_right = cam_rot.x;
        match self.mode {
            EditModeE::Centered => {
                // a unit vector points to cursor_pos
                res = (cursor_pos - v_pos).normalize();
            }
            EditModeE::Shift => res = cursor_pos - md_pos,
            EditModeE::Rotate => {
                if cursor_pos == md_pos {
                    res = v;
                } else {
                    // everything should be in relation to the position of the cursor on mouse down:
                    // md_pos

                    // normalized direction of the cursor
                    let dc = cursor_pos - md_pos;

                    // direction of the vectors position
                    let dv = (v_pos - md_pos).normalize();
                    let dir = dv.cross(cam_dir);
                    res = dir * dc.dot(cam_right);
                }
            }
        }
        let res_len = res.magnitude();
        res = match self.falloff {
            Falloff::Abrupt => res,
            Falloff::Linear => {
                let mag = (md_pos - v_pos).magnitude();
                let factor = mag / self.falloff_dist;
                (1.0 - factor) * res
            }
            Falloff::InverseDistance => (self.falloff_dist / (res_len + 1.0)) * res,
        };
        res * self.strength
    }
}

impl Default for EditMode {
    fn default() -> Self {
        EditMode {
            mode: EditModeE::Shift,
            ra: RelAbE::Relative,
            falloff: Falloff::Abrupt,
            falloff_dist: 1.0,
            strength: 1.0,
        }
    }
}
