use std::fmt::Debug;

use bitflags::bitflags;
use cgmath::{vec3, InnerSpace, Vector3, Zero};

use crate::util::profile;

#[derive(Clone)]
pub struct Field<T, const D: usize> {
    voxels: Vec<T>,
    extent: usize,
}

impl<T, const D: usize> std::ops::Index<[usize; D]> for Field<T, D> {
    type Output = T;

    #[track_caller]
    fn index(&self, index: [usize; D]) -> &Self::Output {
        &self.voxels[self.linear(index)]
    }
}

impl<T, const D: usize> std::ops::IndexMut<[usize; D]> for Field<T, D> {
    #[track_caller]
    fn index_mut(&mut self, index: [usize; D]) -> &mut Self::Output {
        let linear_index = self.linear(index);
        &mut self.voxels[linear_index]
    }
}

#[derive(Clone, Copy)]
struct CoordinateIter<const D: usize>(Option<[usize; D]>, usize);

impl<const D: usize> CoordinateIter<D> {
    fn new(extent: usize) -> Self {
        CoordinateIter(None, extent)
    }
}

impl<const D: usize> Iterator for CoordinateIter<D> {
    type Item = [usize; D];

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            None => {
                self.0 = Some([0; D]);
                Some([0; D])
            }
            Some(co) => {
                for i in (0..D).rev() {
                    if co[i] < self.1 - 1 {
                        co[i] += 1;
                        return Some(*co);
                    } else {
                        co[i] = 0;
                    }
                }
                None
            }
        }
    }
}

pub fn coordinates<const D: usize>(extent: usize) -> impl Iterator<Item = [usize; D]> {
    CoordinateIter::new(extent)
}

impl<T, const D: usize> Field<T, D> {
    pub fn new(label: &str, extent: usize, mut f: impl FnMut([usize; D]) -> T) -> Self {
        let mut voxels = Vec::with_capacity(extent.pow(3));
        profile(label, || {
            for co in coordinates(extent) {
                voxels.push(f(co));
            }
        });
        Field { voxels, extent }
    }

    pub fn coordinates(&self) -> impl Iterator<Item = [usize; D]> {
        coordinates(self.extent)
    }

    fn linear(&self, coordinate: [usize; D]) -> usize {
        let mut index = 0;
        for c in coordinate.iter().take(D - 1) {
            index += c;
            index *= self.extent;
        }
        index += coordinate[D - 1];
        index
    }

    pub fn extent(&self) -> usize {
        self.extent
    }
}

impl<T: Copy, const D: usize> Field<T, D> {
    pub fn map<R>(&self, label: &str, f: impl Fn(T) -> R) -> Field<R, D> {
        Field::new(label, self.extent, |co| f(self[co]))
    }

