use std::time::Instant;

use cgmath::{vec3, Vector3};
use noise::{NoiseFn, Seedable};

use crate::{
    debug::DebugLines,
    mesh::Mesh,
    raster::{self, Raster, Visibility, WIDTH},
    renderer::Renderer,
    transform::Transform,
    util::rescale,
};

pub struct World {
    pub mesh: Mesh,
    pub transform: Transform,
    pub debug_lines: DebugLines,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        let debug_lines = DebugLines::default();

        let mut noise = noise::Fbm::new();
        noise.frequency = 0.01;
        let noise = noise::Turbulence::new(noise);

        let raster = raster::height_map(|v| {
            let mut n = noise.get([v.x, v.y]);
            n = rescale(n, -1.0..1.0, 0.0..1.0);
            n -= 0.3;
            n.powf(1.5)

            // v.x / WIDTH as f64
        });

        // let colors = raster.colored();
        // let colors = colors.map(|n| n.map(|x| rescale(x, -1.0..1.0, 0.0..1.0)));

        let env = raster.environment();
        let vis = raster.visibility(&env);
        let shell = raster.shell(&env);

        let colors = vis
            .normals()
            // .map(|n| n.map(|x| rescale(x, -1.0..1.0, 0.0..1.0)))
            .steepness()
            .smooth(&shell, &env)
            .smooth(&shell, &env)
            .smooth(&shell, &env)
            .grayscale();

        let mesh = Mesh::new(renderer, &raster, &colors);
        World {
            mesh,
            transform: Transform::default(),
            debug_lines,
        }
    }

    #[allow(unused)]
    pub fn integrate(&mut self) {}
}
