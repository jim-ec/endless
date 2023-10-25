use std::f32::consts::TAU;

use cgmath::{vec3, Matrix4, Quaternion, Rotation3, Vector3, Vector4};

use crate::{symmetry::Symmetry, world::N};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub translation: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub fovy: f32,
}

/// Converts from a Z-up right-handed coordinate system into a Y-up left-handed coordinate system.
const Y_UP: Matrix4<f32> = Matrix4::from_cols(
    Vector4::new(0.0, 0.0, 1.0, 0.0),
    Vector4::new(1.0, 0.0, 0.0, 0.0),
    Vector4::new(0.0, 1.0, 0.0, 0.0),
    Vector4::new(0.0, 0.0, 0.0, 1.0),
);

impl Camera {
    pub fn initial() -> Self {
        Self {
            translation: Vector3::new(0.5 * N as f32, 0.5 * N as f32, 1.5 * N as f32),
            yaw: 0.4 * TAU,
            pitch: -0.1 * TAU,
            fovy: 60.0,
        }
    }

    pub fn rotation(&self) -> Quaternion<f32> {
        Quaternion::from_angle_z(cgmath::Rad(self.yaw))
            * Quaternion::from_angle_y(cgmath::Rad(self.pitch))
    }

    pub fn symmetry(&self) -> Symmetry {
        let yaw = Quaternion::from_angle_z(cgmath::Rad(self.yaw));
        let pitch = Quaternion::from_angle_y(cgmath::Rad(self.pitch));
        let view_to_world = Symmetry {
            rotation: yaw * pitch,
            translation: self.translation,
            ..Default::default()
        };
        view_to_world.inverse()
    }

    pub fn forward(&self) -> Vector3<f32> {
        let yaw = Quaternion::from_angle_z(cgmath::Rad(self.yaw));
        let pitch = Quaternion::from_angle_y(cgmath::Rad(self.pitch));
        (yaw * pitch) * vec3(-1.0, 0.0, 0.0)
    }

    pub fn left(&self) -> Vector3<f32> {
        let yaw = Quaternion::from_angle_z(cgmath::Rad(self.yaw));
        let pitch = Quaternion::from_angle_y(cgmath::Rad(self.pitch));
        (yaw * pitch) * vec3(0.0, -1.0, 0.0)
    }

    pub fn up(&self) -> Vector3<f32> {
        let yaw = Quaternion::from_angle_z(cgmath::Rad(self.yaw));
        let pitch = Quaternion::from_angle_y(cgmath::Rad(self.pitch));
        (yaw * pitch) * vec3(0.0, 0.0, 1.0)
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        let yaw = Quaternion::from_angle_z(cgmath::Rad(self.yaw));
        let pitch = Quaternion::from_angle_y(cgmath::Rad(self.pitch));
        Matrix4::from(pitch * yaw) * Matrix4::from_translation(-self.translation)
    }

    pub fn proj_matrix(&self, aspect: f32) -> Matrix4<f32> {
        perspective_matrix(f32::to_radians(self.fovy), aspect, 0.1, None) * Y_UP
    }
}

fn perspective_matrix(fovy: f32, aspect: f32, near: f32, far: Option<f32>) -> Matrix4<f32> {
    let tan_half_fovy = (0.5 * fovy).tan();
    if let Some(far) = far {
        Matrix4::from_cols(
            Vector4::new(1.0 / (aspect * tan_half_fovy), 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0 / tan_half_fovy, 0.0, 0.0),
            Vector4::new(0.0, 0.0, -(far + near) / (far - near), -1.0),
            Vector4::new(0.0, 0.0, -2.0 * far * near / (far - near), 0.0),
        )
    } else {
        Matrix4::from_cols(
            Vector4::new(1.0 / (aspect * tan_half_fovy), 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0 / tan_half_fovy, 0.0, 0.0),
            Vector4::new(0.0, 0.0, -1.0, -1.0),
            Vector4::new(0.0, 0.0, -2.0 * near, 0.0),
        )
    }
}
