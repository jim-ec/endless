use std::collections::HashMap;

use cgmath::{vec3, Vector3};
use noise::NoiseFn;

use crate::{field::Field, renderer::voxels::VoxelMesh, util::rescale};

pub const K: usize = 6;
pub const N: usize = 1 << K;

#[derive(Default)]
pub struct World {
    pub chunks: HashMap<Vector3<isize>, Chunk>,
}

pub struct Chunk {
    pub lod: usize,
    pub voxel_mesh: VoxelMesh,
}

impl Chunk {
    pub fn new(key: Vector3<isize>, lod: usize, device: &wgpu::Device) -> Self {
        puffin::profile_function!();

        let noise = {
            use noise::{Fbm, Perlin, Turbulence};
            let mut noise = Fbm::<Perlin>::new(0);
            noise.frequency = 0.01;
            Turbulence::<_, Perlin>::new(noise)
        };

        let extent = N >> lod;
        let scale = 1 << lod;

        let offset = N as isize * key.cast().unwrap();

        let height = {
            puffin::profile_scope!("Height");
            Field::new(extent, |[i, j]| {
                // Compute world-space coordinates
                let [x, y, z] = [
                    (i << lod) as f32 + offset.x as f32,
                    (j << lod) as f32 + offset.y as f32,
                    offset.z as f32,
                ];

                let mut n = noise.get([x as f64, y as f64]) as f32;
                n = rescale(n, -1.0..1.0, -0.2..1.0);
                n = n.abs().powf(1.2).copysign(n);
                n *= 50.0;
                n -= z;

                n
            })
        };

        let mask = {
            puffin::profile_scope!("Mask");
            Field::new(extent, |[i, j, k]| {
                // Compute world-space coordinates
                let [_x, _y, z] = [
                    (i << lod) as f32 + offset.x as f32,
                    (j << lod) as f32 + offset.y as f32,
                    (k << lod) as f32 + offset.z as f32,
                ];

                z <= height[[i, j]]
            })
        };

        let env = {
            puffin::profile_scope!("Env");
            mask.environment()
        };
        let shell = {
            puffin::profile_scope!("Shell");
            mask.shell(&env)
        };
        let vis = {
            puffin::profile_scope!("Visibility");
            env.visibility()
        };

        let color = {
            puffin::profile_scope!("Color");
            vis.normals()
                .map(|n| 0.67 * (0.5 * n + vec3(0.5, 0.5, 0.5)))
        };

        let voxel_mesh = {
            puffin::profile_scope!("Voxel Mesh");
            VoxelMesh::new(
                device,
                &shell,
                &vis,
                &color,
                N as f32 * key.cast().unwrap(),
                scale as f32,
            )
        };

        Self { lod, voxel_mesh }
    }
}
