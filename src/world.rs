use std::time::Instant;

use crate::{
    debug::DebugLines,
    mesh::Mesh,
    raster::{Raster, DIM},
    renderer::Renderer,
    transform::Transform,
};

pub struct World {
    pub mesh: Mesh,
    pub frame: Transform,
    pub debug_lines: DebugLines,
}

impl World {
    pub fn new(renderer: &Renderer) -> World {
        let debug_lines = DebugLines::default();

        let t0 = Instant::now();
        let mut raster = Raster::default();

        raster.populate(|v| {
            let x = v.x - (DIM / 2) as f64;
            let y = v.y - (DIM / 2) as f64;
            v.z <= x * x + y * y
        });

        let t1 = Instant::now();
        let dt = t1 - t0;
        println!("Generation took {dt:?}");

        let mesh = Mesh::new_voxels(renderer, &raster);
        World {
            mesh,
            frame: Transform::default(),
            debug_lines,
        }
    }

    #[allow(unused)]
    pub fn integrate(&mut self) {}
}
