#![allow(unused)]

mod app;
mod camera;
mod raster;
mod render_pass;
mod renderer;
mod transform;
mod util;
mod world;

#[async_std::main]
async fn main() {
    app::run().await
}
