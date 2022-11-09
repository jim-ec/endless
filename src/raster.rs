use std::ops::{Index, IndexMut};

use cgmath::{vec2, vec3, Vector2, Vector3};
use itertools::Itertools;

use crate::util::perf;

pub const WIDTH: usize = 100;
pub const HEIGHT: usize = 50;

#[derive(Clone)]
pub struct Raster {
    pub voxels: Vec<bool>,
}

impl Index<(usize, usize, usize)> for Raster {
    type Output = bool;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        &self.voxels[index.0 * WIDTH * HEIGHT + index.1 * HEIGHT + index.2]
    }
}

impl IndexMut<(usize, usize, usize)> for Raster {
    fn index_mut(&mut self, index: (usize, usize, usize)) -> &mut Self::Output {
        &mut self.voxels[index.0 * WIDTH * HEIGHT + index.1 * HEIGHT + index.2]
    }
}

impl Default for Raster {
    fn default() -> Self {
        Self {
            voxels: vec![false; WIDTH * WIDTH * HEIGHT],
        }
    }
}

impl Raster {
    pub fn populate<F: Fn(Vector3<f64>) -> bool>(&mut self, f: F) {
        perf("Volume Population", || {
            for (x, y, z) in Self::indices() {
                let position = vec3(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5);
                self[(x, y, z)] = f(position);
            }
        });
    }

    pub fn populate_height<F: Fn(Vector2<f64>) -> f64>(&mut self, f: F) {
        perf("Height Population", || {
            for (x, y) in (0..WIDTH).cartesian_product((0..WIDTH)) {
                let position = vec2(x as f64 + 0.5, y as f64 + 0.5);

                let h = (f(position).clamp(0.0, 1.0) * HEIGHT as f64) as usize;
                for z in 0..h {
                    self[(x, y, z)] = true;
                }
                for z in h..HEIGHT {
                    self[(x, y, z)] = false;
                }
            }
        });
    }

    pub fn indices() -> impl Iterator<Item = (usize, usize, usize)> {
        (0..WIDTH)
            .cartesian_product((0..WIDTH))
            .cartesian_product(0..HEIGHT)
            .map(|((x, y), z)| (x, y, z))
    }

    /// Remove voxels the are completely enclosed.
    pub fn shell(&self) -> Self {
        perf("Shell generation", || {
            let mut pruned = self.clone();
            for x in 1..WIDTH - 1 {
                for y in 1..WIDTH - 1 {
                    for z in 1..HEIGHT - 1 {
                        if self[(x, y, z)]
                            && self[(x, y, z + 1)]
                            && self[(x, y, z - 1)]
                            && self[(x, y + 1, z)]
                            && self[(x, y + 1, z + 1)]
                            && self[(x, y + 1, z - 1)]
                            && self[(x, y - 1, z)]
                            && self[(x, y - 1, z + 1)]
                            && self[(x, y - 1, z - 1)]
                            && self[(x + 1, y, z)]
                            && self[(x + 1, y, z + 1)]
                            && self[(x + 1, y, z - 1)]
                            && self[(x + 1, y + 1, z)]
                            && self[(x + 1, y + 1, z + 1)]
                            && self[(x + 1, y + 1, z - 1)]
                            && self[(x + 1, y - 1, z)]
                            && self[(x + 1, y - 1, z + 1)]
                            && self[(x + 1, y - 1, z - 1)]
                            && self[(x - 1, y, z)]
                            && self[(x - 1, y, z + 1)]
                            && self[(x - 1, y, z - 1)]
                            && self[(x - 1, y + 1, z)]
                            && self[(x - 1, y + 1, z + 1)]
                            && self[(x - 1, y + 1, z - 1)]
                            && self[(x - 1, y - 1, z)]
                            && self[(x - 1, y - 1, z + 1)]
                            && self[(x - 1, y - 1, z - 1)]
                        {
                            pruned[(x, y, z)] = false;
                        }
                    }
                }
            }
            pruned
        })
    }
}
