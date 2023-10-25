#![allow(dead_code)]

mod camera;
mod field;
mod renderer;
mod symmetry;
mod util;
mod world;

use cgmath::{InnerSpace, Vector3};
use lerp::Lerp;
use pollster::FutureExt;
use renderer::{voxels::Voxels, RenderJob};
use std::time::{Duration, Instant};
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
        // .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut renderer = renderer::Renderer::new(&window).await;
    let mut world = util::profile("World generation", || world::World::new(&mut renderer));
    println!("Triangle count: {}", renderer.triangle_count);

    let mut camera = camera::Camera::initial();
    let mut w_down = false;
    let mut s_down = false;
    let mut a_down = false;
    let mut d_down = false;
    let mut shift_down = false;
    let mut alt_down = false;

    let mut egui_renderer = egui_wgpu::Renderer::new(
        &renderer.device,
        renderer.config.format,
        Some(renderer::DEPTH_FORMAT),
        1,
    );

    let ctx = egui::Context::default();
    let mut events = vec![];

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
                renderer.resize(size.width, size.height);
                window.request_redraw();
            }

            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(delta),
                ..
            } => {
                camera.yaw += camera.fovy * camera.up().z.signum() * 0.00008 * delta.x as f32;
                camera.pitch += camera.fovy * 0.00008 * delta.y as f32;
            }

            WindowEvent::CursorMoved { position, .. } => {
                events.push(egui::Event::PointerMoved(egui::pos2(
                    position.x as f32 / window.scale_factor() as f32,
                    position.y as f32 / window.scale_factor() as f32,
                )));
            }

            WindowEvent::MouseInput { button, state, .. } => {
                let Some(pos) = events
                    .iter()
                    .rev()
                    .filter_map(|e| match e {
                        &egui::Event::PointerMoved(pos) => Some(pos),
                        _ => None,
                    })
                    .next()
                else {
                    return;
                };
                events.push(egui::Event::PointerButton {
                    pos,
                    button: match button {
                        MouseButton::Left => egui::PointerButton::Primary,
                        MouseButton::Right => egui::PointerButton::Secondary,
                        MouseButton::Middle => egui::PointerButton::Middle,
                        MouseButton::Other(_) => return,
                    },
                    pressed: state == ElementState::Pressed,
                    modifiers: Default::default(),
                })
            }

            WindowEvent::TouchpadMagnify { delta, .. } => {
                camera.fovy *= 1.0 + 0.5 * -delta as f32;
                camera.fovy = camera.fovy.min(180.0);
            }

            WindowEvent::SmartMagnify { .. } => {
                camera.fovy = camera::Camera::initial().fovy;
            }

            _ => {}
        },

        Event::RedrawRequested(..) => {
            last_render_time = Instant::now();

            let mut translation = Vector3::new(0.0, 0.0, 0.0);
            if w_down && !alt_down {
                translation += camera.forward();
            }
            if w_down && alt_down {
                translation.z += camera.up().z.signum();
            }
            if s_down && !alt_down {
                translation -= camera.forward();
            }
            if s_down && alt_down {
                translation.z -= camera.up().z.signum();
            }
            if a_down {
                translation += camera.left();
            }
            if d_down {
                translation -= camera.left();
            }
            if translation.magnitude2() > 0.0 {
                let speed = if shift_down { 500.0 } else { 100.0 };
                camera.translation += FRAME_TIME * speed * translation.normalize_to(1.0);
            }

            world.voxels.camera.lerp_to(camera, 0.5);
            world.gizmos.camera.lerp_to(camera, 0.5);

            let mut jobs: Vec<&dyn RenderJob> = vec![];
            jobs.push(&world.gizmos);
            let mut voxel_passes = vec![];
            jobs.push(&world.gizmos);
            for chunk in world.chunks.values() {
                voxel_passes.push(Voxels(&world.voxels, &chunk.mesh));
            }
            jobs.extend(voxel_passes.iter().map(|p| p as &dyn RenderJob));

            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(renderer.config.width as f32, renderer.config.height as f32),
                )),
                pixels_per_point: Some(window.scale_factor() as f32),
                events: std::mem::take(&mut events),
                ..Default::default()
            };

            let output: egui::FullOutput = ctx.run(input, |ctx| {
                egui::Window::new("")
                    .title_bar(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        #[cfg(debug_assertions)]
                        ui.label(egui::RichText::new("Debug Build").strong());
                        #[cfg(not(debug_assertions))]
                        ui.label(egui::RichText::new("Release Build").strong());

                        ui.add(egui::Slider::new(&mut camera.fovy, 1.0..=180.0).text("FoV"));
                    });
            });

            window.set_cursor_icon(match output.platform_output.cursor_icon {
                egui::CursorIcon::Default => winit::window::CursorIcon::Default,
                egui::CursorIcon::None => winit::window::CursorIcon::Default,
                egui::CursorIcon::ContextMenu => winit::window::CursorIcon::ContextMenu,
                egui::CursorIcon::Help => winit::window::CursorIcon::Help,
                egui::CursorIcon::PointingHand => winit::window::CursorIcon::Hand,
                egui::CursorIcon::Progress => winit::window::CursorIcon::Progress,
                egui::CursorIcon::Wait => winit::window::CursorIcon::Wait,
                egui::CursorIcon::Cell => winit::window::CursorIcon::Cell,
                egui::CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
                egui::CursorIcon::Text => winit::window::CursorIcon::Text,
                egui::CursorIcon::VerticalText => winit::window::CursorIcon::VerticalText,
                egui::CursorIcon::Alias => winit::window::CursorIcon::Alias,
                egui::CursorIcon::Copy => winit::window::CursorIcon::Copy,
                egui::CursorIcon::Move => winit::window::CursorIcon::Move,
                egui::CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
                egui::CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
                egui::CursorIcon::Grab => winit::window::CursorIcon::Grab,
                egui::CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
                egui::CursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
                egui::CursorIcon::ResizeHorizontal => winit::window::CursorIcon::EwResize,
                egui::CursorIcon::ResizeNeSw => winit::window::CursorIcon::NeswResize,
                egui::CursorIcon::ResizeNwSe => winit::window::CursorIcon::NwseResize,
                egui::CursorIcon::ResizeVertical => winit::window::CursorIcon::NsResize,
                egui::CursorIcon::ResizeEast => winit::window::CursorIcon::EResize,
                egui::CursorIcon::ResizeSouthEast => winit::window::CursorIcon::SeResize,
                egui::CursorIcon::ResizeSouth => winit::window::CursorIcon::SResize,
                egui::CursorIcon::ResizeSouthWest => winit::window::CursorIcon::SwResize,
                egui::CursorIcon::ResizeWest => winit::window::CursorIcon::WResize,
                egui::CursorIcon::ResizeNorthWest => winit::window::CursorIcon::NwResize,
                egui::CursorIcon::ResizeNorth => winit::window::CursorIcon::NResize,
                egui::CursorIcon::ResizeNorthEast => winit::window::CursorIcon::NeResize,
                egui::CursorIcon::ResizeColumn => winit::window::CursorIcon::ColResize,
                egui::CursorIcon::ResizeRow => winit::window::CursorIcon::RowResize,
                egui::CursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
                egui::CursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
            });

            for (id, delta) in &output.textures_delta.set {
                egui_renderer.update_texture(&renderer.device, &renderer.queue, *id, delta);
            }
            for id in &output.textures_delta.free {
                egui_renderer.free_texture(id);
            }

            let tris = ctx.tessellate(output.shapes);

            match renderer.render(
                &jobs,
                &mut egui_renderer,
                &tris,
                window.scale_factor() as f32,
            ) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    renderer.resize(renderer.config.width, renderer.config.height);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(wgpu::SurfaceError::Timeout) | Err(wgpu::SurfaceError::Outdated) => (),
            }

            // TODO: Remove
            // renderer.render_(|device, queue| {});
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
