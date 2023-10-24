#![allow(dead_code)]

mod camera;
mod field;
mod gizmo_pass;
mod renderer;
mod symmetry;
mod util;
mod voxel_pass;
mod world;

use cgmath::{InnerSpace, Vector3};
use lerp::Lerp;
use pollster::FutureExt;
use renderer::RenderPass;
use std::time::{Duration, Instant};
use voxel_pass::VoxelPass;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const FRAME_TIME: f32 = 1.0 / 60.0;

fn main() {
    run().block_on()
}

async fn run() {
    env_logger::init();
    let mut last_render_time = Instant::now();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("")
        .with_visible(false)
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut renderer = renderer::Renderer::new(&window).await;
    let world = util::profile("World generation", || world::World::new(&mut renderer));
    println!("Triangle count: {}", renderer.triangle_count);

    let mut camera = camera::Camera::initial();
    let mut camera_target = camera;
    let mut w_down = false;
    let mut s_down = false;
    let mut a_down = false;
    let mut d_down = false;
    let mut shift_down = false;
    let mut alt_down = false;

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(winit::event::StartCause::Init) => {
            window.set_visible(true);
        }

        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(code),
                        state,
                        ..
                    },
                ..
            } => {
                *match code {
                    VirtualKeyCode::W => &mut w_down,
                    VirtualKeyCode::S => &mut s_down,
                    VirtualKeyCode::A => &mut a_down,
                    VirtualKeyCode::D => &mut d_down,
                    VirtualKeyCode::LShift => &mut shift_down,
                    VirtualKeyCode::RShift => &mut shift_down,
                    VirtualKeyCode::LAlt => &mut alt_down,
                    VirtualKeyCode::RAlt => &mut alt_down,
                    _ => return,
                } = state == ElementState::Pressed;
            }

            WindowEvent::Resized(size)
            | WindowEvent::ScaleFactorChanged {
                new_inner_size: &mut size,
                ..
            } => {
                renderer.resize(size);
                window.request_redraw();
            }

            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(delta),
                ..
            } => {
                camera_target.yaw += camera.up().z.signum() * 0.005 * delta.x as f32;
                camera_target.pitch += 0.005 * delta.y as f32;
            }

            _ => {}
        },

        Event::RedrawRequested(..) => {
            last_render_time = Instant::now();

            let mut translation = Vector3::new(0.0, 0.0, 0.0);
            if w_down && !alt_down {
                translation += camera_target.forward();
            }
            if w_down && alt_down {
                translation.z += camera.up().z.signum();
            }
            if s_down && !alt_down {
                translation -= camera_target.forward();
            }
            if s_down && alt_down {
                translation.z -= camera.up().z.signum();
            }
            if a_down {
                translation += camera_target.left();
            }
            if d_down {
                translation -= camera_target.left();
            }
            if translation.magnitude2() > 0.0 {
                let speed = if shift_down { 500.0 } else { 100.0 };
                camera_target.translation += FRAME_TIME * speed * translation.normalize_to(1.0);
            }

            camera.translation.lerp_to(camera_target.translation, 0.4);
            camera.yaw.lerp_to(camera_target.yaw, 0.6);
            camera.pitch.lerp_to(camera_target.pitch, 0.6);

            let mut passes: Vec<&dyn RenderPass> = vec![];
            passes.push(&world.gizmo_pass);
            let mut voxel_passes = vec![];
            passes.push(&world.gizmo_pass);
            for chunk in world.chunks.values() {
                voxel_passes.push(VoxelPass(&world.voxel_pipeline, &chunk.mesh));
            }
            passes.extend(voxel_passes.iter().map(|p| p as &dyn RenderPass));

            match renderer.render(&camera, &passes) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(wgpu::SurfaceError::Timeout) | Err(wgpu::SurfaceError::Outdated) => (),
            }
        }

        Event::MainEventsCleared => {
            let target_frametime = Duration::from_secs_f32(FRAME_TIME);
            let time_since_last_frame = last_render_time.elapsed();
            if time_since_last_frame >= target_frametime {
                window.request_redraw();
            } else {
                *control_flow = ControlFlow::WaitUntil(
                    Instant::now() + target_frametime - time_since_last_frame,
                );
            }
        }

        _ => {}
    });
}