    pub fn map_with_coordinate<R>(
        &self,
        label: &str,
        mut f: impl FnMut(T, [usize; D]) -> R,
    ) -> Field<R, D> {
        Field::new(label, self.extent, |co| f(self[co], co))
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Vis: u8 {
        const XP = 1 << 0;
        const XN = 1 << 1;
        const YP = 1 << 2;
        const YN = 1 << 3;
        const ZP = 1 << 4;
        const ZN = 1 << 5;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Env: u32 {
        const ZZZ = 1 << 0;
        const ZZP = 1 << 1;
        const ZZN = 1 << 2;
        const ZPZ = 1 << 3;
        const ZPP = 1 << 4;
        const ZPN = 1 << 5;
        const ZNZ = 1 << 6;
        const ZNP = 1 << 7;
        const ZNN = 1 << 8;
        const PZZ = 1 << 9;
        const PZP = 1 << 10;
        const PZN = 1 << 11;
        const PPZ = 1 << 12;
        const PPP = 1 << 13;
        const PPN = 1 << 14;
        const PNZ = 1 << 15;
        const PNP = 1 << 16;
        const PNN = 1 << 17;
        const NZZ = 1 << 18;
        const NZP = 1 << 19;
        const NZN = 1 << 20;
        const NPZ = 1 << 21;
        const NPP = 1 << 22;
        const NPN = 1 << 23;
        const NNZ = 1 << 24;
        const NNP = 1 << 25;
        const NNN = 1 << 26;
    }
}

impl Field<bool, 3> {
    /// Compute the direct neighourhood of each voxel.
    pub fn environment(&self) -> Field<Env, 3> {
        self.map_with_coordinate("Environment", |set, [x, y, z]| {
            let mut env = Env::empty();

            let xp = x < self.extent - 1;
            let xn = x > 0;
            let yp = y < self.extent - 1;
            let yn = y > 0;
            let zp = z < self.extent - 1;
            let zn = z > 0;

            env.set(Env::ZZZ, set);
            env.set(Env::ZZP, zp && self[[x, y, z + 1]]);
            env.set(Env::ZZN, zn && self[[x, y, z - 1]]);
            env.set(Env::ZPZ, yp && self[[x, y + 1, z]]);
            env.set(Env::ZPP, yp && zp && self[[x, y + 1, z + 1]]);
            env.set(Env::ZPN, yp && zn && self[[x, y + 1, z - 1]]);
            env.set(Env::ZNZ, yn && self[[x, y - 1, z]]);
            env.set(Env::ZNP, yn && zp && self[[x, y - 1, z + 1]]);
            env.set(Env::ZNN, yn && zn && self[[x, y - 1, z - 1]]);
            env.set(Env::PZZ, xp && self[[x + 1, y, z]]);
            env.set(Env::PZP, xp && zp && self[[x + 1, y, z + 1]]);
            env.set(Env::PZN, xp && zn && self[[x + 1, y, z - 1]]);
            env.set(Env::PPZ, xp && yp && self[[x + 1, y + 1, z]]);
            env.set(Env::PPP, xp && yp && zp && self[[x + 1, y + 1, z + 1]]);
            env.set(Env::PPN, xp && yp && zn && self[[x + 1, y + 1, z - 1]]);
            env.set(Env::PNZ, xp && yn && self[[x + 1, y - 1, z]]);
            env.set(Env::PNP, xp && yn && zp && self[[x + 1, y - 1, z + 1]]);
            env.set(Env::PNN, xp && yn && zn && self[[x + 1, y - 1, z - 1]]);
            env.set(Env::NZZ, xn && self[[x - 1, y, z]]);
            env.set(Env::NZP, xn && zp && self[[x - 1, y, z + 1]]);
            env.set(Env::NZN, xn && zn && self[[x - 1, y, z - 1]]);
            env.set(Env::NPZ, xn && yp && self[[x - 1, y + 1, z]]);
            env.set(Env::NPP, xn && yp && zp && self[[x - 1, y + 1, z + 1]]);
            env.set(Env::NPN, xn && yp && zn && self[[x - 1, y + 1, z - 1]]);
            env.set(Env::NNZ, xn && yn && self[[x - 1, y - 1, z]]);
            env.set(Env::NNP, xn && yn && zp && self[[x - 1, y - 1, z + 1]]);
            env.set(Env::NNN, xn && yn && zn && self[[x - 1, y - 1, z - 1]]);
            env
        })
    }

    pub fn shell(&self, env: &Field<Env, 3>) -> Field<bool, 3> {
        self.map_with_coordinate("Shell", |set, c| set && !env[c].is_all())
    }
}

impl Field<Env, 3> {
    /// Compute visible faces of each voxel i.e. faces that are not internal.
    /// Faces at the field boundary are not considered to be visible.
    pub fn visibility(&self) -> Field<Vis, 3> {
        self.map_with_coordinate("Visibility", |env, [x, y, z]| {
            let mut vis = Vis::empty();

            let xp = x < self.extent - 1;
            let xn = x > 0;
            let yp = y < self.extent - 1;
            let yn = y > 0;
            let zp = z < self.extent - 1;
            let zn = z > 0;

            vis.set(Vis::XP, xp && !env.contains(Env::PZZ));
            vis.set(Vis::XN, xn && !env.contains(Env::NZZ));
            vis.set(Vis::YP, yp && !env.contains(Env::ZPZ));
            vis.set(Vis::YN, yn && !env.contains(Env::ZNZ));
            vis.set(Vis::ZP, zp && !env.contains(Env::ZZP));
            vis.set(Vis::ZN, zn && !env.contains(Env::ZZN));
            vis
        })
    }
}

impl Field<Vis, 3> {
    pub fn normals(&self) -> Field<Vector3<f32>, 3> {
        self.map("Normals", |vis| {
            let mut n = Vector3::zero();

            if vis.contains(Vis::XP) {
                n.x += 1.0;
            }
            if vis.contains(Vis::XN) {
                n.x -= 1.0;
            }
            if vis.contains(Vis::YP) {
                n.y += 1.0;
            }
            if vis.contains(Vis::YN) {
                n.y -= 1.0;
            }
            if vis.contains(Vis::ZP) {
                n.z += 1.0;
            }
            if vis.contains(Vis::ZN) {
                n.z -= 1.0;
            }

            if n != Vector3::zero() {
                n = n.normalize();
            }

            n
        })
    }

    pub fn coverage(&self) -> Field<f32, 3> {
        self.map("Coverage", |v| {
            let count = v.bits().count_ones();
            count as f32 / 8.0
        })
    }
}

impl<const D: usize> Field<f32, D> {
    pub fn grayscale(&self) -> Field<Vector3<f32>, D> {
        self.map("Grayscale", |f| {
            let f = f.powf(2.2);
            vec3(f, f, f)
        })
    }
}

impl<const D: usize> Field<Vector3<f32>, D> {
    pub fn steepness(&self) -> Field<f32, D> {
        self.map("Steepness", |n| 1.0 - n.dot(vec3(0.0, 0.0, 1.0)))
    }
}

impl<T: Copy + std::ops::AddAssign<T> + std::ops::DivAssign<f32>> Field<T, 3> {
    pub fn smooth(&self, mask: &Field<bool, 3>, env: &Field<Env, 3>) -> Field<T, 3> {
        self.map_with_coordinate("Smooth", |mut v, c| {
            let mut count: usize = 1;
            if mask[c] {
                let env = env[c];
                for (e, o) in [
                    (Env::ZZP, [0, 0, 1]),
                    (Env::ZZN, [0, 0, -1]),
                    (Env::ZPZ, [0, 1, 0]),
                    (Env::ZPP, [0, 1, 1]),
                    (Env::ZPN, [0, 1, -1]),
                    (Env::ZNZ, [0, -1, 0]),
                    (Env::ZNP, [0, -1, 1]),
                    (Env::ZNN, [0, -1, -1]),
                    (Env::PZZ, [1, 0, 0]),
                    (Env::PZP, [1, 0, 1]),
                    (Env::PZN, [1, 0, -1]),
                    (Env::PPZ, [1, 1, 0]),
                    (Env::PPP, [1, 1, 1]),
                    (Env::PPN, [1, 1, -1]),
                    (Env::PNZ, [1, -1, 0]),
                    (Env::PNP, [1, -1, 1]),
                    (Env::PNN, [1, -1, -1]),
                    (Env::NZZ, [-1, 0, 0]),
                    (Env::NZP, [-1, 0, 1]),
                    (Env::NZN, [-1, 0, -1]),
                    (Env::NPZ, [-1, 1, 0]),
                    (Env::NPP, [-1, 1, 1]),
                    (Env::NPN, [-1, 1, -1]),
                    (Env::NNZ, [-1, -1, 0]),
                    (Env::NNP, [-1, -1, 1]),
                    (Env::NNN, [-1, -1, -1]),
                ] {
                    if env.contains(e) {
                        let volume = self.extent.pow(3) as isize;
                        let co = [
                            (c[0] as isize + o[0]).clamp(0, volume - 1) as usize,
                            (c[1] as isize + o[1]).clamp(0, volume - 1) as usize,
                            (c[2] as isize + o[2]).clamp(0, volume - 1) as usize,
                        ];
                        if mask[co] {
                            v += self[co];
                            count += 1;
                        }
                    }
                }
                v /= count as f32
            }
            v
        })
    }
}
