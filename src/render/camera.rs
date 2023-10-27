use cgmath::InnerSpace;

use crate::console_log;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,

    pub pitch: f32,
    pub yaw: f32,

    pub aspect: f32,
    pub fovy: f32,
    znear: f32,
    zfar: f32
}

impl Camera {
    pub fn new(eye: cgmath::Point3<f32>, up: cgmath::Vector3<f32>, pitch: f32, yaw: f32, aspect: f32, fovy: f32) -> Self {
        Camera {
            eye,
            up,
            pitch,
            yaw,
            aspect,
            fovy,
            znear: 0.01,
            zfar: 1000.0
        }
    }

    pub fn get_direction(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
    }

    pub fn get_forward(&self) -> cgmath::Vector3<f32> {
        cgmath::Vector3::new(
            self.yaw.cos(),
            0.0,
            self.yaw.sin()
        )
    }

    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_to_rh(
            self.eye,
            self.get_direction(),
            self.up
        );

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn do_move(&mut self, forward: f32, right: f32, up: f32) {
        let forward = self.get_forward() * forward;
        let right = self.get_forward().cross(self.up).normalize() * right;
        let up = self.up.normalize() * up;

        self.eye += forward;
        self.eye += right;
        self.eye += up;
    }
}