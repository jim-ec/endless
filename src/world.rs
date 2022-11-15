use cgmath::{vec3, Vector3, Zero};
use noise::NoiseFn;

use crate::{
    raster::{self, HEIGHT, WIDTH},
    render_pass::{line_pass::LinePass, voxel_pass::VoxelPass, water_pass::WaterPass, RenderPass},
    renderer::Renderer,
    util::{perf, rescale, rgb},
};

pub struct World {
    pub voxel_pass: VoxelPass,
    pub line_pass: LinePass,
    pub water_pass: WaterPass,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        let mut noise = noise::Fbm::new();
        noise.frequency = 0.01 / 8.0;
        let noise = noise::Turbulence::new(noise);

        let raster = raster::height_map(|v| {
            let mut n = noise.get([v.x - 10.0, v.y - 10.0]);
            n = rescale(n, -1.0..1.0, 0.0..1.0);
            n = n.powf(3.0);
            n -= 0.1;
            n *= 2.5;
            n
        });

        let env = raster.environment();

        let shell = raster.shell(&env);

        let vis = env.visibility();
        let normal = vis.normals();
        let steepness = normal
            .steepness()
            .smooth(&shell, &env)
            .smooth(&shell, &env)
            .smooth(&shell, &env);
        let elevation = raster::elevation();

        let color = steepness.map_with_coordinate("Color", |s, (x, y, z)| {
            let sand = rgb(194, 150, 80);
            let grass = rgb(120, 135, 5);
            let snow = rgb(200, 200, 200);
            let rock = rgb(40, 40, 50);

            if z <= 10 {
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
            voxel_pass: VoxelPass::new(renderer, &shell, &color),
            line_pass: LinePass::new(
                renderer,
                [
                    [
                        vec3(0.0, 0.0, 0.0),
                        vec3(WIDTH as f32, 0.0, 0.0),
                        vec3(WIDTH as f32, WIDTH as f32, 0.0),
                        vec3(0.0, WIDTH as f32, 0.0),
                        vec3(0.0, 0.0, 0.0),
                    ]
                    .as_slice()
                    .into_iter()
                    .copied(),
                    [
                        vec3(0.0, 0.0, HEIGHT as f32),
                        vec3(WIDTH as f32, 0.0, HEIGHT as f32),
                        vec3(WIDTH as f32, WIDTH as f32, HEIGHT as f32),
                        vec3(0.0, WIDTH as f32, HEIGHT as f32),
                        vec3(0.0, 0.0, HEIGHT as f32),
                    ]
                    .as_slice()
                    .into_iter()
                    .copied(),
                    [vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, HEIGHT as f32)]
                        .as_slice()
                        .into_iter()
                        .copied(),
                    [
                        vec3(WIDTH as f32, 0.0, 0.0),
                        vec3(WIDTH as f32, 0.0, HEIGHT as f32),
                    ]
                    .as_slice()
                    .into_iter()
                    .copied(),
                    [
                        vec3(0.0, WIDTH as f32, 0.0),
                        vec3(0.0, WIDTH as f32, HEIGHT as f32),
                    ]
                    .as_slice()
                    .into_iter()
                    .copied(),
                    [
                        vec3(WIDTH as f32, WIDTH as f32, 0.0),
                        vec3(WIDTH as f32, WIDTH as f32, HEIGHT as f32),
                    ]
                    .as_slice()
                    .into_iter()
                    .copied(),
                ],
            ),
            water_pass: WaterPass::new(renderer),
        }
    }
}
