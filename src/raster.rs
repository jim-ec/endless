use std::ops::{Add, Index, IndexMut};

use bitflags::bitflags;
use cgmath::{vec2, vec3, InnerSpace, Vector2, Vector3, Zero};
use itertools::Itertools;

use crate::util::{perf, rescale, rgb};

pub const WIDTH: usize = 150;
pub const HEIGHT: usize = 50;

pub const CAPACITY: usize = WIDTH * WIDTH * HEIGHT;

#[derive(Clone)]
pub struct Raster<T> {
    pub voxels: Vec<T>,
}

fn linear(x: usize, y: usize, z: usize) -> usize {
    x * WIDTH * HEIGHT + y * HEIGHT + z
}

impl<T> Index<(usize, usize, usize)> for Raster<T> {
    type Output = T;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        &self.voxels[linear(index.0, index.1, index.2)]
    }
}

impl<T> IndexMut<(usize, usize, usize)> for Raster<T> {
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        &mut self.voxels[linear(index.0, index.1, index.2)]
    }
}

impl<T: Default + Clone> Default for Raster<T> {
    fn default() -> Self {
        Self {
            voxels: vec![T::default(); CAPACITY],
        }
    }
}

pub fn indices() -> impl Iterator<Item = (usize, usize, usize)> {
    (0..WIDTH)
        .cartesian_product((0..WIDTH))
        .cartesian_product(0..HEIGHT)
        .map(|((x, y), z)| (x, y, z))
}

impl<T: Copy> Raster<T> {
    pub fn new(init: T) -> Self {
        Self {
            voxels: vec![init; CAPACITY],
        }
    }

    pub fn generate<F: Fn(usize, usize, usize) -> T>(f: F) -> Self {
        let mut voxels = Vec::with_capacity(CAPACITY);
        for (x, y, z) in indices() {
            voxels.push(f(x, y, z));
        }
        Raster { voxels }
    }

    pub fn map<R: Copy, F: Fn(T) -> R>(&self, f: F) -> Raster<R> {
        let mut voxels = Vec::with_capacity(CAPACITY);
        for (x, y, z) in indices() {
            voxels.push(f(self[(x, y, z)]));
        }
        Raster { voxels }
    }

    pub fn map_with_coordinate<R: Copy, F: Fn(T, usize, usize, usize) -> R>(
        &self,
        f: F,
    ) -> Raster<R> {
        let mut voxels = Vec::with_capacity(CAPACITY);
        for (x, y, z) in indices() {
            voxels.push(f(self[(x, y, z)], x, y, z));
        }
        Raster { voxels }
    }
}

bitflags! {
    pub struct Visibility: u8 {
        const XP = 1 << 0;
        const XN = 1 << 1;
        const YP = 1 << 2;
        const YN = 1 << 3;
        const ZP = 1 << 4;
        const ZN = 1 << 5;
    }
}

