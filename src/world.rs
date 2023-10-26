use std::collections::HashMap;

use cgmath::Vector3;
use noise::NoiseFn;

use crate::{
    field::Field,
    renderer::voxels::VoxelMesh,
    util::{rescale, rgb},
};

pub const K: usize = 6;
pub const N: usize = 1 << K;

#[derive(Default)]
pub struct World {
    pub chunks: HashMap<Vector3<isize>, Chunk>,
}

pub struct Chunk {
    pub lod: usize,
    pub mask: Field<bool, 3>,
    pub color: Field<Vector3<f32>, 3>,
    pub voxel_mesh: VoxelMesh,
}

impl Chunk {
    pub fn new(key: Vector3<isize>, lod: usize, device: &wgpu::Device) -> Self {
        use noise::{Fbm, Perlin, Turbulence};
        let mut noise = Fbm::<Perlin>::new(0);
        noise.frequency = 0.01;
        let noise = Turbulence::<_, Perlin>::new(noise);

        let extent = N >> lod;
        let scale = 1 << lod;

        let offset = N as isize * key.cast().unwrap();

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Sediment {
            Rock,
            Grass,
            Air,
        }

        let height = Field::new(extent, |[i, j]| {
            // Compute world-space coordinates
            let [x, y, z] = [
                (i << lod) as f32 + offset.x as f32,
                (j << lod) as f32 + offset.y as f32,
                offset.z as f32,
            ];

            let mut n = noise.get([x as f64, y as f64]) as f32;
            n = rescale(n, -1.0..1.0, -0.2..1.0);
            n = n.abs().powf(1.5).copysign(n);
            n *= 50.0;
            n -= z;

            n
        });

        let blurred_normal = height.normal().map(|v| v.z).blur(3.0);

        let sediments = Field::new(extent, |[i, j, k]| {
            // Compute world-space coordinates
            let [x, y, z] = [
                (i << lod) as f32 + offset.x as f32,
                (j << lod) as f32 + offset.y as f32,
                (k << lod) as f32 + offset.z as f32,
            ];

            let rock = height[[i, j]];
            let grass = (3.0 * blurred_normal[[i, j]])
                * rescale(noise.get([x as f64, y as f64]) as f32, -1.0..1.0, 0.5..1.0);

            if z <= rock.ceil() {
                Sediment::Rock
            } else if z <= rock.ceil() + grass {
                Sediment::Grass
            } else {
                Sediment::Air
            }
        });

        let mask = Field::new(extent, |[x, y, z]| sediments[[x, y, z]] != Sediment::Air);

        let color = sediments.map(|s| match s {
            Sediment::Rock => rgb(50, 40, 50),
            Sediment::Grass => rgb(120, 135, 5),
            Sediment::Air => rgb(0, 0, 0),
        });

        let env = mask.environment();
        let shell = mask.shell(&env);
        let vis = env.visibility();
        let voxel_mesh = VoxelMesh::new(
            device,
            &shell,
            &vis,
            &color,
            N as f32 * key.cast().unwrap(),
            scale as f32,
        );

        Self {
            lod,
            mask,
            color,
            voxel_mesh,
        }
    }
}
