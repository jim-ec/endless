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
            n
            // v.x / WIDTH as f64
        });

        // let colors = raster.colored();

        let env = raster.environment();
        let vis = raster.visibility(&env);
        let colors = vis.normals();
        // let colors = colors.map(|n| n.map(|x| rescale(x, -1.0..1.0, 0.0..1.0)));

        // TODO: Debug coverage, i.e. number of visible faces
        // let colors = vis.map(f)

        // let colors = vis.map(|v| {
        //     let count = v.bits().count_ones();
        //     let x = count as f32 / 8.0;
        //     vec3(x, x, x)
        // });
        // let colors = vis.coverage().grayscale();

        // let colors = raster::elevation().grayscale();

        let mesh = Mesh::new(renderer, &raster.shell(&env), &colors);
        World {
            mesh,
            transform: Transform::default(),
            debug_lines,
        }
    }

    #[allow(unused)]
    pub fn integrate(&mut self) {}
}
