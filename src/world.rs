use std::time::Instant;

use cgmath::{vec3, InnerSpace, MetricSpace, Vector3};
use noise::{NoiseFn, Seedable};

use crate::{
    debug::DebugLines,
    mesh::Mesh,
    raster::{self, Raster, Vis, HEIGHT, WIDTH},
    renderer::Renderer,
    transform::Transform,
    util::{rescale, rgb},
};

pub struct World {
    pub mesh: Mesh,
    pub transform: Transform,
    pub debug_lines: DebugLines,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        let mut debug_lines = DebugLines::default();

        let mut noise = noise::Fbm::new();
        noise.frequency = 0.01;
        let noise = noise::Turbulence::new(noise);

        let raster = raster::height_map(|v| {
            let mut n = noise.get([v.x - 10.0, v.y - 10.0]);
            n = rescale(n, -1.0..1.0, 0.0..1.0);
            n = n.powf(3.0);
            n -= 0.1;
            n.max(1.0 / HEIGHT as f64)
        });

        let env = raster.environment();

        let shell = raster
            .shell(&env)
            .map_with_coordinate(|b, (_, _, z)| b || z == 0);

        let vis = raster.visibility(&env);
        let normal = vis.normals();
        let steepness = normal
            .steepness()
            .smooth(&shell, &env)
            .smooth(&shell, &env)
            .smooth(&shell, &env);
        let elevation = raster::elevation();

        let color = steepness.map_with_coordinate(|s, (x, y, z)| {
            let water = rgb(0, 50, 255);
            let sand = rgb(194, 150, 80);
            let grass = rgb(120, 135, 5);
            let snow = rgb(200, 200, 200);
            let rock = rgb(40, 40, 50);

            if z == 0 {
                return water;
            } else if z <= 2 {
                return sand;
            }

            let e = elevation[(x, y, z)];

            if s > 0.7 {
                return rock;
            }

            if e > 0.5 {
                snow
            } else {
                grass
            }
        });

        World {
            mesh: Mesh::new(renderer, &shell, &color),
            transform: Transform::default(),
            debug_lines,
        }
    }

    #[allow(unused)]
    pub fn integrate(&mut self) {}
}
