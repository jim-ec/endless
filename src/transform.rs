use cgmath::{Matrix4, Quaternion, Vector3, Zero};
use derive_setters::Setters;

#[derive(Debug, Clone, Copy, Setters)]
pub struct Transform {
    pub position: Vector3<f64>,
    pub rotation: Quaternion<f64>,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: Vector3::zero(),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Matrix4<f32> {
        let mut m: Matrix4<f64> = self.rotation.into();
        m.w.x = self.position.x;
        m.w.y = self.position.y;
        m.w.z = self.position.z;
        m.cast().unwrap()
    }

    pub fn inverse(&self) -> Transform {
        let inverse_orientation = self.rotation.conjugate();
        let inverse_position = inverse_orientation * -self.position;
        Transform {
            position: inverse_position,
            rotation: inverse_orientation,
        }
    }
}

impl std::ops::Mul<Vector3<f64>> for Transform {
    type Output = Vector3<f64>;

    fn mul(self, rhs: Vector3<f64>) -> Self::Output {
        self.rotation * rhs + self.position
    }
}

/// The resulting frame first applies `other` and then `self`.
impl std::ops::Mul for Transform {
    type Output = Transform;

    fn mul(self, rhs: Transform) -> Self::Output {
        Transform {
            position: self.position + self.rotation * rhs.position,
            rotation: self.rotation * rhs.rotation,
        }
    }
}
