use cgmath::{Matrix4, Quaternion, Vector3, Zero};
use derive_setters::Setters;

/// A symmetry in homogenous Euclidean space.
#[derive(Debug, Clone, Copy, Setters)]
pub struct Symmetry {
    pub translation: Vector3<f64>,
    pub rotation: Quaternion<f64>,
}

impl Default for Symmetry {
    fn default() -> Self {
        Symmetry {
            translation: Vector3::zero(),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        }
    }
}

impl Symmetry {
    pub fn matrix(&self) -> Matrix4<f32> {
        let mut m: Matrix4<f64> = self.rotation.into();
        m.w.x = self.translation.x;
        m.w.y = self.translation.y;
        m.w.z = self.translation.z;
        m.cast().unwrap()
    }

    pub fn inverse(&self) -> Symmetry {
        let inverse_orientation = self.rotation.conjugate();
        let inverse_position = inverse_orientation * -self.translation;
        Symmetry {
            translation: inverse_position,
            rotation: inverse_orientation,
        }
    }
}

impl std::ops::Mul<Vector3<f64>> for Symmetry {
    type Output = Vector3<f64>;

    fn mul(self, rhs: Vector3<f64>) -> Self::Output {
        self.rotation * rhs + self.translation
    }
}

/// Compose the right symmetry with the left symmetry.
impl std::ops::Mul for Symmetry {
    type Output = Symmetry;

    fn mul(self, rhs: Symmetry) -> Self::Output {
        Symmetry {
            translation: self.translation + self.rotation * rhs.translation,
            rotation: self.rotation * rhs.rotation,
        }
    }
}
