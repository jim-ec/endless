#![allow(unused)]

mod app;
mod camera;
mod debug;
mod mesh;
mod raster;
mod renderer;
mod transform;
mod util;
mod world;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run().await
}
