use cgmath::vec3;
use noise::NoiseFn;

use crate::{
    gizmo_pass::GizmoPass,
    grid::{self, N},
    renderer::Renderer,
    util::{rescale, rgb},
    voxel_pass::VoxelPass,
};

pub struct World {
    pub voxel_pass: VoxelPass,
    pub gizmo_pass: GizmoPass,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        use noise::{Fbm, Perlin, Turbulence};
        let mut noise = Fbm::<Perlin>::new(0);
        noise.frequency = 0.01;
        let noise = Turbulence::<_, Perlin>::new(noise);

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Sediment {
            Rock,
            Soil,
            Sand,
            Air,
        }

        let rock_height_map: grid::Grid<f32, 2> = grid::Grid::generate("Height map", |[x, y]| {
            let mut n = noise.get([x as f64, y as f64]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..1.0);
            n = n.powf(2.0);
            n
        });
        let soil_height_map: grid::Grid<f32, 2> = grid::Grid::generate("Height map", |[x, y]| {
            let mut n = noise.get([x as f64 + 20.0, y as f64 + 20.0]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.3);
            n
        });
        let sand_height_map: grid::Grid<f32, 2> = grid::Grid::generate("Height map", |[x, y]| {
            let mut n = noise.get([x as f64 + 80.0, y as f64 + 80.0]) as f32;
            n = rescale(n, -1.0..1.0, 0.1..0.2);
            n
        });

        let sediments: grid::Grid<Sediment, 3> = grid::Grid::generate("Sediments", |[x, y, z]| {
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

        let grid = sediments.map("Occupancy", |s| !matches!(s, Sediment::Air));

        let env = grid.environment();

        let shell = grid.shell(&env);

        let color = sediments.map("Color", |s| match s {
            Sediment::Rock => rgb(40, 40, 50),
            Sediment::Soil => rgb(100, 40, 20),
            Sediment::Sand => rgb(194, 150, 80),
            Sediment::Air => rgb(0, 0, 0),
        });

        let mut gizmo_pass = GizmoPass::new(renderer);
        gizmo_pass.aabb(vec3(0.0, 0.0, 0.0), vec3(N as f32, N as f32, N as f32));

        World {
            voxel_pass: VoxelPass::new(renderer, &shell, &color),
            gizmo_pass,
        }
    }
}
