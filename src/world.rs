use std::collections::HashMap;

use cgmath::{vec3, Vector3};
use noise::NoiseFn;

use crate::{
    field::Field,
    gizmo_pass::GizmoPass,
    renderer::Renderer,
    util::{rescale, rgb},
    voxel_pass::{VoxelMesh, VoxelPipeline},
};

pub const N: usize = 64;

pub struct World {
    pub voxel_pipeline: VoxelPipeline,
    pub gizmo_pass: GizmoPass,
    pub chunks: HashMap<Vector3<isize>, Chunk>,
}

pub struct Chunk {
    pub field: Field<bool, 3>,
    pub color: Field<Vector3<f32>, 3>,
    pub voxel_mesh: VoxelMesh,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        println!("Voxels: {}x{}x{} = {}", N, N, N, N.pow(3));

        let mut chunks = HashMap::new();

        for x in -1_isize..=1 {
            for y in -1_isize..=1 {
                let c = vec3(x, y, 0);
                let fidelity = x.unsigned_abs() + y.unsigned_abs();
                chunks.insert(c, Chunk::new(renderer, c, fidelity));
            }
        }

        World {
            voxel_pipeline: VoxelPipeline::new(renderer),
            gizmo_pass: GizmoPass::new(renderer),
            chunks,
        }
    }
}

impl Chunk {
    pub fn new(renderer: &Renderer, translation: Vector3<isize>, fidelity: usize) -> Self {
        use noise::{Fbm, Perlin, Turbulence};
        let mut noise = Fbm::<Perlin>::new(0);
        noise.frequency = 0.01;
        let noise = Turbulence::<_, Perlin>::new(noise);

        let extent = N >> fidelity;
        let scale = 1 << fidelity;

        let t = N as f64
            * vec3(
                translation.x as f64,
                translation.y as f64,
                translation.z as f64,
            );

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Sediment {
            Rock,
            Soil,
            Sand,
            Air,
        }

        let rock_height_map: Field<f32, 2> = Field::new("Rock Height Map", extent, |[x, y]| {
            let mut n =
                noise.get([scale as f64 * x as f64 + t.x, scale as f64 * y as f64 + t.y]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..1.0);
            n = n.powf(2.0);
            n
        });
        let soil_height_map: Field<f32, 2> = Field::new("Soil Height Map", extent, |[x, y]| {
            let mut n = noise.get([
                scale as f64 * x as f64 + t.x + 20.0,
                scale as f64 * y as f64 + t.y + 20.0,
            ]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.3);
            n
        });
        let sand_height_map: Field<f32, 2> = Field::new("Sand Height Map", extent, |[x, y]| {
            let mut n = noise.get([
                scale as f64 * x as f64 + t.x + 80.0,
                scale as f64 * y as f64 + t.y + 80.0,
            ]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.2);
            n
        });

        let sediments: Field<Sediment, 3> = Field::new("Sediments", extent, |[x, y, z]| {
            let rock = (rock_height_map[[x, y]] * extent as f32) as usize;
            let soil = (soil_height_map[[x, y]] * extent as f32) as usize;
            let sand = (sand_height_map[[x, y]] * extent as f32) as usize;

            if z < rock {
                Sediment::Rock
            } else if z < rock + soil {
                Sediment::Soil
            } else if z < rock + soil + sand {
                Sediment::Sand
            } else {
                Sediment::Air
            }
        });

        let field: Field<bool, 3> = Field::new("Field", extent, |[x, y, z]| {
            let rock = (rock_height_map[[x, y]] * extent as f32) as usize;
            let soil = (soil_height_map[[x, y]] * extent as f32) as usize;
            let sand = (sand_height_map[[x, y]] * extent as f32) as usize;
            z < rock + soil + sand
        });

        let color = sediments.map("Color", |s| match s {
            Sediment::Rock => rgb(40, 40, 50),
            Sediment::Soil => rgb(100, 40, 20),
            Sediment::Sand => rgb(194, 150, 80),
            Sediment::Air => rgb(0, 0, 0),
        });

        let env = field.environment();

        Self {
            voxel_mesh: VoxelMesh::new(
                renderer,
                &field.shell(&env),
                &color,
                N as isize * translation,
                scale,
            ),
            field,
            color,
        }
    }
}
