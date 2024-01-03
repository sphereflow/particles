use crate::cursor::Cursor;
use crate::{framework, zero_v3, V3};
use cgmath::prelude::*;
use cgmath::{Deg, Matrix, Matrix4, Quaternion, Rotation3};

const NEAR_PLANE_DISTANCE: f32 = 0.1;

pub struct Camera {
    persp_mat: Matrix4<f32>,
    screen_width: f32,
    screen_height: f32,
    fov_degrees: f32,
    pub cursor: Cursor,
    pub look_at_distance: Option<(V3, f32)>,
    pos: V3,
    pub units_per_second: f32,
    angle_per_second: f32,
    rot: Quaternion<f32>,
}

impl Camera {
    pub fn new(screen_width: f32, screen_height: f32, fov_degrees: f32) -> Self {
        let aspect = screen_width / screen_height;
        let persp_mat = cgmath::perspective(Deg(fov_degrees), aspect, NEAR_PLANE_DISTANCE, 100.0);
        Camera {
            persp_mat,
            screen_width,
            screen_height,
            fov_degrees,
            cursor: Cursor::new(),
            pos: zero_v3(),
            units_per_second: 10.0,
            angle_per_second: 45.0,
            rot: Quaternion::from_sv(1.0, zero_v3()),
            look_at_distance: Some((zero_v3(), 5.0)),
        }
    }

    pub fn resize(&mut self, screen_width: f32, screen_height: f32) {
        let aspect = screen_width / screen_height;
        self.screen_width = screen_width;
        self.screen_height = screen_height;
        self.update_cursor();
        self.persp_mat = cgmath::perspective(Deg(self.fov_degrees), aspect, 0.1, 100.0);
    }

    pub fn update_cursor(&mut self) {
        self.cursor
            .update(self.screen_width, self.screen_height, self.pos, self.rot);
    }

    // move is a keyword in Rust so this function can not be named 'move'
    pub fn motion(&mut self, direction: Direction, delta_t: f32) {
        let rotation_matrix: Matrix4<f32> = self.rot.into();
        let r2 = rotation_matrix.transpose();
        let fwd = r2.z;
        let right = r2.x;
        let up = r2.y;
        let amount_units = delta_t * self.units_per_second;
        let amount_angle = delta_t * self.angle_per_second;
        let rot_right = Quaternion::from_axis_angle(up.truncate(), Deg(amount_angle));
        let rot_left = Quaternion::from_axis_angle(up.truncate(), Deg(-amount_angle));

        match direction {
            Direction::Up => self.pos += (up * amount_units).truncate(),
            Direction::Down => self.pos -= (up * amount_units).truncate(),
            Direction::Forward => {
                if let Some((_, distance)) = self.look_at_distance.as_mut() {
                    *distance -= amount_units;
                } else {
                    self.pos += (fwd * amount_units).truncate();
                }
            }
            Direction::Backward => {
                if let Some((_, distance)) = self.look_at_distance.as_mut() {
                    *distance += amount_units;
                } else {
                    self.pos -= (fwd * amount_units).truncate()
                }
            }
            Direction::Left => self.pos += (right * amount_units).truncate(),
            Direction::Right => self.pos -= (right * amount_units).truncate(),
            Direction::RotateLeft => self.rot = self.rot * rot_left,
            Direction::RotateRight => self.rot = self.rot * rot_right,
        }
    }

    pub fn get_view_matrix(&mut self) -> Matrix4<f32> {
        if let Some((look_at, distance)) = self.look_at_distance {
            if look_at == self.pos {
                self.pos += V3::new(0., 0., -1.);
            }
            let diff = look_at - self.pos;
            self.pos = look_at - diff.normalize() * distance;
            let up_v = self.rot.invert().rotate_vector(V3::new(0., 1., 0.));
            self.rot = Rotation::look_at(diff, up_v);
        }
        let trans = Matrix4::from_translation(self.pos);
        let rot = Matrix4::from(self.rot);
        framework::OPENGL_TO_WGPU_MATRIX * self.persp_mat * rot * trans
    }
}

pub enum Direction {
    Up,
    Down,
    Forward,
    Backward,
    Left,
    Right,
    RotateLeft,
    RotateRight,
}
