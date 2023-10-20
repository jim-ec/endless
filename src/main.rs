#![allow(dead_code)]

use pollster::FutureExt;

mod app;
mod camera;
mod gizmo_pass;
mod grid;
mod renderer;
mod symmetry;
mod util;
mod voxel_pass;
mod world;

fn main() {
    app::run().block_on()
}
