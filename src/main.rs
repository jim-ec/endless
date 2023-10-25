#![allow(dead_code)]

mod camera;
mod field;
mod renderer;
mod symmetry;
mod util;
mod world;

use cgmath::{vec3, InnerSpace, Vector3};
use pollster::FutureExt;
use std::f32::consts::TAU;
use std::sync::mpsc;
use std::thread;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use world::{Chunk, N};

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

    let mut camera = camera::Camera::initial();
    let mut w_down = false;
    let mut s_down = false;
    let mut a_down = false;
    let mut d_down = false;
    let mut shift_down = false;
    let mut alt_down = false;

    let mut events = vec![];

    let (task_sender, task_receiver) = mpsc::channel::<(Vector3<isize>, usize)>();
    let (chunk_sender, chunk_receiver) = mpsc::channel::<(Vector3<isize>, Chunk)>();
    let mut world = world::World::default();
    let mut ordered_chunks = HashSet::<(Vector3<isize>, usize)>::new();

    {
        let device = renderer.device.clone();
        thread::spawn(move || {
            while let Ok((key, lod)) = task_receiver.recv() {
                let chunk = world::Chunk::new(key, lod, &device);
                chunk_sender.send((key, chunk)).unwrap();
            }
        });
    }

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
                camera.yaw += camera.fovy * camera.up().z.signum() * 0.00008 * -delta.x as f32;
                camera.pitch += camera.fovy * 0.00008 * -delta.y as f32;
                camera.pitch = camera.pitch.clamp(-0.25 * TAU, 0.25 * TAU);
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

            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(renderer.config.width as f32, renderer.config.height as f32),
                )),
                pixels_per_point: Some(window.scale_factor() as f32),
                events: std::mem::take(&mut events),
                ..Default::default()
            };

            let ui_output = renderer.ctx().run(input, |ctx| {
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

            window.set_cursor_icon(match ui_output.platform_output.cursor_icon {
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

            let camera_index: Vector3<isize> = (camera.translation / world::N as f32)
                .map(f32::floor)
                .cast()
                .unwrap();

            let lod_shift = 1;
            let radius = 2;

            let mut required_chunks = HashSet::new();
            for x in -radius..=radius {
                for y in -radius..=radius {
                    for z in 0..=1 {
                        let c = vec3(camera_index.x + x, camera_index.y + y, z);
                        let lod = x.unsigned_abs() + y.unsigned_abs();
                        let lod = lod >> lod_shift;
                        if (N >> lod) > 0 {
                            required_chunks.insert((c, lod));
                        }
                    }
                }
            }

            world.chunks.retain(|&key, chunk| {
                let exists = required_chunks.contains(&(key, chunk.lod));
                if exists {
                    required_chunks.remove(&(key, chunk.lod));
                }
                exists
            });

            // required_chunks.iter().sorted_by_key(|(key, lod)| {});
            // for required_chunk in required_chunks {
            //     let (key, lod) = required_chunk;
            //     world
            //         .chunks
            //         .insert(key, world::Chunk::new(key, lod, &renderer.device));
            // }
            for required_chunk in required_chunks {
                if !ordered_chunks.contains(&required_chunk) {
                    task_sender.send(required_chunk).unwrap();
                    ordered_chunks.insert(required_chunk);
                }
            }
            while let Ok((key, chunk)) = chunk_receiver.try_recv() {
                ordered_chunks.remove(&(key, chunk.lod));
                world.chunks.insert(
                    key,
                    world::Chunk {
                        lod: chunk.lod,
                        mask: chunk.mask,
                        color: chunk.color,
                        voxel_mesh: chunk.voxel_mesh,
                    },
                );
            }

            match renderer.render(
                camera,
                ui_output,
                &world.chunks,
                window.scale_factor() as f32,
            ) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    renderer.resize(renderer.config.width, renderer.config.height);
                }
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