bitflags! {
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

pub fn height_map<F: Fn(Vector2<f64>) -> f64>(f: F) -> Raster<bool> {
    let mut voxels = Vec::with_capacity(CAPACITY);

    for (x, y) in (0..WIDTH).cartesian_product((0..WIDTH)) {
        let position = vec2(x as f64 + 0.5, y as f64 + 0.5);

        let h = (f(position).clamp(0.0, 1.0) * HEIGHT as f64) as usize;
        voxels.extend(std::iter::repeat(true).take(h));
        voxels.extend(std::iter::repeat(false).take(HEIGHT - h));
    }

    Raster { voxels }
}

pub fn sediment_layers() -> Raster<Vector3<f32>> {
    Raster::generate(|_, _, z| match z {
        0..=1 => vec3(0.2, 0.2, 1.0),
        2..=4 => rgb(194, 178, 128),
        _ => rgb(91, 135, 49),
    })
}

pub fn elevation() -> Raster<f32> {
    Raster::generate(|_, _, z| z as f32 / HEIGHT as f32)
}

impl Raster<bool> {
    pub fn environment(&self) -> Raster<Env> {
        perf("Env", || {
            self.map_with_coordinate(|b, x, y, z| {
                let mut env = Env::empty();

                let xp = x < WIDTH - 1;
                let xn = x > 0;
                let yp = y < WIDTH - 1;
                let yn = y > 0;
                let zp = z < HEIGHT - 1;
                let zn = z > 0;

                env.set(Env::ZZZ, self[(x, y, z)]);
                env.set(Env::ZZP, zp && self[(x, y, z + 1)]);
                env.set(Env::ZZN, zn && self[(x, y, z - 1)]);
                env.set(Env::ZPZ, yp && self[(x, y + 1, z)]);
                env.set(Env::ZPP, yp && zp && self[(x, y + 1, z + 1)]);
                env.set(Env::ZPN, yp && zn && self[(x, y + 1, z - 1)]);
                env.set(Env::ZNZ, yn && self[(x, y - 1, z)]);
                env.set(Env::ZNP, yn && zp && self[(x, y - 1, z + 1)]);
                env.set(Env::ZNN, yn && zn && self[(x, y - 1, z - 1)]);
                env.set(Env::PZZ, xp && self[(x + 1, y, z)]);
                env.set(Env::PZP, xp && zp && self[(x + 1, y, z + 1)]);
                env.set(Env::PZN, xp && zn && self[(x + 1, y, z - 1)]);
                env.set(Env::PPZ, xp && yp && self[(x + 1, y + 1, z)]);
                env.set(Env::PPP, xp && yp && zp && self[(x + 1, y + 1, z + 1)]);
                env.set(Env::PPN, xp && yp && zn && self[(x + 1, y + 1, z - 1)]);
                env.set(Env::PNZ, xp && yn && self[(x + 1, y - 1, z)]);
                env.set(Env::PNP, xp && yn && zp && self[(x + 1, y - 1, z + 1)]);
                env.set(Env::PNN, xp && yn && zn && self[(x + 1, y - 1, z - 1)]);
                env.set(Env::NZZ, xn && self[(x - 1, y, z)]);
                env.set(Env::NZP, xn && zp && self[(x - 1, y, z + 1)]);
                env.set(Env::NZN, xn && zn && self[(x - 1, y, z - 1)]);
                env.set(Env::NPZ, xn && yp && self[(x - 1, y + 1, z)]);
                env.set(Env::NPP, xn && yp && zp && self[(x - 1, y + 1, z + 1)]);
                env.set(Env::NPN, xn && yp && zn && self[(x - 1, y + 1, z - 1)]);
                env.set(Env::NNZ, xn && yn && self[(x - 1, y - 1, z)]);
                env.set(Env::NNP, xn && yn && zp && self[(x - 1, y - 1, z + 1)]);
                env.set(Env::NNN, xn && yn && zn && self[(x - 1, y - 1, z - 1)]);
                env
            })
        })
    }

    pub fn shell(&self, env: &Raster<Env>) -> Raster<bool> {
        perf("Shell", || {
            self.map_with_coordinate(|b, x, y, z| self[(x, y, z)] && !env[(x, y, z)].is_all())
        })
    }

    pub fn visibility(&self, env: &Raster<Env>) -> Raster<Visibility> {
        perf("Visibility", || {
            self.map_with_coordinate(|vis, x, y, z| {
                let mut vis = Visibility::empty();
                let env = env[(x, y, z)];
                vis.set(Visibility::XP, !env.contains(Env::PZZ));
                vis.set(Visibility::XN, !env.contains(Env::NZZ));
                vis.set(Visibility::YP, !env.contains(Env::ZPZ));
                vis.set(Visibility::YN, !env.contains(Env::ZNZ));
                vis.set(Visibility::ZP, !env.contains(Env::ZZP));
                vis.set(Visibility::ZN, !env.contains(Env::ZZN));
                vis
            })
        })
    }
}

impl Raster<Visibility> {
    pub fn normals(&self) -> Raster<Vector3<f32>> {
        self.map(|vis| {
            let mut n = Vector3::zero();

            if vis.contains(Visibility::XP) {
                n.x += 1.0;
            }
            if vis.contains(Visibility::XN) {
                n.x -= 1.0;
            }
            if vis.contains(Visibility::YP) {
                n.y += 1.0;
            }
            if vis.contains(Visibility::YN) {
                n.y -= 1.0;
            }
            if vis.contains(Visibility::ZP) {
                n.z += 1.0;
            }
            if vis.contains(Visibility::ZN) {
                n.z -= 1.0;
            }

            if n != Vector3::zero() {
                n = n.normalize();
            }

            n
        })
    }

    pub fn coverage(&self) -> Raster<f32> {
        self.map(|v| {
            let count = v.bits().count_ones();
            count as f32 / 8.0
        })
    }
}

impl Raster<f32> {
    pub fn grayscale(&self) -> Raster<Vector3<f32>> {
        self.map(|f| {
            // let f = f.powf(2.2);
            vec3(f, f, f)
        })
    }
}
