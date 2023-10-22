use noise::NoiseFn;

use crate::{
    field,
    gizmo_pass::GizmoPass,
    renderer::Renderer,
    util::{rescale, rgb},
    voxel_pass::{VoxelMesh, VoxelPipeline},
};

pub const N: usize = 64;

pub struct World {
    pub voxel_pipeline: VoxelPipeline,
    pub voxel_mesh: VoxelMesh,
    pub gizmo_pass: GizmoPass,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        use noise::{Fbm, Perlin, Turbulence};
        let mut noise = Fbm::<Perlin>::new(0);
        noise.frequency = 0.01;
        let noise = Turbulence::<_, Perlin>::new(noise);

        println!("Voxels: {}x{}x{} = {}", N, N, N, N.pow(3));

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Sediment {
            Rock,
            Soil,
            Sand,
            Air,
        }

        use field::Field;

        let rock_height_map: Field<f32, 2> = Field::new("Rock Height Map", N, |[x, y]| {
            let mut n = noise.get([x as f64, y as f64]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..1.0);
            n = n.powf(2.0);
            n
        });
        let soil_height_map: Field<f32, 2> = Field::new("Soil Height Map", N, |[x, y]| {
            let mut n = noise.get([x as f64 + 20.0, y as f64 + 20.0]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.3);
            n
        });
        let sand_height_map: Field<f32, 2> = Field::new("Sand Height Map", N, |[x, y]| {
            let mut n = noise.get([x as f64 + 80.0, y as f64 + 80.0]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.2);
            n
        });

        let sediments: Field<Sediment, 3> = Field::new("Sediments", N, |[x, y, z]| {
            let rock = (rock_height_map[[x, y]] * N as f32) as usize;
            let soil = (soil_height_map[[x, y]] * N as f32) as usize;
            let sand = (sand_height_map[[x, y]] * N as f32) as usize;

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

        let field: Field<bool, 3> = Field::new("Field", N, |[x, y, z]| {
            let rock = (rock_height_map[[x, y]] * N as f32) as usize;
            let soil = (soil_height_map[[x, y]] * N as f32) as usize;
            let sand = (sand_height_map[[x, y]] * N as f32) as usize;
            z < rock + soil + sand
        });

        let color = sediments.map("Color", |s| match s {
            Sediment::Rock => rgb(40, 40, 50),
            Sediment::Soil => rgb(100, 40, 20),
            Sediment::Sand => rgb(194, 150, 80),
            Sediment::Air => rgb(0, 0, 0),
        });

        World {
            voxel_pipeline: VoxelPipeline::new(renderer),
            voxel_mesh: VoxelMesh::new(renderer, &field, &color),
            gizmo_pass: GizmoPass::new(renderer),
        }
    }
}
