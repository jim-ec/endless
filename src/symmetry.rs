use cgmath::{Matrix4, Quaternion, Vector3, VectorSpace, Zero};
use derive_setters::Setters;

/// A conformal symmetry.
#[derive(Debug, Clone, Copy, Setters)]
pub struct Symmetry {
    pub rotation: Quaternion<f32>,
    pub translation: Vector3<f32>,
    pub scale: f32,
}

impl Default for Symmetry {
    fn default() -> Self {
        Symmetry {
            translation: Vector3::zero(),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: 1.0,
        }
    }
}

impl Symmetry {
    pub fn matrix(&self) -> Matrix4<f32> {
        let mut m: Matrix4<f32> = self.rotation.into();
        m[0] *= self.scale;
        m[1] *= self.scale;
        m[2] *= self.scale;
        m.w.x = self.translation.x;
        m.w.y = self.translation.y;
        m.w.z = self.translation.z;
        m
    }

    pub fn inverse(&self) -> Symmetry {
        let inverse_scale = 1.0 / self.scale;
        let inverse_rotation = self.rotation.conjugate();
        let inverse_translation = inverse_rotation * (inverse_scale * -self.translation);
        Symmetry {
            translation: inverse_translation,
            rotation: inverse_rotation,
            scale: inverse_scale,
        }
    }

    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        Symmetry {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale + (other.scale - self.scale) * t,
        }
    }
}

impl std::ops::Mul<Vector3<f32>> for Symmetry {
    type Output = Vector3<f32>;

    fn mul(self, rhs: Vector3<f32>) -> Self::Output {
        self.rotation * rhs + self.translation
    }
}

/// Compose the right symmetry with the left symmetry.
impl std::ops::Mul for Symmetry {
    type Output = Symmetry;

    fn mul(self, rhs: Symmetry) -> Self::Output {
        Symmetry {
            translation: self.translation + self.rotation * (self.scale * rhs.translation),
            rotation: self.rotation * rhs.rotation,
            scale: self.scale * rhs.scale,
        }
    }
}

#[cfg(test)]
mod test {
    use cgmath::{vec3, AbsDiffEq, Deg, Rotation3, SquareMatrix};

    use super::*;

    #[test]
    fn compose() {
        let a = Symmetry {
            translation: Vector3::new(1.0, 2.0, 3.0),
            rotation: Quaternion::from_angle_x(Deg(80.0)),
            scale: 2.0,
        };

        let ma = Matrix4::from_translation(vec3(1.0, 2.0, 3.0))
            * Matrix4::from_scale(2.0)
            * Matrix4::from_angle_x(Deg(80.0));

        let b = Symmetry {
            translation: Vector3::new(4.0, 5.0, 6.0),
            rotation: Quaternion::from_angle_y(Deg(90.0)),
            scale: 3.0,
        };

        let mb = Matrix4::from_translation(vec3(4.0, 5.0, 6.0))
            * Matrix4::from_scale(3.0)
            * Matrix4::from_angle_y(Deg(90.0));

        let c = a * b;
        let m = ma * mb;

        assert!(c.matrix().abs_diff_eq(&m, 1e-6));
    }

    #[test]
    fn matrix() {
        let s = Symmetry {
            translation: Vector3::new(1.0, 2.0, 3.0),
            rotation: Quaternion::from_angle_x(Deg(80.0)),
            scale: 2.0,
        };

        let m = Matrix4::from_translation(vec3(1.0, 2.0, 3.0))
            * Matrix4::from_scale(2.0)
            * Matrix4::from_angle_x(Deg(80.0));

        assert!(s.matrix().abs_diff_eq(&m, 1e-6));
    }

    #[test]
    fn inverse() {
        let s = Symmetry {
            translation: Vector3::new(1.0, 2.0, 3.0),
            rotation: Quaternion::from_angle_y(Deg(80.0)),
            scale: 2.0,
        };

        let si = s.inverse();

        let m = Matrix4::from_translation(vec3(1.0, 2.0, 3.0))
            * Matrix4::from(Quaternion::from_angle_y(Deg(80.0)))
            * Matrix4::from_scale(2.0);
        let mi = m.invert().unwrap();

        assert!(si.matrix().abs_diff_eq(&mi, 1e-6));
    }

    #[test]
    fn composing_with_inverse() {
        let s = Symmetry {
            translation: Vector3::new(1.0, 2.0, 3.0),
            rotation: Quaternion::from_angle_y(Deg(80.0)),
            scale: 2.0,
        };

        let si = s.inverse();

        let i = s * si;
        assert!(i.translation.abs_diff_eq(&Vector3::zero(), 1e-6));
        assert!(i
            .rotation
            .abs_diff_eq(&Quaternion::new(1.0, 0.0, 0.0, 0.0), 1e-6));
        assert!(i.scale.abs_diff_eq(&1.0, 1e-6));
    }
}
