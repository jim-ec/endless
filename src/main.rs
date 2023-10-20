#![allow(dead_code)]

use pollster::FutureExt;

mod app;
mod camera;
mod grid;
mod render_pass;
mod renderer;
mod transform;
mod util;
mod world;

fn main() {
    app::run().block_on()
}
