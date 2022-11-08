use cgmath::{vec3, Vector3};
use itertools::Itertools;

pub const DIM: usize = 64;

pub struct Raster {
    pub voxels: [[[bool; DIM]; DIM]; DIM],
}

impl Default for Raster {
    fn default() -> Self {
        Self {
            voxels: [[[false; DIM]; DIM]; DIM],
        }
    }
}

impl Raster {
    pub fn populate<F: Fn(Vector3<f64>) -> bool>(&mut self, f: F) {
        for (z, y, x) in Self::indices() {
            let position = vec3(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5);
            self.voxels[x][y][z] = f(position);
        }
    }

    pub fn indices() -> impl Iterator<Item = (usize, usize, usize)> {
        let iter = 0..DIM;
        iter.clone()
            .cartesian_product(iter.clone())
            .cartesian_product(iter)
            .map(|((x, y), z)| (x, y, z))
    }
}
