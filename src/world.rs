use std::time::Instant;

use noise::{NoiseFn, Seedable};

use crate::{
    debug::DebugLines, mesh::Mesh, raster::Raster, renderer::Renderer, transform::Transform,
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

        let mut raster = Raster::default();

        let mut noise = noise::Fbm::new();
        noise.frequency = 0.01;
        let noise = noise::Turbulence::new(noise);

        raster.populate_height(|v| {
            let mut n = noise.get([v.x, v.y]);
            n = rescale(n, -1.0..1.0, 0.0..1.0);
            n -= 0.3;
            n
        });

        let mesh = Mesh::new(renderer, &raster.shell());
        World {
            mesh,
            transform: Transform::default(),
            debug_lines,
        }
    }

    #[allow(unused)]
    pub fn integrate(&mut self) {}
}
