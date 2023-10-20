use std::f64::consts::TAU;

use cgmath::{vec3, Matrix4, Quaternion, Rotation3, SquareMatrix, Vector3, Vector4};

use crate::grid::N;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub orbit: f64,
    pub tilt: f64,
    pub distance: f64,
    pub origin: Vector3<f64>,
    pub fovy: f64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniforms {
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub pos: Vector4<f32>,
}

unsafe impl bytemuck::Pod for CameraUniforms {}
unsafe impl bytemuck::Zeroable for CameraUniforms {}

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
            orbit: -0.3 * TAU,
            tilt: 0.3 * 0.4 * TAU,
            distance: N as f64,
            origin: 0.5 * vec3(N as f64, N as f64, N as f64),
            fovy: 60.0,
        }
    }

    pub fn clamp_tilt(&mut self) {
        self.tilt = self.tilt.clamp(-TAU / 4.0, TAU / 4.0);
    }

    pub fn pan(&mut self, x: f64, y: f64) {
        let orbit = Quaternion::from_angle_z(cgmath::Rad(self.orbit));
        let tilt = Quaternion::from_angle_y(cgmath::Rad(self.tilt));
        let rotation = tilt * orbit;
        self.origin += rotation.conjugate() * vec3(0.0, x, y);
    }

    pub fn uniforms(&self, aspect: f32) -> CameraUniforms {
        let orbit = Quaternion::from_angle_z(cgmath::Rad(self.orbit));
        let tilt = Quaternion::from_angle_y(cgmath::Rad(self.tilt));
        let translation = Matrix4::from_translation(Vector3::new(-1.0 * self.distance, 0.0, 0.0));

        let view =
            translation * Matrix4::from(tilt * orbit) * Matrix4::from_translation(-self.origin);

        let pos = view.invert().unwrap() * Vector4::new(0.0, 0.0, 0.0, 1.0);

        let proj = perspective_matrix(f32::to_radians(60.0), aspect, 0.1, None) * Y_UP;

        CameraUniforms {
            view: view.cast().unwrap(),
            proj: proj.cast().unwrap(),
            pos: pos.cast().unwrap(),
        }
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
