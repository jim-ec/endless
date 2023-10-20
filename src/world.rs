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

        let height_map: grid::Grid<f32, 2> = grid::Grid::generate("Height map", |[x, y]| {
            let mut n = noise.get([x as f64 - 10.0, y as f64 - 10.0]) as f32;
            n = rescale(n, -1.0..1.0, 0.0..1.0);
            n = n.powf(2.0);
            n -= 0.1;
            n
        });

        let grid: grid::Grid<bool, 3> = grid::Grid::generate("Grid", |[x, y, z]| {
            let h = height_map[[x, y]];
            (z as f32) < h * N as f32
        });

        let env = grid.environment();

        let shell = grid.shell(&env);

        let vis = env.visibility();
        let normal = vis.normals();
        let steepness = normal
            .steepness()
            .smooth(&shell, &env)
            .smooth(&shell, &env)
            .smooth(&shell, &env);
        let elevation = grid::elevation();

        let color = steepness.map_with_coordinate("Color", |s, [x, y, z]| {
            let sand = rgb(194, 150, 80);
            let grass = rgb(120, 135, 5);
            let snow = rgb(200, 200, 200);
            let rock = rgb(40, 40, 50);

            if z <= 2 {
                return sand;
            }

            let e = elevation[[x, y, z]];

            if s > 0.7 {
                return rock;
            }

            if e > 0.5 {
                snow
            } else {
                grass
            }
        });

        let mut gizmo_pass = GizmoPass::new(renderer);
        gizmo_pass.aabb(vec3(0.0, 0.0, 0.0), vec3(N as f32, N as f32, N as f32));

        World {
            voxel_pass: VoxelPass::new(renderer, &shell, &color),
            gizmo_pass,
        }
    }
}
